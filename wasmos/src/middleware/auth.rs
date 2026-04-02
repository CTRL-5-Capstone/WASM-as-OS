#[cfg(feature = "jwt-auth")]
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::sync::Arc;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

pub struct AuthService {
    pub secret: String,
    pub(crate) expiry_hours: i64,
    pub enabled: bool,
}

impl AuthService {
    pub fn new(secret: String, expiry_hours: i64, enabled: bool) -> Self {
        Self { secret, expiry_hours, enabled }
    }

    #[cfg(feature = "jwt-auth")]
    pub fn generate_token(&self, user_id: &str, role: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as usize;
        let claims = Claims {
            sub: user_id.to_string(),
            role: role.to_string(),
            iat: now,
            exp: now + (self.expiry_hours as usize * 3600),
        };
        encode(&Header::default(), &claims, &EncodingKey::from_secret(self.secret.as_bytes()))
    }

    #[cfg(feature = "jwt-auth")]
    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )?;
        Ok(data.claims)
    }

    #[cfg(not(feature = "jwt-auth"))]
    pub fn validate_token(&self, _token: &str) -> Result<Claims, String> {
        Err("JWT feature not compiled".to_string())
    }
}

// ─── Actix middleware ────────────────────────────────────────────────────────

pub struct JwtAuth {
    pub auth_service: Arc<AuthService>,
}

impl JwtAuth {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }
}

impl<S, B> Transform<S, ServiceRequest> for JwtAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = JwtAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JwtAuthMiddleware {
            service: std::rc::Rc::new(service),
            auth_service: self.auth_service.clone(),
        }))
    }
}

pub struct JwtAuthMiddleware<S> {
    service: std::rc::Rc<S>,
    auth_service: Arc<AuthService>,
}

impl<S, B> Service<ServiceRequest> for JwtAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // If auth is disabled, pass through
        if !self.auth_service.enabled {
            let fut = self.service.call(req);
            return Box::pin(async move {
                Ok(fut.await?.map_into_left_body())
            });
        }

        // Allow health, metrics, WebSocket connections, and the auth bootstrap endpoint
        // without a bearer-token.  The auth endpoint is the bootstrap: calling it IS
        // how a client obtains a token, so requiring a token to reach it is a deadlock.
        // Browser WebSocket APIs cannot send custom headers, so /ws is exempted here
        // (capability-token check is enforced inside ws_handler instead).
        let path = req.path().to_string();
        if path.starts_with("/health")
            || path == "/metrics"
            || path.starts_with("/ws")
            || path == "/v1/auth/token"
        {
            let fut = self.service.call(req);
            return Box::pin(async move {
                Ok(fut.await?.map_into_left_body())
            });
        }

        // Extract Bearer token
        let token = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        let auth = self.auth_service.clone();
        let svc = self.service.clone();

        Box::pin(async move {
            match token {
                None => {
                    let resp = HttpResponse::Unauthorized().json(serde_json::json!({
                        "error": "Missing Authorization header",
                        "status": 401
                    }));
                    Ok(ServiceResponse::new(req.into_parts().0, resp).map_into_right_body())
                }
                Some(tok) => {
                    #[cfg(feature = "jwt-auth")]
                    match auth.validate_token(&tok) {
                        Ok(claims) => {
                            req.extensions_mut().insert(claims);
                            Ok(svc.call(req).await?.map_into_left_body())
                        }
                        Err(_) => {
                            let resp = HttpResponse::Unauthorized().json(serde_json::json!({
                                "error": "Invalid or expired token",
                                "status": 401
                            }));
                            Ok(ServiceResponse::new(req.into_parts().0, resp).map_into_right_body())
                        }
                    }
                    #[cfg(not(feature = "jwt-auth"))]
                    {
                        let _ = tok;
                        let resp = HttpResponse::Unauthorized().json(serde_json::json!({
                            "error": "JWT auth not compiled",
                            "status": 401
                        }));
                        Ok(ServiceResponse::new(req.into_parts().0, resp).map_into_right_body())
                    }
                }
            }
        })
    }
}

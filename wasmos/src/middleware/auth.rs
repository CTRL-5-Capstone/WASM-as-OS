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

// ─── In-source tests ─────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test as awtest;
    use actix_web::{web, App, HttpResponse};
    use serde_json::Value;
 
    fn make_service(secret: &str, hours: i64, enabled: bool) -> Arc<AuthService> {
        Arc::new(AuthService::new(secret.into(), hours, enabled))
    }
 
    // ─── AuthService — JWT round-trip (default jwt-auth feature) ────────────
 
    #[cfg(feature = "jwt-auth")]
    #[test]
    fn generate_then_validate_round_trips_claims() {
        let svc = make_service("super-secret-test-key", 1, true);
        let token = svc.generate_token("user-42", "admin").expect("token signs");
        let claims: Claims = svc.validate_token(&token).expect("token validates");
 
        assert_eq!(claims.sub, "user-42");
        assert_eq!(claims.role, "admin");
        assert!(claims.exp > claims.iat, "exp must be after iat");
        assert_eq!(claims.exp - claims.iat, 3600, "1-hour expiry → 3600 seconds");
    }
 
    #[cfg(feature = "jwt-auth")]
    #[test]
    fn validate_token_rejects_tampered_signature() {
        let svc = make_service("super-secret-test-key", 1, true);
        let token = svc.generate_token("u", "admin").expect("signs");
 
        let mut bad = token.clone();
        let last = bad.pop().unwrap();
        bad.push(if last == 'a' { 'b' } else { 'a' });
 
        assert!(svc.validate_token(&bad).is_err());
    }
 
    #[cfg(feature = "jwt-auth")]
    #[test]
    fn validate_token_rejects_token_signed_with_a_different_secret() {
        let svc_a = make_service("secret-A", 1, true);
        let svc_b = make_service("secret-B", 1, true);
 
        let token = svc_a.generate_token("u", "admin").expect("signs");
        assert!(svc_b.validate_token(&token).is_err(),
            "a token signed with secret-A must not validate under secret-B");
    }
 
    #[cfg(feature = "jwt-auth")]
    #[test]
    fn validate_token_rejects_garbage_input() {
        let svc = make_service("secret", 1, true);
        assert!(svc.validate_token("not-a-jwt").is_err());
        assert!(svc.validate_token("").is_err());
    }
 
    // ─── JwtAuth middleware tests ───────────────────────────────────────────
 
    async fn protected_handler(req: actix_web::HttpRequest) -> HttpResponse {
        use actix_web::HttpMessage;
        let role = req
            .extensions()
            .get::<Claims>()
            .map(|c| c.role.clone())
            .unwrap_or_default();
        HttpResponse::Ok().json(serde_json::json!({ "role": role }))
    }
 
    async fn ok_handler() -> HttpResponse {
        HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }))
    }
 
    #[actix_web::test]
    async fn middleware_passes_through_when_disabled() {
        let svc = make_service("secret", 1, /*enabled=*/ false);
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc))
                .route("/protected", web::get().to(protected_handler)),
        ).await;
 
        let req = awtest::TestRequest::get().uri("/protected").to_request();
        let resp = awtest::call_service(&app, req).await;
        assert!(resp.status().is_success(),
            "with auth disabled, protected routes should return 200; got {}",
            resp.status());
    }
 
    #[actix_web::test]
    async fn middleware_exempts_health_endpoints() {
        let svc = make_service("secret", 1, true);
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc))
                .route("/health/live", web::get().to(ok_handler))
                .route("/health/ready", web::get().to(ok_handler)),
        ).await;
 
        for path in ["/health/live", "/health/ready"] {
            let req = awtest::TestRequest::get().uri(path).to_request();
            let resp = awtest::call_service(&app, req).await;
            assert!(resp.status().is_success(),
                "{path} must be exempt from JWT — got {}", resp.status());
        }
    }
 
    #[actix_web::test]
    async fn middleware_exempts_metrics() {
        let svc = make_service("secret", 1, true);
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc))
                .route("/metrics", web::get().to(ok_handler)),
        ).await;
 
        let req = awtest::TestRequest::get().uri("/metrics").to_request();
        let resp = awtest::call_service(&app, req).await;
        assert!(resp.status().is_success(),
            "Prometheus scrape must not require a token");
    }
 
    #[actix_web::test]
    async fn middleware_exempts_websocket_paths() {
        // Browsers can't set Authorization headers on WebSocket upgrades.
        let svc = make_service("secret", 1, true);
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc))
                .route("/ws", web::get().to(ok_handler))
                .route("/ws/events", web::get().to(ok_handler)),
        ).await;
 
        for path in ["/ws", "/ws/events"] {
            let req = awtest::TestRequest::get().uri(path).to_request();
            let resp = awtest::call_service(&app, req).await;
            assert!(resp.status().is_success(),
                "{path} must be exempt for WebSocket compatibility");
        }
    }
 
    #[actix_web::test]
    async fn middleware_exempts_auth_bootstrap_endpoint() {
        // /v1/auth/token IS the bootstrap — requiring a token would deadlock.
        let svc = make_service("secret", 1, true);
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc))
                .route("/v1/auth/token", web::post().to(ok_handler)),
        ).await;
 
        let req = awtest::TestRequest::post().uri("/v1/auth/token").to_request();
        let resp = awtest::call_service(&app, req).await;
        assert!(resp.status().is_success(), "/v1/auth/token must be exempt");
    }
 
    #[actix_web::test]
    async fn middleware_returns_401_when_authorization_header_missing() {
        let svc = make_service("secret", 1, true);
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc))
                .route("/protected", web::get().to(protected_handler)),
        ).await;
 
        let req = awtest::TestRequest::get().uri("/protected").to_request();
        let resp = awtest::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
 
        let body: Value = awtest::read_body_json(resp).await;
        assert_eq!(body["status"], 401);
        assert!(body["error"].as_str().unwrap_or("").to_lowercase().contains("missing"));
    }
 
    #[actix_web::test]
    async fn middleware_returns_401_on_invalid_bearer_token() {
        let svc = make_service("secret", 1, true);
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc))
                .route("/protected", web::get().to(protected_handler)),
        ).await;
 
        let req = awtest::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", "Bearer not-a-real-jwt"))
            .to_request();
 
        let resp = awtest::call_service(&app, req).await;
        assert_eq!(resp.status(), 401);
 
        let body: Value = awtest::read_body_json(resp).await;
        assert_eq!(body["status"], 401);
    }
 
    #[cfg(feature = "jwt-auth")]
    #[actix_web::test]
    async fn middleware_returns_401_when_header_lacks_bearer_prefix() {
        let svc = make_service("secret", 1, true);
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc.clone()))
                .route("/protected", web::get().to(protected_handler)),
        ).await;
 
        let token = svc.generate_token("u", "admin").unwrap();
 
        let req = awtest::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", token.as_str()))
            .to_request();
 
        let resp = awtest::call_service(&app, req).await;
        assert_eq!(resp.status(), 401, "Authorization without Bearer prefix → 401");
    }
 
    #[cfg(feature = "jwt-auth")]
    #[actix_web::test]
    async fn middleware_passes_through_with_valid_bearer_token_and_injects_claims() {
        let svc = make_service("test-secret", 1, true);
        let token = svc.generate_token("alice", "admin").unwrap();
 
        let app = awtest::init_service(
            App::new()
                .wrap(JwtAuth::new(svc))
                .route("/protected", web::get().to(protected_handler)),
        ).await;
 
        let req = awtest::TestRequest::get()
            .uri("/protected")
            .insert_header(("Authorization", format!("Bearer {token}")))
            .to_request();
 
        let resp = awtest::call_service(&app, req).await;
        assert!(resp.status().is_success(), "valid token → 200; got {}", resp.status());
 
        let body: Value = awtest::read_body_json(resp).await;
        assert_eq!(body["role"], "admin");
    }
}
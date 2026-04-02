use actix_web::{
    body::EitherBody,
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpResponse,
};
use futures_util::future::LocalBoxFuture;
use governor::{
    clock::DefaultClock,
    state::keyed::DefaultKeyedStateStore,
    Quota, RateLimiter as GovernorRateLimiter,
};
use std::future::{ready, Ready};
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

/// Per-IP rate limiter — each unique client IP gets its own token bucket.
pub struct RateLimiter {
    limiter: Arc<GovernorRateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u32) -> Self {
        let quota = Quota::per_minute(NonZeroU32::new(requests_per_minute).unwrap());
        let limiter = Arc::new(GovernorRateLimiter::keyed(quota));
        Self { limiter }
    }
}

impl<S, B> Transform<S, ServiceRequest> for RateLimiter
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = RateLimiterMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RateLimiterMiddleware {
            service,
            limiter: self.limiter.clone(),
        }))
    }
}

pub struct RateLimiterMiddleware<S> {
    service: S,
    limiter: Arc<GovernorRateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>>,
}

impl<S, B> Service<ServiceRequest> for RateLimiterMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        // Health probes and Prometheus scrapes must never be rate-limited —
        // monitoring systems rely on them being reliably reachable even under load.
        let path = req.path().to_string();
        if path.starts_with("/health") || path == "/metrics" {
            let fut = self.service.call(req);
            return Box::pin(async move {
                let res = fut.await?;
                Ok(res.map_into_left_body())
            });
        }

        // Use the TCP peer address as the rate-limit key.
        // X-Forwarded-For is intentionally NOT trusted here because it can be
        // trivially spoofed by any client, allowing unlimited bypass of the limiter.
        // If you run behind a trusted reverse proxy (nginx, cloudflare, etc.) that
        // you control, set WASMOS__SERVER__TRUSTED_PROXY_IP and move the XFF check
        // behind that guard — but the default must be the conservative/safe option.
        let ip: IpAddr = req
            .peer_addr()
            .map(|a| a.ip())
            .unwrap_or_else(|| "0.0.0.0".parse().unwrap());

        if self.limiter.check_key(&ip).is_err() {
            return Box::pin(async move {
                let response = HttpResponse::TooManyRequests()
                    .json(serde_json::json!({
                        "error": "Rate limit exceeded",
                        "status": 429
                    }));
                Ok(ServiceResponse::new(req.into_parts().0, response).map_into_right_body())
            });
        }

        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            Ok(res.map_into_left_body())
        })
    }
}


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


// ─── In-source tests ─────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test as awtest;
    use actix_web::{web, App, HttpResponse};
    use serde_json::Value;
 
    async fn ok_handler() -> HttpResponse {
        HttpResponse::Ok().json(serde_json::json!({ "ok": true }))
    }
 
    // Notes: actix_web::test::TestRequest does not supply a peer_addr by
    // default, so every test request appears to come from "0.0.0.0" — they
    // all share a single bucket. That's exactly what we want here for
    // deterministic budget assertions.
 
    #[actix_web::test]
    async fn limiter_allows_requests_within_budget() {
        let app = awtest::init_service(
            App::new()
                .wrap(RateLimiter::new(60))
                .route("/api", web::get().to(ok_handler)),
        ).await;
 
        let req = awtest::TestRequest::get().uri("/api").to_request();
        let resp = awtest::call_service(&app, req).await;
        assert!(resp.status().is_success(), "first request within budget should pass");
    }
 
    #[actix_web::test]
    async fn limiter_returns_429_with_structured_body_once_budget_exhausted() {
        // 1 request/minute — burst capacity is 1, the second request must 429.
        let app = awtest::init_service(
            App::new()
                .wrap(RateLimiter::new(1))
                .route("/api", web::get().to(ok_handler)),
        ).await;
 
        let r1 = awtest::call_service(
            &app,
            awtest::TestRequest::get().uri("/api").to_request(),
        ).await;
        assert!(r1.status().is_success());
 
        let r2 = awtest::call_service(
            &app,
            awtest::TestRequest::get().uri("/api").to_request(),
        ).await;
        assert_eq!(r2.status(), 429);
 
        let body: Value = awtest::read_body_json(r2).await;
        assert_eq!(body["status"], 429);
        assert!(
            body["error"].as_str().unwrap_or("").to_lowercase().contains("rate"),
            "error message should mention rate limiting; got {body:?}"
        );
    }
 
    #[actix_web::test]
    async fn limiter_exempts_health_endpoints_even_under_pressure() {
        // 1/min budget — but /health/* must remain reachable indefinitely
        // because monitoring systems poll it constantly.
        let app = awtest::init_service(
            App::new()
                .wrap(RateLimiter::new(1))
                .route("/api", web::get().to(ok_handler))
                .route("/health/live", web::get().to(ok_handler))
                .route("/health/ready", web::get().to(ok_handler)),
        ).await;
 
        // Burn the budget on /api
        let _ = awtest::call_service(
            &app,
            awtest::TestRequest::get().uri("/api").to_request(),
        ).await;
        let r2 = awtest::call_service(
            &app,
            awtest::TestRequest::get().uri("/api").to_request(),
        ).await;
        assert_eq!(r2.status(), 429, "second /api call should be limited");
 
        // Health probes must still succeed
        for path in ["/health/live", "/health/ready"] {
            for _ in 0..5 {
                let req = awtest::TestRequest::get().uri(path).to_request();
                let resp = awtest::call_service(&app, req).await;
                assert!(resp.status().is_success(),
                    "{path} must always return 200 — got {}", resp.status());
            }
        }
    }
 
    #[actix_web::test]
    async fn limiter_exempts_metrics_endpoint() {
        let app = awtest::init_service(
            App::new()
                .wrap(RateLimiter::new(1))
                .route("/api", web::get().to(ok_handler))
                .route("/metrics", web::get().to(ok_handler)),
        ).await;
 
        // Exhaust the budget on /api
        let _ = awtest::call_service(
            &app,
            awtest::TestRequest::get().uri("/api").to_request(),
        ).await;
        let _ = awtest::call_service(
            &app,
            awtest::TestRequest::get().uri("/api").to_request(),
        ).await;
 
        // /metrics should still be reachable many times
        for _ in 0..10 {
            let req = awtest::TestRequest::get().uri("/metrics").to_request();
            let resp = awtest::call_service(&app, req).await;
            assert!(resp.status().is_success());
        }
    }
 
    #[actix_web::test]
    async fn limiter_does_not_trust_x_forwarded_for_for_bypass() {
        // Security regression test — XFF must NOT be trusted by default.
        let app = awtest::init_service(
            App::new()
                .wrap(RateLimiter::new(1))
                .route("/api", web::get().to(ok_handler)),
        ).await;
 
        // Burn the budget once
        let _ = awtest::call_service(
            &app,
            awtest::TestRequest::get().uri("/api").to_request(),
        ).await;
 
        // Try to bypass via spoofed XFF — must still be rate limited.
        let r = awtest::call_service(
            &app,
            awtest::TestRequest::get()
                .uri("/api")
                .insert_header(("X-Forwarded-For", "1.2.3.4"))
                .to_request(),
        ).await;
        assert_eq!(
            r.status(), 429,
            "XFF spoofing must NOT bypass the rate limiter (security regression)"
        );
    }
}

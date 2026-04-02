use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    Error, HttpMessage,
};
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use uuid::Uuid;

pub struct RequestId;

impl<S, B> Transform<S, ServiceRequest> for RequestId
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = RequestIdMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddleware { service }))
    }
}

pub struct RequestIdMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_id = Uuid::new_v4().to_string();
        let method = req.method().to_string();
        let raw_path = req.path().to_string();
        // Normalise path for Prometheus labels to avoid per-resource cardinality explosion.
        // Replace path segments that look like UUIDs or pure integers with `{id}`.
        let path = normalise_path_label(&raw_path);
        let start = std::time::Instant::now();

        req.extensions_mut().insert(request_id.clone());

        let fut = self.service.call(req);

        Box::pin(async move {
            let mut res = fut.await?;
            let status = res.status().as_u16().to_string();
            let elapsed = start.elapsed().as_secs_f64();

            // Record Prometheus HTTP metrics
            crate::metrics::HTTP_REQUESTS_TOTAL
                .with_label_values(&[&method, &path, &status])
                .inc();
            crate::metrics::HTTP_REQUEST_DURATION
                .with_label_values(&[&method, &path])
                .observe(elapsed);

            res.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-request-id"),
                actix_web::http::header::HeaderValue::from_str(&request_id).unwrap(),
            );
            Ok(res)
        })
    }
}

/// Replace UUID-shaped or integer-only path segments with the literal `{id}`.
/// e.g. `/v1/tasks/550e8400-e29b-41d4-a716-446655440000/start` → `/v1/tasks/{id}/start`
///      `/v1/snapshots/1234` → `/v1/snapshots/{id}`
fn normalise_path_label(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            // UUID pattern (8-4-4-4-12 hex with hyphens)
            if seg.len() == 36
                && seg.chars().filter(|&c| c == '-').count() == 4
                && seg.replace('-', "").chars().all(|c| c.is_ascii_hexdigit())
            {
                return "{id}";
            }
            // Pure numeric ID
            if !seg.is_empty() && seg.chars().all(|c| c.is_ascii_digit()) {
                return "{id}";
            }
            seg
        })
        .collect::<Vec<_>>()
        .join("/")
}

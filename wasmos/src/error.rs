use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmOsError {
    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Validation error: {0}")]
    Validation(String),

    /// Database errors — the raw sqlx message is kept for server-side logging
    /// but is **never** forwarded to HTTP clients (see error_response below).
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// 404 — generic "not found" for any resource (task, snapshot, tenant, …)
    #[error("{0} not found")]
    NotFound(String),

    /// Legacy alias kept for backwards compat with existing call-sites
    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task already running: {0}")]
    TaskAlreadyRunning(String),

    /// 422 — request is well-formed but semantically invalid for current state
    #[error("Task not running: {0}")]
    TaskNotRunning(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// 401 — missing or invalid credentials
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

impl ResponseError for WasmOsError {
    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        // Sanitize database errors — never leak raw SQL detail to the client.
        let message = match self {
            WasmOsError::Database(_) => "A database error occurred".to_string(),
            WasmOsError::Config(_)   => "A configuration error occurred".to_string(),
            other => other.to_string(),
        };
        HttpResponse::build(status).json(ErrorResponse {
            error: message,
            code: error_code(self),
            status: status.as_u16(),
        })
    }

    fn status_code(&self) -> StatusCode {
        match self {
            WasmOsError::ExecutionError(_)    => StatusCode::INTERNAL_SERVER_ERROR,
            WasmOsError::ResourceLimit(_)     => StatusCode::PAYLOAD_TOO_LARGE,
            WasmOsError::Validation(_)        => StatusCode::BAD_REQUEST,
            WasmOsError::Database(_)          => StatusCode::INTERNAL_SERVER_ERROR,
            WasmOsError::Config(_)            => StatusCode::INTERNAL_SERVER_ERROR,
            WasmOsError::NotFound(_)          => StatusCode::NOT_FOUND,
            WasmOsError::TaskNotFound(_)      => StatusCode::NOT_FOUND,
            WasmOsError::TaskAlreadyRunning(_)=> StatusCode::CONFLICT,
            WasmOsError::TaskNotRunning(_)    => StatusCode::UNPROCESSABLE_ENTITY,
            WasmOsError::Io(_)                => StatusCode::INTERNAL_SERVER_ERROR,
            WasmOsError::Unauthorized(_)      => StatusCode::UNAUTHORIZED,
        }
    }
}

/// Machine-readable error code (stable string clients can switch on).
fn error_code(e: &WasmOsError) -> &'static str {
    match e {
        WasmOsError::ExecutionError(_)     => "EXECUTION_ERROR",
        WasmOsError::ResourceLimit(_)      => "RESOURCE_LIMIT",
        WasmOsError::Validation(_)         => "VALIDATION_ERROR",
        WasmOsError::Database(_)           => "DATABASE_ERROR",
        WasmOsError::Config(_)             => "CONFIG_ERROR",
        WasmOsError::NotFound(_)           => "NOT_FOUND",
        WasmOsError::TaskNotFound(_)       => "NOT_FOUND",
        WasmOsError::TaskAlreadyRunning(_) => "ALREADY_RUNNING",
        WasmOsError::TaskNotRunning(_)     => "NOT_RUNNING",
        WasmOsError::Io(_)                 => "IO_ERROR",
        WasmOsError::Unauthorized(_)       => "UNAUTHORIZED",
    }
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
    /// Machine-readable code (e.g. "NOT_FOUND", "VALIDATION_ERROR")
    code: &'static str,
    status: u16,
}

pub type Result<T> = std::result::Result<T, WasmOsError>;

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::error::ResponseError;
 
    #[test]
    fn test_execution_error_500() {
        let err = WasmOsError::ExecutionError("boom".into());
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert!(err.to_string().contains("boom"));
    }
 
    #[test]
    fn test_resource_limit_413() {
        let err = WasmOsError::ResourceLimit("too big".into());
        assert_eq!(err.status_code(), StatusCode::PAYLOAD_TOO_LARGE);
    }
 
    #[test]
    fn test_validation_400() {
        let err = WasmOsError::Validation("bad input".into());
        assert_eq!(err.status_code(), StatusCode::BAD_REQUEST);
    }
 
    #[test]
    fn test_not_found_404() {
        let err = WasmOsError::NotFound("task-123".into());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
        assert!(err.to_string().contains("task-123"));
    }
 
    #[test]
    fn test_task_not_found_404() {
        let err = WasmOsError::TaskNotFound("abc".into());
        assert_eq!(err.status_code(), StatusCode::NOT_FOUND);
    }
 
    #[test]
    fn test_task_already_running_409() {
        let err = WasmOsError::TaskAlreadyRunning("xyz".into());
        assert_eq!(err.status_code(), StatusCode::CONFLICT);
    }
 
    #[test]
    fn test_task_not_running_422() {
        let err = WasmOsError::TaskNotRunning("xyz".into());
        assert_eq!(err.status_code(), StatusCode::UNPROCESSABLE_ENTITY);
    }
 
    #[test]
    fn test_unauthorized_401() {
        let err = WasmOsError::Unauthorized("no token".into());
        assert_eq!(err.status_code(), StatusCode::UNAUTHORIZED);
    }
 
    #[test]
    fn test_io_error_500() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err = WasmOsError::Io(io_err);
        assert_eq!(err.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
    }
 
    #[test]
    fn test_error_response_returns_correct_status() {
        let err = WasmOsError::Validation("bad".into());
        let resp = err.error_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
 
    #[test]
    fn test_error_code_mapping() {
        assert_eq!(error_code(&WasmOsError::ExecutionError("".into())), "EXECUTION_ERROR");
        assert_eq!(error_code(&WasmOsError::ResourceLimit("".into())), "RESOURCE_LIMIT");
        assert_eq!(error_code(&WasmOsError::Validation("".into())), "VALIDATION_ERROR");
        assert_eq!(error_code(&WasmOsError::NotFound("".into())), "NOT_FOUND");
        assert_eq!(error_code(&WasmOsError::TaskNotFound("".into())), "NOT_FOUND");
        assert_eq!(error_code(&WasmOsError::TaskAlreadyRunning("".into())), "ALREADY_RUNNING");
        assert_eq!(error_code(&WasmOsError::TaskNotRunning("".into())), "NOT_RUNNING");
        assert_eq!(error_code(&WasmOsError::Unauthorized("".into())), "UNAUTHORIZED");
    }
}

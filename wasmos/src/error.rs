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

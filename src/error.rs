use actix_web::{error::ResponseError, http::StatusCode, HttpResponse};
use std::fmt;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WasmOsError {
    #[error("WASM parsing error: {0}")]
    WasmParse(String),

    #[error("WASM execution error: {0}")]
    WasmExecution(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Task not found: {0}")]
    TaskNotFound(String),

    #[error("Task already running: {0}")]
    TaskAlreadyRunning(String),

    #[error("Task not running: {0}")]
    TaskNotRunning(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal server error: {0}")]
    Internal(String),
}

impl ResponseError for WasmOsError {
    fn error_response(&self) -> HttpResponse {
        let status = self.status_code();
        let error_message = ErrorResponse {
            error: self.to_string(),
            status: status.as_u16(),
        };
        
        HttpResponse::build(status).json(error_message)
    }

    fn status_code(&self) -> StatusCode {
        match self {
            WasmOsError::WasmParse(_) => StatusCode::BAD_REQUEST,
            WasmOsError::WasmExecution(_) => StatusCode::INTERNAL_SERVER_ERROR,
            WasmOsError::ResourceLimit(_) => StatusCode::PAYLOAD_TOO_LARGE,
            WasmOsError::Validation(_) => StatusCode::BAD_REQUEST,
            WasmOsError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            WasmOsError::Config(_) => StatusCode::INTERNAL_SERVER_ERROR,
            WasmOsError::Auth(_) => StatusCode::UNAUTHORIZED,
            WasmOsError::TaskNotFound(_) => StatusCode::NOT_FOUND,
            WasmOsError::TaskAlreadyRunning(_) => StatusCode::CONFLICT,
            WasmOsError::TaskNotRunning(_) => StatusCode::CONFLICT,
            WasmOsError::Io(_) => StatusCode::INTERNAL_SERVER_ERROR,
            WasmOsError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error: String,
    status: u16,
}

pub type Result<T> = std::result::Result<T, WasmOsError>;

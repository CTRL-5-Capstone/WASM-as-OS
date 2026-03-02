use actix_web::{get, post, delete, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use validator::Validate;

use crate::db::models::{Task, TaskStatus};
use crate::db::repository::{SystemStats, TaskRepository};
use crate::error::{Result, WasmOsError};
use crate::metrics;
use crate::run_wasm::execute_wasm_file;
use std::path::Path;

pub struct AppState {
    pub task_repo: Arc<TaskRepository>,
    pub config: Arc<crate::config::Config>,
}

#[derive(Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub path: String,
    pub created_at: String,
    pub file_size_bytes: i64,
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            name: task.name,
            status: task.status.to_string(),
            path: task.path,
            created_at: task.created_at.to_rfc3339(),
            file_size_bytes: task.file_size_bytes,
        }
    }
}

#[derive(Deserialize, Validate)]
pub struct CreateTaskRequest {
    #[validate(length(min = 1, max = 255))]
    pub name: String,
    #[validate(length(min = 1))]
    pub wasm_data: Vec<u8>,
}

// Health check endpoints
#[get("/health/live")]
pub async fn health_live() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

#[get("/health/ready")]
pub async fn health_ready(data: web::Data<AppState>) -> impl Responder {
    // Check database connectivity
    match data.task_repo.get_stats().await {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "status": "ready",
            "database": "connected",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
        Err(_) => HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "status": "not_ready",
            "database": "disconnected",
            "timestamp": chrono::Utc::now().to_rfc3339()
        })),
    }
}

// Metrics endpoint
#[get("/metrics")]
pub async fn get_metrics() -> impl Responder {
    match metrics::encode_metrics() {
        Ok(metrics) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4")
            .body(metrics),
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

// API v1 endpoints
#[get("/v1/stats")]
pub async fn get_stats(data: web::Data<AppState>) -> Result<impl Responder> {
    let stats = data.task_repo.get_stats().await?;
    Ok(HttpResponse::Ok().json(stats))
}

#[get("/v1/tasks")]
pub async fn get_tasks(data: web::Data<AppState>) -> Result<impl Responder> {
    let tasks = data.task_repo.list_all().await?;
    let responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();
    
    metrics::TASKS_TOTAL
        .with_label_values(&["total"])
        .set(responses.len() as f64);
    
    Ok(HttpResponse::Ok().json(responses))
}

#[get("/v1/tasks/{id}")]
pub async fn get_task(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    
    match data.task_repo.get_by_id(&id).await? {
        Some(task) => {
            let metrics = data.task_repo.get_metrics(&id).await?.unwrap_or_default();
            let history = data.task_repo.get_execution_history(&id, 10).await?;
            
            Ok(HttpResponse::Ok().json(serde_json::json!({
                "task": TaskResponse::from(task),
                "metrics": metrics,
                "recent_executions": history
            })))
        }
        None => Err(WasmOsError::TaskNotFound(id)),
    }
}

#[post("/v1/tasks")]
pub async fn upload_task(
    data: web::Data<AppState>,
    task_req: web::Json<CreateTaskRequest>,
) -> Result<impl Responder> {
    task_req.validate().map_err(|e| WasmOsError::Validation(e.to_string()))?;
    
    // Check file size limit
    let max_size = data.config.limits.max_wasm_size_mb * 1024 * 1024;
    if task_req.wasm_data.len() > max_size {
        return Err(WasmOsError::ResourceLimit(format!(
            "WASM file size {} exceeds limit of {} MB",
            task_req.wasm_data.len(),
            data.config.limits.max_wasm_size_mb
        )));
    }
    
    // Ensure wasm_files directory exists
    std::fs::create_dir_all("wasm_files")
        .map_err(|e| WasmOsError::Io(e))?;
    
    // Sanitize filename
    let filename = task_req
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>();
    let filepath = format!("wasm_files/{}.wasm", filename);
    
    // Write WASM data to file
    std::fs::write(&filepath, &task_req.wasm_data)
        .map_err(|e| WasmOsError::Io(e))?;
    
    // Create task
    let task = Task::new(
        task_req.name.clone(),
        filepath,
        task_req.wasm_data.len() as i64,
    );
    
    data.task_repo.create(&task).await?;
    
    tracing::info!("Created task: {} ({})", task.name, task.id);
    metrics::TASKS_TOTAL.with_label_values(&["pending"]).inc();
    
    Ok(HttpResponse::Created().json(TaskResponse::from(task)))
}

#[post("/v1/tasks/{id}/start")]
pub async fn start_task(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    
    let task = data
        .task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;
    
    if task.status == TaskStatus::Running {
        return Err(WasmOsError::TaskAlreadyRunning(id));
    }
    
    // Update status to running
    data.task_repo.update_status(&id, TaskStatus::Running).await?;
    
    let start_time = std::time::Instant::now();
    
    // Execute WASM using the unified execute_wasm_file function
    let exec_result = match execute_wasm_file(&task.path) {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("WASM engine error for {}: {}", task.name, e);
            crate::run_wasm::ExecutionResult::failure(
                e,
                0,
                0,
                0,
                start_time.elapsed().as_micros() as u64,
                vec![],
            )
        }
    };
    
    let duration_us = start_time.elapsed().as_micros() as i64;
    
    // Update status based on result
    let final_status = if exec_result.success {
        TaskStatus::Completed
    } else {
        TaskStatus::Failed
    };
    
    let status_str = final_status.to_string();
    data.task_repo.update_status(&id, final_status).await?;
    
    // Record execution with REAL metrics from the engine
    data.task_repo
        .add_execution(
            &id,
            duration_us,
            exec_result.success,
            exec_result.error.clone(),
            exec_result.instructions_executed as i64,
            exec_result.syscalls_executed as i64,
            exec_result.memory_used_bytes as i64,
        )
        .await?;
    
    metrics::TASK_EXECUTIONS_TOTAL
        .with_label_values(&[if exec_result.success { "success" } else { "failed" }])
        .inc();
    
    metrics::TASK_EXECUTION_DURATION
        .with_label_values(&[&task.name])
        .observe(duration_us as f64 / 1_000_000.0);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": status_str,
        "duration_us": duration_us,
        "success": exec_result.success,
        "instructions_executed": exec_result.instructions_executed,
        "syscalls_executed": exec_result.syscalls_executed,
        "memory_used_bytes": exec_result.memory_used_bytes,
        "stdout_log": exec_result.stdout_log,
        "return_value": exec_result.return_value,
        "error": exec_result.error
    })))
}

#[post("/v1/tasks/{id}/stop")]
pub async fn stop_task(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    
    let task = data
        .task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;
    
    if task.status != TaskStatus::Running {
        return Err(WasmOsError::TaskNotRunning(id));
    }
    
    data.task_repo.update_status(&id, TaskStatus::Stopped).await?;
    
    tracing::info!("Stopped task: {}", task.name);
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "stopped"
    })))
}

#[delete("/v1/tasks/{id}")]
pub async fn delete_task(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    
    let task = data
        .task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;
    
    // Delete file if it exists
    if Path::new(&task.path).exists() {
        std::fs::remove_file(&task.path).ok();
    }
    
    data.task_repo.delete(&id).await?;
    
    tracing::info!("Deleted task: {}", task.name);
    metrics::TASKS_TOTAL.with_label_values(&["total"]).dec();
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "deleted"
    })))
}

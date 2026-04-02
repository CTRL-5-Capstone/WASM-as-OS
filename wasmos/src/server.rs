use actix_web::{get, post, put, delete, web, HttpRequest, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::capability::{CapabilityRegistry, IssueTokenRequest, IssueTokenResponse, TokenSummary};
use crate::db::models::{Snapshot, Task, TaskStatus};
use crate::db::repository::TaskRepository;
use crate::error::{Result, WasmOsError};
use crate::metrics;
use crate::middleware::auth::AuthService;
use crate::plugins::PluginManager;
use crate::query_cache::QueryCache;
use crate::run_wasm::execute_wasm_file;
use crate::scheduler::Scheduler;
use crate::tracing_spans::TraceStore;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::fs;
use uuid::Uuid;

/// Live task events broadcast to all connected WebSocket clients
#[derive(Debug, Clone, Serialize)]
pub struct TaskEvent {
    pub event: String,   // "started" | "completed" | "failed"
    pub task_id: String,
    pub task_name: String,
    pub status: String,
}

// ─── Static WASM binary analyser ─────────────────────────────────────────────

/// Parse import/export sections from raw WASM bytes without executing.
/// Returns (imports, exports, suspicious_capabilities).
fn analyse_wasm_bytes(bytes: &[u8]) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut imports: Vec<String> = Vec::new();
    let mut exports: Vec<String> = Vec::new();
    let mut suspicious: Vec<String> = Vec::new();

    if bytes.len() < 8 || bytes[0..4] != [0x00, 0x61, 0x73, 0x6D] {
        suspicious.push("Not a valid WASM binary (bad magic)".into());
        return (imports, exports, suspicious);
    }

    // Minimal section scanner — skip version (bytes 4-7), then walk sections
    let mut pos = 8usize;
    while pos + 2 <= bytes.len() {
        let section_id = bytes[pos];
        pos += 1;
        // Read LEB128 section length
        let (sec_len, leb_bytes) = read_leb128_u32(bytes, pos);
        pos += leb_bytes;
        let section_end = pos + sec_len as usize;
        if section_end > bytes.len() {
            break;
        }

        match section_id {
            // Import section (id=2)
            2 => {
                let (count, n) = read_leb128_u32(bytes, pos);
                let mut p = pos + n;
                for _ in 0..count {
                    if p >= section_end { break; }
                    // module name
                    let (mlen, n) = read_leb128_u32(bytes, p);
                    p += n;
                    let mname = read_utf8(bytes, p, mlen as usize);
                    p += mlen as usize;
                    // field name
                    let (flen, n) = read_leb128_u32(bytes, p);
                    p += n;
                    let fname = read_utf8(bytes, p, flen as usize);
                    p += flen as usize;
                    // import kind + skip type index
                    if p < section_end {
                        let kind = bytes[p]; p += 1;
                        if p < section_end {
                            let (_idx, n) = read_leb128_u32(bytes, p);
                            p += n;
                        }
                        let kind_str = match kind {
                            0 => "func", 1 => "table", 2 => "mem", 3 => "global", _ => "?",
                        };
                        imports.push(format!("{mname}::{fname} ({kind_str})"));
                        // Suspicious capability detection
                        let combined = format!("{mname}::{fname}").to_lowercase();
                        if combined.contains("file") || combined.contains("open") || combined.contains("read") || combined.contains("write") {
                            suspicious.push(format!("File I/O import: {mname}::{fname}"));
                        }
                        if combined.contains("net") || combined.contains("socket") || combined.contains("connect") || combined.contains("send") || combined.contains("recv") {
                            suspicious.push(format!("Network import: {mname}::{fname}"));
                        }
                        if combined.contains("exec") || combined.contains("spawn") || combined.contains("process") || combined.contains("popen") {
                            suspicious.push(format!("Process execution import: {mname}::{fname}"));
                        }
                        if combined.contains("env") && (combined.contains("get") || combined.contains("set")) {
                            suspicious.push(format!("Environment access import: {mname}::{fname}"));
                        }
                        if combined.contains("clock") || combined.contains("time") || combined.contains("random") || combined.contains("rand") {
                            suspicious.push(format!("Side-channel-prone import: {mname}::{fname}"));
                        }
                    }
                }
            }
            // Export section (id=7)
            7 => {
                let (count, n) = read_leb128_u32(bytes, pos);
                let mut p = pos + n;
                for _ in 0..count {
                    if p >= section_end { break; }
                    let (elen, n) = read_leb128_u32(bytes, p);
                    p += n;
                    let ename = read_utf8(bytes, p, elen as usize);
                    p += elen as usize;
                    let kind = if p < section_end { let k = bytes[p]; p += 1; k } else { 0xff };
                    let (_idx, n) = read_leb128_u32(bytes, p);
                    p += n;
                    let kind_str = match kind {
                        0 => "func", 1 => "table", 2 => "mem", 3 => "global", _ => "?",
                    };
                    exports.push(format!("{ename} ({kind_str})"));
                }
            }
            _ => {}
        }
        pos = section_end;
    }

    // Heuristic: no exports is suspicious (module with no public interface)
    if exports.is_empty() {
        suspicious.push("No exported functions — module has no callable entry points".into());
    }

    (imports, exports, suspicious)
}

fn read_leb128_u32(bytes: &[u8], mut pos: usize) -> (u32, usize) {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    let mut consumed = 0usize;
    while pos < bytes.len() {
        let byte = bytes[pos];
        pos += 1;
        consumed += 1;
        result |= ((byte & 0x7f) as u32) << shift;
        shift += 7;
        if byte & 0x80 == 0 { break; }
        if consumed >= 5 { break; }
    }
    (result, consumed)
}

fn read_utf8(bytes: &[u8], pos: usize, len: usize) -> String {
    if pos + len > bytes.len() {
        return "<truncated>".into();
    }
    String::from_utf8_lossy(&bytes[pos..pos + len]).into_owned()
}

pub struct AppState {
    pub task_repo: Arc<TaskRepository>,
    pub config: Arc<crate::config::Config>,
    pub plugin_manager: Arc<PluginManager>,
    pub auth_service: Arc<AuthService>,
    /// Broadcast channel — send task events to all connected WebSocket clients
    pub event_tx: broadcast::Sender<TaskEvent>,
    /// Capability token registry (zero-trust access control)
    pub cap_registry: Arc<CapabilityRegistry>,
    /// Distributed trace store
    pub trace_store: Arc<TraceStore>,
    /// Scheduler handle for status/preempt API
    pub scheduler: Arc<Scheduler>,
    /// In-memory query result cache (moka, TTL-based)
    pub query_cache: Arc<QueryCache>,
}

/// Guard: when auth is enabled the caller's JWT must carry `role = "admin"`.
/// When auth is disabled (dev mode) this is a no-op so local development is unaffected.
/// Returns `Err(Unauthorized)` if the check fails; callers propagate it with `?`.
fn require_admin(req: &HttpRequest, state: &AppState) -> Result<()> {
    if !state.auth_service.enabled {
        return Ok(());
    }
    use actix_web::HttpMessage as _;
    use crate::middleware::auth::Claims;
    let claims = req
        .extensions()
        .get::<Claims>()
        .cloned()
        .ok_or_else(|| WasmOsError::Unauthorized("Missing credentials".into()))?;
    if claims.role != "admin" {
        return Err(WasmOsError::Unauthorized(
            "Admin role required for this operation".into(),
        ));
    }
    Ok(())
}

/// Extract client IP from X-Forwarded-For or peer_addr
fn extract_ip(req: &HttpRequest) -> String {
    req.headers()
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| req.peer_addr().map(|a| a.ip().to_string()))
        .unwrap_or_else(|| "unknown".into())
}

fn wasm_files_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("wasm_files")
}

/// Public wrapper used by advanced_execution_endpoints for path resolution.
pub fn resolve_wasm_file_path_pub(path: &str) -> String {
    resolve_wasm_file_path(path)
}

fn resolve_wasm_file_path(path_str: &str) -> String {
    let p = Path::new(path_str);
    if p.exists() {
        return path_str.to_string();
    }

    let Some(file_name) = p.file_name().and_then(|s| s.to_str()) else {
        return path_str.to_string();
    };

    let candidate = wasm_files_dir().join(file_name);
    if candidate.exists() {
        return candidate.to_string_lossy().to_string();
    }

    path_str.to_string()
}

/// Validates that the resolved path is strictly inside `wasm_files_dir()`.
///
/// Prevents path-traversal attacks where a DB-stored `task.path` containing
/// `../../etc/passwd` (or any path outside the WASM sandbox) would cause
/// `fs::read` to expose arbitrary host-filesystem files.
///
/// Returns the canonicalized, safe `PathBuf` on success.
fn validate_wasm_path_boundary(resolved_path: &str) -> Result<PathBuf> {
    // canonicalize() resolves symlinks and `..` segments — it will fail if the
    // file does not exist, which is exactly what we want (no file → error out).
    let resolved = std::fs::canonicalize(resolved_path)
        .map_err(|_| WasmOsError::Validation("Task file not found or inaccessible".into()))?;

    // Canonicalize the base directory too so the prefix check is reliable even
    // when CARGO_MANIFEST_DIR contains symlinks.
    let safe_dir = std::fs::canonicalize(wasm_files_dir())
        .unwrap_or_else(|_| wasm_files_dir());

    if !resolved.starts_with(&safe_dir) {
        tracing::warn!(
            "Path traversal attempt blocked: {:?} is outside safe dir {:?}",
            resolved, safe_dir
        );
        return Err(WasmOsError::Unauthorized(
            "Task file path is outside the permitted WASM directory".into(),
        ));
    }

    Ok(resolved)
}

fn is_wasm_binary(bytes: &[u8]) -> bool {
    bytes.len() >= 4 && bytes[0..4] == [0x00, 0x61, 0x73, 0x6D]
}

fn compile_wat_to_wasm(wat_bytes: &[u8]) -> Result<Vec<u8>> {
    wat::parse_bytes(wat_bytes)
        .map(|b| b.into_owned())
        .map_err(|e| WasmOsError::Validation(format!("Invalid .wat file: {e}")))
}

fn execute_wasm_or_wat_file(path_str: &str) -> std::result::Result<crate::run_wasm::ExecutionResult, String> {
    let path = Path::new(path_str);
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "wasm" {
        return execute_wasm_file(path_str);
    }
    if ext != "wat" {
        return Err(format!("Unsupported file extension: {ext}"));
    }

    let wat_bytes = fs::read(path)
        .map_err(|e| format!("Failed to read .wat file '{path_str}': {e}"))?;
    let wasm_bytes = wat::parse_bytes(&wat_bytes)
        .map_err(|e| format!("Invalid .wat file '{path_str}': {e}"))?;

    let tmp_path = std::env::temp_dir().join(format!("wasmos_{}.wasm", Uuid::new_v4()));
    fs::write(&tmp_path, &wasm_bytes)
        .map_err(|e| format!("Failed to write temp .wasm for '{path_str}': {e}"))?;

    let tmp_str = tmp_path.to_string_lossy().to_string();
    let result = execute_wasm_file(&tmp_str);
    let _ = fs::remove_file(&tmp_path);
    result
}

#[derive(Serialize)]
pub struct TaskResponse {
    pub id: String,
    pub name: String,
    pub status: String,
    pub path: String,
    pub created_at: String,
    /// ISO 8601 timestamp of the last status/metadata change (maintained by DB trigger).
    pub updated_at: String,
    pub file_size_bytes: i64,
    pub tenant_id: Option<String>,
    pub priority: i16,
}

impl From<Task> for TaskResponse {
    fn from(task: Task) -> Self {
        Self {
            id: task.id,
            name: task.name,
            status: task.status.to_string(),
            path: task.path,
            created_at: task.created_at.to_rfc3339(),
            updated_at: task.updated_at.to_rfc3339(),
            file_size_bytes: task.file_size_bytes,
            tenant_id: task.tenant_id,
            priority: task.priority,
        }
    }
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub wasm_data: Vec<u8>,
    /// Optional — when provided the task is owned by this tenant and quotas are enforced.
    #[serde(default)]
    pub tenant_id: Option<String>,
}

// Health check endpoints
#[get("/health/live")]
pub async fn health_live() -> impl Responder {
    HttpResponse::Ok()
        .insert_header(("Cache-Control", "public, max-age=30"))
        .json(serde_json::json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }))
}

#[post("/health/live")]
pub async fn health_live_post() -> impl Responder {
    HttpResponse::MethodNotAllowed().finish()
}

#[get("/health/ready")]
pub async fn health_ready(data: web::Data<AppState>) -> impl Responder {
    // Check database connectivity
    match data.task_repo.get_stats().await {
        Ok(_) => HttpResponse::Ok()
            .insert_header(("Cache-Control", "public, max-age=10"))
            .json(serde_json::json!({
                "status": "ready",
                "database": "connected",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
        Err(_) => HttpResponse::ServiceUnavailable()
            .insert_header(("Cache-Control", "no-store"))
            .json(serde_json::json!({
                "status": "not_ready",
                "database": "disconnected",
                "timestamp": chrono::Utc::now().to_rfc3339()
            })),
    }
}

// Metrics endpoint — 10 s public cache so Prometheus scrapers don't hammer the encoder
#[get("/metrics")]
pub async fn get_metrics() -> impl Responder {
    match metrics::encode_metrics() {
        Ok(m) => HttpResponse::Ok()
            .content_type("text/plain; version=0.0.4")
            .insert_header(("Cache-Control", "public, max-age=10"))
            .body(m),
        Err(e) => {
            tracing::error!("Failed to encode metrics: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

// API v1 endpoints
#[get("/v1/stats")]
pub async fn get_stats(data: web::Data<AppState>) -> Result<impl Responder> {
    // Serve from cache when available (10 s TTL keeps dashboards snappy)
    if let Some(cached) = data.query_cache.get_stats().await {
        return Ok(HttpResponse::Ok()
            .insert_header(("Cache-Control", "public, max-age=10"))
            .insert_header(("X-Cache", "HIT"))
            .json(cached));
    }
    let stats = data.task_repo.get_stats().await?;
    let json = serde_json::to_value(&stats).unwrap_or_default();
    data.query_cache.insert_stats(json).await;
    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "public, max-age=10"))
        .insert_header(("X-Cache", "MISS"))
        .json(stats))
}

#[derive(Deserialize)]
pub struct TaskListQuery {
    pub tenant_id: Option<String>,
    pub status: Option<String>,
    /// Max rows to return (default 100, max 500). Only applied to unfiltered listing.
    pub limit: Option<i64>,
    /// Row offset for pagination (default 0). Only applied to unfiltered listing.
    pub offset: Option<i64>,
}

#[get("/v1/tasks")]
pub async fn get_tasks(
    data: web::Data<AppState>,
    query: web::Query<TaskListQuery>,
) -> Result<impl Responder> {
    let limit  = query.limit.unwrap_or(100).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let tid    = query.tenant_id.as_deref();
    let status = query.status.as_deref();

    // Return cached result when available (15 s TTL)
    if let Some(cached) = data.query_cache.get_tasks(tid, status, limit, offset).await {
        return Ok(HttpResponse::Ok()
            .insert_header(("Cache-Control", "public, max-age=15"))
            .insert_header(("X-Cache", "HIT"))
            .json(cached));
    }

    let tasks = match (&query.tenant_id, &query.status) {
        (Some(t), Some(s)) => {
            let st: TaskStatus = s.parse().unwrap_or(TaskStatus::Pending);
            data.task_repo.list_by_tenant_and_status(t, st).await?
        }
        (Some(t), None) => data.task_repo.list_by_tenant(t).await?,
        (None, Some(s)) => {
            let st: TaskStatus = s.parse().unwrap_or(TaskStatus::Pending);
            data.task_repo.list_by_status(st).await?
        }
        (None, None) => data.task_repo.list_all_paginated(limit, offset).await?,
    };
    let responses: Vec<TaskResponse> = tasks.into_iter().map(TaskResponse::from).collect();
    metrics::TASKS_TOTAL.with_label_values(&["total"]).set(responses.len() as f64);

    let json = serde_json::to_value(&responses).unwrap_or_default();
    data.query_cache.insert_tasks(tid, status, limit, offset, json).await;

    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "public, max-age=15"))
        .insert_header(("X-Cache", "MISS"))
        .json(responses))
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
    req: HttpRequest,
    data: web::Data<AppState>,
    task_req: web::Json<CreateTaskRequest>,
) -> Result<impl Responder> {
    // Basic validation (inline, no validator crate needed)
    if task_req.name.is_empty() || task_req.name.len() > 255 {
        return Err(WasmOsError::Validation("name must be 1-255 chars".into()));
    }
    if task_req.wasm_data.is_empty() {
        return Err(WasmOsError::Validation("wasm_data cannot be empty".into()));
    }

    // Check file size limit
    let max_size = data.config.limits.max_wasm_size_mb * 1024 * 1024;
    if task_req.wasm_data.len() > max_size {
        return Err(WasmOsError::ResourceLimit(format!(
            "WASM file size {} exceeds limit of {} MB",
            task_req.wasm_data.len(),
            data.config.limits.max_wasm_size_mb
        )));
    }

    // Accept either binary .wasm or text .wat; if it's WAT, compile to WASM.
    let wasm_bytes = if is_wasm_binary(&task_req.wasm_data) {
        task_req.wasm_data.clone()
    } else {
        compile_wat_to_wasm(&task_req.wasm_data)?
    };

    // Enforce size limit on compiled binary as well
    if wasm_bytes.len() > max_size {
        return Err(WasmOsError::ResourceLimit(format!(
            "Compiled WASM size {} exceeds limit of {} MB",
            wasm_bytes.len(),
            data.config.limits.max_wasm_size_mb
        )));
    }

    // ── Per-tenant quota enforcement ─────────────────────────────────────────
    // Checks active status, per-tenant WASM size limit, and max_tasks quota.
    // Performed after compilation so we know the actual binary size.
    if let Some(ref tid) = task_req.tenant_id {
        match data.task_repo.get_tenant_by_id(tid).await {
            Ok(Some(tenant)) => {
                if !tenant.active {
                    return Err(WasmOsError::Unauthorized(
                        format!("Tenant '{}' is inactive", tenant.name)
                    ));
                }
                // Per-tenant WASM size limit (may be stricter than global)
                let tenant_size_limit = tenant.max_wasm_size_mb as usize * 1024 * 1024;
                if wasm_bytes.len() > tenant_size_limit {
                    return Err(WasmOsError::ResourceLimit(format!(
                        "WASM size {} exceeds tenant limit of {} MB",
                        wasm_bytes.len(), tenant.max_wasm_size_mb
                    )));
                }
                // max_tasks quota: count existing tasks for this tenant
                if let Ok(task_count) = data.task_repo.count_tasks_by_tenant(tid).await {
                    if task_count >= tenant.max_tasks as i64 {
                        return Err(WasmOsError::ResourceLimit(format!(
                            "Tenant task limit ({}) reached — delete tasks before uploading more",
                            tenant.max_tasks
                        )));
                    }
                }
            }
            Ok(None) => {
                return Err(WasmOsError::NotFound(format!("Tenant '{tid}' not found")));
            }
            Err(e) => {
                tracing::error!("DB error checking tenant quota: {e}");
                // Fail open on DB error — don't block the upload
            }
        }
    }

    let wasm_dir = wasm_files_dir();
    // Ensure wasm_files directory exists
    tokio::fs::create_dir_all(&wasm_dir).await
        .map_err(|e| WasmOsError::Io(e))?;
    
    // Sanitize filename
    let filename = task_req
        .name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
        .collect::<String>();
    
    // Use a unique ID prefix to avoid filename collisions
    let unique_id = uuid::Uuid::new_v4().to_string();
    let filepath = wasm_dir.join(format!("{}_{}.wasm", filename, &unique_id[..8]));
    
    // Write WASM data to file
    tokio::fs::write(&filepath, &wasm_bytes).await
        .map_err(|e| WasmOsError::Io(e))?;
    
    // Create task — set tenant_id if provided in the request
    let mut task = Task::new(
        task_req.name.clone(),
        filepath.to_string_lossy().to_string(),
        wasm_bytes.len() as i64,
    );
    if let Some(ref tid) = task_req.tenant_id {
        task.tenant_id = Some(tid.clone());
    }

    data.task_repo.create(&task).await?;

    // Audit log
    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "system", "task.upload", Some(&task.id), task.tenant_id.as_deref(), Some(&ip)).await;

    tracing::info!("Created task: {} ({})", task.name, task.id);
    metrics::TASKS_TOTAL.with_label_values(&["pending"]).inc();

    // Invalidate cached task list and stats — a new task was added
    data.query_cache.invalidate_tasks().await;
    data.query_cache.invalidate_stats().await;

    Ok(HttpResponse::Created().json(TaskResponse::from(task)))
}

#[post("/v1/tasks/{id}/start")]
pub async fn start_task(
    req: HttpRequest,
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

    data.task_repo.update_status(&id, TaskStatus::Running).await?;
    data.plugin_manager.trigger_task_start(&id).await;

    // Broadcast live event to WebSocket clients
    let _ = data.event_tx.send(TaskEvent {
        event: "started".into(), task_id: id.clone(),
        task_name: task.name.clone(), status: "running".into(),
    });

    // Audit log
    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "system", "task.start", Some(&id), None, Some(&ip)).await;

    let start_time = std::time::Instant::now();
    let task_path_raw = resolve_wasm_file_path(&task.path);
    // Validate that the resolved path is inside the WASM sandbox before executing.
    let safe_path = validate_wasm_path_boundary(&task_path_raw)?;
    let task_path = safe_path.to_string_lossy().to_string();
    let timeout_secs = data.config.limits.execution_timeout_secs;

    // Execute WASM in a blocking thread with a configurable timeout
    let exec_result = match tokio::time::timeout(
        tokio::time::Duration::from_secs(timeout_secs),
        web::block(move || execute_wasm_file(&task_path)),
    )
    .await
    {
        Ok(Ok(Ok(result))) => result,
        Ok(Ok(Err(e))) => {
            tracing::error!("WASM engine error for {}: {}", task.name, e);
            crate::run_wasm::ExecutionResult::failure(e, 0, 0, 0, start_time.elapsed().as_micros() as u64, vec![])
        }
        Ok(Err(e)) => {
            tracing::error!("WASM execution thread panicked for {}: {}", task.name, e);
            crate::run_wasm::ExecutionResult::failure(
                format!("Execution thread error: {}", e), 0, 0, 0,
                start_time.elapsed().as_micros() as u64, vec![],
            )
        }
        Err(_) => {
            tracing::warn!("WASM execution timed out after {}s for {}", timeout_secs, task.name);
            crate::run_wasm::ExecutionResult::failure(
                format!("Execution timed out after {}s", timeout_secs), 0, 0, 0,
                start_time.elapsed().as_micros() as u64, vec![],
            )
        }
    };

    let duration_us = start_time.elapsed().as_micros() as i64;
    let final_status = if exec_result.success { TaskStatus::Completed } else { TaskStatus::Failed };
    let status_str = final_status.to_string();

    data.task_repo.update_status(&id, final_status).await?;
    data.task_repo
        .add_execution(
            &id, duration_us, exec_result.success, exec_result.error.clone(),
            exec_result.instructions_executed as i64,
            exec_result.syscalls_executed as i64,
            exec_result.memory_used_bytes as i64,
        )
        .await?;

    // Fire plugin lifecycle hooks
    if exec_result.success {
        if let Ok(Some(exec_record)) = data.task_repo
            .get_execution_history(&id, 1).await
            .map(|h| h.into_iter().next())
        {
            data.plugin_manager.trigger_task_complete(&id, &exec_record).await;
        }
    } else {
        data.plugin_manager
            .trigger_task_failed(&id, exec_result.error.as_deref().unwrap_or("unknown"))
            .await;
    }

    // Broadcast final status to all WebSocket clients
    let _ = data.event_tx.send(TaskEvent {
        event: if exec_result.success { "completed".into() } else { "failed".into() },
        task_id: id.clone(),
        task_name: task.name.clone(),
        status: status_str.clone(),
    });

    // Update Prometheus metrics
    metrics::TASK_EXECUTIONS_TOTAL
        .with_label_values(&[if exec_result.success { "success" } else { "failed" }])
        .inc();
    metrics::TASK_EXECUTION_DURATION
        .with_label_values(&[&task.name])
        .observe(duration_us as f64 / 1_000_000.0);
    metrics::WASM_INSTRUCTIONS_TOTAL
        .with_label_values(&[&task.name])
        .inc_by(exec_result.instructions_executed as f64);
    metrics::WASM_MEMORY_USAGE
        .with_label_values(&[&task.name])
        .set(exec_result.memory_used_bytes as f64);

    // Invalidate cached task list/stats — status has changed
    data.query_cache.invalidate_tasks().await;
    data.query_cache.invalidate_stats().await;

    Ok(HttpResponse::Ok()
        .insert_header(("Cache-Control", "no-store"))
        .json(serde_json::json!({
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
    req: HttpRequest,
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
    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "system", "task.stop", Some(&id), task.tenant_id.as_deref(), Some(&ip)).await;
    tracing::info!("Stopped task: {}", task.name);
    data.query_cache.invalidate_tasks().await;
    data.query_cache.invalidate_stats().await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "stopped"
    })))
}

#[delete("/v1/tasks/{id}")]
pub async fn delete_task(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    
    let task = data
        .task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;

    // Prevent deleting a running task — stop it first to avoid
    // leaving the scheduler in an inconsistent state.
    if task.status == TaskStatus::Running {
        return Err(WasmOsError::Validation(
            "Cannot delete a running task; stop or wait for it to complete first".into(),
        ));
    }

    if Path::new(&task.path).exists() {
        std::fs::remove_file(&task.path).ok();
    }
    
    data.task_repo.delete(&id).await?;
    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "system", "task.delete", Some(&id), task.tenant_id.as_deref(), Some(&ip)).await;
    tracing::info!("Deleted task: {}", task.name);
    metrics::TASKS_TOTAL.with_label_values(&["total"]).dec();
    data.query_cache.invalidate_tasks().await;
    data.query_cache.invalidate_stats().await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "deleted"
    })))
}

// ─── Test Files Discovery & Execution ───────────────────────────────

/// Scans multiple known directories for .wasm / .wat test files
fn discover_test_files() -> Vec<TestFileInfo> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let search_dirs: Vec<(&'static str, PathBuf)> = vec![
        // WasmOSTest folder (repo root)
        ("WasmOSTest", manifest_dir.join("../WasmOSTest")),
        ("WasmOSTest", manifest_dir.join("WasmOSTest")),
        ("WasmOSTest", PathBuf::from("../WasmOSTest")),
        ("WasmOSTest", PathBuf::from("WasmOSTest")),
        // wasm_files folder (mounted in Docker, also used by the app)
        ("wasm_files", manifest_dir.join("wasm_files")),
        ("wasm_files", PathBuf::from("wasm_files")),
    ];
    
    let mut files = Vec::new();
    let mut seen_paths = HashSet::<String>::new();
    
    for (source, dir_path) in &search_dirs {
        if !dir_path.exists() || !dir_path.is_dir() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(dir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
                let ext = path.extension().unwrap_or_default().to_string_lossy().to_lowercase();
                
                // Include .wasm and .wat (WAT is compiled to WASM before execution)
                if ext != "wasm" && ext != "wat" {
                    continue;
                }
                
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let path_str = path.to_string_lossy().to_string();
                if !seen_paths.insert(path_str.clone()) {
                    continue;
                }
                
                files.push(TestFileInfo {
                    name: name.clone(),
                    source: source.to_string(),
                    path: path_str,
                    size_bytes: size,
                    category: categorize_test_file(&name, size),
                });
            }
        }
    }
    
    // Sort by name for consistent ordering
    files.sort_by(|a, b| a.name.cmp(&b.name));
    files
}

fn categorize_test_file(name: &str, size: u64) -> String {
    let lower = name.to_lowercase();
    if lower.contains("simple") || lower.contains("add") { return "arithmetic".to_string(); }
    if lower.contains("test") { return "integration".to_string(); }
    if lower.contains("loop") { return "control-flow".to_string(); }
    if lower.contains("game") || lower.contains("snake") { return "application".to_string(); }
    if lower.contains("eagle") || lower.contains("lyft") { return "complex".to_string(); }
    if size > 1_000_000 { return "large".to_string(); }
    "general".to_string()
}

#[derive(Serialize, Clone)]
pub struct TestFileInfo {
    pub name: String,
    pub source: String,
    pub path: String,
    pub size_bytes: u64,
    pub category: String,
}

#[derive(Serialize)]
pub struct TestRunResult {
    pub file: String,
    pub success: bool,
    pub duration_us: u64,
    pub instructions_executed: u64,
    pub syscalls_executed: u64,
    pub memory_used_bytes: u64,
    pub stdout_log: Vec<String>,
    pub return_value: Option<String>,
    pub error: Option<String>,
}

#[get("/v1/test-files")]
pub async fn list_test_files() -> Result<impl Responder> {
    let files = discover_test_files();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "total": files.len(),
        "files": files
    })))
}

#[post("/v1/test-files/{filename}/run")]
pub async fn run_test_file(
    path: web::Path<String>,
) -> Result<impl Responder> {
    let filename = path.into_inner();
    
    // Find the file in known directories
    let files = discover_test_files();
    let file = files.iter().find(|f| f.name == filename)
        .ok_or_else(|| WasmOsError::Validation(format!("Test file '{}' not found", filename)))?;
    
    let file_path = file.path.clone();
    let file_name = file.name.clone();
    
    // Verify file exists
    if !Path::new(&file_path).exists() {
        return Err(WasmOsError::Validation(format!("File '{}' not found on disk", file_path)));
    }
    
    let start_time = std::time::Instant::now();

    // Execute in a blocking thread, bounded by a 30 s timeout to prevent infinite-loop
    // WASM from hanging the request indefinitely (matches the run-all per-file timeout).
    let exec_result = match tokio::time::timeout(
        tokio::time::Duration::from_secs(30),
        web::block(move || execute_wasm_or_wat_file(&file_path)),
    ).await {
        Ok(Ok(Ok(result))) => result,
        Ok(Ok(Err(e))) => {
            tracing::error!("Test file execution error for {}: {}", file_name, e);
            crate::run_wasm::ExecutionResult::failure(
                e, 0, 0, 0, start_time.elapsed().as_micros() as u64, vec![],
            )
        }
        Ok(Err(e)) => {
            tracing::error!("Test file execution thread panicked for {}: {}", file_name, e);
            crate::run_wasm::ExecutionResult::failure(
                format!("Execution thread error: {}", e),
                0, 0, 0, start_time.elapsed().as_micros() as u64, vec![],
            )
        }
        Err(_elapsed) => {
            tracing::warn!("Test file '{}' timed out after 30 s", file_name);
            crate::run_wasm::ExecutionResult::failure(
                "Execution timed out after 30 seconds".to_string(),
                0, 0, 0, 30_000_000, vec!["[TIMEOUT] Execution exceeded 30 s limit".to_string()],
            )
        }
    };
    
    let result = TestRunResult {
        file: filename,
        success: exec_result.success,
        duration_us: exec_result.duration_us,
        instructions_executed: exec_result.instructions_executed,
        syscalls_executed: exec_result.syscalls_executed,
        memory_used_bytes: exec_result.memory_used_bytes,
        stdout_log: exec_result.stdout_log,
        return_value: exec_result.return_value,
        error: exec_result.error,
    };
    
    Ok(HttpResponse::Ok().json(result))
}

#[derive(Deserialize)]
pub struct RunAllTestsQuery {
    pub category: Option<String>,
}

#[post("/v1/test-files/run-all")]
pub async fn run_all_test_files(
    query: web::Query<RunAllTestsQuery>,
) -> Result<impl Responder> {
    let mut files = discover_test_files();
    
    // Filter by category if specified
    if let Some(ref cat) = query.category {
        files.retain(|f| f.category == *cat);
    }
    
    // Skip very large files (>10MB) to avoid timeouts in batch mode
    files.retain(|f| f.size_bytes < 10_000_000);
    
    let total = files.len();
    let mut results: Vec<TestRunResult> = Vec::new();
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut total_duration_us = 0u64;
    
    for file in &files {
        let file_path = file.path.clone();
        let file_name = file.name.clone();
        let start_time = std::time::Instant::now();

        // Wrap each file execution in a 30s timeout so one stuck module
        // cannot block the entire batch forever.
        let exec_result = match tokio::time::timeout(
            tokio::time::Duration::from_secs(30),
            web::block(move || execute_wasm_or_wat_file(&file_path)),
        ).await {
            Ok(Ok(Ok(result))) => result,
            Ok(Ok(Err(e))) => {
                crate::run_wasm::ExecutionResult::failure(
                    e,
                    0, 0, 0,
                    start_time.elapsed().as_micros() as u64,
                    vec![],
                )
            }
            Ok(Err(e)) => {
                crate::run_wasm::ExecutionResult::failure(
                    format!("Thread error: {}", e),
                    0, 0, 0,
                    start_time.elapsed().as_micros() as u64,
                    vec![],
                )
            }
            Err(_) => {
                crate::run_wasm::ExecutionResult::failure(
                    "Execution timed out (30s)".to_string(),
                    0, 0, 0,
                    start_time.elapsed().as_micros() as u64,
                    vec![],
                )
            }
        };
        
        if exec_result.success { passed += 1; } else { failed += 1; }
        total_duration_us += exec_result.duration_us;
        
        results.push(TestRunResult {
            file: file_name,
            success: exec_result.success,
            duration_us: exec_result.duration_us,
            instructions_executed: exec_result.instructions_executed,
            syscalls_executed: exec_result.syscalls_executed,
            memory_used_bytes: exec_result.memory_used_bytes,
            stdout_log: exec_result.stdout_log,
            return_value: exec_result.return_value,
            error: exec_result.error,
        });
    }
    
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "total": total,
        "passed": passed,
        "failed": failed,
        "total_duration_us": total_duration_us,
        "results": results,
    })))
}

// ─── Security Analysis ───────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SecurityCapability {
    pub name: String,
    pub description: String,
    pub level: String, // "info" | "warn" | "severe"
}

#[derive(Serialize)]
pub struct SecurityReport {
    pub task_id: String,
    pub task_name: String,
    pub file_size_bytes: i64,
    pub imports: Vec<String>,
    pub exports: Vec<String>,
    pub capabilities: Vec<SecurityCapability>,
    pub risk_level: String, // "low" | "medium" | "high"
    pub summary: String,
}

#[get("/v1/tasks/{id}/security")]
pub async fn get_task_security(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    let task = data
        .task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;

    let file_path = resolve_wasm_file_path(&task.path);
    // Enforce that the resolved path is inside the wasm_files sandbox directory.
    // This blocks path-traversal if task.path was tampered with in the database.
    let safe_path = validate_wasm_path_boundary(&file_path)?;
    let bytes = fs::read(&safe_path)
        .map_err(|e| WasmOsError::Validation(format!("Cannot read file for analysis: {e}")))?;

    // If .wat, compile to .wasm bytes first for analysis
    let bytes = if file_path.ends_with(".wat") {
        wat::parse_bytes(&bytes)
            .map(|b| b.into_owned())
            .unwrap_or(bytes)
    } else {
        bytes
    };

    let (imports, exports, suspicious) = analyse_wasm_bytes(&bytes);

    let mut capabilities: Vec<SecurityCapability> = Vec::new();

    // File size rating
    if task.file_size_bytes > 10_000_000 {
        capabilities.push(SecurityCapability {
            name: "Large Binary".into(),
            description: format!("{} bytes — potentially packed or polyglot", task.file_size_bytes),
            level: "warn".into(),
        });
    } else {
        capabilities.push(SecurityCapability {
            name: "Binary Size".into(),
            description: format!("{} bytes — within normal range", task.file_size_bytes),
            level: "info".into(),
        });
    }

    // Imports
    capabilities.push(SecurityCapability {
        name: "Import Count".into(),
        description: format!("{} host import(s) declared", imports.len()),
        level: if imports.len() > 20 { "warn" } else { "info" }.into(),
    });

    // Exports
    capabilities.push(SecurityCapability {
        name: "Export Count".into(),
        description: format!("{} export(s) declared", exports.len()),
        level: "info".into(),
    });

    // Suspicious findings
    for s in &suspicious {
        capabilities.push(SecurityCapability {
            name: "Suspicious Capability".into(),
            description: s.clone(),
            level: if s.contains("File") || s.contains("Network") || s.contains("Process") {
                "severe"
            } else {
                "warn"
            }.into(),
        });
    }

    let has_severe = capabilities.iter().any(|c| c.level == "severe");
    let has_warn   = capabilities.iter().any(|c| c.level == "warn");
    let risk_level = if has_severe { "high" } else if has_warn { "medium" } else { "low" }.to_string();
    let summary = format!(
        "{} import(s), {} export(s), {} suspicious capability(ies) — Risk: {}",
        imports.len(), exports.len(), suspicious.len(), risk_level
    );

    Ok(HttpResponse::Ok().json(SecurityReport {
        task_id: task.id,
        task_name: task.name,
        file_size_bytes: task.file_size_bytes,
        imports,
        exports,
        capabilities,
        risk_level,
        summary,
    }))
}

// ─── Execution Logs ──────────────────────────────────────────────────────────

#[get("/v1/tasks/{id}/logs")]
pub async fn get_task_logs(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();

    // Validate task exists
    let task = data
        .task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;

    // Fetch most recent execution history entry
    let history = data.task_repo.get_execution_history(&id, 1).await?;

    if let Some(last) = history.first() {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "task_id": id,
            "task_name": task.name,
            "started_at": last.started_at,
            "completed_at": last.completed_at,
            "duration_us": last.duration_us,
            "success": last.success,
            "error": last.error,
            "instructions_executed": last.instructions_executed,
            "syscalls_executed": last.syscalls_executed,
            "memory_used_bytes": last.memory_used_bytes,
            "stdout_log": [],
        })))
    } else {
        Ok(HttpResponse::Ok().json(serde_json::json!({
            "task_id": id,
            "task_name": task.name,
            "started_at": null,
            "completed_at": null,
            "duration_us": null,
            "success": null,
            "error": "No executions recorded yet",
            "instructions_executed": 0,
            "syscalls_executed": 0,
            "memory_used_bytes": 0,
            "stdout_log": [],
        })))
    }
}

// ─── Auth: issue JWT token ────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TokenRequest {
    /// Optional subject/username — defaults to "admin" if omitted.
    /// The frontend login page only sends admin_key, not user_id.
    #[serde(default = "default_user_id")]
    pub user_id: String,
    /// Optional role — defaults to "admin" when logging in with the admin key.
    pub role: Option<String>,
    /// Must match WASMOS__SECURITY__ADMIN_KEY in config (or "changeme" in dev).
    pub admin_key: Option<String>,
}

fn default_user_id() -> String {
    "admin".to_string()
}

/// POST /v1/auth/token — exchange admin_key for a signed JWT.
///
/// This endpoint is intentionally **exempt from JWT middleware** (the bootstrap
/// problem: you need to call this to get a token in the first place).
/// It is, however, protected by the admin_key secret configured via
/// `WASMOS__SECURITY__ADMIN_KEY` (defaults to "changeme" — change in production).
#[post("/v1/auth/token")]
pub async fn get_token(
    data: web::Data<AppState>,
    body: web::Json<TokenRequest>,
) -> Result<impl Responder> {
    // Validate the admin key before issuing any token.
    // Return 401 (not 400) to give no hint about whether the key was missing vs wrong.
    let expected_key = &data.config.security.admin_key;
    let provided_key = body.admin_key.as_deref().unwrap_or("");
    if expected_key.is_empty() || provided_key != expected_key.as_str() {
        return Err(WasmOsError::Unauthorized("Invalid credentials".into()));
    }

    #[cfg(feature = "jwt-auth")]
    {
        let role = body.role.as_deref().unwrap_or("admin");
        match data.auth_service.generate_token(&body.user_id, role) {
            Ok(token) => {
                tracing::info!(user_id = %body.user_id, role = role, "JWT issued");
                Ok(HttpResponse::Ok().json(serde_json::json!({
                    "token":      token,
                    "expires_in": data.auth_service.expiry_hours * 3600,
                    "token_type": "Bearer",
                    "role":       role,
                    "user_id":    body.user_id,
                })))
            }
            Err(e) => Err(WasmOsError::ExecutionError(e.to_string())),
        }
    }
    #[cfg(not(feature = "jwt-auth"))]
    {
        let _ = (data, body);
        Err(WasmOsError::Validation(
            "JWT auth not compiled into this build (feature 'jwt-auth' required)".into()
        ))
    }
}

// ─── Snapshots ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CreateSnapshotRequest {
    pub memory_mb: f64,
    pub instructions: i64,
    pub stack_depth: i32,
    pub globals_json: Option<serde_json::Value>,
    pub note: Option<String>,
}

#[get("/v1/tasks/{id}/snapshots")]
pub async fn list_snapshots(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    // Ensure task exists
    data.task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;

    let snapshots = data.task_repo.list_snapshots(&id).await?;
    Ok(HttpResponse::Ok().json(snapshots))
}

#[post("/v1/tasks/{id}/snapshots")]
pub async fn create_snapshot(
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<CreateSnapshotRequest>,
) -> Result<impl Responder> {
    let task_id = path.into_inner();
    data.task_repo
        .get_by_id(&task_id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(task_id.clone()))?;

    let globals_json = body.globals_json
        .as_ref()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "{}".into());

    let mut snapshot = Snapshot::new(
        task_id.clone(),
        "manual".to_string(),
        body.memory_mb as f32,
        body.instructions,
        body.stack_depth,
    );
    snapshot.globals_json = globals_json;
    snapshot.note = body.note.clone();

    data.task_repo.create_snapshot(&snapshot).await?;

    Ok(HttpResponse::Created().json(snapshot))
}

#[delete("/v1/snapshots/{id}")]
pub async fn delete_snapshot(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    // Verify snapshot exists first
    match data.task_repo.get_snapshot(&id).await? {
        None => Err(WasmOsError::NotFound(format!("Snapshot {id}"))),
        Some(_) => {
            data.task_repo.delete_snapshot(&id).await?;
            Ok(HttpResponse::Ok().json(serde_json::json!({"deleted": true, "id": id})))
        }
    }
}

// ─── Audit Log ────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AuditQuery {
    pub limit: Option<i64>,
    pub action: Option<String>,
}

#[get("/v1/audit")]
pub async fn list_audit_log(
    data: web::Data<AppState>,
    query: web::Query<AuditQuery>,
) -> Result<impl Responder> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let logs = if let Some(ref action) = query.action {
        data.task_repo.list_audit_log_filtered(action, limit).await?
    } else {
        data.task_repo.list_audit_log(limit).await?
    };
    // Wrap in a pagination envelope so the React frontend can destructure
    // `{logs, total, page, per_page}` without branching on array vs object.
    let total = logs.len();
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "logs":     logs,
        "total":    total,
        "page":     1,
        "per_page": limit,
    })))
}

// ─── Tenants ──────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct CreateTenantRequest {
    pub name: String,
    pub max_tasks: Option<i32>,
    pub max_memory_mb: Option<i32>,
    pub max_cpu_percent: Option<i16>,
    pub max_concurrent: Option<i32>,
    pub max_wasm_size_mb: Option<i32>,
}

#[get("/v1/tenants")]
pub async fn list_tenants(data: web::Data<AppState>) -> Result<impl Responder> {
    let tenants = data.task_repo.list_tenants().await?;
    Ok(HttpResponse::Ok().json(tenants))
}

#[post("/v1/tenants")]
pub async fn create_tenant(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<CreateTenantRequest>,
) -> Result<impl Responder> {
    if body.name.is_empty() || body.name.len() > 128 {
        return Err(WasmOsError::Validation("Tenant name must be 1-128 chars".into()));
    }
    let tenant_id = uuid::Uuid::new_v4().to_string();
    data.task_repo.create_tenant(
        &tenant_id,
        &body.name,
        body.max_tasks.unwrap_or(100),
        body.max_memory_mb.unwrap_or(512),
        body.max_cpu_percent.unwrap_or(80),
        body.max_concurrent.unwrap_or(10),
        body.max_wasm_size_mb.unwrap_or(50),
    ).await.map_err(|e| {
        // Postgres unique_violation code = 23505
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.code().as_deref() == Some("23505") {
                return WasmOsError::Validation(format!("Tenant name '{}' already exists", body.name));
            }
        }
        WasmOsError::Database(e)
    })?;
    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "admin", "tenant.create", Some(&tenant_id), Some(&body.name), Some(&ip)).await;
    Ok(HttpResponse::Created().json(serde_json::json!({
        "id": tenant_id,
        "name": body.name,
    })))
}

#[delete("/v1/tenants/{id}")]
pub async fn delete_tenant(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let tenant_id = path.into_inner();
    data.task_repo.delete_tenant(&tenant_id).await?;
    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "admin", "tenant.delete", Some(&tenant_id), None, Some(&ip)).await;
    Ok(HttpResponse::Ok().json(serde_json::json!({"deleted": true, "id": tenant_id})))
}

// ─── Execution History ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ExecutionHistoryQuery {
    pub limit: Option<i64>,
}

/// GET /v1/tasks/{id}/execution-history
/// Returns paginated execution history for a specific task.
#[get("/v1/tasks/{id}/execution-history")]
pub async fn get_task_execution_history(
    data: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<ExecutionHistoryQuery>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    let limit = query.limit.unwrap_or(50).clamp(1, 500);

    // Validate task exists
    data.task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;

    let history = data.task_repo.get_execution_history(&id, limit).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({
        "task_id": id,
        "count": history.len(),
        "executions": history,
    })))
}

// ─── Update Task ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub priority: Option<i16>,
}

/// PUT /v1/tasks/{id} — update task name and/or priority
#[put("/v1/tasks/{id}")]
pub async fn update_task(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
    body: web::Json<UpdateTaskRequest>,
) -> Result<impl Responder> {
    let id = path.into_inner();

    let task = data.task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;

    // Validate name if provided
    if let Some(ref n) = body.name {
        if n.is_empty() || n.len() > 255 {
            return Err(WasmOsError::Validation("name must be 1-255 chars".into()));
        }
    }
    // Validate priority if provided
    if let Some(p) = body.priority {
        if !(1..=10).contains(&p) {
            return Err(WasmOsError::Validation("priority must be 1-10".into()));
        }
    }

    data.task_repo
        .update_task(&id, body.name.as_deref(), body.priority)
        .await?;

    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "system", "task.update", Some(&id), task.tenant_id.as_deref(), Some(&ip)).await;
    data.query_cache.invalidate_tasks().await;

    // Return updated task
    let updated = data.task_repo.get_by_id(&id).await?.unwrap_or(task);
    Ok(HttpResponse::Ok().json(TaskResponse::from(updated)))
}

// ─── Pause Task (alias for stop — WASM execution is not preemptible) ─────────

/// POST /v1/tasks/{id}/pause — stops a running task (WASM is not preemptible)
#[post("/v1/tasks/{id}/pause")]
pub async fn pause_task(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();

    let task = data.task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;

    if task.status != TaskStatus::Running {
        return Err(WasmOsError::TaskNotRunning(id));
    }

    data.task_repo.update_status(&id, TaskStatus::Stopped).await?;
    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "system", "task.pause", Some(&id), task.tenant_id.as_deref(), Some(&ip)).await;
    data.query_cache.invalidate_tasks().await;
    data.query_cache.invalidate_stats().await;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "stopped",
        "note": "WASM execution is not preemptible; task has been stopped"
    })))
}

// ─── Restart Task ─────────────────────────────────────────────────────────────

/// POST /v1/tasks/{id}/restart — resets status to Pending so the scheduler picks it up
#[post("/v1/tasks/{id}/restart")]
pub async fn restart_task(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();

    let task = data.task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| WasmOsError::TaskNotFound(id.clone()))?;

    // Only allow restarting stopped or failed tasks
    if task.status == TaskStatus::Running {
        return Err(WasmOsError::TaskAlreadyRunning(id));
    }
    if task.status == TaskStatus::Pending {
        return Ok(HttpResponse::Ok().json(serde_json::json!({
            "status": "pending",
            "note": "Task is already queued for execution"
        })));
    }

    data.task_repo.update_status(&id, TaskStatus::Pending).await?;

    let ip = extract_ip(&req);
    let _ = data.task_repo.write_audit("api", "system", "task.restart", Some(&id), task.tenant_id.as_deref(), Some(&ip)).await;
    data.query_cache.invalidate_tasks().await;
    data.query_cache.invalidate_stats().await;

    let _ = data.event_tx.send(TaskEvent {
        event: "restarted".into(),
        task_id: id.clone(),
        task_name: task.name.clone(),
        status: "pending".into(),
    });

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "status": "pending",
        "note": "Task queued for re-execution by the scheduler"
    })))
}

// ─── Get Single Snapshot ──────────────────────────────────────────────────────

/// GET /v1/snapshots/{id} — fetch a single snapshot by its ID
#[get("/v1/snapshots/{id}")]
pub async fn get_snapshot(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    match data.task_repo.get_snapshot(&id).await? {
        Some(snap) => Ok(HttpResponse::Ok().json(snap)),
        None => Err(WasmOsError::NotFound(format!("Snapshot {id}"))),
    }
}

// ─── Get Single Tenant ────────────────────────────────────────────────────────

/// GET /v1/tenants/{id} — fetch a single tenant
#[get("/v1/tenants/{id}")]
pub async fn get_tenant(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    match data.task_repo.get_tenant_by_id(&id).await? {
        Some(t) => Ok(HttpResponse::Ok().json(t)),
        None => Err(WasmOsError::NotFound(format!("Tenant {id}"))),
    }
}

// ─── Scheduler Status ─────────────────────────────────────────────────────────

/// GET /v1/scheduler/status — live scheduler snapshot (queue depth, running, slice config)
#[get("/v1/scheduler/status")]
pub async fn scheduler_status(data: web::Data<AppState>) -> impl Responder {
    let snap = data.scheduler.status_snapshot().await;
    HttpResponse::Ok().json(snap)
}

/// POST /v1/scheduler/preempt/{task_id} — forcefully cancel a running task
#[post("/v1/scheduler/preempt/{task_id}")]
pub async fn scheduler_preempt(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let task_id = path.into_inner();
    let preempted = data.scheduler.preempt_task(&task_id).await;
    if preempted {
        HttpResponse::Ok().json(serde_json::json!({
            "task_id": task_id,
            "preempted": true,
            "message": "Task cancelled by preemption"
        }))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({
            "error": "Task not currently running",
            "task_id": task_id
        }))
    }
}

// ─── Capability Tokens ───────────────────────────────────────────────────────

/// POST /v1/tokens — issue a new capability token
/// POST /v1/tokens — issue a new capability token (admin only)
#[post("/v1/tokens")]
pub async fn issue_token(
    req: HttpRequest,
    data: web::Data<AppState>,
    body: web::Json<IssueTokenRequest>,
) -> Result<impl Responder> {
    require_admin(&req, &data)?;
    let request = body.into_inner();
    let caps: HashSet<_> = request.capabilities.into_iter().collect();
    let token = data
        .cap_registry
        .issue(request.label, request.subject, request.tenant_id, caps, request.ttl_hours)
        .await;
    let resp = IssueTokenResponse::from(token);
    Ok(HttpResponse::Created().json(resp))
}

/// GET /v1/tokens — list all tokens (admin only)
#[get("/v1/tokens")]
pub async fn list_tokens(req: HttpRequest, data: web::Data<AppState>) -> Result<impl Responder> {
    require_admin(&req, &data)?;
    let all = data.cap_registry.list_all().await;
    let summaries: Vec<TokenSummary> = all.into_iter().map(TokenSummary::from).collect();
    Ok(HttpResponse::Ok().json(summaries))
}

/// DELETE /v1/tokens/{id} — revoke a token (admin only)
#[delete("/v1/tokens/{id}")]
pub async fn revoke_token(
    req: HttpRequest,
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    require_admin(&req, &data)?;
    let id = path.into_inner();
    let revoked = data.cap_registry.revoke(&id).await;
    if revoked {
        Ok(HttpResponse::Ok().json(serde_json::json!({ "revoked": true, "token_id": id })))
    } else {
        Ok(HttpResponse::NotFound().json(serde_json::json!({ "error": "Token not found", "token_id": id })))
    }
}

/// GET /v1/tokens/check — verify a token has a given capability
/// Query: ?token_id=...&capability=task_read
#[get("/v1/tokens/check")]
pub async fn check_token(
    data: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let token_id = query.get("token_id").cloned().unwrap_or_default();
    let cap_str = query.get("capability").cloned().unwrap_or_default();
    let cap: crate::capability::Capability = match cap_str.to_lowercase().as_str() {
        "task_read" => crate::capability::Capability::TaskRead,
        "task_write" => crate::capability::Capability::TaskWrite,
        "task_execute" => crate::capability::Capability::TaskExecute,
        "task_delete" => crate::capability::Capability::TaskDelete,
        "metrics_read" => crate::capability::Capability::MetricsRead,
        "metrics_system" => crate::capability::Capability::MetricsSystem,
        "tenant_admin" => crate::capability::Capability::TenantAdmin,
        "snapshot_read" => crate::capability::Capability::SnapshotRead,
        "snapshot_write" => crate::capability::Capability::SnapshotWrite,
        "terminal_access" => crate::capability::Capability::TerminalAccess,
        "audit_read" => crate::capability::Capability::AuditRead,
        "admin" => crate::capability::Capability::Admin,
        _ => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": "Unknown capability", "capability": cap_str }));
        }
    };
    let valid = data.cap_registry.check(&token_id, &cap).await;
    HttpResponse::Ok().json(serde_json::json!({
        "token_id": token_id,
        "capability": cap_str,
        "granted": valid,
    }))
}

// ─── Distributed Tracing ─────────────────────────────────────────────────────

/// GET /v1/traces — recent traces (default 50)
#[get("/v1/traces")]
pub async fn list_traces(
    data: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let limit: usize = query
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50)
        .min(200);
    let traces = data.trace_store.recent(limit).await;
    HttpResponse::Ok().json(traces)
}

/// GET /v1/traces/{task_id} — all traces for a given task
#[get("/v1/traces/{task_id}")]
pub async fn get_task_traces(
    data: web::Data<AppState>,
    path: web::Path<String>,
) -> impl Responder {
    let task_id = path.into_inner();
    let traces = data.trace_store.for_task(&task_id).await;
    HttpResponse::Ok().json(traces)
}

/// GET /v1/traces/metrics/live — computed P50/P95/P99 + error rate + throughput
#[get("/v1/traces/metrics/live")]
pub async fn live_trace_metrics(
    data: web::Data<AppState>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> impl Responder {
    let window: usize = query
        .get("window")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100)
        .min(500);
    let metrics = data.trace_store.live_metrics(window).await;
    HttpResponse::Ok().json(metrics)
}

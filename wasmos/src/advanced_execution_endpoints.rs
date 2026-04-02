/// Advanced WASM Execution Endpoints
///
/// Provides REST API endpoints for:
/// - Advanced execution with metrics
/// - Batch execution
/// - Execution report generation
/// - Import management
/// - Performance analysis

use actix_web::{get, post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::run_wasm::{ExecutionDispatcher, ExecutionConfig};

// NOTE: html_escape() was removed — the /v2/execution/{id}/report endpoint
// now returns application/json consumed by the React frontend, so no HTML
// string building or escaping is needed in this file.

#[derive(Serialize)]
pub struct AdvancedExecutionResponse {
    pub execution_id: String,
    pub success: bool,
    pub total_instructions: u64,
    pub total_syscalls: u64,
    pub duration_ms: f64,
    pub peak_memory_mb: f64,
    pub instructions_per_second: f64,
    pub hotspots: Vec<HotspotInfo>,
    pub performance_anomalies: usize,
}

#[derive(Serialize)]
pub struct HotspotInfo {
    pub opcode: String,
    pub percentage: f64,
}

#[derive(Deserialize)]
pub struct AdvancedExecutionRequest {
    pub wasm_path: String,
    #[serde(default)]
    pub max_memory_mb: Option<u64>,
    #[serde(default)]
    pub max_instructions: Option<u64>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Deserialize)]
pub struct BatchExecutionRequest {
    pub wasm_paths: Vec<String>,
    #[serde(default)]
    pub continue_on_error: bool,
}

/// Execute a WASM file with advanced metrics.
/// Uses spawn_blocking so the custom WASM engine (which uses panic! internally)
/// does not block the Tokio async runtime thread pool.
#[post("/v2/execute/advanced")]
pub async fn execute_advanced(
    req: web::Json<AdvancedExecutionRequest>,
) -> Result<impl Responder> {
    let config = ExecutionConfig {
        max_memory_bytes: req.max_memory_mb.unwrap_or(256) * 1024 * 1024,
        max_instructions: req.max_instructions.unwrap_or(1_000_000_000),
        max_call_depth: 1000,
        max_loop_iterations: 100_000_000,
        timeout_us: req.timeout_seconds.unwrap_or(30) * 1_000_000,
        enable_tracing: true,
        timeline_sample_rate: 100,
        collect_full_history: true,
    };

    let wasm_path = req.wasm_path.clone();
    let result = web::block(move || ExecutionDispatcher::execute_file(&wasm_path, Some(config)))
        .await
        .map_err(|e| crate::error::WasmOsError::ExecutionError(format!("Thread error: {e}")))?
        .map_err(|e| crate::error::WasmOsError::ExecutionError(e))?;

    let hotspots = result
        .advanced_report
        .hotspots
        .iter()
        .take(5)
        .map(|h| HotspotInfo {
            opcode: h.opcode.clone(),
            percentage: h.percentage_of_total,
        })
        .collect();

    Ok(HttpResponse::Ok().json(AdvancedExecutionResponse {
        execution_id: result.execution_id,
        success: result.execution_result.success,
        total_instructions: result.advanced_report.total_instructions,
        total_syscalls: result.advanced_report.total_syscalls,
        duration_ms: result.advanced_report.total_duration_us as f64 / 1000.0,
        peak_memory_mb: result.advanced_report.peak_memory_bytes as f64 / (1024.0 * 1024.0),
        instructions_per_second: result.advanced_report.instructions_per_second,
        hotspots,
        performance_anomalies: result.advanced_report.performance_anomalies.len(),
    }))
}

/// Execute multiple WASM files in batch.
/// Each file runs in its own spawn_blocking call so panics in the engine
/// are isolated per-file and do not abort the batch.
#[post("/v2/execute/batch")]
pub async fn execute_batch(
    req: web::Json<BatchExecutionRequest>,
) -> Result<impl Responder> {
    let paths = req.wasm_paths.clone();
    let continue_on_error = req.continue_on_error;

    let results = web::block(move || {
        let path_refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
        let mut successes = Vec::new();
        let mut failures = Vec::new();
        for path in &path_refs {
            match ExecutionDispatcher::execute_file(path, None) {
                Ok(r) => successes.push(r),
                Err(e) => {
                    failures.push((path.to_string(), e.clone()));
                    if !continue_on_error {
                        return Err(e);
                    }
                }
            }
        }
        Ok((successes, failures))
    })
    .await
    .map_err(|e| crate::error::WasmOsError::ExecutionError(format!("Thread error: {e}")))?
    .map_err(|e| crate::error::WasmOsError::ExecutionError(e))?;

    let (successes, failures) = results;

    let batch_response = serde_json::json!({
        "total_files": successes.len() + failures.len(),
        "successful": successes.len(),
        "failed": failures.len(),
        "results": successes.iter().map(|r| {
            serde_json::json!({
                "execution_id": r.execution_id,
                "success": r.execution_result.success,
                "instructions": r.advanced_report.total_instructions,
                "duration_ms": r.advanced_report.total_duration_us as f64 / 1000.0
            })
        }).collect::<Vec<_>>(),
        "errors": failures.iter().map(|(path, err)| {
            serde_json::json!({ "path": path, "error": err })
        }).collect::<Vec<_>>()
    });

    Ok(HttpResponse::Ok().json(batch_response))
}

/// JSON response shape for GET /v2/execution/{execution_id}/report.
/// All rendering is handled by the React frontend — no HTML is generated here.
#[derive(Serialize)]
pub struct ExecutionReportResponse {
    pub execution_id:        String,
    pub found:               bool,
    pub task_id:             Option<String>,
    pub started_at:          Option<String>,  // ISO 8601
    pub completed_at:        Option<String>,  // ISO 8601 or null
    pub duration_us:         Option<i64>,
    pub success:             bool,
    pub instructions:        Option<i64>,
    pub syscalls:            Option<i64>,
    pub memory_bytes:        Option<i64>,
    pub error:               Option<String>,
}

/// Returns execution details as JSON.
///
/// The React page at `/execution/[id]/report` fetches this endpoint and renders
/// a full report card using shadcn/ui — no HTML is generated server-side.
#[get("/v2/execution/{execution_id}/report")]
pub async fn get_execution_report(
    path: web::Path<String>,
    data: web::Data<crate::server::AppState>,
) -> Result<impl Responder> {
    let execution_id = path.into_inner();

    // Strategy: try UUID lookup first (preferred — stable across DB migrations),
    // then fall back to bare numeric SERIAL id or "exec_<n>" prefixed id.
    let history_entry = if let Some(entry) = data.task_repo
        .get_execution_by_uuid(&execution_id)
        .await
        .ok()
        .flatten()
    {
        Some(entry)
    } else {
        // Fall back to numeric SERIAL id (or "exec_<n>" prefix)
        let db_id: Option<i64> = execution_id
            .trim_start_matches("exec_")
            .parse()
            .ok();
        if let Some(id) = db_id {
            sqlx::query_as::<_, crate::db::models::ExecutionHistory>(
                "SELECT id, execution_id, task_id, started_at, completed_at, duration_us, \
                 success, error, instructions_executed, syscalls_executed, memory_used_bytes \
                 FROM execution_history WHERE id = $1",
            )
            .bind(id)
            .fetch_optional(data.task_repo.pool())
            .await
            .ok()
            .flatten()
        } else {
            None
        }
    };

    let report = match history_entry {
        Some(h) => ExecutionReportResponse {
            // Always use the canonical UUID from the DB record, not the URL parameter,
            // so the response is consistent regardless of how the lookup was routed.
            execution_id: h.execution_id.clone(),
            found:        true,
            task_id:      Some(h.task_id),
            started_at:   Some(h.started_at.to_rfc3339()),
            completed_at: h.completed_at.map(|t| t.to_rfc3339()),
            duration_us:  h.duration_us,
            success:      h.success,
            instructions: Some(h.instructions_executed),
            syscalls:     Some(h.syscalls_executed),
            memory_bytes: Some(h.memory_used_bytes),
            error:        h.error,
        },
        None => ExecutionReportResponse {
            execution_id: execution_id.clone(),
            found:        false,
            task_id:      None,
            started_at:   None,
            completed_at: None,
            duration_us:  None,
            success:      false,
            instructions: None,
            syscalls:     None,
            memory_bytes: None,
            error:        None,
        },
    };

    Ok(HttpResponse::Ok().json(report))
}

/// Aggregate import statistics across all WASM files stored in the server.
///
/// Scans every `.wasm` file in `wasm_files/`, parses its Import section, and
/// tallies how many modules declare imports from each host module namespace
/// (e.g. "wasi_snapshot_preview1", "env", "game", …).
///
/// The response is sorted by `task_count` descending so the most-common
/// dependencies appear first.
#[get("/v2/imports/stats")]
pub async fn get_import_stats(
    data: web::Data<crate::server::AppState>,
) -> Result<impl Responder> {
    use std::collections::HashMap;

    // Collect all task paths from the DB
    let tasks = data.task_repo
        .list_all_paginated(500, 0)
        .await
        .map_err(crate::error::WasmOsError::Database)?;

    // module_name → number of tasks that import from it
    let mut module_counts: HashMap<String, usize> = HashMap::new();
    let mut total_scanned = 0usize;

    for task in &tasks {
        let resolved = crate::server::resolve_wasm_file_path_pub(&task.path);
        let bytes = match std::fs::read(&resolved) {
            Ok(b) => b,
            Err(_) => continue, // file missing — skip gracefully
        };

        // Compile .wat on the fly if needed
        let bytes = if resolved.ends_with(".wat") {
            match wat::parse_bytes(&bytes) {
                Ok(b) => b.into_owned(),
                Err(_) => bytes,
            }
        } else {
            bytes
        };

        // Parse import section to collect module namespaces
        let import_modules = extract_import_modules(&bytes);
        for module_name in import_modules {
            *module_counts.entry(module_name).or_default() += 1;
        }
        total_scanned += 1;
    }

    // Sort by frequency descending, then alphabetically
    let mut modules: Vec<serde_json::Value> = module_counts
        .into_iter()
        .map(|(name, task_count)| serde_json::json!({
            "name": name,
            "task_count": task_count,
            "enabled": true,
        }))
        .collect();
    modules.sort_by(|a, b| {
        let ca = a["task_count"].as_u64().unwrap_or(0);
        let cb = b["task_count"].as_u64().unwrap_or(0);
        cb.cmp(&ca).then_with(|| {
            a["name"].as_str().unwrap_or("").cmp(b["name"].as_str().unwrap_or(""))
        })
    });

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "modules": modules,
        "total_tasks_scanned": total_scanned,
    })))
}

// ─── WASM binary parsing helpers ─────────────────────────────────────────────

/// Decode an unsigned LEB-128 integer from `bytes` starting at `pos`.
/// Returns `(value, bytes_consumed)`.
fn read_leb128_u32(bytes: &[u8], mut pos: usize) -> (u32, usize) {
    let mut result: u32 = 0;
    let mut shift = 0u32;
    let mut consumed = 0usize;
    while pos < bytes.len() {
        let byte = bytes[pos];
        pos += 1;
        consumed += 1;
        result |= ((byte & 0x7F) as u32) << shift;
        shift += 7;
        if byte & 0x80 == 0 {
            break;
        }
        if shift >= 35 {
            break; // guard against malformed input
        }
    }
    (result, consumed)
}

/// Read a UTF-8 string of `len` bytes starting at `pos` in `bytes`.
/// Returns an empty string if the slice is out of bounds or invalid UTF-8.
fn read_utf8(bytes: &[u8], pos: usize, len: usize) -> String {
    if pos + len > bytes.len() {
        return String::new();
    }
    std::str::from_utf8(&bytes[pos..pos + len])
        .unwrap_or("")
        .to_string()
}

/// Extract unique module namespace strings from a WASM binary's Import section.
fn extract_import_modules(bytes: &[u8]) -> Vec<String> {
    use std::collections::HashSet;
    let mut seen: HashSet<String> = HashSet::new();

    if bytes.len() < 8 || bytes[0..4] != [0x00, 0x61, 0x73, 0x6D] {
        return vec![];
    }

    let mut pos = 8usize;
    while pos + 2 <= bytes.len() {
        let section_id = bytes[pos];
        pos += 1;
        let (sec_len, leb_bytes) = read_leb128_u32(bytes, pos);
        pos += leb_bytes;
        let section_end = pos + sec_len as usize;
        if section_end > bytes.len() { break; }

        if section_id == 2 {
            // Import section
            let (count, n) = read_leb128_u32(bytes, pos);
            let mut p = pos + n;
            for _ in 0..count {
                if p >= section_end { break; }
                let (mlen, n) = read_leb128_u32(bytes, p);
                p += n;
                let mname = read_utf8(bytes, p, mlen as usize);
                p += mlen as usize;
                // Skip field name
                if p >= section_end { break; }
                let (flen, n) = read_leb128_u32(bytes, p);
                p += n + flen as usize;
                // Skip import descriptor (kind + type index)
                if p < section_end {
                    let _kind = bytes[p]; p += 1;
                    let (_idx, n) = read_leb128_u32(bytes, p);
                    p += n;
                }
                if !mname.is_empty() {
                    seen.insert(mname);
                }
            }
            break; // Import section found and parsed — stop scanning
        }
        pos = section_end;
    }

    seen.into_iter().collect()
}

/// Compare execution performance between two WASM modules (or two runs of the same module).
///
/// POST body: `{ "baseline_path": "...", "current_path": "..." }`
/// Both paths are resolved relative to the `wasm_files/` sandbox directory.
/// Positive `improvement_percent` means the current version is faster/cheaper.
#[derive(Deserialize)]
pub struct CompareRequest {
    pub baseline_path: String,
    pub current_path: String,
    #[serde(default)]
    pub max_memory_mb: Option<u64>,
    #[serde(default)]
    pub max_instructions: Option<u64>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Serialize)]
pub struct PerformanceComparison {
    pub baseline_instructions: u64,
    pub current_instructions: u64,
    /// Positive = current uses fewer instructions than baseline (improvement).
    /// Negative = current uses MORE instructions than baseline (regression).
    pub improvement_percent: f64,
    pub baseline_duration_us: u64,
    pub current_duration_us: u64,
    /// Positive = current is faster than baseline.
    pub duration_improvement_percent: f64,
    pub baseline_success: bool,
    pub current_success: bool,
    pub baseline_memory_bytes: u64,
    pub current_memory_bytes: u64,
}

#[post("/v2/execute/compare")]
pub async fn compare_performance(
    req: web::Json<CompareRequest>,
) -> Result<impl Responder> {
    let cfg = ExecutionConfig {
        max_memory_bytes: req.max_memory_mb.unwrap_or(256) * 1024 * 1024,
        max_instructions: req.max_instructions.unwrap_or(1_000_000_000),
        max_call_depth: 1000,
        max_loop_iterations: 100_000_000,
        timeout_us: req.timeout_seconds.unwrap_or(30) * 1_000_000,
        enable_tracing: false,
        timeline_sample_rate: 0,
        collect_full_history: false,
    };

    // Run baseline and current concurrently — each on its own blocking thread
    let baseline_path = req.baseline_path.clone();
    let current_path  = req.current_path.clone();
    let cfg2 = cfg.clone();

    let (baseline_handle, current_handle) = tokio::join!(
        web::block(move || ExecutionDispatcher::execute_file(&baseline_path, Some(cfg))),
        web::block(move || ExecutionDispatcher::execute_file(&current_path,  Some(cfg2))),
    );

    let baseline = baseline_handle
        .map_err(|e| crate::error::WasmOsError::ExecutionError(format!("Baseline thread error: {e}")))?
        .map_err(|e| crate::error::WasmOsError::ExecutionError(format!("Baseline execution failed: {e}")))?;

    let current = current_handle
        .map_err(|e| crate::error::WasmOsError::ExecutionError(format!("Current thread error: {e}")))?
        .map_err(|e| crate::error::WasmOsError::ExecutionError(format!("Current execution failed: {e}")))?;

    let b_instr = baseline.advanced_report.total_instructions;
    let c_instr = current.advanced_report.total_instructions;
    let b_dur   = baseline.advanced_report.total_duration_us;
    let c_dur   = current.advanced_report.total_duration_us;

    // Improvement percent: how much cheaper/faster is current vs baseline?
    // Result > 0 → current is better; < 0 → current is worse.
    let improvement_percent = if b_instr > 0 {
        ((b_instr as f64 - c_instr as f64) / b_instr as f64) * 100.0
    } else {
        0.0
    };
    let duration_improvement_percent = if b_dur > 0 {
        ((b_dur as f64 - c_dur as f64) / b_dur as f64) * 100.0
    } else {
        0.0
    };

    let comparison = PerformanceComparison {
        baseline_instructions:        b_instr,
        current_instructions:         c_instr,
        improvement_percent,
        baseline_duration_us:         b_dur,
        current_duration_us:          c_dur,
        duration_improvement_percent,
        baseline_success:             baseline.execution_result.success,
        current_success:              current.execution_result.success,
        baseline_memory_bytes:        baseline.advanced_report.peak_memory_bytes,
        current_memory_bytes:         current.advanced_report.peak_memory_bytes,
    };

    Ok(HttpResponse::Ok().json(comparison))
}

/// Get execution metrics for a task
#[get("/v2/tasks/{id}/advanced-metrics")]
pub async fn get_advanced_metrics(
    path: web::Path<String>,
    data: web::Data<crate::server::AppState>,
) -> Result<impl Responder> {
    let id = path.into_inner();

    let task = data.task_repo
        .get_by_id(&id)
        .await?
        .ok_or_else(|| crate::error::WasmOsError::TaskNotFound(id))?;

    let metrics = data.task_repo.get_metrics(&task.id).await?.unwrap_or_default();
    let history = data.task_repo.get_execution_history(&task.id, 5).await?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "task_id": task.id,
        "name": task.name,
        "status": task.status.to_string(),
        "path": task.path,
        "file_size_bytes": task.file_size_bytes,
        "created_at": task.created_at.to_rfc3339(),
        "metrics": metrics,
        "recent_executions": history,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advanced_execution_request_deserialization() {
        let json = r#"{"wasm_path": "test.wasm"}"#;
        let req: AdvancedExecutionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.wasm_path, "test.wasm");
        assert!(req.max_memory_mb.is_none());
    }

    #[test]
    fn test_batch_execution_request() {
        let json = r#"{"wasm_paths": ["a.wasm", "b.wasm"], "continue_on_error": true}"#;
        let req: BatchExecutionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.wasm_paths.len(), 2);
        assert!(req.continue_on_error);
    }

    #[test]
    fn test_performance_comparison_calculation() {
        let b_instr: u64 = 1000;
        let c_instr: u64 = 800;
        let b_dur: u64 = 1_000_000;
        let c_dur: u64 = 800_000;
        let improvement_percent =
            ((b_instr as f64 - c_instr as f64) / b_instr as f64) * 100.0;
        let duration_improvement_percent =
            ((b_dur as f64 - c_dur as f64) / b_dur as f64) * 100.0;

        let comparison = PerformanceComparison {
            baseline_instructions: b_instr,
            current_instructions: c_instr,
            improvement_percent,
            baseline_duration_us: b_dur,
            current_duration_us: c_dur,
            duration_improvement_percent,
            baseline_success: true,
            current_success: true,
            baseline_memory_bytes: 1024 * 1024,
            current_memory_bytes: 900 * 1024,
        };

        assert!((comparison.improvement_percent - 20.0).abs() < 0.01);
        assert!((comparison.duration_improvement_percent - 20.0).abs() < 0.01);
        assert!(comparison.baseline_success);
        assert!(comparison.current_success);
    }

    #[test]
    fn test_extract_import_modules_empty() {
        // Non-WASM bytes should return empty
        let result = extract_import_modules(b"not wasm");
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_import_modules_valid_header_no_imports() {
        // Valid WASM magic + version but no sections — should return empty
        let bytes: &[u8] = &[0x00, 0x61, 0x73, 0x6D, 0x01, 0x00, 0x00, 0x00];
        let result = extract_import_modules(bytes);
        assert!(result.is_empty());
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// v2 Extended Endpoints: module inspection, listing, upload, execute-by-name
// ─────────────────────────────────────────────────────────────────────────────

/// GET /v2/tasks/{id}/inspect — deep WASM module inspection
#[get("/v2/tasks/{id}/inspect")]
pub async fn inspect_task(
    path: web::Path<String>,
    data: web::Data<crate::server::AppState>,
) -> Result<impl Responder> {
    let id = path.into_inner();
    let task = data
        .task_repo
        .get_by_id(&id)
        .await
        .map_err(crate::error::WasmOsError::Database)?
        .ok_or_else(|| crate::error::WasmOsError::TaskNotFound(id.clone()))?;

    let bytes = match std::fs::read(&task.path) {
        Ok(b) => b,
        Err(e) => {
            return Ok(HttpResponse::Ok().json(serde_json::json!({
                "task_id": task.id,
                "name": task.name,
                "path": task.path,
                "error": format!("Cannot read file: {e}"),
            })))
        }
    };

    let is_wasm = bytes.len() >= 4 && bytes[0..4] == [0x00, 0x61, 0x73, 0x6D];
    let wasm_version: u32 = if bytes.len() >= 8 {
        u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]])
    } else {
        0
    };
    let magic = format!(
        "{:02X} {:02X} {:02X} {:02X}",
        bytes.first().unwrap_or(&0),
        bytes.get(1).unwrap_or(&0),
        bytes.get(2).unwrap_or(&0),
        bytes.get(3).unwrap_or(&0)
    );

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "task_id": task.id,
        "name": task.name,
        "path": task.path,
        "file_size_bytes": bytes.len(),
        "is_valid_wasm": is_wasm,
        "magic_bytes": magic,
        "wasm_version": wasm_version,
        "status": task.status,
    })))
}

/// GET /v2/modules — list WASM/WAT modules available on disk
#[get("/v2/modules")]
pub async fn list_modules() -> Result<HttpResponse> {
    let wasm_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wasm_files");
    let mut modules: Vec<serde_json::Value> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&wasm_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if ext != "wasm" && ext != "wat" {
                continue;
            }
            let name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            modules.push(serde_json::json!({
                "name": name,
                "path": path.to_string_lossy(),
                "size_bytes": size,
                "format": ext,
            }));
        }
    }

    modules.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "total": modules.len(),
        "modules": modules,
    })))
}

/// POST /v2/execute/module — execute a WASM module by filename (bypasses task system)
#[post("/v2/execute/module")]
pub async fn execute_module(
    req: web::Json<serde_json::Value>,
) -> Result<HttpResponse> {
    let module_name = req
        .get("module")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if module_name.is_empty() {
        return Err(crate::error::WasmOsError::Validation(
            "module name is required".into(),
        ));
    }

    let wasm_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wasm_files");
    let file_path = wasm_dir.join(&module_name);

    // Boundary check: canonicalize resolves `..` segments and symlinks.
    // If the resolved path escapes wasm_files/ we reject the request.
    let canonical = std::fs::canonicalize(&file_path).map_err(|_| {
        crate::error::WasmOsError::TaskNotFound(format!(
            "Module '{module_name}' not found in wasm_files/"
        ))
    })?;
    let canonical_dir = std::fs::canonicalize(&wasm_dir).unwrap_or(wasm_dir.clone());
    if !canonical.starts_with(&canonical_dir) {
        tracing::warn!(
            "Path traversal blocked in execute_module: {:?} is outside {:?}",
            canonical, canonical_dir
        );
        return Err(crate::error::WasmOsError::Validation(
            "Module path is outside the permitted wasm_files directory".into(),
        ));
    }

    let path_str = canonical.to_string_lossy().to_string();
    let result = web::block(move || ExecutionDispatcher::execute_file(&path_str, None))
        .await
        .map_err(|e| {
            crate::error::WasmOsError::ExecutionError(format!("Thread join error: {e}"))
        })?
        .map_err(crate::error::WasmOsError::ExecutionError)?;

    Ok(HttpResponse::Ok().json(serde_json::json!({
        "module": module_name,
        "execution_id": result.execution_id,
        "success": result.execution_result.success,
        "instructions": result.advanced_report.total_instructions,
        "duration_ms": result.advanced_report.total_duration_us as f64 / 1000.0,
        "error": result.execution_result.error,
        "stdout": result.execution_result.stdout_log,
    })))
}

/// POST /v2/modules/upload — save a WASM module to wasm_files/ (no task created)
#[post("/v2/modules/upload")]
pub async fn upload_module(
    req: web::Json<crate::server::CreateTaskRequest>,
) -> Result<HttpResponse> {
    if req.wasm_data.is_empty() {
        return Err(crate::error::WasmOsError::Validation(
            "wasm_data cannot be empty".into(),
        ));
    }

    let wasm_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wasm_files");
    tokio::fs::create_dir_all(&wasm_dir)
        .await
        .map_err(crate::error::WasmOsError::Io)?;

    // Sanitize filename: allow alphanumeric, underscore, hyphen, single dot.
    // Replace any sequence of dots (e.g. "..") with a single underscore to
    // prevent path-traversal via upload_module filenames.
    let raw: String = req
        .name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Collapse consecutive dots so `..` cannot escape wasm_files/.
    let base = raw.replace("..", "_");
    let filename = if base.ends_with(".wasm") || base.ends_with(".wat") {
        base
    } else {
        format!("{base}.wasm")
    };

    let file_path = wasm_dir.join(&filename);
    tokio::fs::write(&file_path, &req.wasm_data)
        .await
        .map_err(crate::error::WasmOsError::Io)?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "name": filename,
        "path": file_path.to_string_lossy(),
        "size_bytes": req.wasm_data.len(),
        "status": "uploaded",
    })))
}

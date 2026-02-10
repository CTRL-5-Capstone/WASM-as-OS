use actix_web::{get, post, delete, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use crate::struct_files::wasm_list::WasmList;
use crate::run_wasm::wasm_control::start_wasm_by_id;
use crate::run_wasm::wasm_control::halt_wasm_by_id;

pub struct AppState {
    pub wasm_list: Arc<Mutex<WasmList>>,
}

#[derive(Serialize)]
pub struct SystemStats {
    pub total_tasks: usize,
    pub running_tasks: usize,
    pub failed_tasks: usize,
    pub total_instructions: u64,
    pub total_syscalls: u64,
}

use crate::struct_files::wasm_struct::{WasmMetrics, ExecutionRecord};

#[derive(Serialize)]
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub path: String,
    pub metrics: WasmMetrics,
    pub execution_history: Vec<ExecutionRecord>,
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub wasm_data: Vec<u8>,
}

#[get("/stats")]
pub async fn get_stats(data: web::Data<AppState>) -> impl Responder {
    let mut list = data.wasm_list.lock().unwrap();
    let (running, halted) = list.list_runningvec();
    
    // Placeholder stats for now
    let stats = SystemStats {
        total_tasks: running.len() + halted.len(),
        running_tasks: running.len(),
        failed_tasks: 0,
        total_instructions: 0,
        total_syscalls: 0,
    };
    
    HttpResponse::Ok().json(stats)
}

#[get("/tasks")]
pub async fn get_tasks(data: web::Data<AppState>) -> impl Responder {
    let mut list = data.wasm_list.lock().unwrap();
    
    let mut tasks = Vec::new();
    
    // Get running tasks
    let (running_nodes, _) = list.list_runningvec();
    for wasm in running_nodes {
        let w = wasm.lock().unwrap();
        tasks.push(TaskInfo {
            id: w.wasm_file.name.clone(),
            name: w.wasm_file.name.clone(),
            status: "Running".to_string(),
            path: w.wasm_file.path_to.clone(),
            metrics: w.wasm_file.metrics.clone(),
            execution_history: w.wasm_file.execution_history.clone(),
        });
    }

    // Get halted tasks
    let (halted_nodes, _) = list.list_haltedvec();
    for wasm in halted_nodes {
        let w = wasm.lock().unwrap();
        tasks.push(TaskInfo {
            id: w.wasm_file.name.clone(),
            name: w.wasm_file.name.clone(),
            status: "Stopped".to_string(), // Changed from "Halted" to match frontend expectation better
            path: w.wasm_file.path_to.clone(),
            metrics: w.wasm_file.metrics.clone(),
            execution_history: w.wasm_file.execution_history.clone(),
        });
    }

    HttpResponse::Ok().json(tasks)
}

#[post("/tasks")]
pub async fn upload_task(data: web::Data<AppState>, task: web::Json<CreateTaskRequest>) -> impl Responder {
    let mut list = data.wasm_list.lock().unwrap();
    
    // Ensure wasm_files directory exists
    if std::fs::create_dir_all("wasm_files").is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to create directory"}));
    }

    // Sanitize filename or use a safe default
    let filename = format!("{}.wasm", task.name.replace(|c: char| !c.is_alphanumeric(), "_"));
    let filepath = format!("wasm_files/{}", filename);
    
    // Write WASM data to file
    if let Err(e) = std::fs::write(&filepath, &task.wasm_data) {
        println!("Failed to write file: {}", e);
        return HttpResponse::InternalServerError().json(serde_json::json!({"error": "Failed to write file"}));
    }
    
    // Create WasmFile struct
    // We need to import WasmFile if not already visible, but it is used in WasmList so it should be available via crate::struct_files::wasm_struct::WasmFile
    // But server.rs doesn't import WasmFile directly.
    // Let's add the import or use fully qualified path.
    // WasmList uses super::wasm_struct::WasmFile.
    // In server.rs we can use crate::struct_files::wasm_struct::WasmFile.
    
    let wasm_file = crate::struct_files::wasm_struct::WasmFile::new_wasm(task.name.clone(), filepath.clone());
    
    // Add to list
    list.insert(wasm_file);
    
    println!("Loaded task: {}", task.name);
    HttpResponse::Ok().json(task.name.clone()) // Return ID (name)
}

#[post("/tasks/{id}/start")]
pub async fn start_task(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let mut list = data.wasm_list.lock().unwrap();
    let id = path.into_inner();
    
    // We need to implement start_wasm_by_id in wasm_control
    if start_wasm_by_id(&mut list, &id) {
        HttpResponse::Ok().json(serde_json::json!({"status": "started"}))
    } else {
        HttpResponse::BadRequest().json(serde_json::json!({"error": "failed to start"}))
    }
}

#[post("/tasks/{id}/stop")]
pub async fn stop_task(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let mut list = data.wasm_list.lock().unwrap();
    let id = path.into_inner();
    
    if halt_wasm_by_id(&mut list, &id) {
        HttpResponse::Ok().json(serde_json::json!({"status": "stopped"}))
    } else {
        HttpResponse::BadRequest().json(serde_json::json!({"error": "failed to stop"}))
    }
}

#[delete("/tasks/{id}")]
pub async fn delete_task(data: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let mut list = data.wasm_list.lock().unwrap();
    let id = path.into_inner();
    
    list.delete(id);
    HttpResponse::Ok().json(serde_json::json!({"status": "deleted"}))
}

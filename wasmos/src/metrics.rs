use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, register_int_gauge, CounterVec,
    Encoder, GaugeVec, HistogramVec, IntGauge, TextEncoder,
};

lazy_static! {
    // Always-on baseline metric so /metrics is never empty (even in minimal tests)
    pub static ref WASMOS_UP: IntGauge = register_int_gauge!(
        "wasmos_up",
        "WasmOS process is running (1 = up)"
    )
    .unwrap();

    // HTTP Metrics
    pub static ref HTTP_REQUESTS_TOTAL: CounterVec = register_counter_vec!(
        "wasmos_http_requests_total",
        "Total number of HTTP requests",
        &["method", "endpoint", "status"]
    )
    .unwrap();

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "wasmos_http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "endpoint"]
    )
    .unwrap();

    // Task Metrics
    pub static ref TASKS_TOTAL: GaugeVec = register_gauge_vec!(
        "wasmos_tasks_total",
        "Total number of tasks",
        &["status"]
    )
    .unwrap();

    pub static ref TASK_EXECUTIONS_TOTAL: CounterVec = register_counter_vec!(
        "wasmos_task_executions_total",
        "Total number of task executions",
        &["status"]
    )
    .unwrap();

    pub static ref TASK_EXECUTION_DURATION: HistogramVec = register_histogram_vec!(
        "wasmos_task_execution_duration_seconds",
        "Task execution duration in seconds",
        &["task_name"]
    )
    .unwrap();

    // WASM Metrics
    pub static ref WASM_INSTRUCTIONS_TOTAL: CounterVec = register_counter_vec!(
        "wasmos_wasm_instructions_total",
        "Total number of WASM instructions executed",
        &["task_name"]
    )
    .unwrap();

    pub static ref WASM_MEMORY_USAGE: GaugeVec = register_gauge_vec!(
        "wasmos_wasm_memory_usage_bytes",
        "Current WASM memory usage in bytes",
        &["task_name"]
    )
    .unwrap();

    // System Metrics
    pub static ref SYSTEM_ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "wasmos_system_errors_total",
        "Total number of system errors",
        &["error_type"]
    )
    .unwrap();
}

pub fn encode_metrics() -> Result<String, Box<dyn std::error::Error>> {
    let encoder = TextEncoder::new();
    // Force registration and set a stable value.
    WASMOS_UP.set(1);
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer)?;
    Ok(String::from_utf8(buffer)?)
}

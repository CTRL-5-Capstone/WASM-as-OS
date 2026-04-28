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


// ─── In-source tests ─────────────────────────────────────────────────────────
//
// NOTE: Prometheus uses a *global* default registry, so these tests share
// state with each other and with any other test that touches the metrics
// module.  Each test asserts on substrings/relative changes rather than
// exact equality so concurrent test execution stays safe.
#[cfg(test)]
mod tests {
    use super::*;
 
    #[test]
    fn encode_metrics_returns_non_empty_text() {
        let out = encode_metrics().expect("encode_metrics must succeed");
        assert!(!out.is_empty(), "Prometheus encoding should always produce output");
    }
 
    #[test]
    fn encode_metrics_advertises_wasmos_up() {
        let out = encode_metrics().expect("encode succeeds");
        // wasmos_up is the always-on baseline metric — by contract `encode_metrics`
        // sets it to 1 every call so the /metrics endpoint never appears empty.
        assert!(out.contains("wasmos_up"), "wasmos_up must appear in output: {out}");
        assert!(
            out.contains("wasmos_up 1") || out.contains("wasmos_up{} 1"),
            "wasmos_up should be set to 1 — got:\n{out}"
        );
        assert_eq!(WASMOS_UP.get(), 1);
    }
 
    #[test]
    fn encode_metrics_emits_help_and_type_lines() {
        // Prometheus text format requires `# HELP` and `# TYPE` lines for every
        // metric family — these are the contract Grafana/Prometheus agents depend on.
        let out = encode_metrics().expect("encode succeeds");
        assert!(out.contains("# HELP wasmos_up"));
        assert!(out.contains("# TYPE wasmos_up gauge"));
    }
 
    #[test]
    fn http_requests_counter_increments_visible_in_output() {
        // Use a unique label set so this test doesn't collide with any other.
        HTTP_REQUESTS_TOTAL
            .with_label_values(&["GET", "/test_metrics_marker", "200"])
            .inc();
 
        let out = encode_metrics().expect("encode succeeds");
        assert!(out.contains("/test_metrics_marker"), "label value should appear in output");
        assert!(out.contains("wasmos_http_requests_total"));
    }
 
    #[test]
    fn tasks_gauge_can_be_set_and_read_back() {
        TASKS_TOTAL.with_label_values(&["running"]).set(7.0);
        TASKS_TOTAL.with_label_values(&["completed"]).set(42.0);
 
        let out = encode_metrics().expect("encode succeeds");
        assert!(out.contains("wasmos_tasks_total"));
        assert_eq!(TASKS_TOTAL.with_label_values(&["running"]).get(), 7.0);
        assert_eq!(TASKS_TOTAL.with_label_values(&["completed"]).get(), 42.0);
    }
 
    #[test]
    fn system_errors_counter_is_monotonic() {
        let before = SYSTEM_ERRORS_TOTAL.with_label_values(&["test_metric_err"]).get();
        SYSTEM_ERRORS_TOTAL.with_label_values(&["test_metric_err"]).inc();
        SYSTEM_ERRORS_TOTAL.with_label_values(&["test_metric_err"]).inc();
        let after = SYSTEM_ERRORS_TOTAL.with_label_values(&["test_metric_err"]).get();
 
        assert_eq!(after - before, 2.0, "counter must increment by exactly 2");
    }
 
    #[test]
    fn encode_metrics_output_is_repeatedly_callable() {
        // Calling /metrics on a hot endpoint must be idempotent.
        let a = encode_metrics().expect("first call succeeds");
        let b = encode_metrics().expect("second call succeeds");
        assert!(!a.is_empty());
        assert!(!b.is_empty());
    }
}
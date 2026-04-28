/// Distributed Tracing & Live Span Metrics
///
/// Design:
///  - Each WASM task execution produces a root Span with child spans for
///    distinct phases: load, validate, execute, persist.
///  - Spans are stored in memory (bounded ring buffer) and exposed via
///    `/v1/traces` and `/v1/traces/{task_id}` REST endpoints.
///  - Live metrics (P50/P95/P99 latency, error rate, throughput) are computed
///    on the fly from the span store and pushed over WebSocket every 2 s.
///  - A Prometheus gauge series is updated on every span completion for
///    Grafana integration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::metrics::{TASK_EXECUTION_DURATION, WASM_INSTRUCTIONS_TOTAL};

// ─── Span types ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SpanKind {
    Root,
    Load,
    Validate,
    Execute,
    Persist,
    Plugin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub span_id: String,
    pub trace_id: String,
    pub parent_id: Option<String>,
    pub task_id: String,
    pub task_name: String,
    pub kind: SpanKind,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    /// Microseconds from span start to end
    pub duration_us: Option<i64>,
    pub success: bool,
    pub error: Option<String>,
    /// Arbitrary key-value tags (e.g. wasm_path, instructions, memory_bytes)
    pub tags: std::collections::HashMap<String, serde_json::Value>,
}

#[allow(dead_code)]
impl Span {
    pub fn new_root(task_id: impl Into<String>, task_name: impl Into<String>) -> (Self, String) {
        let trace_id = Uuid::new_v4().to_string();
        let span = Self {
            span_id: Uuid::new_v4().to_string(),
            trace_id: trace_id.clone(),
            parent_id: None,
            task_id: task_id.into(),
            task_name: task_name.into(),
            kind: SpanKind::Root,
            started_at: Utc::now(),
            ended_at: None,
            duration_us: None,
            success: false,
            error: None,
            tags: Default::default(),
        };
        (span, trace_id)
    }

    pub fn child(
        &self,
        kind: SpanKind,
        task_id: impl Into<String>,
        task_name: impl Into<String>,
    ) -> Self {
        Self {
            span_id: Uuid::new_v4().to_string(),
            trace_id: self.trace_id.clone(),
            parent_id: Some(self.span_id.clone()),
            task_id: task_id.into(),
            task_name: task_name.into(),
            kind,
            started_at: Utc::now(),
            ended_at: None,
            duration_us: None,
            success: false,
            error: None,
            tags: Default::default(),
        }
    }

    pub fn finish(&mut self, success: bool, error: Option<String>, wall_start: &Instant) {
        let dur = wall_start.elapsed().as_micros() as i64;
        self.ended_at = Some(Utc::now());
        self.duration_us = Some(dur);
        self.success = success;
        self.error = error;
    }

    pub fn tag(&mut self, key: impl Into<String>, val: impl Into<serde_json::Value>) {
        self.tags.insert(key.into(), val.into());
    }
}

// ─── Trace (collection of spans for one task execution) ──────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Trace {
    pub trace_id: String,
    pub task_id: String,
    pub task_name: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub total_duration_us: Option<i64>,
    pub success: bool,
    pub spans: Vec<Span>,
}

#[allow(dead_code)]
impl Trace {
    pub fn new(root: &Span) -> Self {
        Self {
            trace_id: root.trace_id.clone(),
            task_id: root.task_id.clone(),
            task_name: root.task_name.clone(),
            started_at: root.started_at,
            ended_at: None,
            total_duration_us: None,
            success: false,
            spans: vec![root.clone()],
        }
    }

    pub fn add_span(&mut self, span: Span) {
        self.spans.push(span);
    }

    pub fn finish(&mut self, success: bool) {
        self.ended_at = Some(Utc::now());
        self.success = success;
        // Sum all root-level span durations as total
        self.total_duration_us = self
            .spans
            .iter()
            .filter(|s| s.parent_id.is_none())
            .filter_map(|s| s.duration_us)
            .next();
    }
}

// ─── Span store (bounded ring buffer) ────────────────────────────────────────

/// Maximum number of completed traces to keep in memory.
const MAX_TRACES: usize = 500;

pub struct TraceStore {
    traces: RwLock<VecDeque<Trace>>,
}

#[allow(dead_code)]
impl TraceStore {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            traces: RwLock::new(VecDeque::with_capacity(MAX_TRACES)),
        })
    }

    /// Commit a finished trace to the store (evicts oldest if full).
    pub async fn commit(&self, trace: Trace) {
        // Update Prometheus metrics
        if let Some(dur_us) = trace.total_duration_us {
            let dur_secs = dur_us as f64 / 1_000_000.0;
            let _ = TASK_EXECUTION_DURATION
                .with_label_values(&[&trace.task_name])
                .observe(dur_secs);
        }
        // Record instructions from execute span
        for span in &trace.spans {
            if span.kind == SpanKind::Execute {
                if let Some(instr) = span.tags.get("instructions").and_then(|v| v.as_i64()) {
                    WASM_INSTRUCTIONS_TOTAL
                        .with_label_values(&[&trace.task_name])
                        .inc_by(instr as f64);
                }
            }
        }

        let mut traces = self.traces.write().await;
        if traces.len() >= MAX_TRACES {
            traces.pop_front();
        }
        traces.push_back(trace);
    }

    /// Return the last `limit` traces (most recent first).
    pub async fn recent(&self, limit: usize) -> Vec<Trace> {
        let traces = self.traces.read().await;
        traces.iter().rev().take(limit).cloned().collect()
    }

    /// Return all traces for a given task_id.
    pub async fn for_task(&self, task_id: &str) -> Vec<Trace> {
        self.traces
            .read()
            .await
            .iter()
            .filter(|t| t.task_id == task_id)
            .cloned()
            .collect()
    }

    /// Return live aggregate metrics computed from stored traces.
    pub async fn live_metrics(&self, window: usize) -> LiveMetrics {
        let traces = self.traces.read().await;
        let recent: Vec<&Trace> = traces.iter().rev().take(window).collect();
        if recent.is_empty() {
            return LiveMetrics::default();
        }

        let total = recent.len();
        let successes = recent.iter().filter(|t| t.success).count();
        let mut durations: Vec<i64> = recent
            .iter()
            .filter_map(|t| t.total_duration_us)
            .collect();
        durations.sort_unstable();

        let p50 = percentile(&durations, 50);
        let p95 = percentile(&durations, 95);
        let p99 = percentile(&durations, 99);

        LiveMetrics {
            window_size: total,
            success_rate: successes as f64 / total as f64,
            error_rate: (total - successes) as f64 / total as f64,
            p50_us: p50,
            p95_us: p95,
            p99_us: p99,
            avg_us: if !durations.is_empty() {
                durations.iter().sum::<i64>() / durations.len() as i64
            } else {
                0
            },
            throughput_per_min: {
                // Count traces in the last 60 s
                let cutoff = Utc::now() - chrono::Duration::seconds(60);
                traces.iter().filter(|t| t.started_at >= cutoff).count() as f64
            },
        }
    }
}

fn percentile(sorted: &[i64], p: usize) -> i64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((p as f64 / 100.0) * (sorted.len() as f64 - 1.0)).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

// ─── Live metrics snapshot ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LiveMetrics {
    pub window_size: usize,
    pub success_rate: f64,
    pub error_rate: f64,
    pub p50_us: i64,
    pub p95_us: i64,
    pub p99_us: i64,
    pub avg_us: i64,
    /// Completions per minute in the last 60 s
    pub throughput_per_min: f64,
}

// ─── Tracer — convenience builder ───────────────────────────────────────────

/// Per-execution tracer that collects spans and commits the trace on drop.
#[allow(dead_code)]
pub struct Tracer {
    pub trace: Trace,
    pub root_span: Span,
    store: Arc<TraceStore>,
    wall_start: Instant,
}

#[allow(dead_code)]
impl Tracer {
    pub fn start(
        store: Arc<TraceStore>,
        task_id: impl Into<String>,
        task_name: impl Into<String>,
    ) -> Self {
        let (root, _trace_id) = Span::new_root(task_id, task_name);
        let trace = Trace::new(&root);
        Self {
            trace,
            root_span: root,
            store,
            wall_start: Instant::now(),
        }
    }

    /// Record a child span with explicit timing.
    pub fn record_span(
        &mut self,
        kind: SpanKind,
        success: bool,
        error: Option<String>,
        tags: Vec<(String, serde_json::Value)>,
        duration_us: i64,
    ) {
        let mut span = self.root_span.child(
            kind,
            self.trace.task_id.clone(),
            self.trace.task_name.clone(),
        );
        let fake_start = Instant::now();
        // Manually set duration since we don't have a real wall-clock start here
        span.ended_at = Some(Utc::now());
        span.duration_us = Some(duration_us);
        span.success = success;
        span.error = error;
        for (k, v) in tags {
            span.tags.insert(k, v);
        }
        let _ = fake_start;
        self.trace.add_span(span);
    }

    /// Finish the root span and commit the trace to the store.
    pub async fn finish(mut self, success: bool, error: Option<String>) {
        self.root_span.finish(success, error, &self.wall_start);
        self.trace.spans[0] = self.root_span.clone();
        self.trace.finish(success);
        self.store.commit(self.trace).await;
    }
}


// ─── In-source tests ─────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
 
    // ─── Span ────────────────────────────────────────────────────────────────
 
    #[test]
    fn span_new_root_assigns_unique_ids_and_returns_trace_id() {
        let (span, trace_id) = Span::new_root("task-1", "hello.wasm");
 
        assert_eq!(span.task_id, "task-1");
        assert_eq!(span.task_name, "hello.wasm");
        assert_eq!(span.kind, SpanKind::Root);
        assert!(span.parent_id.is_none(), "root span must have no parent");
        assert_eq!(span.trace_id, trace_id);
        assert_ne!(span.span_id, span.trace_id, "span_id and trace_id are independent UUIDs");
        assert!(!span.success, "freshly-created spans default to success=false until finished");
        assert!(span.ended_at.is_none());
        assert!(span.duration_us.is_none());
        assert!(span.tags.is_empty());
    }
 
    #[test]
    fn span_new_root_generates_distinct_trace_ids() {
        let (_, t1) = Span::new_root("a", "a.wasm");
        let (_, t2) = Span::new_root("a", "a.wasm");
        assert_ne!(t1, t2, "every root span gets a fresh UUID");
    }
 
    #[test]
    fn span_child_inherits_trace_id_and_records_parent() {
        let (root, _) = Span::new_root("task-1", "hello.wasm");
        let child = root.child(SpanKind::Execute, "task-1", "hello.wasm");
 
        assert_eq!(child.trace_id, root.trace_id);
        assert_eq!(child.parent_id.as_deref(), Some(root.span_id.as_str()));
        assert_eq!(child.kind, SpanKind::Execute);
        assert_ne!(child.span_id, root.span_id);
    }
 
    #[test]
    fn span_finish_populates_timing_and_status() {
        let (mut span, _) = Span::new_root("task", "x.wasm");
        let start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(2));
        span.finish(true, None, &start);
 
        assert!(span.ended_at.is_some());
        assert!(span.duration_us.unwrap() >= 2_000, "duration should reflect at least 2ms");
        assert!(span.success);
        assert!(span.error.is_none());
    }
 
    #[test]
    fn span_finish_records_failure_and_error_message() {
        let (mut span, _) = Span::new_root("task", "x.wasm");
        let start = Instant::now();
        span.finish(false, Some("trap: out of memory".into()), &start);
 
        assert!(!span.success);
        assert_eq!(span.error.as_deref(), Some("trap: out of memory"));
    }
 
    #[test]
    fn span_tag_round_trips_through_serde_json() {
        let (mut span, _) = Span::new_root("task", "x.wasm");
        span.tag("instructions", 1_234_i64);
        span.tag("wasm_path", "/tmp/x.wasm");
 
        assert_eq!(span.tags.get("instructions").and_then(|v| v.as_i64()), Some(1_234));
        assert_eq!(span.tags.get("wasm_path").and_then(|v| v.as_str()), Some("/tmp/x.wasm"));
    }
 
    // ─── Trace ───────────────────────────────────────────────────────────────
 
    #[test]
    fn trace_new_seeds_root_span_into_spans_vec() {
        let (root, _) = Span::new_root("t", "x.wasm");
        let trace = Trace::new(&root);
 
        assert_eq!(trace.trace_id, root.trace_id);
        assert_eq!(trace.task_id, root.task_id);
        assert_eq!(trace.spans.len(), 1);
        assert!(trace.ended_at.is_none());
        assert!(trace.total_duration_us.is_none());
    }
 
    #[test]
    fn trace_add_span_appends_in_order() {
        let (root, _) = Span::new_root("t", "x.wasm");
        let mut trace = Trace::new(&root);
 
        let load = root.child(SpanKind::Load, "t", "x.wasm");
        let exec = root.child(SpanKind::Execute, "t", "x.wasm");
        trace.add_span(load.clone());
        trace.add_span(exec.clone());
 
        assert_eq!(trace.spans.len(), 3);
        assert_eq!(trace.spans[1].span_id, load.span_id);
        assert_eq!(trace.spans[2].span_id, exec.span_id);
    }
 
    #[test]
    fn trace_finish_pulls_total_duration_from_root_span() {
        let (mut root, _) = Span::new_root("t", "x.wasm");
        let start = Instant::now();
        std::thread::sleep(std::time::Duration::from_millis(1));
        root.finish(true, None, &start);
 
        let mut trace = Trace::new(&root);
        trace.spans[0] = root;
        trace.finish(true);
 
        assert!(trace.success);
        assert!(trace.ended_at.is_some());
        assert!(trace.total_duration_us.unwrap() >= 1_000);
    }
 
    // ─── TraceStore ─────────────────────────────────────────────────────────
 
    fn make_finished_trace(task_id: &str, task_name: &str, dur_us: i64, success: bool) -> Trace {
        let (mut root, _) = Span::new_root(task_id, task_name);
        root.duration_us = Some(dur_us);
        root.ended_at = Some(chrono::Utc::now());
        root.success = success;
        let mut trace = Trace::new(&root);
        trace.spans[0] = root;
        trace.finish(success);
        trace
    }
 
    #[tokio::test]
    async fn store_recent_returns_traces_in_reverse_insertion_order() {
        let store = TraceStore::new();
        store.commit(make_finished_trace("t1", "a.wasm", 100, true)).await;
        store.commit(make_finished_trace("t2", "b.wasm", 200, true)).await;
        store.commit(make_finished_trace("t3", "c.wasm", 300, true)).await;
 
        let recent = store.recent(10).await;
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].task_id, "t3", "most recent first");
        assert_eq!(recent[1].task_id, "t2");
        assert_eq!(recent[2].task_id, "t1");
    }
 
    #[tokio::test]
    async fn store_recent_respects_limit() {
        let store = TraceStore::new();
        for i in 0..5 {
            store.commit(make_finished_trace(&format!("t{i}"), "x.wasm", 100, true)).await;
        }
        let recent = store.recent(2).await;
        assert_eq!(recent.len(), 2);
    }
 
    #[tokio::test]
    async fn store_for_task_filters_by_task_id() {
        let store = TraceStore::new();
        store.commit(make_finished_trace("alpha", "a.wasm", 100, true)).await;
        store.commit(make_finished_trace("beta", "b.wasm", 200, true)).await;
        store.commit(make_finished_trace("alpha", "a.wasm", 150, false)).await;
 
        let alpha = store.for_task("alpha").await;
        assert_eq!(alpha.len(), 2);
        assert!(alpha.iter().all(|t| t.task_id == "alpha"));
 
        let none = store.for_task("nonexistent").await;
        assert!(none.is_empty());
    }
 
    #[tokio::test]
    async fn store_live_metrics_returns_default_when_empty() {
        let store = TraceStore::new();
        let m = store.live_metrics(100).await;
        assert_eq!(m.window_size, 0);
        assert_eq!(m.success_rate, 0.0);
        assert_eq!(m.error_rate, 0.0);
        assert_eq!(m.p50_us, 0);
    }
 
    #[tokio::test]
    async fn store_live_metrics_computes_success_rate_and_percentiles() {
        let store = TraceStore::new();
        // 8 successes, 2 failures, durations 100..1000
        for (i, dur) in [100, 200, 300, 400, 500, 600, 700, 800, 900, 1000].iter().enumerate() {
            let success = i < 8;
            store.commit(make_finished_trace("t", "x.wasm", *dur, success)).await;
        }
 
        let m = store.live_metrics(10).await;
        assert_eq!(m.window_size, 10);
        assert!((m.success_rate - 0.8).abs() < 1e-9);
        assert!((m.error_rate - 0.2).abs() < 1e-9);
        // Sorted [100..1000]; P50 idx = round(0.5 * 9) = 5 → 600
        assert_eq!(m.p50_us, 600);
        assert_eq!(m.p95_us, 1000);
        assert_eq!(m.p99_us, 1000);
        assert_eq!(m.avg_us, (100+200+300+400+500+600+700+800+900+1000)/10);
    }
 
    #[tokio::test]
    async fn store_live_metrics_window_caps_to_recent() {
        let store = TraceStore::new();
        for _ in 0..5 { store.commit(make_finished_trace("t", "x.wasm", 100, true)).await; }
        for _ in 0..5 { store.commit(make_finished_trace("t", "x.wasm", 1000, true)).await; }
 
        let m = store.live_metrics(5).await;
        assert_eq!(m.window_size, 5);
        assert_eq!(m.p50_us, 1000);
        assert_eq!(m.avg_us, 1000);
    }
 
    // ─── Tracer (full builder) ──────────────────────────────────────────────
 
    #[tokio::test]
    async fn tracer_full_lifecycle_commits_to_store() {
        let store = TraceStore::new();
        let mut tracer = Tracer::start(store.clone(), "task-1", "x.wasm");
 
        tracer.record_span(
            SpanKind::Load, true, None,
            vec![("file_size".into(), serde_json::json!(1024))],
            500,
        );
        tracer.record_span(
            SpanKind::Execute, true, None,
            vec![("instructions".into(), serde_json::json!(42))],
            2_000,
        );
 
        tracer.finish(true, None).await;
 
        let stored = store.recent(10).await;
        assert_eq!(stored.len(), 1);
        let trace = &stored[0];
        assert_eq!(trace.task_id, "task-1");
        assert!(trace.success);
        // Root + Load + Execute = 3 spans
        assert_eq!(trace.spans.len(), 3);
        let kinds: Vec<&SpanKind> = trace.spans.iter().map(|s| &s.kind).collect();
        assert_eq!(kinds, vec![&SpanKind::Root, &SpanKind::Load, &SpanKind::Execute]);
    }
 
    #[tokio::test]
    async fn tracer_finish_with_failure_records_error() {
        let store = TraceStore::new();
        let tracer = Tracer::start(store.clone(), "task-x", "boom.wasm");
        tracer.finish(false, Some("trap".into())).await;
 
        let stored = store.recent(10).await;
        assert_eq!(stored.len(), 1);
        assert!(!stored[0].success);
        assert_eq!(stored[0].spans[0].error.as_deref(), Some("trap"));
    }
}
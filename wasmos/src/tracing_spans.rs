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

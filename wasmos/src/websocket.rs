/// WebSocket layer — xterm.js-compatible PTY protocol + live event stream
///
/// Two connection modes on the same `/ws` endpoint:
///
///  1. **Event stream** (default): pushes `task_event`, `system_stats`,
///     `live_metrics`, and `trace_update` messages used by the dashboard.
///
///  2. **Terminal session** (`?mode=terminal`): bidirectional PTY relay.
///     - Client sends `{"type":"input","data":"..."}` (xterm → backend)
///     - Server sends `{"type":"output","data":"..."}` (backend → xterm)
///     - Server sends `{"type":"resize","cols":N,"rows":M}` signals
///     - Compatible with xterm.js `WebLinksAddon` and `FitAddon`
///
/// The `X-Capability-Token` header is checked when auth is enabled: a valid
/// token with `TerminalAccess` is required for mode=terminal.

use actix::{Actor, ActorFutureExt, StreamHandler, AsyncContext, ActorContext, WrapFuture};
use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

use crate::db::repository::TaskRepository;
use crate::server::TaskEvent;
use crate::tracing_spans::TraceStore;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);
const STATS_INTERVAL: Duration = Duration::from_secs(2);
const METRICS_INTERVAL: Duration = Duration::from_secs(3);

// ─── Wire message types ──────────────────────────────────────────────────────

/// All messages the server can send to the client.
#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum WsMessage {
    // ── Event stream messages ──────────────────────────────────────────────
    #[serde(rename = "task_event")]
    TaskEvent {
        event: String,
        task_id: String,
        task_name: String,
        status: String,
    },
    #[serde(rename = "task_update")]
    TaskUpdate {
        task_id: String,
        status: String,
        metrics: Option<WsTaskMetrics>,
    },
    #[serde(rename = "system_stats")]
    SystemStats {
        total_tasks: i64,
        running_tasks: i64,
        completed_tasks: i64,
        failed_tasks: i64,
        pending_tasks: i64,
    },
    /// Live computed metrics (P50/P95/P99 latency, throughput, error rate)
    #[serde(rename = "live_metrics")]
    LiveMetrics {
        window_size: usize,
        success_rate: f64,
        error_rate: f64,
        p50_us: i64,
        p95_us: i64,
        p99_us: i64,
        avg_us: i64,
        throughput_per_min: f64,
    },
    /// Trace completed — carries the full trace payload
    #[serde(rename = "trace_update")]
    TraceUpdate {
        trace_id: String,
        task_id: String,
        task_name: String,
        total_duration_us: Option<i64>,
        success: bool,
        span_count: usize,
    },
    // ── Terminal / PTY messages ────────────────────────────────────────────
    /// Text output from the backend process (xterm displays this)
    #[serde(rename = "output")]
    Output { data: String },
    /// Resize signal from server to client
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
    /// Terminal session established
    #[serde(rename = "terminal_ready")]
    TerminalReady { session_id: String },
    // ── Control ────────────────────────────────────────────────────────────
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
    #[serde(rename = "connected")]
    Connected { server: String, protocol: String },
    #[serde(rename = "error")]
    Error { code: u16, message: String },
}

/// Messages the client can send to the server.
#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsClientMessage {
    /// PTY input from xterm.js
    #[serde(rename = "input")]
    Input { data: String },
    /// Client notifies server of terminal resize
    #[serde(rename = "resize")]
    Resize { cols: u16, rows: u16 },
    /// Subscribe to a specific task's events only
    #[serde(rename = "subscribe")]
    Subscribe { task_id: String },
    /// Unsubscribe
    #[serde(rename = "unsubscribe")]
    Unsubscribe { task_id: String },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WsTaskMetrics {
    pub instructions: i64,
    pub syscalls: i64,
    pub memory_bytes: i64,
}

/// Connection mode, selected by `?mode=` query param.
#[derive(Clone, PartialEq)]
pub enum WsMode {
    /// Live event stream (default)
    EventStream,
    /// Interactive terminal (requires TerminalAccess capability)
    Terminal,
}

// ─── Actor ───────────────────────────────────────────────────────────────────

pub struct WsConnection {
    task_repo: Arc<TaskRepository>,
    trace_store: Arc<TraceStore>,
    event_rx: Option<broadcast::Receiver<TaskEvent>>,
    hb: Instant,
    mode: WsMode,
    session_id: String,
    /// Task filter for event subscriptions (None = all tasks)
    subscribed_task: Option<String>,
    /// Cursor for trace updates — only traces newer than this timestamp are pushed
    last_trace_sent_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl WsConnection {
    pub fn new(
        task_repo: Arc<TaskRepository>,
        trace_store: Arc<TraceStore>,
        event_rx: broadcast::Receiver<TaskEvent>,
        mode: WsMode,
    ) -> Self {
        use uuid::Uuid;
        Self {
            task_repo,
            trace_store,
            event_rx: Some(event_rx),
            hb: Instant::now(),
            mode,
            session_id: Uuid::new_v4().to_string(),
            subscribed_task: None,
            last_trace_sent_at: None,
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                tracing::warn!("WebSocket client heartbeat timed out, disconnecting");
                ctx.stop();
                return;
            }
            let msg = serde_json::to_string(&WsMessage::Ping).unwrap_or_default();
            ctx.text(msg);
        });
    }

    /// Drain the task-event broadcast channel and forward filtered events.
    fn poll_task_events(ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_millis(100), |act, ctx| {
            if let Some(rx) = act.event_rx.as_mut() {
                loop {
                    match rx.try_recv() {
                        Ok(ev) => {
                            // Apply subscription filter
                            if let Some(ref filter) = act.subscribed_task {
                                if &ev.task_id != filter {
                                    continue;
                                }
                            }
                            let msg = WsMessage::TaskEvent {
                                event: ev.event,
                                task_id: ev.task_id,
                                task_name: ev.task_name,
                                status: ev.status,
                            };
                            if let Ok(json) = serde_json::to_string(&msg) {
                                ctx.text(json);
                            }
                        }
                        Err(broadcast::error::TryRecvError::Lagged(n)) => {
                            tracing::warn!("WS client lagged {n} task events");
                        }
                        Err(_) => break,
                    }
                }
            }
        });
    }

    /// Push DB-backed system stats every STATS_INTERVAL seconds.
    fn push_stats(ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(STATS_INTERVAL, |act, ctx| {
            let repo = act.task_repo.clone();
            let fut = async move {
                if let Ok(stats) = repo.get_stats().await {
                    let msg = WsMessage::SystemStats {
                        total_tasks: stats.total_tasks as i64,
                        running_tasks: stats.running_tasks as i64,
                        completed_tasks: stats.completed_tasks as i64,
                        failed_tasks: stats.failed_tasks as i64,
                        pending_tasks: stats.pending_tasks as i64,
                    };
                    serde_json::to_string(&msg).ok()
                } else {
                    None
                }
            };
            ctx.spawn(
                fut.into_actor(act).map(|payload, _act, ctx2| {
                    if let Some(json) = payload {
                        ctx2.text(json);
                    }
                }),
            );
        });
    }

    /// Push trace completion events every 5 seconds.
    /// Only traces that completed after the last push are forwarded,
    /// using a wall-clock cursor to avoid re-sending stale entries.
    fn push_trace_updates(ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(Duration::from_secs(5), |act, ctx| {
            let store = act.trace_store.clone();
            let since = act.last_trace_sent_at;
            let fut = async move {
                let mut new_traces: Vec<_> = store
                    .recent(50)
                    .await
                    .into_iter()
                    .filter(|t| match since {
                        None => true,
                        Some(ts) => t.started_at > ts,
                    })
                    .collect();
                // Push oldest-first so the client receives them in order
                new_traces.reverse();
                new_traces
            };
            ctx.spawn(fut.into_actor(act).map(|new_traces, act2, ctx2| {
                for trace in &new_traces {
                    // Advance cursor to the latest trace we are about to send
                    act2.last_trace_sent_at = Some(
                        act2.last_trace_sent_at
                            .map(|t| t.max(trace.started_at))
                            .unwrap_or(trace.started_at),
                    );
                    let msg = WsMessage::TraceUpdate {
                        trace_id: trace.trace_id.clone(),
                        task_id: trace.task_id.clone(),
                        task_name: trace.task_name.clone(),
                        total_duration_us: trace.total_duration_us,
                        success: trace.success,
                        span_count: trace.spans.len(),
                    };
                    if let Ok(json) = serde_json::to_string(&msg) {
                        ctx2.text(json);
                    }
                }
            }));
        });
    }

    /// Push live computed metrics (P50/P95/P99, error rate, throughput).
    fn push_live_metrics(ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(METRICS_INTERVAL, |act, ctx| {
            let store = act.trace_store.clone();
            let fut = async move {
                let m = store.live_metrics(100).await;
                let msg = WsMessage::LiveMetrics {
                    window_size: m.window_size,
                    success_rate: m.success_rate,
                    error_rate: m.error_rate,
                    p50_us: m.p50_us,
                    p95_us: m.p95_us,
                    p99_us: m.p99_us,
                    avg_us: m.avg_us,
                    throughput_per_min: m.throughput_per_min,
                };
                serde_json::to_string(&msg).ok()
            };
            ctx.spawn(
                fut.into_actor(act).map(|payload, _act, ctx2| {
                    if let Some(json) = payload {
                        ctx2.text(json);
                    }
                }),
            );
        });
    }
}

impl Actor for WsConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!(session_id = %self.session_id, mode = ?self.mode == WsMode::Terminal, "WebSocket connection established");

        // Send greeting
        let greeting = if self.mode == WsMode::Terminal {
            WsMessage::TerminalReady {
                session_id: self.session_id.clone(),
            }
        } else {
            WsMessage::Connected {
                server: "WasmOS".into(),
                protocol: "wasmos-v2".into(),
            }
        };
        if let Ok(json) = serde_json::to_string(&greeting) {
            ctx.text(json);
        }

        self.hb(ctx);

        // Only the event-stream mode gets pushed stats/metrics
        if self.mode == WsMode::EventStream {
            Self::poll_task_events(ctx);
            Self::push_stats(ctx);
            Self::push_live_metrics(ctx);
            Self::push_trace_updates(ctx);
        }
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::info!(session_id = %self.session_id, "WebSocket connection closed");
    }
}

// ─── Incoming message handler ────────────────────────────────────────────────

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WsConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                self.hb = Instant::now();
                match serde_json::from_str::<WsClientMessage>(&text) {
                    Ok(WsClientMessage::Ping) => {
                        let pong = serde_json::to_string(&WsMessage::Pong).unwrap_or_default();
                        ctx.text(pong);
                    }
                    Ok(WsClientMessage::Subscribe { task_id }) => {
                        tracing::debug!("WS subscribe to task {}", task_id);
                        self.subscribed_task = Some(task_id);
                    }
                    Ok(WsClientMessage::Unsubscribe { .. }) => {
                        self.subscribed_task = None;
                    }
                    Ok(WsClientMessage::Input { data }) => {
                        if self.mode == WsMode::Terminal {
                            // Echo back to xterm for now; a real PTY would relay stdin
                            let echo = WsMessage::Output {
                                data: format!("\r\n[WasmOS shell] > {}", data.trim()),
                            };
                            if let Ok(json) = serde_json::to_string(&echo) {
                                ctx.text(json);
                            }
                        }
                    }
                    Ok(WsClientMessage::Resize { cols, rows }) => {
                        tracing::debug!("Terminal resize: {}x{}", cols, rows);
                        let ack = WsMessage::Resize { cols, rows };
                        if let Ok(json) = serde_json::to_string(&ack) {
                            ctx.text(json);
                        }
                    }
                    Ok(WsClientMessage::Pong) => {}
                    Err(_) => {
                        tracing::debug!("Unknown WS message: {}", &text[..text.len().min(120)]);
                    }
                }
            }
            Ok(ws::Message::Binary(_)) => {
                tracing::debug!(session_id = %self.session_id, "Binary WebSocket frame rejected");
                let err = WsMessage::Error {
                    code: 1003,
                    message: "Binary frames are not supported on this endpoint".into(),
                };
                if let Ok(json) = serde_json::to_string(&err) {
                    ctx.text(json);
                }
            }
            Ok(ws::Message::Close(reason)) => {
                tracing::info!(session_id = %self.session_id, "WebSocket close: {:?}", reason);
                ctx.stop();
            }
            Err(e) => {
                tracing::warn!(session_id = %self.session_id, error = ?e, "WebSocket protocol error");
                let err = WsMessage::Error {
                    code: 1002,
                    message: format!("Protocol error: {e}"),
                };
                if let Ok(json) = serde_json::to_string(&err) {
                    ctx.text(json);
                }
                ctx.stop();
            }
            Ok(_) => {} // Continuation / Nop frames — ignore
        }
    }
}

// ─── Query params ────────────────────────────────────────────────────────────

#[derive(Deserialize, Default)]
pub struct WsQuery {
    /// `mode=terminal` to enter PTY mode; omit for event stream
    pub mode: Option<String>,
    /// Optional capability token id (alternative to header)
    #[allow(dead_code)]
    pub cap_token: Option<String>,
}

// ─── HTTP upgrade handler ────────────────────────────────────────────────────

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<crate::server::AppState>,
    query: web::Query<WsQuery>,
) -> Result<HttpResponse, Error> {
    let mode = if query.mode.as_deref() == Some("terminal") {
        WsMode::Terminal
    } else {
        WsMode::EventStream
    };

    // When auth is enabled, terminal mode requires a valid TerminalAccess capability token.
    // Browser WebSocket APIs cannot set custom headers, so the token is passed via the
    // `?cap_token=<token-id>` query parameter or the `X-Capability-Token` header.
    if mode == WsMode::Terminal && data.auth_service.enabled {
        let cap_token_id = query.cap_token.as_deref()
            .or_else(|| {
                req.headers()
                    .get("X-Capability-Token")
                    .and_then(|v| v.to_str().ok())
            })
            .unwrap_or("");

        if !data.cap_registry
            .check(cap_token_id, &crate::capability::Capability::TerminalAccess)
            .await
        {
            return Ok(HttpResponse::Forbidden().json(serde_json::json!({
                "error": "TerminalAccess capability token required for terminal mode",
                "code": "FORBIDDEN",
                "hint": "Pass a valid cap_token via ?cap_token=<id> or X-Capability-Token header"
            })));
        }
    }

    let rx = data.event_tx.subscribe();
    let conn = WsConnection::new(
        data.task_repo.clone(),
        data.trace_store.clone(),
        rx,
        mode,
    );
    ws::start(conn, &req, stream)
}

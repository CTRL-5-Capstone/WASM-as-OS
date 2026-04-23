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

use crate::config::Config;
use crate::db::models::TaskStatus;
use crate::db::repository::TaskRepository;
use crate::metrics;
use crate::run_wasm::execute_wasm_file;
use crate::scheduler::Scheduler;
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
    scheduler: Arc<Scheduler>,
    config: Arc<Config>,
    event_tx: broadcast::Sender<TaskEvent>,
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
        scheduler: Arc<Scheduler>,
        config: Arc<Config>,
        event_tx: broadcast::Sender<TaskEvent>,
        event_rx: broadcast::Receiver<TaskEvent>,
        mode: WsMode,
    ) -> Self {
        use uuid::Uuid;
        Self {
            task_repo,
            trace_store,
            scheduler,
            config,
            event_tx,
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
                            let line = data.trim().to_string();
                            if line.is_empty() {
                                // Just re-prompt on empty line
                                let prompt = WsMessage::Output {
                                    data: "\r\n".to_string(),
                                };
                                if let Ok(json) = serde_json::to_string(&prompt) {
                                    ctx.text(json);
                                }
                            } else {
                                // Parse and dispatch the command asynchronously
                                let repo = self.task_repo.clone();
                                let trace_store = self.trace_store.clone();
                                let scheduler = self.scheduler.clone();
                                let config = self.config.clone();
                                let event_tx = self.event_tx.clone();

                                let fut = async move {
                                    handle_terminal_command(&line, repo, trace_store, scheduler, config, event_tx).await
                                };
                                ctx.spawn(
                                    fut.into_actor(self).map(|output, _act, ctx2| {
                                        let msg = WsMessage::Output { data: output };
                                        if let Ok(json) = serde_json::to_string(&msg) {
                                            ctx2.text(json);
                                        }
                                    }),
                                );
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
        data.scheduler.clone(),
        data.config.clone(),
        data.event_tx.clone(),
        rx,
        mode,
    );
    ws::start(conn, &req, stream)
}

// ─── Terminal command processor ──────────────────────────────────────────────

/// Process a CLI command from the WebSocket terminal and return ANSI-formatted output.
/// All output lines are `\r\n`-terminated for xterm.js compatibility.
async fn handle_terminal_command(
    line: &str,
    repo: Arc<TaskRepository>,
    trace_store: Arc<TraceStore>,
    scheduler: Arc<Scheduler>,
    config: Arc<Config>,
    event_tx: broadcast::Sender<TaskEvent>,
) -> String {
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.is_empty() {
        return String::new();
    }
    let cmd = parts[0];
    let args = &parts[1..];
    let mut out = String::new();

    match cmd {
        // ── Help ──────────────────────────────────────────────────────────
        "help" | "?" => {
            out.push_str("\r\n\x1b[96m╔══════════════════════════════════════════════════════════╗\x1b[0m");
            out.push_str("\r\n\x1b[96m║        WASM-OS Command Center CLI v5.0 (WebSocket)       ║\x1b[0m");
            out.push_str("\r\n\x1b[96m╚══════════════════════════════════════════════════════════╝\x1b[0m");
            out.push_str("\r\n");
            out.push_str("\r\n\x1b[33mTask Management:\x1b[0m");
            out.push_str("\r\n  \x1b[92mps\x1b[0m / \x1b[92mls\x1b[0m              List all WASM modules");
            out.push_str("\r\n  \x1b[92mrun\x1b[0m <id>              Execute a WASM module");
            out.push_str("\r\n  \x1b[92mstop\x1b[0m <id>             Stop a running module");
            out.push_str("\r\n  \x1b[92mkill\x1b[0m <id>             Stop + preempt a running module");
            out.push_str("\r\n  \x1b[92mrm\x1b[0m <id>               Delete a module");
            out.push_str("\r\n  \x1b[92minfo\x1b[0m <id>             Show detailed task info");
            out.push_str("\r\n  \x1b[92mstatus\x1b[0m <id>           Show task status");
            out.push_str("\r\n");
            out.push_str("\r\n\x1b[33mAnalysis:\x1b[0m");
            out.push_str("\r\n  \x1b[92msecurity\x1b[0m <id>         Static binary analysis");
            out.push_str("\r\n  \x1b[92mlogs\x1b[0m <id>             Show task logs");
            out.push_str("\r\n  \x1b[92mhistory\x1b[0m <id>          Execution history for a task");
            out.push_str("\r\n");
            out.push_str("\r\n\x1b[33mSystem:\x1b[0m");
            out.push_str("\r\n  \x1b[92mstats\x1b[0m                 System-wide statistics");
            out.push_str("\r\n  \x1b[92mhealth\x1b[0m                Backend health check");
            out.push_str("\r\n  \x1b[92mmetrics\x1b[0m               Prometheus metrics (first 25 lines)");
            out.push_str("\r\n  \x1b[92mscheduler\x1b[0m             Scheduler status");
            out.push_str("\r\n  \x1b[92mtraces\x1b[0m [n]            Recent traces (default 10)");
            out.push_str("\r\n  \x1b[92maudit\x1b[0m [n]             Recent audit log (default 10)");
            out.push_str("\r\n  \x1b[92msnapshots\x1b[0m <id>        List snapshots for a task");
            out.push_str("\r\n  \x1b[92mtestfiles\x1b[0m             List available test files");
            out.push_str("\r\n  \x1b[92mconfig\x1b[0m                Show server configuration");
            out.push_str("\r\n  \x1b[92mlive\x1b[0m                  Live metrics (P50/P95/P99, throughput)");
            out.push_str("\r\n");
            out.push_str("\r\n\x1b[33mMisc:\x1b[0m");
            out.push_str("\r\n  \x1b[92mclear\x1b[0m                 Clear terminal");
            out.push_str("\r\n  \x1b[92mversion\x1b[0m               Show version info");
            out.push_str("\r\n  \x1b[92mwhoami\x1b[0m                Show session info");
            out.push_str("\r\n  \x1b[92muptime\x1b[0m                Show server uptime");
        }

        // ── Task listing ──────────────────────────────────────────────────
        "ps" | "ls" | "list" | "tasks" => {
            match repo.list_all().await {
                Ok(tasks) => {
                    if tasks.is_empty() {
                        out.push_str("\r\n\x1b[90m(no modules loaded)\x1b[0m");
                    } else {
                        out.push_str("\r\n\x1b[90m");
                        out.push_str(&"─".repeat(80));
                        out.push_str("\x1b[0m");
                        out.push_str(&format!(
                            "\r\n\x1b[1m{:<36} {:<22} {:<12} {:<10}\x1b[0m",
                            "ID", "NAME", "STATUS", "SIZE"
                        ));
                        out.push_str("\r\n\x1b[90m");
                        out.push_str(&"─".repeat(80));
                        out.push_str("\x1b[0m");
                        for t in &tasks {
                            let status_str = format!("{:?}", t.status);
                            let color = match t.status {
                                TaskStatus::Running   => "\x1b[92m",
                                TaskStatus::Failed    => "\x1b[91m",
                                TaskStatus::Completed => "\x1b[96m",
                                TaskStatus::Stopped   => "\x1b[93m",
                                _                     => "\x1b[90m",
                            };
                            let size_kb = t.file_size_bytes / 1024;
                            out.push_str(&format!(
                                "\r\n{:<36} {:<22} {}{:<12}\x1b[0m {:<10}",
                                t.id, t.name, color, status_str, format!("{}KB", size_kb)
                            ));
                        }
                        out.push_str("\r\n\x1b[90m");
                        out.push_str(&"─".repeat(80));
                        out.push_str("\x1b[0m");
                        out.push_str(&format!("\r\n\x1b[90m{} module(s) total\x1b[0m", tasks.len()));
                    }
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ failed to fetch tasks: {}\x1b[0m", e));
                }
            }
        }

        // ── Task info ─────────────────────────────────────────────────────
        "info" | "describe" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: info <task-id>\x1b[0m"); return out; }
            };
            match repo.get_by_id(id).await {
                Ok(Some(t)) => {
                    out.push_str(&format!("\r\n\x1b[1m📋 Task Detail: {}\x1b[0m", t.id));
                    out.push_str(&format!("\r\n  Name:       \x1b[96m{}\x1b[0m", t.name));
                    out.push_str(&format!("\r\n  Status:     \x1b[92m{:?}\x1b[0m", t.status));
                    out.push_str(&format!("\r\n  Priority:   \x1b[33m{}\x1b[0m", t.priority));
                    out.push_str(&format!("\r\n  Size:       {} bytes", t.file_size_bytes));
                    out.push_str(&format!("\r\n  Path:       {}", t.path));
                    out.push_str(&format!("\r\n  Tenant:     {}", t.tenant_id.as_deref().unwrap_or("default")));
                    out.push_str(&format!("\r\n  Created:    {}", t.created_at));
                    out.push_str(&format!("\r\n  Updated:    {}", t.updated_at));
                }
                Ok(None) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ task not found: {}\x1b[0m", id));
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ error: {}\x1b[0m", e));
                }
            }
        }

        // ── Task status ───────────────────────────────────────────────────
        "status" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: status <task-id>\x1b[0m"); return out; }
            };
            match repo.get_by_id(id).await {
                Ok(Some(t)) => {
                    let color = match t.status {
                        TaskStatus::Running   => "\x1b[92m",
                        TaskStatus::Failed    => "\x1b[91m",
                        TaskStatus::Completed => "\x1b[96m",
                        TaskStatus::Stopped   => "\x1b[93m",
                        _                     => "\x1b[90m",
                    };
                    out.push_str(&format!("\r\n{} → {}{:?}\x1b[0m", t.name, color, t.status));
                }
                Ok(None) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ task not found: {}\x1b[0m", id));
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ error: {}\x1b[0m", e));
                }
            }
        }

        // ── Execute / Run ─────────────────────────────────────────────────
        "run" | "start" | "execute" | "wasm-run" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: run <task-id>\x1b[0m"); return out; }
            };
            match repo.get_by_id(id).await {
                Ok(Some(task)) => {
                    if task.status == TaskStatus::Running {
                        out.push_str(&format!("\r\n\x1b[93m⚠ task {} is already running\x1b[0m", id));
                        return out;
                    }
                    out.push_str(&format!("\r\n\x1b[33m▶ executing\x1b[0m {}…", task.name));

                    // Update status to running
                    let _ = repo.update_status(id, TaskStatus::Running).await;
                    let _ = event_tx.send(TaskEvent {
                        event: "started".into(),
                        task_id: id.to_string(),
                        task_name: task.name.clone(),
                        status: "running".into(),
                    });

                    let task_path = crate::server::resolve_wasm_file_path_pub(&task.path);
                    let start_time = std::time::Instant::now();
                    let timeout_secs = config.limits.execution_timeout_secs;

                    let task_path_ws = task_path.clone();
                    let exec_result = match tokio::time::timeout(
                        tokio::time::Duration::from_secs(timeout_secs),
                        actix_web::web::block(move || execute_wasm_file(&task_path_ws, None)),
                    ).await {
                        Ok(Ok(Ok(result))) => result,
                        Ok(Ok(Err(e))) => {
                            crate::run_wasm::ExecutionResult::failure(
                                e, 0, 0, 0, vec![], 0,
                                start_time.elapsed().as_micros() as u64, vec![],
                                "Permissive".to_string(),
                            )
                        }
                        Ok(Err(e)) => {
                            crate::run_wasm::ExecutionResult::failure(
                                format!("Thread error: {}", e), 0, 0, 0, vec![], 0,
                                start_time.elapsed().as_micros() as u64, vec![],
                                "Permissive".to_string(),
                            )
                        }
                        Err(_) => {
                            crate::run_wasm::ExecutionResult::failure(
                                format!("Execution timed out after {}s", timeout_secs),
                                0, 0, 0, vec![], 0,
                                start_time.elapsed().as_micros() as u64, vec![],
                                "Permissive".to_string(),
                            )
                        }
                    };

                    let duration_us = start_time.elapsed().as_micros() as i64;
                    let final_status = if exec_result.success { TaskStatus::Completed } else { TaskStatus::Failed };
                    let status_label = format!("{:?}", final_status);

                    // Update DB
                    let _ = repo.update_status(id, final_status).await;
                    let _ = repo.add_execution(
                        id, duration_us, exec_result.success, exec_result.error.clone(),
                        exec_result.instructions_executed as i64,
                        exec_result.syscalls_executed as i64,
                        exec_result.memory_used_bytes as i64,
                    ).await;

                    // Broadcast event
                    let _ = event_tx.send(TaskEvent {
                        event: if exec_result.success { "completed".into() } else { "failed".into() },
                        task_id: id.to_string(),
                        task_name: task.name.clone(),
                        status: status_label,
                    });

                    // Update Prometheus
                    metrics::TASK_EXECUTIONS_TOTAL
                        .with_label_values(&[if exec_result.success { "success" } else { "failed" }])
                        .inc();
                    metrics::TASK_EXECUTION_DURATION
                        .with_label_values(&[&task.name])
                        .observe(duration_us as f64 / 1_000_000.0);

                    if exec_result.success {
                        out.push_str(&format!(
                            "\r\n\x1b[92m✓ completed\x1b[0m  duration={}µs  instr={}  syscalls={}  mem={}B",
                            exec_result.duration_us,
                            exec_result.instructions_executed,
                            exec_result.syscalls_executed,
                            exec_result.memory_used_bytes,
                        ));
                        if !exec_result.stdout_log.is_empty() {
                            out.push_str("\r\n\x1b[90m── stdout ──\x1b[0m");
                            for l in &exec_result.stdout_log {
                                out.push_str(&format!("\r\n  \x1b[96m{}\x1b[0m", l));
                            }
                        }
                        if let Some(ref rv) = exec_result.return_value {
                            out.push_str(&format!("\r\n\x1b[33m↩ return:\x1b[0m {}", rv));
                        }
                    } else {
                        out.push_str(&format!(
                            "\r\n\x1b[91m✗ failed\x1b[0m  {}",
                            exec_result.error.as_deref().unwrap_or("unknown error")
                        ));
                    }
                }
                Ok(None) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ task not found: {}\x1b[0m", id));
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ error: {}\x1b[0m", e));
                }
            }
        }

        // ── Stop ──────────────────────────────────────────────────────────
        "stop" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: stop <task-id>\x1b[0m"); return out; }
            };
            match repo.get_by_id(id).await {
                Ok(Some(task)) => {
                    if task.status != TaskStatus::Running {
                        out.push_str(&format!("\r\n\x1b[93m⚠ task {} is not running (status: {:?})\x1b[0m", id, task.status));
                        return out;
                    }
                    let _ = repo.update_status(id, TaskStatus::Stopped).await;
                    out.push_str(&format!("\r\n\x1b[92m✓ stopped\x1b[0m {}", task.name));
                }
                Ok(None) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ task not found: {}\x1b[0m", id));
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ error: {}\x1b[0m", e));
                }
            }
        }

        // ── Kill (stop + scheduler preempt) ───────────────────────────────
        "kill" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: kill <task-id>\x1b[0m"); return out; }
            };
            let preempted = scheduler.preempt_task(id).await;
            let _ = repo.update_status(id, TaskStatus::Stopped).await;
            if preempted {
                out.push_str(&format!("\r\n\x1b[92m✓ killed\x1b[0m {} (preempted from scheduler)", id));
            } else {
                out.push_str(&format!("\r\n\x1b[92m✓ stopped\x1b[0m {} (was not in scheduler queue)", id));
            }
        }

        // ── Delete ────────────────────────────────────────────────────────
        "rm" | "delete" | "remove" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: rm <task-id>\x1b[0m"); return out; }
            };
            // Check if task exists first
            match repo.get_by_id(id).await {
                Ok(Some(_)) => {
                    match repo.delete(id).await {
                        Ok(()) => {
                            out.push_str(&format!("\r\n\x1b[92m✓ deleted\x1b[0m {}", id));
                        }
                        Err(e) => {
                            out.push_str(&format!("\r\n\x1b[91m✗ delete failed: {}\x1b[0m", e));
                        }
                    }
                }
                Ok(None) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ task not found: {}\x1b[0m", id));
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ error: {}\x1b[0m", e));
                }
            }
        }

        // ── Stats ─────────────────────────────────────────────────────────
        "stats" => {
            match repo.get_stats().await {
                Ok(s) => {
                    out.push_str("\r\n\x1b[1m📊 System Stats\x1b[0m");
                    out.push_str(&format!("\r\n  Total Tasks:      \x1b[96m{}\x1b[0m", s.total_tasks));
                    out.push_str(&format!("\r\n  Running:          \x1b[92m{}\x1b[0m", s.running_tasks));
                    out.push_str(&format!("\r\n  Completed:        \x1b[96m{}\x1b[0m", s.completed_tasks));
                    out.push_str(&format!("\r\n  Failed:           \x1b[91m{}\x1b[0m", s.failed_tasks));
                    out.push_str(&format!("\r\n  Pending:          \x1b[33m{}\x1b[0m", s.pending_tasks));
                    out.push_str(&format!("\r\n  Total Runs:       \x1b[96m{}\x1b[0m", s.total_runs));
                    out.push_str(&format!("\r\n  Instructions:     \x1b[33m{}\x1b[0m", s.total_instructions));
                    out.push_str(&format!("\r\n  Syscalls:         \x1b[33m{}\x1b[0m", s.total_syscalls));
                    out.push_str(&format!("\r\n  Avg Duration:     \x1b[33m{}µs\x1b[0m", s.avg_duration_us));
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ stats failed: {}\x1b[0m", e));
                }
            }
        }

        // ── Health ────────────────────────────────────────────────────────
        "health" | "ping" => {
            match repo.health_check().await {
                Ok(()) => {
                    out.push_str("\r\n\x1b[92m● backend\x1b[0m  status=\x1b[92mhealthy\x1b[0m  db=\x1b[92mconnected\x1b[0m");
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ backend\x1b[0m  status=\x1b[91munhealthy\x1b[0m  error={}", e));
                }
            }
        }

        // ── Metrics ───────────────────────────────────────────────────────
        "metrics" => {
            match metrics::encode_metrics() {
                Ok(txt) => {
                    let lines: Vec<&str> = txt.lines()
                        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                        .take(25)
                        .collect();
                    out.push_str("\r\n\x1b[90m── Prometheus metrics (first 25 non-comment lines) ──\x1b[0m");
                    for l in lines {
                        out.push_str(&format!("\r\n  \x1b[90m{}\x1b[0m", l));
                    }
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ metrics error: {}\x1b[0m", e));
                }
            }
        }

        // ── Scheduler status ──────────────────────────────────────────────
        "scheduler" | "sched" => {
            let ss = scheduler.status_snapshot().await;
            out.push_str("\r\n\x1b[1m⏱ Scheduler Status\x1b[0m");
            out.push_str(&format!("\r\n  Queued:           \x1b[33m{}\x1b[0m", ss.queued));
            out.push_str(&format!("\r\n  Running:          \x1b[92m{}\x1b[0m", ss.running));
            out.push_str(&format!("\r\n  Max Concurrent:   \x1b[96m{}\x1b[0m", ss.max_concurrent));
            out.push_str(&format!("\r\n  Time Slice:       {}ms", ss.slice_ms));
            out.push_str(&format!("\r\n  Timeout:          {}s", ss.timeout_secs));
        }

        // ── Traces ────────────────────────────────────────────────────────
        "traces" | "trace" => {
            let n: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(10);
            let traces = trace_store.recent(n).await;
            if traces.is_empty() {
                out.push_str("\r\n\x1b[90m(no traces recorded)\x1b[0m");
            } else {
                out.push_str(&format!("\r\n\x1b[1m🔍 Recent Traces ({})\x1b[0m", traces.len()));
                out.push_str("\r\n\x1b[90m");
                out.push_str(&"─".repeat(90));
                out.push_str("\x1b[0m");
                out.push_str(&format!(
                    "\r\n{:<36} {:<20} {:<10} {:<12} {:<8}",
                    "TRACE ID", "TASK", "SUCCESS", "DURATION", "SPANS"
                ));
                out.push_str("\r\n\x1b[90m");
                out.push_str(&"─".repeat(90));
                out.push_str("\x1b[0m");
                for t in &traces {
                    let sc = if t.success { "\x1b[92m✓\x1b[0m" } else { "\x1b[91m✗\x1b[0m" };
                    let dur = t.total_duration_us.map(|d| format!("{}µs", d)).unwrap_or_else(|| "—".into());
                    out.push_str(&format!(
                        "\r\n{:<36} {:<20} {:<10} {:<12} {:<8}",
                        &t.trace_id[..t.trace_id.len().min(35)],
                        &t.task_name[..t.task_name.len().min(19)],
                        sc,
                        dur,
                        t.spans.len()
                    ));
                }
                out.push_str("\r\n\x1b[90m");
                out.push_str(&"─".repeat(90));
                out.push_str("\x1b[0m");
            }
        }

        // ── Audit log ─────────────────────────────────────────────────────
        "audit" => {
            let n: usize = args.first().and_then(|a| a.parse().ok()).unwrap_or(10);
            match repo.list_audit_log(n as i64).await {
                Ok(entries) => {
                    if entries.is_empty() {
                        out.push_str("\r\n\x1b[90m(no audit entries)\x1b[0m");
                    } else {
                        out.push_str(&format!("\r\n\x1b[1m📋 Audit Log (last {})\x1b[0m", entries.len()));
                        for e in &entries {
                            out.push_str(&format!(
                                "\r\n  \x1b[90m{}\x1b[0m  \x1b[33m{}\x1b[0m ({})  {}  {}",
                                e.ts,
                                e.user_name,
                                e.role,
                                e.action,
                                e.resource.as_deref().unwrap_or("")
                            ));
                        }
                    }
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ audit log error: {}\x1b[0m", e));
                }
            }
        }

        // ── Security analysis ─────────────────────────────────────────────
        "security" | "scan" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: security <task-id>\x1b[0m"); return out; }
            };
            match repo.get_by_id(id).await {
                Ok(Some(task)) => {
                    let file_path = crate::server::resolve_wasm_file_path_pub(&task.path);
                    match std::fs::read(&file_path) {
                        Ok(bytes) => {
                            let is_wasm = bytes.len() >= 4 && bytes[0..4] == [0x00, 0x61, 0x73, 0x6D];
                            out.push_str(&format!("\r\n\x1b[1m🔒 Security Report: {} ({})\x1b[0m", task.name, id));
                            out.push_str(&format!("\r\n  File Size:     {} bytes", bytes.len()));
                            out.push_str(&format!("\r\n  Valid WASM:    {}", if is_wasm { "\x1b[92myes\x1b[0m" } else { "\x1b[91mno\x1b[0m" }));
                            let content = String::from_utf8_lossy(&bytes);
                            let suspicious: Vec<&str> = ["fd_write", "fd_read", "proc_exit", "environ", "args_get", "sock_"]
                                .iter()
                                .filter(|s| content.contains(**s))
                                .copied()
                                .collect();
                            if suspicious.is_empty() {
                                out.push_str("\r\n  Risk Level:    \x1b[92mLOW\x1b[0m");
                                out.push_str("\r\n  Summary:       No suspicious WASI imports detected");
                            } else {
                                let risk = if suspicious.len() > 3 { "HIGH" } else { "MEDIUM" };
                                let color = if risk == "HIGH" { "91" } else { "93" };
                                out.push_str(&format!("\r\n  Risk Level:    \x1b[{}m{}\x1b[0m", color, risk));
                                out.push_str("\r\n  Capabilities:");
                                for s in &suspicious {
                                    out.push_str(&format!("\r\n    \x1b[93m⚠\x1b[0m  {}", s));
                                }
                            }
                        }
                        Err(e) => {
                            out.push_str(&format!("\r\n\x1b[91m✗ cannot read file: {}\x1b[0m", e));
                        }
                    }
                }
                Ok(None) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ task not found: {}\x1b[0m", id));
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ error: {}\x1b[0m", e));
                }
            }
        }

        // ── Logs ──────────────────────────────────────────────────────────
        "logs" | "log" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: logs <task-id>\x1b[0m"); return out; }
            };
            match repo.get_execution_history(id, 5).await {
                Ok(execs) => {
                    if execs.is_empty() {
                        out.push_str(&format!("\r\n\x1b[90m(no execution history for {})\x1b[0m", id));
                    } else {
                        out.push_str(&format!("\r\n\x1b[1m📄 Logs for {}\x1b[0m", id));
                        for ex in &execs {
                            let sc = if ex.success { "\x1b[92m✓\x1b[0m" } else { "\x1b[91m✗\x1b[0m" };
                            let dur = ex.duration_us.unwrap_or(0);
                            out.push_str(&format!(
                                "\r\n  {} {}  duration={}µs  instr={}  syscalls={}  mem={}B",
                                sc, ex.started_at,
                                dur,
                                ex.instructions_executed,
                                ex.syscalls_executed,
                                ex.memory_used_bytes,
                            ));
                            if let Some(ref err) = ex.error {
                                out.push_str(&format!("\r\n    \x1b[91merror: {}\x1b[0m", err));
                            }
                        }
                    }
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ logs error: {}\x1b[0m", e));
                }
            }
        }

        // ── Execution history ─────────────────────────────────────────────
        "history" | "hist" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: history <task-id>\x1b[0m"); return out; }
            };
            let n: i64 = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(10);
            match repo.get_execution_history(id, n).await {
                Ok(execs) => {
                    if execs.is_empty() {
                        out.push_str(&format!("\r\n\x1b[90m(no execution history for {})\x1b[0m", id));
                    } else {
                        out.push_str(&format!("\r\n\x1b[1m📜 Execution History for {} ({} records)\x1b[0m", id, execs.len()));
                        out.push_str(&format!(
                            "\r\n  {:<36} {:<10} {:<12} {:<10} {:<10}",
                            "EXECUTION ID", "SUCCESS", "DURATION", "INSTR", "SYSCALLS"
                        ));
                        for ex in &execs {
                            let sc = if ex.success { "\x1b[92m✓\x1b[0m " } else { "\x1b[91m✗\x1b[0m " };
                            let dur = ex.duration_us.unwrap_or(0);
                            out.push_str(&format!(
                                "\r\n  {:<36} {}{:<8} {:<12} {:<10} {:<10}",
                                &ex.execution_id,
                                sc,
                                "",
                                format!("{}µs", dur),
                                ex.instructions_executed,
                                ex.syscalls_executed
                            ));
                        }
                    }
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ history error: {}\x1b[0m", e));
                }
            }
        }

        // ── Snapshots ─────────────────────────────────────────────────────
        "snapshots" | "snap" => {
            let id = match args.first() {
                Some(id) => *id,
                None => { out.push_str("\r\n\x1b[91musage: snapshots <task-id>\x1b[0m"); return out; }
            };
            match repo.list_snapshots(id).await {
                Ok(snaps) => {
                    if snaps.is_empty() {
                        out.push_str(&format!("\r\n\x1b[90m(no snapshots for {})\x1b[0m", id));
                    } else {
                        out.push_str(&format!("\r\n\x1b[1m📸 Snapshots for {} ({} total)\x1b[0m", id, snaps.len()));
                        for s in &snaps {
                            out.push_str(&format!(
                                "\r\n  \x1b[96m{}\x1b[0m  captured={}  note={}",
                                s.id, s.captured_at,
                                s.note.as_deref().unwrap_or("—")
                            ));
                        }
                    }
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ snapshot error: {}\x1b[0m", e));
                }
            }
        }

        // ── Test files ────────────────────────────────────────────────────
        "testfiles" | "tests" => {
            let wasm_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("wasm_files");
            match std::fs::read_dir(&wasm_dir) {
                Ok(entries) => {
                    let mut files: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .map(|e| e.file_name().to_string_lossy().to_string())
                        .filter(|n| n.ends_with(".wasm") || n.ends_with(".wat"))
                        .collect();
                    files.sort();
                    if files.is_empty() {
                        out.push_str("\r\n\x1b[90m(no test files found)\x1b[0m");
                    } else {
                        out.push_str(&format!("\r\n\x1b[1m📁 Test Files ({} found)\x1b[0m", files.len()));
                        for f in &files {
                            let ext = if f.ends_with(".wat") { "\x1b[33m" } else { "\x1b[96m" };
                            out.push_str(&format!("\r\n  {}{}\x1b[0m", ext, f));
                        }
                    }
                }
                Err(e) => {
                    out.push_str(&format!("\r\n\x1b[91m✗ cannot list test files: {}\x1b[0m", e));
                }
            }
        }

        // ── Config ────────────────────────────────────────────────────────
        "config" | "cfg" => {
            out.push_str("\r\n\x1b[1m⚙ Server Configuration\x1b[0m");
            out.push_str(&format!("\r\n  Host:               {}:{}", config.server.host, config.server.port));
            out.push_str(&format!("\r\n  Workers:            {}", config.server.workers));
            out.push_str(&format!("\r\n  Auth Enabled:       {}", config.security.auth_enabled));
            out.push_str(&format!("\r\n  Rate Limit:         {}/min", config.security.rate_limit_per_minute));
            out.push_str(&format!("\r\n  Max Memory:         {}MB", config.limits.max_memory_mb));
            out.push_str(&format!("\r\n  Max Instructions:   {}", config.limits.max_instructions));
            out.push_str(&format!("\r\n  Exec Timeout:       {}s", config.limits.execution_timeout_secs));
            out.push_str(&format!("\r\n  Max Concurrent:     {}", config.limits.max_concurrent_tasks));
            out.push_str(&format!("\r\n  Max Stack Depth:    {}", config.limits.max_stack_depth));
            out.push_str(&format!("\r\n  Log Level:          {}", config.logging.level));
        }

        // ── Version ───────────────────────────────────────────────────────
        "version" | "ver" => {
            out.push_str("\r\n\x1b[96mWASM-OS Command Center v5.0\x1b[0m");
            out.push_str("\r\n  Engine:   wasmos-rs");
            out.push_str("\r\n  Protocol: wasmos-v2 (WebSocket)");
            out.push_str("\r\n  Mode:     Terminal (server-side processing)");
        }

        // ── Whoami ────────────────────────────────────────────────────────
        "whoami" | "who" => {
            out.push_str("\r\n\x1b[1m👤 Session Info\x1b[0m");
            out.push_str("\r\n  Mode:     \x1b[96mTerminal (WebSocket)\x1b[0m");
            out.push_str("\r\n  Auth:     ");
            if config.security.auth_enabled {
                out.push_str("\x1b[92menabled\x1b[0m");
            } else {
                out.push_str("\x1b[93mdisabled (dev mode)\x1b[0m");
            }
        }

        // ── Uptime ────────────────────────────────────────────────────────
        "uptime" => {
            out.push_str("\r\n\x1b[92m● server is running\x1b[0m");
            match repo.health_check().await {
                Ok(()) => out.push_str("  db=\x1b[92mconnected\x1b[0m"),
                Err(_) => out.push_str("  db=\x1b[91mdisconnected\x1b[0m"),
            }
        }

        // ── Live metrics ──────────────────────────────────────────────────
        "live" | "livemetrics" => {
            let m = trace_store.live_metrics(100).await;
            out.push_str("\r\n\x1b[1m📈 Live Metrics\x1b[0m");
            out.push_str(&format!("\r\n  Window Size:      {}", m.window_size));
            out.push_str(&format!("\r\n  Success Rate:     \x1b[92m{:.1}%\x1b[0m", m.success_rate * 100.0));
            out.push_str(&format!("\r\n  Error Rate:       \x1b[91m{:.1}%\x1b[0m", m.error_rate * 100.0));
            out.push_str(&format!("\r\n  P50 Latency:      {}µs", m.p50_us));
            out.push_str(&format!("\r\n  P95 Latency:      {}µs", m.p95_us));
            out.push_str(&format!("\r\n  P99 Latency:      {}µs", m.p99_us));
            out.push_str(&format!("\r\n  Avg Latency:      {}µs", m.avg_us));
            out.push_str(&format!("\r\n  Throughput:       {:.2}/min", m.throughput_per_min));
        }

        // ── Clear ─────────────────────────────────────────────────────────
        "clear" | "cls" => {
            out.push_str("\x1b[2J\x1b[H");
        }

        // ── Exit ──────────────────────────────────────────────────────────
        "exit" | "quit" | "logout" => {
            out.push_str("\r\n\x1b[90mClosing terminal session…\x1b[0m");
        }

        // ── Unknown command ───────────────────────────────────────────────
        _ => {
            out.push_str(&format!(
                "\r\n\x1b[91m✗ command not found:\x1b[0m {}  (type \x1b[96mhelp\x1b[0m for available commands)",
                cmd
            ));
        }
    }

    out
}

use actix::{Actor, StreamHandler, AsyncContext, ActorContext};
use actix_web::{web, HttpRequest, HttpResponse, Error};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::db::repository::TaskRepository;

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum WsMessage {
    #[serde(rename = "task_update")]
    TaskUpdate {
        task_id: String,
        status: String,
        metrics: Option<TaskMetrics>,
    },
    #[serde(rename = "system_stats")]
    SystemStats {
        total_tasks: i64,
        running_tasks: i64,
        completed_tasks: i64,
    },
    #[serde(rename = "ping")]
    Ping,
    #[serde(rename = "pong")]
    Pong,
}

#[derive(Serialize, Deserialize)]
struct TaskMetrics {
    instructions: i64,
    syscalls: i64,
    memory_bytes: i64,
}

pub struct WsConnection {
    task_repo: Arc<TaskRepository>,
    hb: Instant,
}

impl WsConnection {
    pub fn new(task_repo: Arc<TaskRepository>) -> Self {
        Self {
            task_repo,
            hb: Instant::now(),
        }
    }

    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                tracing::warn!("WebSocket client heartbeat failed, disconnecting");
                ctx.stop();
                return;
            }

            let msg = serde_json::to_string(&WsMessage::Ping).unwrap();
            ctx.text(msg);
        });
    }

    async fn send_task_update(&self, ctx: &mut ws::WebsocketContext<Self>, task_id: &str) {
        if let Ok(Some(task)) = self.task_repo.get_task_by_id(task_id).await {
            let metrics = self.task_repo.get_task_metrics(task_id).await.ok().flatten();
            
            let msg = WsMessage::TaskUpdate {
                task_id: task.id,
                status: task.status.to_string(),
                metrics: metrics.map(|m| TaskMetrics {
                    instructions: m.total_instructions,
                    syscalls: m.total_syscalls,
                    memory_bytes: 0, // TODO: Add memory tracking
                }),
            };
            
            if let Ok(json) = serde_json::to_string(&msg) {
                ctx.text(json);
            }
        }
    }

    async fn send_system_stats(&self, ctx: &mut ws::WebsocketContext<Self>) {
        if let Ok(stats) = self.task_repo.get_system_stats().await {
            let msg = WsMessage::SystemStats {
                total_tasks: stats.get("total_tasks").and_then(|v| v.as_i64()).unwrap_or(0),
                running_tasks: stats.get("running_tasks").and_then(|v| v.as_i64()).unwrap_or(0),
                completed_tasks: stats.get("completed_tasks").and_then(|v| v.as_i64()).unwrap_or(0),
            };
            
            if let Ok(json) = serde_json::to_string(&msg) {
                ctx.text(json);
            }
        }
    }
}

impl Actor for WsConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        tracing::info!("WebSocket connection established");
        self.hb(ctx);
    }

    fn stopped(&mut self, _: &mut Self::Context) {
        tracing::info!("WebSocket connection closed");
    }
}

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
                
                // Handle incoming messages
                if let Ok(msg) = serde_json::from_str::<WsMessage>(&text) {
                    match msg {
                        WsMessage::Ping => {
                            let pong = serde_json::to_string(&WsMessage::Pong).unwrap();
                            ctx.text(pong);
                        }
                        _ => {}
                    }
                }
            }
            Ok(ws::Message::Binary(_)) => {
                tracing::warn!("Binary messages not supported");
            }
            Ok(ws::Message::Close(reason)) => {
                tracing::info!("WebSocket close: {:?}", reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<crate::server::AppState>,
) -> Result<HttpResponse, Error> {
    let conn = WsConnection::new(data.task_repo.clone());
    ws::start(conn, &req, stream)
}

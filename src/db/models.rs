use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Stopped,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Pending => write!(f, "pending"),
            TaskStatus::Running => write!(f, "running"),
            TaskStatus::Completed => write!(f, "completed"),
            TaskStatus::Failed => write!(f, "failed"),
            TaskStatus::Stopped => write!(f, "stopped"),
        }
    }
}

impl std::str::FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pending" => Ok(TaskStatus::Pending),
            "running" => Ok(TaskStatus::Running),
            "completed" => Ok(TaskStatus::Completed),
            "failed" => Ok(TaskStatus::Failed),
            "stopped" => Ok(TaskStatus::Stopped),
            _ => Err(format!("Invalid task status: {}", s)),
        }
    }
}

impl From<String> for TaskStatus {
    fn from(s: String) -> Self {
        s.parse().unwrap_or(TaskStatus::Pending)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Task {
    pub id: String,
    pub name: String,
    pub path: String,
    #[sqlx(try_from = "String")]
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub file_size_bytes: i64,
}

impl Task {
    pub fn new(name: String, path: String, file_size_bytes: i64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            path,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            file_size_bytes,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionHistory {
    pub task_id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_us: Option<i64>,
    pub success: bool,
    pub error: Option<String>,
    pub instructions_executed: i64,
    pub syscalls_executed: i64,
    pub memory_used_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskMetrics {
    pub task_id: String,
    pub total_runs: i64,
    pub successful_runs: i64,
    pub failed_runs: i64,
    pub total_instructions: i64,
    pub total_syscalls: i64,
    pub avg_duration_us: i64,
    pub last_run_at: Option<DateTime<Utc>>,
}

impl Default for TaskMetrics {
    fn default() -> Self {
        Self {
            task_id: String::new(),
            total_runs: 0,
            successful_runs: 0,
            failed_runs: 0,
            total_instructions: 0,
            total_syscalls: 0,
            avg_duration_us: 0,
            last_run_at: None,
        }
    }
}

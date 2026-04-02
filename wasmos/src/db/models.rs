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
    pub tenant_id: Option<String>,
    pub priority: i16,
}

impl Task {
    #[allow(dead_code)]
    pub fn new(name: String, path: String, file_size_bytes: i64) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            path,
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            file_size_bytes,
            tenant_id: None,
            priority: 5,
        }
    }

    #[allow(dead_code)]
    pub fn with_tenant(mut self, tenant_id: String) -> Self {
        self.tenant_id = Some(tenant_id);
        self
    }

    #[allow(dead_code)]
    pub fn with_priority(mut self, priority: i16) -> Self {
        self.priority = priority.clamp(1, 10);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ExecutionHistory {
    pub id: i64,
    /// Stable UUID assigned at insertion time.
    /// Used by /v2/execution/{execution_id}/report to identify records
    /// independently of the auto-increment SERIAL id.
    pub execution_id: String,
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
pub struct Snapshot {
    pub id: String,
    pub task_id: String,
    pub captured_at: DateTime<Utc>,
    pub state: String,
    pub memory_mb: f32,
    pub instructions: i64,
    pub stack_depth: i32,
    pub globals_json: String,
    pub note: Option<String>,
}

impl Snapshot {
    pub fn new(task_id: String, state: String, memory_mb: f32, instructions: i64, stack_depth: i32) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            captured_at: Utc::now(),
            state,
            memory_mb,
            instructions,
            stack_depth,
            globals_json: "{}".to_string(),
            note: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AuditLog {
    pub id: i64,
    pub ts: DateTime<Utc>,
    pub user_name: String,
    pub role: String,
    pub action: String,
    pub resource: Option<String>,
    pub tenant_id: Option<String>,
    pub ip_addr: Option<String>,
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

/// A multi-tenant isolation boundary stored in the `tenants` table.
/// Resource quota columns control how much this tenant is allowed to consume.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub max_tasks: i32,
    pub max_memory_mb: i32,
    /// Stored as SMALLINT in Postgres.
    pub max_cpu_percent: i16,
    pub max_concurrent: i32,
    pub max_wasm_size_mb: i32,
    pub active: bool,
    pub created_at: DateTime<Utc>,
}

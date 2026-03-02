use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;
use std::sync::Arc;

use crate::db::models::{Task, TaskStatus};
use crate::db::repository::TaskRepository;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledTask {
    pub task_id: String,
    pub priority: u8,
    pub dependencies: Vec<String>,
    pub scheduled_at: DateTime<Utc>,
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then earlier scheduled time
        other.priority.cmp(&self.priority)
            .then_with(|| self.scheduled_at.cmp(&other.scheduled_at))
    }
}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct Scheduler {
    queue: Arc<RwLock<BinaryHeap<ScheduledTask>>>,
    running_tasks: Arc<RwLock<HashMap<String, TaskHandle>>>,
    task_repo: Arc<TaskRepository>,
    max_concurrent: usize,
}

pub struct TaskHandle {
    pub task_id: String,
    pub started_at: DateTime<Utc>,
}

impl Scheduler {
    pub fn new(task_repo: Arc<TaskRepository>, max_concurrent: usize) -> Self {
        Self {
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            task_repo,
            max_concurrent,
        }
    }

    pub async fn schedule(&self, task: ScheduledTask) {
        let mut queue = self.queue.write().await;
        queue.push(task);
    }

    pub async fn run(&self) {
        loop {
            let task = {
                let mut queue = self.queue.write().await;
                queue.pop()
            };

            if let Some(task) = task {
                if self.can_run(&task).await {
                    self.execute(task).await;
                } else {
                    // Put it back in the queue
                    let mut queue = self.queue.write().await;
                    queue.push(task);
                }
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    async fn can_run(&self, task: &ScheduledTask) -> bool {
        let running = self.running_tasks.read().await;
        
        // Check concurrent limit
        if running.len() >= self.max_concurrent {
            return false;
        }

        // Check dependencies
        task.dependencies.iter().all(|dep| !running.contains_key(dep))
    }

    async fn execute(&self, task: ScheduledTask) {
        let handle = TaskHandle {
            task_id: task.task_id.clone(),
            started_at: Utc::now(),
        };

        {
            let mut running = self.running_tasks.write().await;
            running.insert(task.task_id.clone(), handle);
        }

        // Update task status to running
        let _ = self.task_repo.update_task_status(&task.task_id, TaskStatus::Running).await;

        // Execute task (placeholder - actual execution would happen here)
        tracing::info!("Executing task: {}", task.task_id);

        // Simulate execution
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

        // Remove from running tasks
        {
            let mut running = self.running_tasks.write().await;
            running.remove(&task.task_id);
        }

        // Update status to completed
        let _ = self.task_repo.update_task_status(&task.task_id, TaskStatus::Completed).await;
    }

    pub async fn get_queue_size(&self) -> usize {
        self.queue.read().await.len()
    }

    pub async fn get_running_count(&self) -> usize {
        self.running_tasks.read().await.len()
    }
}

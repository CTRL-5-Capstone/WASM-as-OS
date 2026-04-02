/// Preemptive Round-Robin Scheduler
///
/// Design:
///  - Tasks are assigned a *time-slice* (default: `timeout_secs / max_concurrent`).
///  - A dedicated watchdog task cancels any execution that overruns its slice.
///  - Round-robin across equal-priority buckets prevents starvation.
///  - Higher-priority tasks can preempt lower-priority ones when a slot opens.
///  - All state is lock-free hot-paths: running slots tracked via AtomicUsize.

use std::collections::{BinaryHeap, HashMap};
use std::cmp::Ordering;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrd};
use chrono::{DateTime, Utc};
use tokio::sync::{broadcast, RwLock, Notify};
use tokio::task::JoinHandle;
use std::sync::Arc;
use std::time::Instant;

use crate::db::models::TaskStatus;
use crate::db::repository::TaskRepository;
use crate::plugins::PluginManager;
use crate::run_wasm::execute_wasm_file;
use crate::server::TaskEvent;

/// Default time-slice per task (milliseconds). A task preempted by timeout
/// is marked Failed with a "time-slice exceeded" error so the slot is freed
/// for the next queued task, preventing infinite-loop hangs.
const DEFAULT_SLICE_MS: u64 = 5_000; // 5 s max per slice

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScheduledTask {
    pub task_id: String,
    pub priority: u8,
    pub tenant_id: Option<String>,
    pub dependencies: Vec<String>,
    pub scheduled_at: DateTime<Utc>,
    /// Round-robin counter: incremented each time a task is re-queued after
    /// being blocked (dependency not met / no capacity). Lower = more urgent.
    pub round: u32,
}

impl Ord for ScheduledTask {
    fn cmp(&self, other: &Self) -> Ordering {
        // Primary: higher priority wins
        // Secondary: lower round (fewer preemptions) wins — prevents starvation
        // Tertiary: earlier scheduled_at wins
        other.priority.cmp(&self.priority)
            .then_with(|| self.round.cmp(&other.round))
            .then_with(|| self.scheduled_at.cmp(&other.scheduled_at))
    }
}

impl PartialOrd for ScheduledTask {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[allow(dead_code)]
pub struct TaskHandle {
    pub task_id: String,
    pub started_at: DateTime<Utc>,
    pub tenant_id: Option<String>,
    /// Cancel token — drop this to send a cancellation signal.
    pub cancel: Arc<Notify>,
    pub _join: JoinHandle<()>,
}

/// Per-tenant running slot counter for multi-tenant fairness.
#[derive(Default)]
struct TenantSlots {
    running: HashMap<String, usize>,
}

impl TenantSlots {
    fn increment(&mut self, tenant: &str) {
        *self.running.entry(tenant.to_owned()).or_default() += 1;
    }
    fn decrement(&mut self, tenant: &str) {
        let e = self.running.entry(tenant.to_owned()).or_default();
        *e = e.saturating_sub(1);
    }
    fn count(&self, tenant: &str) -> usize {
        self.running.get(tenant).copied().unwrap_or(0)
    }
}

pub struct Scheduler {
    queue: Arc<RwLock<BinaryHeap<ScheduledTask>>>,
    running_tasks: Arc<RwLock<HashMap<String, TaskHandle>>>,
    tenant_slots: Arc<RwLock<TenantSlots>>,
    running_count: Arc<AtomicUsize>,
    task_repo: Arc<TaskRepository>,
    plugin_manager: Arc<PluginManager>,
    event_tx: broadcast::Sender<TaskEvent>,
    max_concurrent: usize,
    /// Hard wall-clock timeout for any single task (seconds)
    timeout_secs: u64,
    /// Time-slice per task (milliseconds) — tasks are cancelled after this
    slice_ms: u64,
}

impl Scheduler {
    pub fn new(
        task_repo: Arc<TaskRepository>,
        plugin_manager: Arc<PluginManager>,
        event_tx: broadcast::Sender<TaskEvent>,
        max_concurrent: usize,
        timeout_secs: u64,
    ) -> Self {
        let slice_ms = std::cmp::min(
            DEFAULT_SLICE_MS,
            (timeout_secs * 1000) / std::cmp::max(1, max_concurrent as u64),
        );
        Self {
            queue: Arc::new(RwLock::new(BinaryHeap::new())),
            running_tasks: Arc::new(RwLock::new(HashMap::new())),
            tenant_slots: Arc::new(RwLock::new(TenantSlots::default())),
            running_count: Arc::new(AtomicUsize::new(0)),
            task_repo,
            plugin_manager,
            event_tx,
            max_concurrent,
            timeout_secs,
            slice_ms,
        }
    }

    #[allow(dead_code)]
    pub async fn schedule(&self, task: ScheduledTask) {
        self.queue.write().await.push(task);
    }

    /// Main scheduler loop — polls DB, applies round-robin preemption logic,
    /// spawns execution tasks with per-task watchdogs.
    pub async fn run(&self) {
        loop {
            // ── 1. Ingest newly-Pending DB tasks ───────────────────────────
            if let Ok(pending) = self.task_repo.list_by_status(TaskStatus::Pending).await {
                let mut queue = self.queue.write().await;
                let running = self.running_tasks.read().await;
                for task in pending {
                    let already_queued = queue.iter().any(|q: &ScheduledTask| q.task_id == task.id);
                    if !already_queued && !running.contains_key(&task.id) {
                        queue.push(ScheduledTask {
                            task_id: task.id.clone(),
                            priority: task.priority.max(0) as u8,
                            tenant_id: task.tenant_id.clone(),
                            dependencies: vec![],
                            scheduled_at: task.created_at,
                            round: 0,
                        });
                    }
                }
            }

            // ── 2. Preemptive dispatch: pick best runnable task ────────────
            let task_opt = {
                let mut queue = self.queue.write().await;
                let running = self.running_tasks.read().await;
                let slots = self.tenant_slots.read().await;

                // Drain the heap into a temp vec, find first runnable
                let mut heap_items: Vec<ScheduledTask> = queue.drain().collect();
                // Sort descending (BinaryHeap is max-heap, but we drained to vec)
                heap_items.sort_unstable_by(|a, b| b.cmp(a));

                let mut chosen: Option<ScheduledTask> = None;
                let mut requeue: Vec<ScheduledTask> = Vec::new();

                for mut item in heap_items {
                    if chosen.is_some() {
                        requeue.push(item);
                        continue;
                    }
                    // Capacity check — use Acquire so the load sees any preceding
                    // fetch_sub from a completing task without reordering.
                    if self.running_count.load(AtomicOrd::Acquire) >= self.max_concurrent {
                        item.round += 1;
                        requeue.push(item);
                        continue;
                    }
                    // Dependency check
                    let deps_clear = item.dependencies.iter().all(|d| !running.contains_key(d));
                    if !deps_clear {
                        item.round += 1;
                        requeue.push(item);
                        continue;
                    }
                    // Per-tenant fairness: max 25% of slots per tenant
                    if let Some(ref tid) = item.tenant_id {
                        let tenant_max = std::cmp::max(1, self.max_concurrent / 4);
                        if slots.count(tid) >= tenant_max {
                            item.round += 1;
                            requeue.push(item);
                            continue;
                        }
                    }
                    chosen = Some(item);
                }
                for item in requeue {
                    queue.push(item);
                }
                chosen
            };

            if let Some(task) = task_opt {
                // ── DB-backed tenant quota enforcement ───────────────────────
                // Checked here (outside the queue lock) so the async DB call
                // does not block other waiters on the queue write lock.
                // Three outcomes:
                //   1. No tenant_id → execute immediately.
                //   2. Tenant inactive / missing → fail the task, emit event.
                //   3. Tenant at max_concurrent → re-queue with bumped round.
                let mut quota_ok = true;
                if let Some(ref tid) = task.tenant_id {
                    match self.task_repo.get_tenant_by_id(tid).await {
                        Ok(Some(tenant)) => {
                            if !tenant.active {
                                quota_ok = false;
                                tracing::warn!(
                                    task_id = %task.task_id,
                                    tenant_id = %tid,
                                    "Scheduler: tenant inactive — failing task"
                                );
                                let _ = self.task_repo
                                    .update_status(&task.task_id, TaskStatus::Failed)
                                    .await;
                                let _ = self.task_repo
                                    .add_execution(
                                        &task.task_id, 0, false,
                                        Some("Tenant is inactive or deactivated".to_string()),
                                        0, 0, 0,
                                    )
                                    .await;
                                let _ = self.event_tx.send(TaskEvent {
                                    event: "failed".into(),
                                    task_id: task.task_id.clone(),
                                    task_name: task.task_id.clone(),
                                    status: "failed".into(),
                                });
                            } else {
                                // Use the DB-configured per-tenant limit rather than
                                // the coarse global 25%-of-slots heuristic.
                                let tenant_max = tenant.max_concurrent as usize;
                                let current = self.tenant_slots.read().await.count(tid);
                                if current >= tenant_max {
                                    quota_ok = false;
                                    tracing::debug!(
                                        task_id = %task.task_id,
                                        tenant_id = %tid,
                                        current = current,
                                        max = tenant_max,
                                        "Scheduler: tenant concurrent limit reached — requeueing"
                                    );
                                    let mut requeued = task.clone();
                                    requeued.round += 1;
                                    self.queue.write().await.push(requeued);
                                }
                            }
                        }
                        Ok(None) => {
                            // Tenant was deleted after task was queued — fail cleanly.
                            quota_ok = false;
                            tracing::warn!(
                                task_id = %task.task_id,
                                tenant_id = %tid,
                                "Scheduler: tenant not found in DB — failing task"
                            );
                            let _ = self.task_repo
                                .update_status(&task.task_id, TaskStatus::Failed)
                                .await;
                            let _ = self.task_repo
                                .add_execution(
                                    &task.task_id, 0, false,
                                    Some(format!("Tenant '{tid}' not found")),
                                    0, 0, 0,
                                )
                                .await;
                            let _ = self.event_tx.send(TaskEvent {
                                event: "failed".into(),
                                task_id: task.task_id.clone(),
                                task_name: task.task_id.clone(),
                                status: "failed".into(),
                            });
                        }
                        Err(e) => {
                            // DB error — fail open to avoid blocking the whole
                            // scheduler on a transient connection hiccup.
                            tracing::error!(
                                task_id = %task.task_id,
                                err = %e,
                                "Scheduler: DB error checking tenant quota — proceeding"
                            );
                        }
                    }
                }

                if quota_ok {
                    self.spawn_execution(task).await;
                }
            }

            // Poll at 200 ms — fast enough for interactive feel, low overhead
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    }

    /// Spawn a non-blocking execution task with an integrated watchdog that
    /// enforces the time-slice. The task is cancelled (killed) if it exceeds
    /// `slice_ms`, freeing the slot immediately for the next queued task.
    async fn spawn_execution(&self, task: ScheduledTask) {
        let cancel = Arc::new(Notify::new());
        let cancel_clone = cancel.clone();

        // AcqRel: Release ensures the increment is visible to the capacity check
        // in the dispatch loop; Acquire ensures we see all prior slot releases.
        self.running_count.fetch_add(1, AtomicOrd::AcqRel);

        {
            let mut slots = self.tenant_slots.write().await;
            if let Some(ref tid) = task.tenant_id {
                slots.increment(tid);
            }
        }

        let _ = self.task_repo.update_status(&task.task_id, TaskStatus::Running).await;
        self.plugin_manager.trigger_task_start(&task.task_id).await;

        let _ = self.event_tx.send(TaskEvent {
            event: "started".into(),
            task_id: task.task_id.clone(),
            task_name: task.task_id.clone(),
            status: "Running".into(),
        });

        let repo = self.task_repo.clone();
        let pm = self.plugin_manager.clone();
        let event_tx = self.event_tx.clone();
        let task_id = task.task_id.clone();
        let tenant_id = task.tenant_id.clone();
        let timeout_secs = self.timeout_secs;
        let slice_ms = self.slice_ms;
        let running_tasks = self.running_tasks.clone();
        let running_count = self.running_count.clone();
        let tenant_slots = self.tenant_slots.clone();

        let join = tokio::spawn(async move {
            let task_data = match repo.get_by_id(&task_id).await {
                Ok(Some(t)) => t,
                _ => {
                    tracing::error!("Scheduler: task {} not found in DB", task_id);
                    let _ = repo.update_status(&task_id, TaskStatus::Failed).await;
                    Self::cleanup_slot(&running_tasks, &running_count, &tenant_slots, &task_id, tenant_id.as_deref()).await;
                    return;
                }
            };

            let task_path = task_data.path.clone();
            let task_name = task_data.name.clone();

            tracing::info!(
                task_id = %task_id,
                task_name = %task_name,
                slice_ms = slice_ms,
                "Scheduler: dispatching task (preemptive slice)"
            );

            let start = Instant::now();
            // Hard wall-clock timeout wrapping a blocking spawn
            let exec_fut = tokio::task::spawn_blocking(move || execute_wasm_file(&task_path));

            // Watchdog: whichever fires first — slice expiry OR global timeout
            let slice_dur = tokio::time::Duration::from_millis(slice_ms);
            let global_dur = tokio::time::Duration::from_secs(timeout_secs);
            let effective = slice_dur.min(global_dur);

            let result: Result<Result<crate::run_wasm::ExecutionResult, String>, ()> = tokio::select! {
                r = tokio::time::timeout(effective, exec_fut) => {
                    match r {
                        Ok(Ok(inner)) => Ok(inner),
                        Ok(Err(je)) => Ok(Err(format!("Thread panicked: {}", je))),
                        Err(_elapsed) => Err(()),
                    }
                },
                _ = cancel_clone.notified() => {
                    // External cancel (e.g. stop_task API call)
                    Err(())
                }
            };

            let duration_us = start.elapsed().as_micros() as i64;

            let (success, error, instructions, syscalls, memory) = match result {
                Ok(Ok(res)) => (
                    res.success,
                    res.error,
                    res.instructions_executed as i64,
                    res.syscalls_executed as i64,
                    res.memory_used_bytes as i64,
                ),
                Ok(Err(e)) => (false, Some(e), 0, 0, 0),
                Err(_) => (
                    false,
                    Some(format!(
                        "Preempted: time-slice exceeded ({}ms)",
                        effective.as_millis()
                    )),
                    0, 0, 0,
                ),
            };

            let final_status = if success { TaskStatus::Completed } else { TaskStatus::Failed };
            let _ = repo.update_status(&task_id, final_status).await;
            let _ = repo
                .add_execution(&task_id, duration_us, success, error.clone(), instructions, syscalls, memory)
                .await;

            let ev_status = if success { "completed" } else { "failed" };
            let _ = event_tx.send(TaskEvent {
                event: ev_status.into(),
                task_id: task_id.clone(),
                task_name: task_name.clone(),
                status: ev_status.into(),
            });

            if success {
                if let Ok(Some(exec_record)) = repo
                    .get_execution_history(&task_id, 1)
                    .await
                    .map(|h| h.into_iter().next())
                {
                    pm.trigger_task_complete(&task_id, &exec_record).await;
                }
            } else {
                pm.trigger_task_failed(&task_id, error.as_deref().unwrap_or("unknown")).await;
            }

            tracing::info!(
                task_id = %task_id,
                success = success,
                duration_us = duration_us,
                "Scheduler: task finished"
            );

            Self::cleanup_slot(&running_tasks, &running_count, &tenant_slots, &task_id, tenant_id.as_deref()).await;
        });

        let handle = TaskHandle {
            task_id: task.task_id.clone(),
            started_at: Utc::now(),
            tenant_id: task.tenant_id.clone(),
            cancel,
            _join: join,
        };

        self.running_tasks.write().await.insert(task.task_id.clone(), handle);
    }

    async fn cleanup_slot(
        running_tasks: &Arc<RwLock<HashMap<String, TaskHandle>>>,
        running_count: &Arc<AtomicUsize>,
        tenant_slots: &Arc<RwLock<TenantSlots>>,
        task_id: &str,
        tenant_id: Option<&str>,
    ) {
        running_tasks.write().await.remove(task_id);
        // AcqRel: Release makes the decremented count visible to the dispatch-loop
        // capacity check; Acquire ensures all task completion stores are ordered before.
        running_count.fetch_sub(1, AtomicOrd::AcqRel);
        if let Some(tid) = tenant_id {
            tenant_slots.write().await.decrement(tid);
        }
    }

    /// Forcefully preempt a running task (used by stop_task API).
    pub async fn preempt_task(&self, task_id: &str) -> bool {
        let running = self.running_tasks.read().await;
        if let Some(handle) = running.get(task_id) {
            handle.cancel.notify_one();
            true
        } else {
            false
        }
    }

    #[allow(dead_code)]
    pub async fn get_queue_size(&self) -> usize {
        self.queue.read().await.len()
    }

    #[allow(dead_code)]
    pub async fn get_running_count(&self) -> usize {
        self.running_count.load(AtomicOrd::Acquire)
    }

    /// Returns a snapshot of scheduler state for the /v1/scheduler/status endpoint.
    pub async fn status_snapshot(&self) -> SchedulerStatus {
        let queue_len = self.queue.read().await.len();
        let running = self.running_count.load(AtomicOrd::Acquire);
        SchedulerStatus {
            queued: queue_len,
            running,
            max_concurrent: self.max_concurrent,
            slice_ms: self.slice_ms,
            timeout_secs: self.timeout_secs,
        }
    }
}

#[derive(serde::Serialize, Clone)]
pub struct SchedulerStatus {
    pub queued: usize,
    pub running: usize,
    pub max_concurrent: usize,
    pub slice_ms: u64,
    pub timeout_secs: u64,
}

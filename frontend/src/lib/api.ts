/**
 * WasmOS API Client — Single source of truth.
 * All requests go through Next.js rewrites → Rust backend on :8080.
 * No hardcoded ports; the proxy handles routing.
 * JWT token is injected via Authorization: Bearer header when present.
 */
import { getToken, clearToken } from "./auth";
import { withSWR, invalidate, clearAll as clearCacheAll } from "./client-cache";

// ─── Types ──────────────────────────────────────────────────────────

export interface Task {
  id: string;
  name: string;
  path: string;
  status: string;           // pending | running | completed | failed | stopped  (backend: lowercase)
  created_at: string;
  /** ISO 8601 — updated by DB trigger on every status or metadata change */
  updated_at: string;
  file_size_bytes: number;
  tenant_id?: string | null;
  priority: number;
}

export interface TaskDetail {
  task: Task;
  metrics: TaskMetrics | null;
  recent_executions: ExecutionHistory[];
}

export interface TaskMetrics {
  task_id: string;
  total_runs: number;
  successful_runs: number;
  failed_runs: number;
  total_instructions: number;
  total_syscalls: number;
  avg_duration_us: number;
  last_run_at?: string;
}

export interface ExecutionHistory {
  id: number;
  /** Stable UUID — use this for /v2/execution/{execution_id}/report links */
  execution_id: string;
  task_id: string;
  started_at: string;
  completed_at?: string;
  duration_us?: number;
  success: boolean;
  error?: string;
  instructions_executed: number;
  syscalls_executed: number;
  memory_used_bytes: number;
}

export interface ExecutionResult {
  success: boolean;
  error?: string;
  instructions_executed: number;
  syscalls_executed: number;
  memory_used_bytes: number;
  duration_us: number;
  stdout_log: string[];
  return_value?: number | string | null;
  /** Present when backend includes the execution record id */
  execution_id?: string;
}

export interface SystemStats {
  total_tasks: number;
  running_tasks: number;
  completed_tasks: number;
  failed_tasks: number;
  pending_tasks: number;
  total_instructions: number;
  total_syscalls: number;
  total_runs: number;
  avg_duration_us: number;
}

export interface HealthStatus {
  status: string;
  database?: string;
  timestamp: string;
}

export interface BatchRequest {
  wasm_paths: string[];
  continue_on_error: boolean;
}

export interface BatchResult {
  total_files: number;
  successful: number;
  failed: number;
  /** Successful executions — each entry has an execution_id, not a path */
  results: BatchFileResult[];
  /** Failed executions — each entry has the path and error message */
  errors: BatchFileError[];
}

/** A successful item in the batch results array */
export interface BatchFileResult {
  execution_id: string;
  success: boolean;
  duration_ms: number;
  instructions: number;
}

/** A failed item in the batch errors array */
export interface BatchFileError {
  path: string;
  error: string;
}

// ─── HTTP helpers ───────────────────────────────────────────────────

export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
    this.name = "ApiError";
  }
}

/**
 * In-flight GET request deduplication map.
 * When two identical GET requests are dispatched simultaneously (e.g. two
 * components mounting at the same time), only one actual fetch is sent and
 * both callers share the same Promise.  The entry is removed once the request
 * settles, so subsequent calls always get a fresh fetch.
 */
const _inflight = new Map<string, Promise<unknown>>();

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const method = (init?.method ?? "GET").toUpperCase();

  // Deduplicate concurrent identical GET requests
  if (method === "GET") {
    const key = path;
    const existing = _inflight.get(key) as Promise<T> | undefined;
    if (existing) return existing;

    const promise = _fetchOnce<T>(path, init).finally(() => {
      _inflight.delete(key);
    });
    _inflight.set(key, promise as Promise<unknown>);
    return promise;
  }

  return _fetchOnce<T>(path, init);
}

/** Retryable status codes: network failures and server overload. */
const RETRYABLE_STATUSES = new Set([408, 429, 502, 503, 504]);

/**
 * Fetch with exponential backoff retry (GET-only).
 * Retries up to 3 times on network failures or retryable HTTP status codes.
 * Write methods (POST/PUT/DELETE) are NOT retried — they are not idempotent.
 */
async function _fetchOnce<T>(path: string, init?: RequestInit): Promise<T> {
  const method = (init?.method ?? "GET").toUpperCase();
  const isWrite = method === "POST" || method === "PUT" || method === "DELETE";
  const maxAttempts = isWrite ? 1 : 3;

  let lastErr: unknown;
  for (let attempt = 1; attempt <= maxAttempts; attempt++) {
    try {
      const result = await _fetchRaw<T>(path, init);
      return result;
    } catch (err) {
      lastErr = err;
      // Don't retry auth errors, client errors, or on last attempt
      if (err instanceof ApiError && !RETRYABLE_STATUSES.has(err.status)) throw err;
      if (attempt === maxAttempts) break;
      // Exponential backoff: 300 ms, 900 ms
      await new Promise((r) => setTimeout(r, 300 * Math.pow(3, attempt - 1)));
    }
  }
  throw lastErr;
}

async function _fetchRaw<T>(path: string, init?: RequestInit): Promise<T> {
  const controller = new AbortController();
  // Use longer timeouts for batch / upload operations, shorter for reads
  const method = (init?.method ?? "GET").toUpperCase();
  const isWrite = method === "POST" || method === "PUT" || method === "DELETE";
  const timeoutMs = isWrite ? 120_000 : 15_000; // 2min for writes, 15s for reads
  const timeout = setTimeout(() => controller.abort(), timeoutMs);

  // Inject JWT if present
  const token = getToken();
  const authHeader: Record<string, string> = token ? { Authorization: `Bearer ${token}` } : {};

  try {
    const res = await fetch(path, {
      ...init,
      signal: init?.signal ?? controller.signal, // allow caller to provide their own signal
      headers: {
        "Content-Type": "application/json",
        ...authHeader,
        ...(init?.headers as Record<string, string> | undefined),
      } as HeadersInit,
    });
    clearTimeout(timeout);

    // Clear token and trigger redirect on 401
    if (res.status === 401) {
      clearToken();
      if (typeof window !== "undefined") {
        window.dispatchEvent(new CustomEvent("wasmos:unauthorized"));
      }
      throw new ApiError("Unauthorized — please log in", 401);
    }

    if (!res.ok) {
      const body = await res.text().catch(() => "");
      throw new ApiError(body || res.statusText, res.status);
    }
    if (res.status === 204) return {} as T;
    return res.json();
  } catch (err) {
    clearTimeout(timeout);
    if (err instanceof ApiError) throw err;
    if (err instanceof Error && err.name === "AbortError")
      throw new ApiError("Request timed out", 408);
    throw err;
  }
}

// ─── Health ─────────────────────────────────────────────────────────

export const healthLive = () => request<HealthStatus>("/health/live");
export const healthReady = () => request<HealthStatus>("/health/ready");
export const checkHealth = healthLive;
export const checkReady = healthReady;

export async function isBackendAlive(): Promise<boolean> {
  try {
    await healthLive();
    return true;
  } catch {
    return false;
  }
}

// ─── Stats ──────────────────────────────────────────────────────────

export const getStats = () =>
  withSWR("/v1/stats", () => request<SystemStats>("/v1/stats"), 10_000);

// ─── Tasks (v1) ─────────────────────────────────────────────────────

export const getTasks = () =>
  withSWR("/v1/tasks", () => request<Task[]>("/v1/tasks"), 15_000);

export const getTask = (id: string) =>
  withSWR(`/v1/tasks/${id}`, () => request<TaskDetail>(`/v1/tasks/${id}`), 30_000);

/** Invalidate all task-related client cache entries after any mutation. */
function invalidateTasks(id?: string) {
  invalidate("/v1/tasks");
  invalidate("/v1/stats");
  if (id) invalidate(`/v1/tasks/${id}`);
}

export const uploadTask = async (
  name: string,
  wasmData: number[],
  /** Optional tenant ID — when provided the backend enforces tenant quotas */
  tenantId?: string | null,
) => {
  const result = await request<Task>("/v1/tasks", {
    method: "POST",
    body: JSON.stringify({
      name,
      wasm_data: wasmData,
      ...(tenantId ? { tenant_id: tenantId } : {}),
    }),
  });
  invalidateTasks();
  return result;
};

export const startTask = async (id: string) => {
  const result = await request<ExecutionResult>(`/v1/tasks/${id}/start`, { method: "POST" });
  invalidateTasks(id);
  return result;
};

export const stopTask = async (id: string) => {
  const result = await request<{ status: string }>(`/v1/tasks/${id}/stop`, { method: "POST" });
  invalidateTasks(id);
  return result;
};

export const deleteTask = async (id: string) => {
  const result = await request<{ status: string }>(`/v1/tasks/${id}`, { method: "DELETE" });
  invalidateTasks(id);
  return result;
};

export interface UpdateTaskRequest {
  name?: string;
  priority?: number;
}

export const updateTask = async (id: string, body: UpdateTaskRequest) => {
  const result = await request<Task>(`/v1/tasks/${id}`, {
    method: "PUT",
    body: JSON.stringify(body),
  });
  invalidateTasks(id);
  return result;
};

export const pauseTask = async (id: string) => {
  const result = await request<{ status: string; note?: string }>(`/v1/tasks/${id}/pause`, { method: "POST" });
  invalidateTasks(id);
  return result;
};

export const restartTask = async (id: string) => {
  const result = await request<{ status: string; note?: string }>(`/v1/tasks/${id}/restart`, { method: "POST" });
  invalidateTasks(id);
  return result;
};

export interface ExecutionHistoryResponse {
  task_id: string;
  count: number;
  executions: ExecutionHistory[];
}

export const getTaskExecutionHistory = (id: string, limit = 50) =>
  request<ExecutionHistoryResponse>(`/v1/tasks/${id}/execution-history?limit=${limit}`);

// ─── Advanced Execution (v2) ────────────────────────────────────────

export const executeAdvanced = (body: Record<string, unknown>) =>
  request<Record<string, unknown>>("/v2/execute/advanced", {
    method: "POST",
    body: JSON.stringify(body),
  });

export const executeBatch = (req: BatchRequest) =>
  request<BatchResult>("/v2/execute/batch", {
    method: "POST",
    body: JSON.stringify(req),
  });

export interface ExecutionReport {
  execution_id:  string;
  found:         boolean;
  task_id:       string | null;
  started_at:    string | null;   // ISO 8601
  completed_at:  string | null;   // ISO 8601
  duration_us:   number | null;
  success:       boolean;
  instructions:  number | null;
  syscalls:      number | null;
  memory_bytes:  number | null;
  error:         string | null;
}

export const getExecutionReport = (executionId: string) =>
  request<ExecutionReport>(`/v2/execution/${executionId}/report`);

export const getAdvancedMetrics = (taskId: string) =>
  request<Record<string, unknown>>(`/v2/tasks/${taskId}/advanced-metrics`);

// ─── Metrics (Prometheus) ───────────────────────────────────────────

export async function getPrometheusMetrics(): Promise<string> {
  // In dev: next.config.mjs rewrites /metrics → http://127.0.0.1:8080/metrics
  // In production: Rust actix-web serves /metrics on the same origin
  try {
    const res = await fetch("/metrics");
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.text();
  } catch (err) {
    console.error("[api] getPrometheusMetrics failed:", err);
    return `# ERROR: Could not fetch metrics\n# ${err instanceof Error ? err.message : String(err)}\n`;
  }
}

// ─── File Utility ───────────────────────────────────────────────────

export function readFileAsBytes(file: File): Promise<number[]> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => {
      const buf = reader.result as ArrayBuffer;
      resolve(Array.from(new Uint8Array(buf)));
    };
    reader.onerror = reject;
    reader.readAsArrayBuffer(file);
  });
}

// ─── Test Files ─────────────────────────────────────────────────────

export interface TestFile {
  name: string;
  source: string;
  path: string;
  size_bytes: number;
  category: string;
}

export interface TestFilesResponse {
  total: number;
  files: TestFile[];
}

export interface TestRunResult {
  file: string;
  success: boolean;
  duration_us: number;
  instructions_executed: number;
  syscalls_executed: number;
  memory_used_bytes: number;
  stdout_log: string[];
  return_value?: string | null;
  error?: string | null;
}

export interface TestRunAllResult {
  total: number;
  passed: number;
  failed: number;
  total_duration_us: number;
  results: TestRunResult[];
}

export const getTestFiles = () =>
  request<TestFilesResponse>("/v1/test-files");

export const runTestFile = (filename: string) =>
  request<TestRunResult>(`/v1/test-files/${encodeURIComponent(filename)}/run`, {
    method: "POST",
  });

export const runAllTestFiles = (category?: string) => {
  const params = category ? `?category=${encodeURIComponent(category)}` : "";
  return request<TestRunAllResult>(`/v1/test-files/run-all${params}`, {
    method: "POST",
    // Increase timeout for batch runs
  });
};

// ─── Security Analysis ──────────────────────────────────────────────

export interface SecurityCapability {
  name: string;
  description: string;
  level: "info" | "warn" | "severe";
}

export interface SecurityReport {
  task_id: string;
  task_name: string;
  file_size_bytes: number;
  imports: string[];
  exports: string[];
  capabilities: SecurityCapability[];
  risk_level: "low" | "medium" | "high";
  summary: string;
}

export const getTaskSecurity = (id: string) =>
  request<SecurityReport>(`/v1/tasks/${id}/security`);

// ─── Execution Logs ─────────────────────────────────────────────────

export interface TaskLog {
  task_id: string;
  task_name: string;
  started_at: string | null;
  completed_at: string | null;
  duration_us: number | null;
  success: boolean | null;
  error: string | null;
  instructions_executed: number;
  syscalls_executed: number;
  memory_used_bytes: number;
  stdout_log: string[];
}

export const getTaskLogs = (id: string) =>
  request<TaskLog>(`/v1/tasks/${id}/logs`);

// ─── Snapshots ──────────────────────────────────────────────────────

export interface Snapshot {
  id: string;
  task_id: string;
  /** ISO timestamp — backend field is captured_at */
  captured_at: string;
  /** Alias so UI can use either name */
  created_at?: string;
  state: string;
  memory_mb: number;
  instructions: number;
  stack_depth: number;
  globals_json: string;
  note?: string;
}

export interface CreateSnapshotRequest {
  memory_mb: number;
  instructions: number;
  stack_depth: number;
  globals_json: string;
  note?: string;
}

export const getSnapshots = (taskId: string) =>
  request<Snapshot[]>(`/v1/tasks/${taskId}/snapshots`);

export const createSnapshot = (taskId: string, body: CreateSnapshotRequest) =>
  request<Snapshot>(`/v1/tasks/${taskId}/snapshots`, {
    method: "POST",
    body: JSON.stringify(body),
  });

export const deleteSnapshot = (_taskId: string, snapId: string) =>
  request<{ deleted: boolean; id: string }>(`/v1/snapshots/${snapId}`, {
    method: "DELETE",
  });

// ─── Audit Log ──────────────────────────────────────────────────────

export interface AuditLog {
  id: number;
  /** ISO timestamp from backend */
  ts: string;
  user_name: string;
  role: string;
  action: string;
  resource?: string;
  tenant_id?: string;
  ip_addr?: string;
}

export interface AuditLogsResponse {
  /** Backend wraps the list in {logs, total, page, per_page} */
  logs: AuditLog[];
  total: number;
  page: number;
  per_page: number;
}

export const getAuditLogs = async (params?: { page?: number; per_page?: number; action?: string }): Promise<AuditLogsResponse> => {
  const qs = new URLSearchParams();
  // Backend supports limit (max 1000). We fetch per_page * page rows so we
  // have enough data for offset-based client-side paging.
  const perPage = params?.per_page ?? 50;
  const page = params?.page ?? 1;
  const limit = Math.min(perPage * page, 1000);
  qs.set("limit", String(limit));
  if (params?.action) qs.set("action", params.action);

  // Backend now returns {logs, total, page, per_page} envelope (not a bare array).
  const response = await request<AuditLogsResponse>(`/v1/audit${qs.toString() ? `?${qs}` : ""}`);
  const allLogs: AuditLog[] = response.logs ?? [];

  // Client-side page slicing (server returns up to `limit` rows, always page=1).
  const start = (page - 1) * perPage;
  const logs = allLogs.slice(start, start + perPage);
  return {
    logs,
    total: response.total ?? allLogs.length,
    page,
    per_page: perPage,
  };
};

// ─── Tenants / RBAC ─────────────────────────────────────────────────

export interface Tenant {
  id: string;
  name: string;
  active: boolean;
  created_at: string;
  max_tasks: number;
  max_memory_mb: number;
  max_cpu_percent: number;
  max_concurrent: number;
  max_wasm_size_mb: number;
}

export interface CreateTenantRequest {
  name: string;
  max_tasks?: number;
  max_concurrent?: number;
}

export const getTenants = () =>
  request<Tenant[]>("/v1/tenants");

export const createTenant = (body: CreateTenantRequest) =>
  request<Tenant>("/v1/tenants", {
    method: "POST",
    body: JSON.stringify(body),
  });

export const deleteTenant = (id: string) =>
  request<{ status: string }>(`/v1/tenants/${id}`, { method: "DELETE" });

export const getTenant = (id: string) =>
  request<Tenant>(`/v1/tenants/${id}`);

// ─── Import Inspection ──────────────────────────────────────────────

export const inspectTask = (id: string) =>
  request<Record<string, unknown>>(`/v2/tasks/${id}/inspect`);

// ─── Capability Tokens ──────────────────────────────────────────────

// Capability variants MUST use snake_case — the Rust backend enum is annotated
// with #[serde(rename_all = "snake_case")] so it serialises/deserialises as
// "task_read", "task_write", etc.  Sending PascalCase values to the backend
// causes an "unknown variant" deserialisation error and token issuance fails.
export type CapabilityVariant =
  | "task_read" | "task_write" | "task_execute" | "task_delete"
  | "metrics_read" | "metrics_system" | "tenant_admin"
  | "snapshot_read" | "snapshot_write"
  | "terminal_access" | "audit_read" | "admin";

export interface TokenSummary {
  id: string;
  label: string;
  subject: string;
  tenant_id?: string | null;
  /** Backend returns snake_case strings e.g. "task_read", "admin" */
  capabilities: string[];
  expires_at?: string | null;
  revoked: boolean;
  valid?: boolean;
}

export interface IssueTokenRequest {
  label: string;
  subject: string;
  tenant_id?: string | null;
  capabilities: CapabilityVariant[];
  /** Hours until expiry — matches backend `ttl_hours` field. None = never expires. */
  ttl_hours?: number | null;
}

export interface IssueTokenResponse {
  token_id: string;
  label: string;
  subject: string;
  tenant_id?: string | null;
  capabilities: string[];
  expires_at?: string | null;
}

export const listTokens = () =>
  request<TokenSummary[]>("/v1/tokens");

export const issueToken = (body: IssueTokenRequest) =>
  request<IssueTokenResponse>("/v1/tokens", {
    method: "POST",
    body: JSON.stringify(body),
  });

export const revokeToken = (id: string) =>
  request<{ status: string }>(`/v1/tokens/${id}`, { method: "DELETE" });

export const checkToken = (tokenId: string, capability: string) =>
  request<{ token_id: string; capability: string; granted: boolean }>(
    `/v1/tokens/check?token_id=${encodeURIComponent(tokenId)}&capability=${encodeURIComponent(capability)}`
  );

// ─── Tracing & Live Metrics ─────────────────────────────────────────

export interface TraceSpan {
  span_id: string;
  trace_id: string;
  task_id: string;
  task_name: string;
  kind: string;
  duration_us: number | null;
  success: boolean;
  error?: string | null;
}

export interface TraceRecord {
  trace_id: string;
  task_id: string;
  task_name: string;
  started_at: string;
  ended_at: string | null;
  spans: TraceSpan[];
  total_duration_us: number | null;
  success: boolean;
}

export interface LiveMetrics {
  window_size?: number;   // backend includes this; optional so zero-filled defaults still work
  success_rate: number;
  error_rate: number;
  p50_us: number;
  p95_us: number;
  p99_us: number;
  avg_us: number;
  throughput_per_min: number;
}

export const listTraces = () =>
  request<TraceRecord[]>("/v1/traces");

export const getTaskTraces = (taskId: string) =>
  request<TraceRecord[]>(`/v1/traces/${taskId}`);

export const getLiveMetrics = () =>
  request<LiveMetrics>("/v1/traces/metrics/live");

/** Seed the trace store with synthetic test traces (dev/test only). */
export const seedTraces = (count: number = 30) =>
  request<{ seeded: number; message: string }>("/v1/traces/seed", {
    method: "POST",
    body: JSON.stringify({ count }),
  });

// ─── Scheduler Status ───────────────────────────────────────────────

export interface SchedulerStatus {
  queued: number;
  running: number;
  max_concurrent: number;
  slice_ms: number;
  timeout_secs: number;
}

export const getSchedulerStatus = () =>
  request<SchedulerStatus>("/v1/scheduler/status");

export const preemptTask = (taskId: string) =>
  request<{ status: string }>(`/v1/scheduler/preempt/${taskId}`, { method: "POST" });

// ─── Module Management (v2) ──────────────────────────────────────────
// These endpoints operate on raw wasm_files/ on disk, bypassing the task system.

export interface WasmModule {
  name: string;
  path: string;
  size_bytes: number;
  format: "wasm" | "wat";
}

export interface ListModulesResponse {
  total: number;
  modules: WasmModule[];
}

export interface ModuleExecutionResult {
  module: string;
  execution_id: string;
  success: boolean;
  instructions: number;
  duration_ms: number;
  error: string | null;
  stdout: string[];
}

export interface ModuleUploadResult {
  name: string;
  path: string;
  size_bytes: number;
  status: "uploaded";
}

/** List all .wasm/.wat files in the server's wasm_files/ directory */
export const listModules = () =>
  request<ListModulesResponse>("/v2/modules");

/** Execute a module by filename directly (bypasses the task system) */
export const executeModule = (module: string) =>
  request<ModuleExecutionResult>("/v2/execute/module", {
    method: "POST",
    body: JSON.stringify({ module }),
  });

/** Upload a raw WASM binary to the server's wasm_files/ directory (no task created) */
export const uploadModule = (name: string, wasmData: number[]) =>
  request<ModuleUploadResult>("/v2/modules/upload", {
    method: "POST",
    body: JSON.stringify({ name, wasm_data: wasmData }),
  });

// ─── Performance Comparison (v2) ─────────────────────────────────────

export interface ComparePerformanceRequest {
  baseline_path: string;
  current_path: string;
  max_memory_mb?: number;
  max_instructions?: number;
  timeout_seconds?: number;
}

export interface PerformanceComparison {
  baseline_instructions: number;
  current_instructions: number;
  /** Positive = current uses fewer instructions (improvement), negative = regression */
  improvement_percent: number;
  baseline_duration_us: number;
  current_duration_us: number;
  /** Positive = current is faster, negative = slower */
  duration_improvement_percent: number;
  baseline_success: boolean;
  current_success: boolean;
  baseline_memory_bytes: number;
  current_memory_bytes: number;
}

/** Run two WASM paths concurrently and compare execution metrics */
export const comparePerformance = (req: ComparePerformanceRequest) =>
  request<PerformanceComparison>("/v2/execute/compare", {
    method: "POST",
    body: JSON.stringify(req),
  });

// ─── Import Stats (v2) ───────────────────────────────────────────────

export interface ImportModuleStat {
  /** Host module namespace, e.g. "wasi_snapshot_preview1", "env", "game" */
  name: string;
  /** Number of tasks whose WASM binary imports from this namespace */
  task_count: number;
  /** Whether this namespace is enabled/allowed in the sandbox */
  enabled: boolean;
}

export interface ImportStats {
  modules: ImportModuleStat[];
  total_tasks_scanned: number;
}

/** Scan all uploaded WASM binaries and aggregate import namespace usage */
export const getImportStats = () => request<ImportStats>("/v2/imports/stats");

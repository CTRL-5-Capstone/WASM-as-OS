"use client";

import { useEffect, useState, useCallback } from "react";
import { getTasks, startTask, stopTask, deleteTask, type Task, type ExecutionResult } from "@/lib/api";
import { formatBytes, relativeTime } from "@/lib/utils";
import {
  Play,
  Square,
  Trash2,
  RefreshCw,
  Info,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";

export default function TaskList() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [execResult, setExecResult] = useState<ExecutionResult | null>(null);
  const [actionLoading, setActionLoading] = useState<string | null>(null);

  const statusColor = (status: string) => {
    switch (status?.toLowerCase()) {
      case "running": return "bg-emerald-500/15 text-emerald-400 border-emerald-500/30";
      case "completed": return "bg-sky-500/15 text-sky-400 border-sky-500/30";
      case "failed": return "bg-rose-500/15 text-rose-400 border-rose-500/30";
      case "stopped": return "bg-amber-500/15 text-amber-400 border-amber-500/30";
      case "pending":
      default: return "bg-muted text-muted-foreground border-border";
    }
  };

  const refresh = useCallback(async () => {
    try {
      const data = await getTasks();
      setTasks(data);
      setError(null);
    } catch {
      setError("Could not load tasks");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 5000);
    return () => clearInterval(id);
  }, [refresh]);

  const handleStart = async (id: string) => {
    setActionLoading(id);
    try {
      const result = await startTask(id);
      setExecResult(result);
      setExpandedId(id);
      await refresh();
    } catch (err) {
      alert(`Start failed: ${err instanceof Error ? err.message : err}`);
    } finally {
      setActionLoading(null);
    }
  };

  const handleStop = async (id: string) => {
    setActionLoading(id);
    try {
      await stopTask(id);
      await refresh();
    } catch (err) {
      alert(`Stop failed: ${err instanceof Error ? err.message : err}`);
    } finally {
      setActionLoading(null);
    }
  };

  const handleDelete = async (id: string) => {
    if (!confirm("Delete this task?")) return;
    setActionLoading(id);
    try {
      await deleteTask(id);
      setTasks((prev) => prev.filter((t) => t.id !== id));
      if (expandedId === id) setExpandedId(null);
    } catch (err) {
      alert(`Delete failed: ${err instanceof Error ? err.message : err}`);
    } finally {
      setActionLoading(null);
    }
  };

  if (loading) {
    return (
      <div
        role="status"
        aria-busy="true"
        aria-label="Loading tasks"
        className="space-y-2"
      >
        {[1, 2, 3].map((i) => (
          <div key={i} className="h-16 rounded-lg border border-border bg-card animate-pulse" />
        ))}
        <span className="sr-only">Loading tasks…</span>
      </div>
    );
  }

  if (error) {
    return (
      <div
        role="alert"
        className="rounded-lg border border-destructive/30 bg-destructive/10 p-4 text-destructive text-sm"
      >
        {error}
      </div>
    );
  }

  if (tasks.length === 0) {
    return (
      <div className="text-center py-16 text-muted-foreground">
        <Info size={40} strokeWidth={1.5} aria-hidden="true" className="mx-auto mb-3 opacity-40" />
        <p className="text-lg font-medium">No tasks yet</p>
        <p className="text-sm mt-1">Upload a .wasm file to get started</p>
      </div>
    );
  }

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-lg font-semibold text-foreground">
          Tasks{" "}
          <span className="text-sm text-muted-foreground font-normal">
            ({tasks.length})
          </span>
        </h2>
        <Button
          onClick={refresh}
          variant="ghost"
          size="sm"
          aria-label="Refresh task list"
          className="text-xs text-muted-foreground hover:text-foreground"
        >
          <RefreshCw size={14} strokeWidth={2.5} aria-hidden="true" /> Refresh
        </Button>
      </div>

      <ul role="list" className="space-y-2">
        {tasks.map((task) => (
          <li
            key={task.id}
            role="listitem"
            className="rounded-lg border border-border bg-card overflow-hidden transition-colors hover:border-primary/20"
          >
            {/* Row */}
            <div className="flex items-center gap-4 px-4 py-3">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <p className="font-medium truncate text-foreground">{task.name}</p>
                  <span
                    className={`inline-flex items-center rounded-md border px-2 py-0.5 text-[11px] font-semibold ${statusColor(task.status)}`}
                    aria-label={`Status: ${task.status}`}
                  >
                    {task.status}
                  </span>
                </div>
                <p className="text-xs text-muted-foreground mt-0.5">
                  {formatBytes(task.file_size_bytes)} · {relativeTime(task.created_at)}
                </p>
              </div>

              {/* Actions */}
              <div className="flex items-center gap-1" role="group" aria-label={`Actions for ${task.name}`}>
                {task.status.toLowerCase() !== "running" && (
                  <Button
                    onClick={() => handleStart(task.id)}
                    disabled={actionLoading === task.id}
                    variant="ghost"
                    size="icon"
                    aria-label={actionLoading === task.id ? `Starting ${task.name}` : `Start ${task.name}`}
                    className="text-emerald-400 hover:bg-emerald-500/15 hover:text-emerald-300 h-8 w-8"
                  >
                    {actionLoading === task.id ? (
                      <RefreshCw size={15} strokeWidth={2.5} aria-hidden="true" className="animate-spin" />
                    ) : (
                      <Play size={15} strokeWidth={2.5} aria-hidden="true" />
                    )}
                  </Button>
                )}
                {task.status.toLowerCase() === "running" && (
                  <Button
                    onClick={() => handleStop(task.id)}
                    disabled={actionLoading === task.id}
                    variant="ghost"
                    size="icon"
                    aria-label={`Stop ${task.name}`}
                    className="text-amber-400 hover:bg-amber-500/15 hover:text-amber-300 h-8 w-8"
                  >
                    <Square size={15} strokeWidth={2.5} aria-hidden="true" />
                  </Button>
                )}
                <Button
                  onClick={() => handleDelete(task.id)}
                  disabled={actionLoading === task.id}
                  variant="ghost"
                  size="icon"
                  aria-label={`Delete ${task.name}`}
                  className="text-rose-400 hover:bg-rose-500/15 hover:text-rose-300 h-8 w-8"
                >
                  <Trash2 size={15} strokeWidth={2.5} aria-hidden="true" />
                </Button>
                <Button
                  onClick={() =>
                    setExpandedId(expandedId === task.id ? null : task.id)
                  }
                  variant="ghost"
                  size="icon"
                  aria-label={expandedId === task.id ? `Collapse details for ${task.name}` : `Expand details for ${task.name}`}
                  aria-expanded={expandedId === task.id}
                  aria-controls={`task-detail-${task.id}`}
                  className="text-muted-foreground h-8 w-8"
                >
                  {expandedId === task.id ? (
                    <ChevronUp size={15} strokeWidth={2.5} aria-hidden="true" />
                  ) : (
                    <ChevronDown size={15} strokeWidth={2.5} aria-hidden="true" />
                  )}
                </Button>
              </div>
            </div>

            {/* Expanded detail panel */}
            {expandedId === task.id && (
              <div
                id={`task-detail-${task.id}`}
                className="px-4 py-3 border-t border-border bg-muted/30"
              >
                <div className="grid grid-cols-2 md:grid-cols-4 gap-3 text-xs">
                  <div>
                    <span className="text-muted-foreground">ID</span>
                    <p className="font-mono truncate text-foreground">{task.id}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Path</span>
                    <p className="font-mono truncate text-foreground">{task.path}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Size</span>
                    <p className="text-foreground">{formatBytes(task.file_size_bytes)}</p>
                  </div>
                  <div>
                    <span className="text-muted-foreground">Created</span>
                    <p className="text-foreground">{new Date(task.created_at).toLocaleString()}</p>
                  </div>
                </div>

                {/* Execution result */}
                {execResult && expandedId === task.id && (
                  <div
                    role="status"
                    aria-live="polite"
                    aria-label="Execution result"
                    className="mt-3 p-3 rounded-lg bg-background border border-border"
                  >
                    <p className="text-xs font-semibold text-muted-foreground mb-2">
                      Last Execution Result
                    </p>
                    <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-xs">
                      <div>
                        <span className="text-muted-foreground">Status</span>
                        <p className={execResult.success ? "text-emerald-400" : "text-rose-400"}>
                          {execResult.success ? "Success" : "Failed"}
                        </p>
                      </div>
                      <div>
                        <span className="text-muted-foreground">Duration</span>
                        <p className="text-foreground">{(execResult.duration_us / 1000).toFixed(1)}ms</p>
                      </div>
                      <div>
                        <span className="text-muted-foreground">Instructions</span>
                        <p className="text-foreground">{execResult.instructions_executed.toLocaleString()}</p>
                      </div>
                      <div>
                        <span className="text-muted-foreground">Memory</span>
                        <p className="text-foreground">{formatBytes(execResult.memory_used_bytes)}</p>
                      </div>
                    </div>
                    {execResult.stdout_log && execResult.stdout_log.length > 0 && (
                      <div className="mt-2">
                        <span className="text-muted-foreground text-xs" id={`stdout-label-${task.id}`}>Stdout</span>
                        <pre
                          aria-labelledby={`stdout-label-${task.id}`}
                          className="bg-background/60 border border-border p-2 rounded mt-1 text-xs text-emerald-400 overflow-x-auto max-h-32"
                        >
                          {execResult.stdout_log.join("\n")}
                        </pre>
                      </div>
                    )}
                    {execResult.error && (
                      <div className="mt-2">
                        <span className="text-muted-foreground text-xs" id={`error-label-${task.id}`}>Error</span>
                        <pre
                          aria-labelledby={`error-label-${task.id}`}
                          className="bg-background/60 border border-border p-2 rounded mt-1 text-xs text-rose-400 overflow-x-auto max-h-32"
                        >
                          {execResult.error}
                        </pre>
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

"use client";

/**
 * Snapshots  real API-backed snapshot management.
 * POST /v1/tasks/:id/snapshots  create
 * GET  /v1/tasks/:id/snapshots  list
 * DELETE /v1/tasks/:id/snapshots/:snap_id  delete
 */

import { useState, useEffect, useCallback } from "react";
import {
  Camera, Trash2, RefreshCw, Plus,
  Box, Clock, Cpu, MemoryStick, Layers, Info,
} from "lucide-react";
import {
  getTasks, getSnapshots, createSnapshot, deleteSnapshot,
  type Task, type Snapshot,
} from "@/lib/api";
import { formatBytes, formatNumber, timeAgo, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { toast } from "sonner";

export default function SnapshotsPage() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [selectedTaskId, setSelectedTaskId] = useState<string>("");
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);
  const [showForm, setShowForm] = useState(false);
  const [deletingId, setDeletingId] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const [memoryMb, setMemoryMb] = useState("4");
  const [instructions, setInstructions] = useState("0");
  const [stackDepth, setStackDepth] = useState("0");
  const [globalsJson, setGlobalsJson] = useState("{}");

  useEffect(() => {
    getTasks()
      .then((data) => {
        setTasks(data);
        if (data.length > 0 && !selectedTaskId) setSelectedTaskId(data[0].id);
      })
      .catch(() => toast.error("Failed to load tasks"));
  }, []);

  const loadSnapshots = useCallback(async () => {
    if (!selectedTaskId) return;
    setLoading(true);
    try {
      const data = await getSnapshots(selectedTaskId);
      setSnapshots(data ?? []);
    } catch {
      toast.error("Failed to load snapshots");
      setSnapshots([]);
    } finally {
      setLoading(false);
    }
  }, [selectedTaskId]);

  useEffect(() => { loadSnapshots(); }, [loadSnapshots]);

  const handleCreate = async () => {
    if (!selectedTaskId) return;
    try { JSON.parse(globalsJson); } catch { toast.error("Globals JSON is invalid"); return; }
    setCreating(true);
    try {
      const snap = await createSnapshot(selectedTaskId, {
        memory_mb: Number(memoryMb) || 4,
        instructions: Number(instructions) || 0,
        stack_depth: Number(stackDepth) || 0,
        globals_json: globalsJson,
        note: note || undefined,
      });
      setSnapshots((prev) => [snap, ...prev]);
      toast.success("Snapshot created");
      setShowForm(false);
      setNote(""); setGlobalsJson("{}"); setInstructions("0"); setStackDepth("0");
    } catch (e: unknown) {
      toast.error(`Create failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setCreating(false);
    }
  };

  const handleDelete = async (snapId: string) => {
    if (!selectedTaskId) return;
    setDeletingId(snapId);
    try {
      await deleteSnapshot(selectedTaskId, snapId);
      setSnapshots((prev) => prev.filter((s) => s.id !== snapId));
      toast.success("Snapshot deleted");
    } catch (e: unknown) {
      toast.error(`Delete failed: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setDeletingId(null);
    }
  };

  const selectedTask = tasks.find((t) => t.id === selectedTaskId);

  return (
    <div className="animate-fade-in space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold gradient-text flex items-center gap-2">
            <Camera size={26} /> Snapshots
          </h1>
          <p className="text-sm text-slate-500 mt-1">Capture and restore WASM execution state</p>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" size="sm" onClick={loadSnapshots} disabled={loading} className="text-xs">
            <RefreshCw size={13} className={cn(loading && "animate-spin")} /> Refresh
          </Button>
          <Button size="sm" onClick={() => setShowForm((v) => !v)} disabled={!selectedTaskId} className="text-xs">
            <Plus size={13} /> New Snapshot
          </Button>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-[280px_1fr]">
        <div className="space-y-2">
          <h2 className="text-xs font-semibold text-slate-500 uppercase tracking-wider">Select Task</h2>
          <ScrollArea className="h-[calc(100vh-220px)] pr-1">
            <div className="space-y-1.5">
              {tasks.length === 0 && (
                <div className="rounded-lg border border-dashed border-slate-200 px-4 py-8 text-center text-xs text-slate-400">No tasks found</div>
              )}
              {tasks.map((t) => (
                <button key={t.id} onClick={() => setSelectedTaskId(t.id)}
                  className={cn("w-full rounded-lg border px-3 py-2.5 text-left transition-all",
                    selectedTaskId === t.id ? "border-indigo-300 bg-indigo-50 shadow-sm" : "border-slate-200 bg-white hover:border-slate-300 hover:bg-slate-50")}>
                  <div className="flex items-center gap-2">
                    <Box size={12} className="shrink-0 text-slate-400" />
                    <p className="truncate text-xs font-medium text-slate-800">{t.name}</p>
                  </div>
                  <div className="mt-1 flex items-center gap-2">
                    <span className={cn("text-[10px] font-semibold rounded-full px-1.5 py-0.5",
                      t.status === "running" ? "bg-emerald-100 text-emerald-700" : t.status === "failed" ? "bg-red-100 text-red-700" : "bg-slate-100 text-slate-600")}>
                      {t.status}
                    </span>
                    <span className="text-[10px] text-slate-400">{formatBytes(t.file_size_bytes)}</span>
                  </div>
                </button>
              ))}
            </div>
          </ScrollArea>
        </div>

        <div className="space-y-4">
          {showForm && selectedTaskId && (
            <Card className="border-indigo-200 bg-indigo-50/40">
              <CardHeader className="pb-2">
                <CardTitle className="text-sm font-medium text-indigo-900 flex items-center gap-2">
                  <Camera size={14} /> Create Snapshot
                  {selectedTask && <span className="text-indigo-500 font-normal"> {selectedTask.name}</span>}
                </CardTitle>
              </CardHeader>
              <CardContent className="space-y-3">
                <div className="grid gap-3 sm:grid-cols-3">
                  <div>
                    <label className="text-[10px] font-medium text-slate-500 uppercase tracking-wider">Memory (MB)</label>
                    <Input type="number" value={memoryMb} onChange={(e) => setMemoryMb(e.target.value)} className="mt-1 h-8 text-xs" min="1" />
                  </div>
                  <div>
                    <label className="text-[10px] font-medium text-slate-500 uppercase tracking-wider">Instructions</label>
                    <Input type="number" value={instructions} onChange={(e) => setInstructions(e.target.value)} className="mt-1 h-8 text-xs" min="0" />
                  </div>
                  <div>
                    <label className="text-[10px] font-medium text-slate-500 uppercase tracking-wider">Stack Depth</label>
                    <Input type="number" value={stackDepth} onChange={(e) => setStackDepth(e.target.value)} className="mt-1 h-8 text-xs" min="0" />
                  </div>
                </div>
                <div>
                  <label className="text-[10px] font-medium text-slate-500 uppercase tracking-wider">Globals JSON</label>
                  <textarea value={globalsJson} onChange={(e) => setGlobalsJson(e.target.value)} rows={3}
                    className="mt-1 w-full rounded-md border border-slate-200 bg-white px-3 py-2 font-mono text-xs focus:outline-none focus:ring-2 focus:ring-indigo-500" spellCheck={false} />
                </div>
                <div>
                  <label className="text-[10px] font-medium text-slate-500 uppercase tracking-wider">Note (optional)</label>
                  <Input value={note} onChange={(e) => setNote(e.target.value)} placeholder="Describe this snapshot" className="mt-1 h-8 text-xs" />
                </div>
                <div className="flex justify-end gap-2">
                  <Button variant="ghost" size="sm" onClick={() => setShowForm(false)} className="text-xs">Cancel</Button>
                  <Button size="sm" onClick={handleCreate} disabled={creating} className="text-xs">
                    {creating ? <RefreshCw size={12} className="animate-spin" /> : <Camera size={12} />}
                    {creating ? "Saving" : "Save Snapshot"}
                  </Button>
                </div>
              </CardContent>
            </Card>
          )}

          {!selectedTaskId ? (
            <Card><CardContent className="flex flex-col items-center justify-center py-16 text-center">
              <Camera size={32} className="text-slate-300 mb-3" />
              <p className="text-sm font-medium text-slate-500">Select a task to view snapshots</p>
            </CardContent></Card>
          ) : loading ? (
            <div className="space-y-3">{[1,2,3].map((i) => <div key={i} className="h-24 animate-pulse rounded-xl bg-slate-100" />)}</div>
          ) : snapshots.length === 0 ? (
            <Card><CardContent className="flex flex-col items-center justify-center py-16 text-center">
              <Layers size={32} className="text-slate-300 mb-3" />
              <p className="text-sm font-medium text-slate-500">No snapshots yet</p>
              <p className="text-xs text-slate-400 mt-1">Click &quot;New Snapshot&quot; to capture the current execution state</p>
            </CardContent></Card>
          ) : (
            <div className="space-y-3">
              {snapshots.map((snap) => (
                <Card key={snap.id} className="transition-shadow hover:shadow-md">
                  <CardContent className="p-4">
                    <div className="flex items-start justify-between gap-4">
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center gap-2 flex-wrap">
                          <span className="font-mono text-xs text-indigo-600 bg-indigo-50 rounded px-2 py-0.5">#{snap.id.slice(0, 8)}</span>
                          {snap.note && <span className="text-xs text-slate-600 italic truncate">{snap.note}</span>}
                          <span className="ml-auto flex items-center gap-1 text-[10px] text-slate-400">
                            <Clock size={10} />{timeAgo(snap.captured_at ?? snap.created_at ?? "")}
                          </span>
                        </div>
                        <div className="mt-3 grid grid-cols-2 sm:grid-cols-4 gap-3">
                          <SnapStat icon={<MemoryStick size={12} />} label="Memory" value={`${snap.memory_mb} MB`} color="text-sky-600" />
                          <SnapStat icon={<Cpu size={12} />} label="Instructions" value={formatNumber(snap.instructions)} color="text-violet-600" />
                          <SnapStat icon={<Layers size={12} />} label="Stack Depth" value={String(snap.stack_depth)} color="text-emerald-600" />
                          <SnapStat icon={<Info size={12} />} label="Globals" value={snap.globals_json === "{}" ? "empty" : `${Object.keys((() => { try { return JSON.parse(snap.globals_json); } catch { return {}; } })()).length} keys`} color="text-amber-600" />
                        </div>
                      </div>
                      <Button variant="ghost" size="icon" className="h-8 w-8 shrink-0 text-slate-400 hover:text-red-500"
                        disabled={deletingId === snap.id} onClick={() => handleDelete(snap.id)}>
                        {deletingId === snap.id ? <RefreshCw size={13} className="animate-spin" /> : <Trash2 size={13} />}
                      </Button>
                    </div>
                  </CardContent>
                </Card>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}

function SnapStat({ icon, label, value, color }: { icon: React.ReactNode; label: string; value: string; color: string }) {
  return (
    <div className="rounded-lg bg-slate-50 border border-slate-100 px-3 py-2">
      <div className={cn("flex items-center gap-1 text-[10px] font-medium mb-0.5", color)}>{icon} {label}</div>
      <p className="text-xs font-semibold text-slate-800 truncate">{value}</p>
    </div>
  );
}

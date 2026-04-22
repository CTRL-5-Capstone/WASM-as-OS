"use client";

/**
 * Batch Execution — production-ready improvements:
 *  - Drag-drop always available (not just empty state)
 *  - Results show file name, success/fail, duration, instructions
 *  - Progress stays at 100% for 2s before hiding
 *  - Export Results (JSON to clipboard) after completion
 *  - Re-run failed button
 *  - Total batch duration
 *  - Accepts .wasm and .wat files
 *  - Formatted numbers
 */

import { useState, useRef, useCallback, useEffect } from "react";
import {
  Layers, Play, Upload, CheckCircle, AlertCircle,
  FileCode, RefreshCw, X, Copy, RotateCcw, Clock,
} from "lucide-react";
import {
  executeBatch, uploadTask, readFileAsBytes,
  type BatchResult, type BatchFileError,
} from "@/lib/api";
import { formatNumber, cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { ScrollArea } from "@/components/ui/scroll-area";
import { toast } from "sonner";

interface QueuedFile {
  file: File;
  name: string;
  /** Server path after upload */
  path?: string;
}

const MAX_BATCH_FILES = 20;
const MAX_FILE_SIZE = 50 * 1024 * 1024; // 50 MB per file
const ACCEPTED_EXTS = [".wasm", ".wat"];

export default function BatchPage() {
  const [queue, setQueue] = useState<QueuedFile[]>([]);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<BatchResult | null>(null);
  const [continueOnError, setContinueOnError] = useState(true);
  const [progress, setProgress] = useState(0);
  const [progressVisible, setProgressVisible] = useState(false);
  const [statusMsg, setStatusMsg] = useState("");
  const [totalDurationMs, setTotalDurationMs] = useState<number | null>(null);
  const [isDragOver, setIsDragOver] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);
  const abortRef = useRef<AbortController | null>(null);
  const batchStartRef = useRef<number>(0);
  // Track original file names keyed by upload path
  const pathToNameRef = useRef<Map<string, string>>(new Map());

  // Show progress at 100% for 2 seconds then hide
  useEffect(() => {
    if (progress === 100 && progressVisible) {
      const id = setTimeout(() => setProgressVisible(false), 2000);
      return () => clearTimeout(id);
    }
  }, [progress, progressVisible]);

  const addFiles = useCallback((files: FileList | File[]) => {
    const arr = Array.isArray(files) ? files : Array.from(files);
    const newFiles: QueuedFile[] = [];
    const skipped: string[] = [];
    for (const f of arr) {
      const ext = ACCEPTED_EXTS.find((e) => f.name.endsWith(e));
      if (!ext) {
        skipped.push(`${f.name} (not .wasm/.wat)`);
        continue;
      }
      if (f.size > MAX_FILE_SIZE) {
        skipped.push(`${f.name} (>${Math.round(MAX_FILE_SIZE / 1024 / 1024)}MB)`);
        continue;
      }
      if (queue.length + newFiles.length >= MAX_BATCH_FILES) {
        skipped.push(`${f.name} (queue full, max ${MAX_BATCH_FILES})`);
        continue;
      }
      const baseName = ACCEPTED_EXTS.reduce((n, e) => n.replace(new RegExp(`\\${e}$`), ""), f.name);
      newFiles.push({ file: f, name: baseName });
    }
    if (skipped.length > 0) {
      toast.warning(`Skipped ${skipped.length} file(s)`, { description: skipped.join(", ") });
    }
    if (newFiles.length > 0) {
      setQueue((prev) => [...prev, ...newFiles]);
    }
  }, [queue.length]);

  const removeFile = (idx: number) => {
    setQueue((prev) => prev.filter((_, i) => i !== idx));
  };

  const cancelBatch = useCallback(() => {
    abortRef.current?.abort();
    setRunning(false);
    setStatusMsg("Cancelled");
    toast.info("Batch cancelled");
  }, []);

  const runBatch = async (filesToRun?: QueuedFile[]) => {
    const targetQueue = filesToRun ?? queue;
    if (targetQueue.length === 0) return;
    setRunning(true);
    setResult(null);
    setProgress(0);
    setProgressVisible(true);
    setStatusMsg("Preparing uploads…");
    setTotalDurationMs(null);
    batchStartRef.current = Date.now();
    pathToNameRef.current = new Map();

    const abort = new AbortController();
    abortRef.current = abort;

    try {
      // Phase 1: Upload files (2 at a time)
      const paths: string[] = [];
      const CONCURRENCY = 2;
      for (let i = 0; i < targetQueue.length; i += CONCURRENCY) {
        if (abort.signal.aborted) throw new Error("Cancelled");
        const chunk = targetQueue.slice(i, i + CONCURRENCY);
        setStatusMsg(
          `Uploading ${i + 1}–${Math.min(i + CONCURRENCY, targetQueue.length)} of ${targetQueue.length}…`
        );
        const results = await Promise.all(
          chunk.map(async ({ file, name }) => {
            const bytes = await readFileAsBytes(file);
            return uploadTask(name, bytes);
          })
        );
        results.forEach((task, idx) => {
          paths.push(task.path);
          pathToNameRef.current.set(task.path, chunk[idx].name);
        });
        setProgress(Math.round(((i + chunk.length) / targetQueue.length) * 50));
      }

      if (abort.signal.aborted) throw new Error("Cancelled");

      // Phase 2: Execute batch
      setStatusMsg("Executing batch…");
      setProgress(60);

      const batchResult = await executeBatch({
        wasm_paths: paths,
        continue_on_error: continueOnError,
      });

      const elapsed = Date.now() - batchStartRef.current;
      setTotalDurationMs(elapsed);
      setResult(batchResult);
      setProgress(100);
      setStatusMsg(`Done — ${batchResult.successful}/${batchResult.total_files} passed in ${(elapsed / 1000).toFixed(1)}s`);

      if (batchResult.failed > 0) {
        toast.warning(`Batch complete: ${batchResult.failed} failed`);
      } else {
        toast.success(`Batch complete: all ${batchResult.successful} passed`);
      }
    } catch (e: unknown) {
      const msg = e instanceof Error ? e.message : String(e);
      if (msg === "Cancelled") {
        setResult(null);
        setProgress(100);
      } else {
        setResult({
          total_files: targetQueue.length,
          successful: 0,
          failed: targetQueue.length,
          results: [],
          errors: [{ path: "batch", error: msg || "Batch execution failed" }],
        });
        setProgress(100);
        toast.error("Batch failed", { description: msg });
      }
    } finally {
      setRunning(false);
      abortRef.current = null;
    }
  };

  const exportResults = () => {
    if (!result) return;
    const payload = JSON.stringify(result, null, 2);
    navigator.clipboard.writeText(payload).then(
      () => toast.success("Results copied to clipboard"),
      () => toast.error("Failed to copy — check clipboard permissions")
    );
  };

  const reRunFailed = () => {
    if (!result) return;
    const failedNames = new Set(
      result.errors.map((e) => {
        // Extract basename from path
        const parts = e.path.replace(/\\/g, "/").split("/");
        return parts[parts.length - 1].replace(/\.(wasm|wat)$/, "");
      })
    );
    const failedFiles = queue.filter((q) => failedNames.has(q.name));
    if (failedFiles.length === 0) {
      toast.warning("No matching files in queue for failed items");
      return;
    }
    runBatch(failedFiles);
  };

  return (
    <div className="animate-fade-in space-y-6">
      <div>
        <h1 className="text-2xl font-bold gradient-text">Batch Execution</h1>
        <p className="mt-1 text-sm text-muted-foreground">
          Execute multiple WASM/WAT modules in a single batch (max {MAX_BATCH_FILES} files,{" "}
          {MAX_FILE_SIZE / 1024 / 1024}MB each)
        </p>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        {/* Left column: queue + options + actions */}
        <div className="space-y-4">
          <Card>
            <CardHeader className="flex-row items-center justify-between space-y-0">
              <CardTitle className="text-sm font-medium text-foreground flex items-center gap-2">
                <Layers size={16} className="text-indigo-400" />
                File Queue
                <Badge variant="secondary" className="text-[10px]">
                  {queue.length}/{MAX_BATCH_FILES}
                </Badge>
              </CardTitle>
              <div className="flex items-center gap-2">
                {queue.length > 0 && !running && (
                  <Button
                    onClick={() => setQueue([])}
                    variant="ghost"
                    size="sm"
                    className="text-xs text-muted-foreground hover:text-red-400"
                  >
                    Clear All
                  </Button>
                )}
                <Button
                  onClick={() => fileRef.current?.click()}
                  variant="secondary"
                  size="sm"
                  className="text-xs"
                  disabled={running}
                >
                  <Upload size={12} /> Add Files
                </Button>
                <input
                  ref={fileRef}
                  type="file"
                  accept=".wasm,.wat"
                  multiple
                  className="hidden"
                  onChange={(e) => {
                    if (e.target.files) addFiles(e.target.files);
                    e.target.value = "";
                  }}
                />
              </div>
            </CardHeader>
            <CardContent className="space-y-3">
              {/* Always-visible drop zone above the list */}
              <div
                className={cn(
                  "flex flex-col items-center justify-center rounded-lg border-2 border-dashed p-4 cursor-pointer transition-colors text-center",
                  isDragOver
                    ? "border-indigo-400/80 bg-indigo-500/10"
                    : "border-border hover:border-indigo-400/60"
                )}
                onClick={() => fileRef.current?.click()}
                onDragOver={(e) => { e.preventDefault(); e.stopPropagation(); setIsDragOver(true); }}
                onDragLeave={() => setIsDragOver(false)}
                onDrop={(e) => {
                  e.preventDefault();
                  e.stopPropagation();
                  setIsDragOver(false);
                  if (e.dataTransfer.files) addFiles(e.dataTransfer.files);
                }}
              >
                <FileCode size={20} className="mb-1.5 text-muted-foreground" />
                <p className="text-xs text-muted-foreground">
                  Drop .wasm / .wat files here or click to browse
                </p>
              </div>

              {/* File list */}
              {queue.length > 0 && (
                <ScrollArea className="h-48 rounded-lg border border-border bg-muted/30">
                  <div className="space-y-1.5 p-2">
                    {queue.map((q, i) => (
                      <div
                        key={i}
                        className="flex items-center justify-between rounded-lg border border-border bg-card px-3 py-2"
                      >
                        <span className="flex items-center gap-2 text-sm min-w-0">
                          <FileCode size={14} className="text-indigo-400 shrink-0" />
                          <span className="text-foreground truncate">{q.name}</span>
                          <span className="text-[10px] text-muted-foreground shrink-0">
                            {(q.file.size / 1024).toFixed(0)}KB
                          </span>
                        </span>
                        <Button
                          onClick={() => removeFile(i)}
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 text-muted-foreground hover:text-red-400"
                          disabled={running}
                        >
                          <X size={14} />
                        </Button>
                      </div>
                    ))}
                  </div>
                </ScrollArea>
              )}
            </CardContent>
          </Card>

          {/* Options */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium text-foreground">Options</CardTitle>
            </CardHeader>
            <CardContent>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={continueOnError}
                  onChange={(e) => setContinueOnError(e.target.checked)}
                  className="h-4 w-4 rounded border-border bg-muted text-indigo-500 focus:ring-indigo-500"
                />
                <span className="text-sm text-foreground">Continue on error</span>
              </label>
            </CardContent>
          </Card>

          {/* Run / cancel */}
          <div className="flex gap-2">
            <Button
              onClick={() => runBatch()}
              disabled={queue.length === 0 || running}
              variant="gradient"
              className="flex-1 h-12"
            >
              {running ? (
                <><RefreshCw size={16} className="animate-spin" /> Executing Batch…</>
              ) : (
                <><Play size={16} /> Execute Batch ({queue.length} file{queue.length !== 1 ? "s" : ""})</>
              )}
            </Button>
            {running && (
              <Button onClick={cancelBatch} variant="destructive" className="h-12 px-4">
                <X size={16} /> Cancel
              </Button>
            )}
          </div>

          {/* Progress */}
          {progressVisible && (
            <Card>
              <CardContent className="p-4">
                <div className="flex justify-between text-xs text-muted-foreground mb-2">
                  <span>{statusMsg || "Progress"}</span>
                  <span>{progress}%</span>
                </div>
                <Progress value={progress} />
              </CardContent>
            </Card>
          )}
        </div>

        {/* Right column: results */}
        <div>
          {result ? (
            <div className="space-y-4">
              {/* Summary */}
              <Card>
                <CardHeader className="flex-row items-center justify-between space-y-0 pb-2">
                  <CardTitle className="text-sm font-medium text-foreground">Batch Results</CardTitle>
                  <div className="flex items-center gap-2">
                    {result.failed > 0 && (
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={reRunFailed}
                        disabled={running}
                        className="text-xs gap-1.5"
                        title="Re-queue and re-run only the failed files"
                      >
                        <RotateCcw size={12} /> Re-run failed
                      </Button>
                    )}
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={exportResults}
                      className="text-xs gap-1.5"
                    >
                      <Copy size={12} /> Export JSON
                    </Button>
                  </div>
                </CardHeader>
                <CardContent>
                  <div className="grid grid-cols-3 gap-3 text-center">
                    <div className="rounded-lg bg-muted/30 border border-border p-3">
                      <p className="text-xl font-bold text-foreground">{result.total_files}</p>
                      <p className="text-xs text-muted-foreground">Total</p>
                    </div>
                    <div className="rounded-lg bg-green-500/10 p-3">
                      <p className="text-xl font-bold text-green-400">{result.successful}</p>
                      <p className="text-xs text-muted-foreground">Passed</p>
                    </div>
                    <div className="rounded-lg bg-red-500/10 p-3">
                      <p className="text-xl font-bold text-red-400">{result.failed}</p>
                      <p className="text-xs text-muted-foreground">Failed</p>
                    </div>
                  </div>
                  {totalDurationMs != null && (
                    <div className="mt-3 flex items-center gap-1.5 text-xs text-muted-foreground">
                      <Clock size={12} />
                      Total batch time: <span className="font-semibold text-foreground">
                        {(totalDurationMs / 1000).toFixed(2)}s
                      </span>
                    </div>
                  )}
                </CardContent>
              </Card>

              {/* Individual results */}
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm font-medium text-foreground">
                    Individual Results
                  </CardTitle>
                </CardHeader>
                <CardContent>
                  <ScrollArea className="max-h-96">
                    <div className="space-y-2 pr-1">
                      {result.results.map((r, i) => {
                        const displayName =
                          pathToNameRef.current.get(r.execution_id) ??
                          queue[i]?.name ??
                          `execution ${i + 1}`;
                        return (
                          <div
                            key={`ok-${i}`}
                            className="flex items-start gap-3 rounded-lg border border-green-500/30 bg-green-500/5 p-3"
                          >
                            <CheckCircle size={15} className="text-green-400 mt-0.5 shrink-0" />
                            <div className="min-w-0 flex-1">
                              <p className="text-sm font-medium text-foreground truncate">
                                {displayName}
                              </p>
                              <p className="text-xs text-muted-foreground mt-0.5 font-mono">
                                {r.duration_ms.toFixed(2)}ms &middot;{" "}
                                {formatNumber(r.instructions)} instr
                              </p>
                              <p className="text-[10px] text-muted-foreground/60 font-mono truncate">
                                {r.execution_id}
                              </p>
                            </div>
                          </div>
                        );
                      })}
                      {(result.errors ?? []).map((e: BatchFileError, i: number) => (
                        <div
                          key={`err-${i}`}
                          className="flex items-start gap-3 rounded-lg border border-red-500/30 bg-red-500/5 p-3"
                        >
                          <AlertCircle size={15} className="text-red-400 mt-0.5 shrink-0" />
                          <div className="min-w-0 flex-1">
                            <p className="text-sm font-medium text-foreground truncate">
                              {e.path.replace(/\\/g, "/").split("/").pop() ?? e.path}
                            </p>
                            <p className="text-xs text-red-400 mt-0.5 break-words">{e.error}</p>
                          </div>
                        </div>
                      ))}
                    </div>
                  </ScrollArea>
                </CardContent>
              </Card>
            </div>
          ) : (
            <Card>
              <CardContent className="p-12 text-center">
                <Layers size={48} className="mx-auto mb-4 text-muted-foreground" />
                <p className="text-muted-foreground">Add files and run to see batch results</p>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}

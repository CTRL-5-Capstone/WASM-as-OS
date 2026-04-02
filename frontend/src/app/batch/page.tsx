"use client";

import { useState, useRef, useCallback } from "react";
import { Layers, Play, Upload, CheckCircle, AlertCircle, FileCode, RefreshCw, X } from "lucide-react";
import { executeBatch, uploadTask, readFileAsBytes, type BatchResult, type BatchFileError } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Progress } from "@/components/ui/progress";
import { ScrollArea } from "@/components/ui/scroll-area";

interface QueuedFile {
  file: File;
  name: string;
}

export default function BatchPage() {
  const [queue, setQueue] = useState<QueuedFile[]>([]);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<BatchResult | null>(null);
  const [continueOnError, setContinueOnError] = useState(true);
  const [progress, setProgress] = useState(0);
  const fileRef = useRef<HTMLInputElement>(null);

  const addFiles = (files: FileList) => {
    const newFiles = Array.from(files)
      .filter((f) => f.name.endsWith(".wasm"))
      .map((f) => ({ file: f, name: f.name.replace(/\.wasm$/, "") }));
    setQueue((prev) => [...prev, ...newFiles]);
  };

  const removeFile = (idx: number) => {
    setQueue((prev) => prev.filter((_, i) => i !== idx));
  };

  const runBatch = async () => {
    if (queue.length === 0) return;
    setRunning(true); setResult(null); setProgress(0);

    try {
      // Upload all files first
      const paths: string[] = [];
      for (let i = 0; i < queue.length; i++) {
        const { file, name } = queue[i];
        const bytes = await readFileAsBytes(file);
        const task = await uploadTask(name, bytes);
        paths.push(task.path);
        setProgress(Math.round(((i + 1) / queue.length) * 50));
      }

      setProgress(60);

      // Execute batch
      const batchResult = await executeBatch({
        wasm_paths: paths,
        continue_on_error: continueOnError,
      });
      setResult(batchResult);
      setProgress(100);
    } catch (e: any) {
      setResult({
        total_files: queue.length,
        successful: 0,
        failed: queue.length,
        results: [],
        errors: [{ path: "batch", error: e.message || "Batch execution failed" }],
      });
      setProgress(100);
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="animate-fade-in space-y-6">
      <div>
        <h1 className="text-3xl font-bold gradient-text">Batch Execution</h1>
        <p className="mt-1 text-sm text-slate-500">Execute multiple WASM modules in a single batch operation</p>
      </div>

      <div className="grid gap-6 lg:grid-cols-2">
        <div className="space-y-4">
          {/* File queue */}
          <Card>
            <CardHeader className="flex-row items-center justify-between space-y-0">
              <CardTitle className="text-sm font-medium text-slate-900 flex items-center gap-2">
                <Layers size={16} className="text-indigo-600" /> File Queue ({queue.length})
              </CardTitle>
              <div>
                <Button onClick={() => fileRef.current?.click()} variant="secondary" size="sm" className="text-xs">
                  <Upload size={12} /> Add Files
                </Button>
                <input
                  ref={fileRef}
                  type="file"
                  accept=".wasm"
                  multiple
                  className="hidden"
                  onChange={(e) => e.target.files && addFiles(e.target.files)}
                />
              </div>
            </CardHeader>
            <CardContent>
              {queue.length === 0 ? (
                <div
                  className="flex flex-col items-center justify-center rounded-lg border-2 border-dashed border-slate-200 p-8 cursor-pointer hover:border-indigo-400/60 transition-colors"
                  onClick={() => fileRef.current?.click()}
                >
                  <FileCode size={32} className="mb-3 text-slate-500" />
                  <p className="text-sm text-slate-400">Add .wasm files to the batch queue</p>
                  <p className="mt-1 text-xs text-slate-500">You can select multiple files</p>
                </div>
              ) : (
                <ScrollArea className="h-60 rounded-lg border border-slate-200 bg-slate-50">
                  <div className="space-y-1.5 p-2">
                    {queue.map((q, i) => (
                      <div key={i} className="flex items-center justify-between rounded-lg border border-slate-200 bg-white px-3 py-2">
                        <span className="flex items-center gap-2 text-sm">
                          <FileCode size={14} className="text-indigo-600" />
                          <span className="text-slate-900">{q.name}</span>
                        </span>
                        <Button
                          onClick={() => removeFile(i)}
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8 text-slate-500 hover:text-red-300"
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
              <CardTitle className="text-sm font-medium text-slate-900">Options</CardTitle>
            </CardHeader>
            <CardContent>
              <label className="flex items-center gap-2 cursor-pointer">
                <input
                  type="checkbox"
                  checked={continueOnError}
                  onChange={(e) => setContinueOnError(e.target.checked)}
                  className="h-4 w-4 rounded border-slate-300 bg-white text-indigo-600 focus:ring-indigo-500"
                />
                <span className="text-sm text-slate-700">Continue on error</span>
              </label>
            </CardContent>
          </Card>

          {/* Run button */}
          <Button
            onClick={runBatch}
            disabled={queue.length === 0 || running}
            variant="gradient"
            className="w-full h-12"
          >
            {running ? (
              <>
                <RefreshCw size={16} className="animate-spin" /> Executing Batch...
              </>
            ) : (
              <>
                <Play size={16} /> Execute Batch ({queue.length} files)
              </>
            )}
          </Button>

          {/* Progress */}
          {running && (
            <Card>
              <CardContent className="p-4">
              <div className="flex justify-between text-xs text-slate-400 mb-2">
                <span>Progress</span><span>{progress}%</span>
              </div>
              <Progress value={progress} />
              </CardContent>
            </Card>
          )}
        </div>

        {/* Results */}
        <div>
          {result ? (
            <div className="space-y-4">
              <Card>
                <CardHeader>
                  <CardTitle className="text-sm font-medium text-slate-900">Batch Results</CardTitle>
                </CardHeader>
                <CardContent>
                <div className="grid grid-cols-3 gap-3 text-center">
                  <div className="rounded-lg bg-slate-50 border border-slate-200 p-3">
                    <p className="text-xl font-bold text-slate-900">{result.total_files}</p>
                    <p className="text-xs text-slate-500">Total</p>
                  </div>
                  <div className="rounded-lg bg-green-500/10 p-3">
                    <p className="text-xl font-bold text-green-400">{result.successful}</p>
                    <p className="text-xs text-slate-500">Passed</p>
                  </div>
                  <div className="rounded-lg bg-red-500/10 p-3">
                    <p className="text-xl font-bold text-red-400">{result.failed}</p>
                    <p className="text-xs text-slate-500">Failed</p>
                  </div>
                </div>
                </CardContent>
              </Card>

              <Card>
                <CardHeader>
                  <CardTitle className="text-sm font-medium text-slate-900">Individual Results</CardTitle>
                </CardHeader>
                <CardContent>
                <div className="space-y-2">
                  {result.results.map((r, i) => (
                    <div key={`ok-${i}`} className="flex items-start gap-3 rounded-lg border border-green-500/30 bg-green-500/5 p-3">
                      <CheckCircle size={16} className="text-green-400 mt-0.5 shrink-0" />
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-slate-900 font-mono truncate">{r.execution_id}</p>
                        <p className="text-xs text-slate-500 mt-0.5">{r.duration_ms.toFixed(2)}ms · {r.instructions.toLocaleString()} instructions</p>
                      </div>
                    </div>
                  ))}
                  {(result.errors ?? []).map((e: BatchFileError, i: number) => (
                    <div key={`err-${i}`} className="flex items-start gap-3 rounded-lg border border-red-500/30 bg-red-500/5 p-3">
                      <AlertCircle size={16} className="text-red-400 mt-0.5 shrink-0" />
                      <div className="min-w-0 flex-1">
                        <p className="text-sm font-medium text-slate-900 truncate">{e.path}</p>
                        <p className="text-xs text-red-400 mt-0.5">{e.error}</p>
                      </div>
                    </div>
                  ))}
                </div>
                </CardContent>
              </Card>
            </div>
          ) : (
            <Card>
              <CardContent className="p-12 text-center">
              <Layers size={48} className="mx-auto mb-4 text-slate-600" />
              <p className="text-slate-500">Add files and run to see batch results</p>
              </CardContent>
            </Card>
          )}
        </div>
      </div>
    </div>
  );
}

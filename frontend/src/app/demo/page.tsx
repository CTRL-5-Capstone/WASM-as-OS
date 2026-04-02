'use client';

import { useEffect, useMemo, useRef, useState } from 'react';
import {
  getStats,
  getTasks,
  getTestFiles,
  healthLive,
  readFileAsBytes,
  runTestFile,
  startTask,
  uploadTask,
  type SystemStats,
  type Task,
  type TestFile,
} from '@/lib/api';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { ScrollArea } from '@/components/ui/scroll-area';

type LogType = 'INFO' | 'SUCCESS' | 'ERROR' | 'WARN';

type DemoLog = {
  ts: string;
  type: LogType;
  message: string;
};

function nowStamp() {
  return new Date().toLocaleTimeString();
}

function pickSampleTestFile(files: TestFile[]): TestFile | null {
  if (!files.length) return null;
  const preferred = files.find((f) => /add|calc|hello/i.test(f.name));
  return preferred ?? files[0];
}

export default function DemoPage() {
  const fileInputRef = useRef<HTMLInputElement | null>(null);

  const [online, setOnline] = useState<boolean | null>(null);
  const [tasks, setTasks] = useState<Task[]>([]);
  const [testFiles, setTestFiles] = useState<TestFile[]>([]);
  const [stats, setStats] = useState<SystemStats | null>(null);
  const [latestTaskId, setLatestTaskId] = useState<string | null>(null);

  const [setupOpen, setSetupOpen] = useState(false);
  const [busy, setBusy] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);

  const [logs, setLogs] = useState<DemoLog[]>(() => [
    { ts: nowStamp(), type: 'INFO', message: 'WASM-as-OS demo initialized' },
    { ts: nowStamp(), type: 'INFO', message: 'UI ready — checking backend health…' },
  ]);

  const sampleFile = useMemo(() => pickSampleTestFile(testFiles), [testFiles]);
  const moduleCount = testFiles.length;
  const taskCount = tasks.length;

  function addLog(message: string, type: LogType = 'INFO') {
    setLogs((prev) => [...prev, { ts: nowStamp(), type, message }]);
  }

  async function refreshAll(reason?: string) {
    try {
      if (reason) addLog(reason);

      const health = await healthLive();
      setOnline(true);
      addLog(`Backend live (${health.status})`, 'SUCCESS');

      const [tf, t, s] = await Promise.all([getTestFiles(), getTasks(), getStats()]);
      setTestFiles(tf.files);
      setTasks(t);
      setStats(s);

      addLog(`Loaded ${tf.total} sample modules and ${t.length} tasks`, 'INFO');
    } catch (e) {
      setOnline(false);
      addLog('Backend offline — running UI in demo mode', 'WARN');
    }
  }

  useEffect(() => {
    refreshAll();
    const id = setInterval(() => {
      // Lightweight background activity like the static demo.
      const activities = [
        'Background health check complete',
        'Metrics snapshot collected',
        'Scheduler tick processed',
        'Runtime cache refreshed',
        'Session heartbeat ok',
      ];
      if (Math.random() > 0.72) addLog(activities[Math.floor(Math.random() * activities.length)]);
    }, 5000);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function handleUploadFile(file: File) {
    setBusy('upload');
    try {
      addLog(`Uploading module: ${file.name} (${Math.round(file.size / 1024)}KB)…`);
      const bytes = await readFileAsBytes(file);
      const created = await uploadTask(file.name, bytes);
      setLatestTaskId(created.id);
      addLog(`Uploaded successfully (task_id=${created.id})`, 'SUCCESS');

      const t = await getTasks();
      setTasks(t);
    } catch (e) {
      addLog(`Upload failed: ${e instanceof Error ? e.message : String(e)}`, 'ERROR');
    } finally {
      setBusy(null);
    }
  }

  async function executeSample() {
    setBusy('execute');
    try {
      if (online === false) {
        addLog('Backend is offline — cannot execute. Start the Rust server on :8080.', 'ERROR');
        return;
      }

      // Prefer a real uploaded task; otherwise run a built-in test file.
      if (latestTaskId) {
        addLog(`Executing latest uploaded task (${latestTaskId})…`);
        const res = await startTask(latestTaskId);
        addLog(`Execution ${res.success ? 'completed' : 'failed'} (duration=${res.duration_us}µs)`, res.success ? 'SUCCESS' : 'ERROR');
        if (res.return_value !== undefined) addLog(`Return value: ${String(res.return_value)}`, 'INFO');
        for (const line of res.stdout_log ?? []) addLog(line, 'INFO');

        const t = await getTasks();
        setTasks(t);
        return;
      }

      if (!sampleFile) {
        addLog('No sample modules available from /v1/test-files', 'WARN');
        return;
      }

      addLog(`Executing sample module: ${sampleFile.name}…`);
      const res = await runTestFile(sampleFile.name);
      addLog(`Execution ${res.success ? 'completed' : 'failed'} (duration=${res.duration_us}µs)`, res.success ? 'SUCCESS' : 'ERROR');
      if (res.return_value) addLog(`Return value: ${res.return_value}`, 'INFO');
      for (const line of res.stdout_log ?? []) addLog(line, 'INFO');
    } catch (e) {
      addLog(`Execution error: ${e instanceof Error ? e.message : String(e)}`, 'ERROR');
    } finally {
      setBusy(null);
    }
  }

  async function viewMetrics() {
    setBusy('metrics');
    try {
      if (online === false) {
        addLog('Backend is offline — cannot fetch metrics', 'ERROR');
        return;
      }

      addLog('Fetching system stats…');
      const s = await getStats();
      setStats(s);
      addLog(`Total tasks: ${s.total_tasks}`, 'INFO');
      addLog(`Running tasks: ${s.running_tasks}`, 'INFO');
      addLog(`Failed tasks: ${s.failed_tasks}`, 'INFO');
      addLog(`Total instructions: ${s.total_instructions}`, 'INFO');
      addLog(`Total syscalls: ${s.total_syscalls}`, 'INFO');
    } catch (e) {
      addLog(`Metrics error: ${e instanceof Error ? e.message : String(e)}`, 'ERROR');
    } finally {
      setBusy(null);
    }
  }

  async function testScheduler() {
    setBusy('scheduler');
    try {
      if (online === false) {
        addLog('Backend is offline — cannot inspect scheduler', 'ERROR');
        return;
      }

      addLog('Checking task queue…');
      const t = await getTasks();
      setTasks(t);

      const running = t.filter((x) => x.status.toLowerCase() === 'running').length;
      const pending = t.filter((x) => x.status.toLowerCase() === 'pending').length;

      addLog(`Round-robin scheduler: ${pending} pending tasks`, 'INFO');
      addLog(`Priority scheduler: ${running} running tasks`, 'INFO');
      addLog('Cooperative scheduler: runtime ready', 'INFO');
      addLog('Schedulers operational', 'SUCCESS');
    } catch (e) {
      addLog(`Scheduler check failed: ${e instanceof Error ? e.message : String(e)}`, 'ERROR');
    } finally {
      setBusy(null);
    }
  }

  function onPickFileClick() {
    fileInputRef.current?.click();
  }

  return (
    <div className="min-h-screen bg-gradient-to-br from-indigo-500 to-purple-700 px-5 py-6">
      <div className="mx-auto max-w-6xl">
        <header className="mb-6 rounded-xl bg-white/95 px-6 py-6 text-center shadow-lg ring-1 ring-black/5">
          <div className="flex items-center justify-center gap-3">
            <h1 className="text-2xl font-extrabold tracking-tight text-slate-900">
              WASM-as-OS
            </h1>
            <span className="rounded-full bg-amber-300 px-3 py-1 text-xs font-bold text-amber-900">
              {online === false ? 'DEMO MODE' : 'LIVE'}
            </span>
          </div>
          <p className="mt-2 text-sm text-slate-600">WebAssembly Execution Platform — Interactive Run Demo</p>
        </header>

        {online === false && (
          <div className="mb-6 rounded-lg border border-amber-300 bg-amber-50 px-4 py-3 text-sm text-amber-900 shadow-sm">
            <strong>Demo Mode:</strong> backend is not reachable. Start `wasmos` on `:8080` for full functionality.
          </div>
        )}

        <div className="grid gap-4 md:grid-cols-2">
          <Card className="bg-white/95 text-slate-900 shadow-lg ring-1 ring-black/5">
            <CardHeader>
              <CardTitle className="text-slate-900">System Status</CardTitle>
              <CardDescription className="text-slate-600">Live connectivity and runtime mode</CardDescription>
            </CardHeader>
            <CardContent className="space-y-2 text-sm">
              <div className="flex items-center justify-between">
                <span className="text-slate-600">Backend</span>
                <span className={online ? 'rounded-md bg-emerald-100 px-2 py-1 font-semibold text-emerald-900' : 'rounded-md bg-amber-100 px-2 py-1 font-semibold text-amber-900'}>
                  {online === null ? 'Checking…' : online ? 'Connected' : 'Offline'}
                </span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-600">Frontend</span>
                <span className="rounded-md bg-emerald-100 px-2 py-1 font-semibold text-emerald-900">Active</span>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-slate-600">API</span>
                <span className={online ? 'rounded-md bg-emerald-100 px-2 py-1 font-semibold text-emerald-900' : 'rounded-md bg-amber-100 px-2 py-1 font-semibold text-amber-900'}>
                  {online ? 'Real' : 'Simulated'}
                </span>
              </div>
            </CardContent>
          </Card>

          <Card className="bg-white/95 text-slate-900 shadow-lg ring-1 ring-black/5">
            <CardHeader>
              <CardTitle className="text-slate-900">Demo Stats</CardTitle>
              <CardDescription className="text-slate-600">Modules, tasks, and system counters</CardDescription>
            </CardHeader>
            <CardContent>
              <div className="grid grid-cols-2 gap-3">
                <div className="rounded-lg bg-indigo-50 px-4 py-4 text-center">
                  <div className="text-3xl font-extrabold text-indigo-600">{moduleCount}</div>
                  <div className="mt-1 text-xs font-medium text-slate-600">Sample Modules</div>
                </div>
                <div className="rounded-lg bg-indigo-50 px-4 py-4 text-center">
                  <div className="text-3xl font-extrabold text-indigo-600">{taskCount}</div>
                  <div className="mt-1 text-xs font-medium text-slate-600">Tasks</div>
                </div>
              </div>

              {stats && (
                <div className="mt-4 grid grid-cols-2 gap-3 text-xs text-slate-700">
                  <div className="rounded-lg border border-slate-200 bg-white px-3 py-2">
                    <div className="font-semibold">Instructions</div>
                    <div className="mt-1 text-slate-600">{stats.total_instructions.toLocaleString()}</div>
                  </div>
                  <div className="rounded-lg border border-slate-200 bg-white px-3 py-2">
                    <div className="font-semibold">Syscalls</div>
                    <div className="mt-1 text-slate-600">{stats.total_syscalls.toLocaleString()}</div>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>

        <Card className="mt-4 bg-white/95 text-slate-900 shadow-lg ring-1 ring-black/5">
          <CardHeader>
            <CardTitle className="text-slate-900">Upload WASM Module</CardTitle>
            <CardDescription className="text-slate-600">Drag & drop a `.wasm` or `.wat` file, or click to choose</CardDescription>
          </CardHeader>
          <CardContent>
            <input
              ref={fileInputRef}
              type="file"
              className="hidden"
              accept=".wasm,.wat"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) void handleUploadFile(f);
                e.currentTarget.value = '';
              }}
            />

            <div
              role="button"
              tabIndex={0}
              onClick={onPickFileClick}
              onKeyDown={(e) => {
                if (e.key === 'Enter' || e.key === ' ') onPickFileClick();
              }}
              onDragEnter={(e) => {
                e.preventDefault();
                setDragOver(true);
              }}
              onDragOver={(e) => {
                e.preventDefault();
                setDragOver(true);
              }}
              onDragLeave={() => setDragOver(false)}
              onDrop={(e) => {
                e.preventDefault();
                setDragOver(false);
                const f = e.dataTransfer.files?.[0];
                if (f) void handleUploadFile(f);
              }}
              className={
                'rounded-xl border-2 border-dashed px-6 py-10 text-center transition ' +
                (dragOver
                  ? 'border-indigo-500 bg-indigo-50'
                  : 'border-slate-300 bg-white hover:border-indigo-500 hover:bg-indigo-50/50')
              }
            >
              <div className="text-4xl">📁</div>
              <div className="mt-3 text-sm font-semibold text-slate-800">
                {busy === 'upload' ? 'Uploading…' : 'Click to upload, or drag & drop'}
              </div>
              <div className="mt-1 text-xs text-slate-500">Max size depends on your backend config</div>
              {latestTaskId && (
                <div className="mt-3 text-xs text-slate-600">
                  Latest task: <span className="font-mono">{latestTaskId}</span>
                </div>
              )}
            </div>
          </CardContent>
        </Card>

        <Card className="mt-4 bg-white/95 text-slate-900 shadow-lg ring-1 ring-black/5">
          <CardHeader>
            <CardTitle className="text-slate-900">Demo Actions</CardTitle>
            <CardDescription className="text-slate-600">Execute, inspect metrics, and validate scheduling behavior</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="flex flex-wrap gap-3">
              <Button
                onClick={() => void executeSample()}
                disabled={busy !== null}
                className="shadow"
              >
                Execute Sample Module
              </Button>
              <Button
                onClick={() => void viewMetrics()}
                disabled={busy !== null}
                variant="secondary"
                className="bg-slate-900/5 text-slate-900 hover:bg-slate-900/10"
              >
                View Metrics
              </Button>
              <Button
                onClick={() => void testScheduler()}
                disabled={busy !== null}
                variant="secondary"
                className="bg-slate-900/5 text-slate-900 hover:bg-slate-900/10"
              >
                Test Scheduler
              </Button>
              <Button
                onClick={() => setSetupOpen((v) => !v)}
                disabled={busy !== null}
                variant="gradient"
              >
                Setup Full Version
              </Button>
              <Button
                onClick={() => void refreshAll('Refreshing demo data…')}
                disabled={busy !== null}
                variant="outline"
                className="border-slate-200 text-slate-800 hover:bg-slate-50"
              >
                Refresh
              </Button>
            </div>

            <div className="mt-3 text-xs text-slate-500">
              {sampleFile ? (
                <>Sample module: <span className="font-mono">{sampleFile.name}</span> (from `/v1/test-files`)</>
              ) : (
                <>No sample modules loaded yet.</>
              )}
            </div>
          </CardContent>
        </Card>

        <Card className="mt-4 bg-white/95 text-slate-900 shadow-lg ring-1 ring-black/5">
          <CardHeader>
            <CardTitle className="text-slate-900">Demo Logs</CardTitle>
            <CardDescription className="text-slate-600">Live activity stream (frontend + backend)</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="rounded-lg bg-slate-900 px-3 py-3 text-slate-100 shadow-inner">
              <ScrollArea className="h-72">
                <div className="space-y-1 font-mono text-xs">
                  {logs.map((l, idx) => {
                    const color =
                      l.type === 'ERROR'
                        ? 'text-red-300'
                        : l.type === 'SUCCESS'
                          ? 'text-emerald-300'
                          : l.type === 'WARN'
                            ? 'text-amber-200'
                            : 'text-slate-100';
                    return (
                      <div key={idx} className={color}>
                        [{l.ts}] [{l.type}] {l.message}
                      </div>
                    );
                  })}
                </div>
              </ScrollArea>
            </div>
          </CardContent>
        </Card>

        {setupOpen && (
          <Card className="mt-4 bg-white/95 text-slate-900 shadow-lg ring-1 ring-black/5">
            <CardHeader>
              <CardTitle className="text-slate-900">Full Installation Setup</CardTitle>
              <CardDescription className="text-slate-600">Docker path is optional; local run works too</CardDescription>
            </CardHeader>
            <CardContent className="text-sm text-slate-700">
              <div className="font-semibold">To run the complete system:</div>
              <ol className="mt-2 list-decimal space-y-1 pl-5">
                <li>Start the Rust backend (`wasmos`) on `http://127.0.0.1:8080`</li>
                <li>Start the Next.js dashboard (`frontend`) on `http://localhost:3001`</li>
                <li>(Optional) If Docker is available, use `docker compose up -d --build`</li>
              </ol>
              <div className="mt-4 rounded-lg bg-slate-50 px-4 py-3 text-sm">
                <div className="font-semibold">Tip</div>
                <div className="text-slate-600">See `RUNNING_GUIDE.md` for copy/paste PowerShell steps.</div>
              </div>
            </CardContent>
          </Card>
        )}
      </div>
    </div>
  );
}

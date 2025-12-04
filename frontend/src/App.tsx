import { useEffect, useState } from "react";
import { getTasks, uploadWasm, deleteTask } from "./api";


function App() {
  const [tasks, setTasks] = useState<any[]>([]);
  const [file, setFile] = useState<File | null>(null);

  useEffect(() => {
    refresh();
  }, []);

  async function refresh() {
    const data = await getTasks();
    setTasks(data);
  }

  async function handleUpload() {
    if (!file) return;
    await uploadWasm(file);
    setFile(null);
    refresh();
  }

  async function handleDelete(id: number) {
    await deleteTask(id);
    refresh();
  }

  return (
    <div style={{ padding: "20px" }}>
      <h1>WASM-as-OS Frontend</h1>

      <input
        type="file"
        accept=".wasm"
        onChange={(e) => setFile(e.target.files?.[0] || null)}
      />

      <button onClick={handleUpload}>Upload WASM</button>

      <h2>Running Tasks</h2>

      {tasks.length === 0 ? (
        <p>No tasks running.</p>
      ) : (
        <ul>
          {tasks.map((t) => (
            <li key={t.id}>
              {t.name}
              <button onClick={() => handleDelete(t.id)}>Stop</button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

export default App;

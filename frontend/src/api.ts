const API_URL = "http://localhost:8080/api";

export async function getTasks() {
  const res = await fetch(`${API_URL}/tasks`);
  return res.json();
}

export async function uploadWasm(file: File) {
  const form = new FormData();
  form.append("file", file);

  const res = await fetch(`${API_URL}/tasks`, {
    method: "POST",
    body: form,
  });

  return res.json();
}

export async function deleteTask(id: number) {
  await fetch(`${API_URL}/tasks/${id}`, {
    method: "DELETE",
  });
}

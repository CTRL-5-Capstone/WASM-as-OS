<template>
  <div class="container">
    <header>
      <h1>🔧 WASM-as-OS Dashboard</h1>
    </header>
    
    <div class="card">
      <h2>📁 Upload WASM File</h2>
      <div class="upload-section">
        <input type="file" @change="selectFile" accept=".wasm" class="file-input">
        <button @click="uploadFile" :disabled="!selectedFile" class="btn btn-primary">
          {{ selectedFile ? 'Start Task' : 'Select File First' }}
        </button>
      </div>
    </div>

    <div class="card">
      <div class="header-row">
        <h2>⚡ Running Tasks ({{ tasks.length }})</h2>
        <button @click="fetchTasks" class="btn btn-secondary">🔄 Refresh</button>
      </div>
      
      <div v-if="tasks.length === 0" class="empty-state">
        No tasks running
      </div>
      
      <div v-else class="task-list">
        <div v-for="task in tasks" :key="task.id" class="task-item">
          <div class="task-info">
            <span class="task-name">{{ task.name }}</span>
            <span class="task-id">ID: {{ task.id }}</span>
          </div>
          <button @click="stopTask(task.id)" class="btn btn-danger">🛑 Stop</button>
        </div>
      </div>
    </div>
  </div>
</template>

<script>
import axios from 'axios'

export default {
  data() {
    return {
      tasks: [],
      selectedFile: null
    }
  },
  async mounted() {
    await this.fetchTasks()
  },
  methods: {
    async fetchTasks() {
      try {
        const response = await axios.get('/api/tasks')
        this.tasks = response.data
      } catch (error) {
        console.error('Failed to fetch tasks:', error)
      }
    },
    selectFile(event) {
      this.selectedFile = event.target.files[0]
    },
    async uploadFile() {
      if (!this.selectedFile) return
      
      const formData = new FormData()
      formData.append('file', this.selectedFile)
      
      try {
        await axios.post('/api/tasks', formData)
        this.selectedFile = null
        await this.fetchTasks()
      } catch (error) {
        console.error('Failed to upload file:', error)
      }
    },
    async stopTask(id) {
      try {
        await axios.delete(`/api/tasks/${id}`)
        await this.fetchTasks()
      } catch (error) {
        console.error('Failed to stop task:', error)
      }
    }
  }
}
</script>

<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
  background: #f5f5f5;
}

.container {
  max-width: 800px;
  margin: 0 auto;
  padding: 20px;
}

header {
  text-align: center;
  margin-bottom: 30px;
}

h1 {
  color: #333;
  font-size: 2.5rem;
  font-weight: 300;
}

.card {
  background: white;
  border-radius: 8px;
  padding: 24px;
  margin-bottom: 20px;
  box-shadow: 0 2px 8px rgba(0,0,0,0.1);
}

h2 {
  color: #555;
  margin-bottom: 16px;
  font-size: 1.3rem;
}

.upload-section {
  display: flex;
  gap: 12px;
  align-items: center;
}

.file-input {
  flex: 1;
  padding: 8px;
  border: 2px dashed #ddd;
  border-radius: 4px;
  background: #fafafa;
}

.btn {
  padding: 10px 16px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
  transition: all 0.2s;
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background: #007bff;
  color: white;
}

.btn-primary:hover:not(:disabled) {
  background: #0056b3;
}

.btn-secondary {
  background: #6c757d;
  color: white;
}

.btn-secondary:hover {
  background: #545b62;
}

.btn-danger {
  background: #dc3545;
  color: white;
  padding: 6px 12px;
  font-size: 0.9rem;
}

.btn-danger:hover {
  background: #c82333;
}

.header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.empty-state {
  text-align: center;
  color: #999;
  padding: 40px;
  font-style: italic;
}

.task-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.task-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px;
  background: #f8f9fa;
  border-radius: 6px;
  border-left: 4px solid #28a745;
}

.task-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.task-name {
  font-weight: 600;
  color: #333;
}

.task-id {
  font-size: 0.85rem;
  color: #666;
  font-family: monospace;
}
</style>
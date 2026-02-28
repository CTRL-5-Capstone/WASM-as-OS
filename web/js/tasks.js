// Refresh task list from API
async function refreshTasks() {
    try {
        const result = await API.getTasks();
        const tasksList = document.getElementById('tasksList');

        if (result.success && result.data.length > 0) {
            tasksList.innerHTML = result.data.map(task => {
                const status = (task.status || 'pending').toLowerCase();
                const statusColors = {
                    pending: 'bg-blue-500/10 text-blue-500 border-blue-500/20',
                    running: 'bg-green-500/10 text-green-500 border-green-500/20',
                    completed: 'bg-emerald-500/10 text-emerald-500 border-emerald-500/20',
                    stopped: 'bg-gray-500/10 text-gray-500 border-gray-500/20',
                    failed: 'bg-red-500/10 text-red-500 border-red-500/20',
                };
                const colorClass = statusColors[status] || statusColors.pending;

                // Determine which buttons to show based on status
                const canStart = status !== 'running';
                const canStop = status === 'running';

                return `
                <div class="bg-card border border-border rounded-xl p-6 hover:border-primary/50 transition-all flex items-center justify-between">
                    <div>
                        <h4 class="text-lg font-semibold mb-1">${task.name}</h4>
                        <p class="text-sm text-muted-foreground mb-2">ID: ${task.id.substring(0, 8)}...</p>
                        <span class="inline-block px-3 py-1 rounded-full text-xs font-medium border ${colorClass}">${status}</span>
                        <p class="text-xs text-muted-foreground mt-2">Size: ${(task.file_size_bytes / 1024).toFixed(1)} KB</p>
                    </div>
                    <div class="flex gap-2">
                        ${canStart ? `<button onclick="startTask('${task.id}')" class="px-4 py-2 bg-green-600 hover:bg-green-700 text-white rounded-lg transition-colors text-sm">Start</button>` : ''}
                        ${canStop ? `<button onclick="stopTask('${task.id}')" class="px-4 py-2 bg-red-600 hover:bg-red-700 text-white rounded-lg transition-colors text-sm">Stop</button>` : ''}
                        <button onclick="deleteTask('${task.id}')" class="px-4 py-2 bg-gray-600 hover:bg-gray-700 text-white rounded-lg transition-colors text-sm">Delete</button>
                    </div>
                </div>
            `}).join('');
        } else if (result.success) {
            tasksList.innerHTML = '<p class="text-muted-foreground text-center py-8">No tasks loaded — upload a WASM file above</p>';
        } else {
            tasksList.innerHTML = `
                <div class="bg-red-500/10 border border-red-500/20 rounded-xl p-6 text-center">
                    <p class="text-red-400 font-semibold mb-2">⚠ Failed to Fetch Tasks</p>
                    <p class="text-muted-foreground text-sm">Cannot connect to API. Make sure WASM-OS server is running.</p>
                    <button onclick="refreshTasks()" class="mt-4 px-4 py-2 bg-secondary text-foreground rounded-lg hover:bg-accent transition-colors text-sm">Retry</button>
                </div>
            `;
        }
    } catch (error) {
        console.error('refreshTasks error:', error);
    }
}

// Upload new WASM task
document.getElementById('loadTaskForm').addEventListener('submit', async (e) => {
    e.preventDefault();

    const name = document.getElementById('taskName').value;
    const file = document.getElementById('wasmFile').files[0];

    if (!file) {
        showNotification('Please select a WASM file', 'error');
        return;
    }
    if (!name.trim()) {
        showNotification('Please enter a task name', 'error');
        return;
    }

    showNotification('Uploading...', 'info');

    const reader = new FileReader();
    reader.onload = async (event) => {
        try {
            const wasmData = new Uint8Array(event.target.result);
            const result = await API.loadTask(name, wasmData);

            if (result.success) {
                showNotification('Task loaded: ' + (result.data.name || result.data.id), 'success');
                document.getElementById('loadTaskForm').reset();
                refreshTasks();
            } else {
                showNotification('Failed: ' + (result.error || 'Unknown error'), 'error');
            }
        } catch (error) {
            showNotification('Error: ' + error.message, 'error');
        }
    };
    reader.onerror = () => showNotification('Failed to read file', 'error');
    reader.readAsArrayBuffer(file);
});

// Task actions with proper error messages
async function startTask(id) {
    try {
        const result = await API.startTask(id);
        if (result.success) {
            showNotification('Task executed successfully', 'success');
        } else {
            showNotification(result.error || 'Failed to start task', 'error');
        }
        refreshTasks();
    } catch (error) {
        showNotification('Error: ' + error.message, 'error');
    }
}

async function stopTask(id) {
    try {
        const result = await API.stopTask(id);
        if (result.success) {
            showNotification('Task stopped', 'success');
        } else {
            showNotification(result.error || 'Failed to stop task', 'error');
        }
        refreshTasks();
    } catch (error) {
        showNotification('Error: ' + error.message, 'error');
    }
}

async function deleteTask(id) {
    if (!confirm('Delete this task?')) return;
    try {
        const result = await API.deleteTask(id);
        if (result.success) {
            showNotification('Task deleted', 'success');
        } else {
            showNotification(result.error || 'Failed to delete task', 'error');
        }
        refreshTasks();
    } catch (error) {
        showNotification('Error: ' + error.message, 'error');
    }
}

// Initial load + auto-refresh every 10 seconds
refreshTasks();
setInterval(refreshTasks, 10000);

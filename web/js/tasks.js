async function refreshTasks() {
    try {
        const result = await API.getTasks();
        const tasksList = document.getElementById('tasksList');
        
        if (result.success && result.data.length > 0) {
            tasksList.innerHTML = result.data.map(task => {
                const statusType = task.status.type || task.status;
                const statusText = task.status.Failed || statusType;
                const execHistory = task.execution_history || [];
                const lastExec = execHistory.length > 0 ? execHistory[execHistory.length - 1] : null;
                
                return `
                <div class="task-card">
                    <div class="task-info">
                        <h4>${task.name}</h4>
                        <p style="font-size: 0.85rem; color: #666;">ID: ${task.id}</p>
                        <span class="task-status status-${statusType.toLowerCase()}">${statusText}</span>
                        ${task.metrics ? `
                            <div style="margin-top: 0.5rem; font-size: 0.85rem; color: #666;">
                                <p>📊 Runs: ${task.metrics.runs} | Instructions: ${task.metrics.total_instructions.toLocaleString()} | Syscalls: ${task.metrics.total_syscalls}</p>
                            </div>
                        ` : ''}
                        ${lastExec ? `
                            <div style="margin-top: 0.5rem; font-size: 0.85rem; color: ${lastExec.success ? '#28a745' : '#dc3545'};">
                                <p>✓ Last: ${lastExec.timestamp} (${lastExec.duration_us}µs) ${lastExec.success ? '✓' : '✗'}</p>
                            </div>
                        ` : ''}
                        ${execHistory.length > 0 ? `
                            <button onclick="showHistory('${task.id}')" style="margin-top: 0.5rem; padding: 0.25rem 0.5rem; font-size: 0.8rem; background: #6c757d; color: white; border: none; border-radius: 3px; cursor: pointer;">View History (${execHistory.length})</button>
                        ` : ''}
                    </div>
                    <div class="task-actions">
                        <button class="btn-start" onclick="startTask('${task.id}')">Start</button>
                        <button class="btn-pause" onclick="pauseTask('${task.id}')">Pause</button>
                        <button class="btn-stop" onclick="stopTask('${task.id}')">Stop</button>
                        <button class="btn-delete" onclick="deleteTask('${task.id}')">Delete</button>
                    </div>
                </div>
            `}).join('');
        } else {
            tasksList.innerHTML = '<p style="color: #666;">No tasks loaded</p>';
        }
    } catch (error) {
        showNotification('Failed to load tasks', 'error');
    }
}

function showHistory(taskId) {
    API.getTask(taskId).then(result => {
        if (result.success && result.data.execution_history) {
            const history = result.data.execution_history;
            const historyHtml = `
                <div style="position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); background: white; padding: 2rem; border-radius: 10px; box-shadow: 0 10px 40px rgba(0,0,0,0.3); max-width: 600px; max-height: 80vh; overflow-y: auto; z-index: 1000;">
                    <h3>Execution History</h3>
                    <button onclick="this.parentElement.remove(); document.getElementById('overlay').remove();" style="position: absolute; top: 1rem; right: 1rem; background: #dc3545; color: white; border: none; padding: 0.5rem 1rem; border-radius: 5px; cursor: pointer;">Close</button>
                    <div style="margin-top: 1rem;">
                        ${history.map((exec, i) => `
                            <div style="padding: 0.75rem; margin: 0.5rem 0; background: ${exec.success ? '#d4edda' : '#f8d7da'}; border-radius: 5px; border-left: 4px solid ${exec.success ? '#28a745' : '#dc3545'};">
                                <p><strong>#${history.length - i}</strong> ${exec.timestamp}</p>
                                <p>Duration: ${exec.duration_us}µs | Instructions: ${exec.instructions} | Syscalls: ${exec.syscalls}</p>
                                <p>Status: ${exec.success ? '✓ Success' : '✗ Failed'}</p>
                                ${exec.error ? `<p style="color: #dc3545; font-size: 0.85rem;">Error: ${exec.error}</p>` : ''}
                            </div>
                        `).reverse().join('')}
                    </div>
                </div>
                <div id="overlay" onclick="this.previousElementSibling.remove(); this.remove();" style="position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 999;"></div>
            `;
            document.body.insertAdjacentHTML('beforeend', historyHtml);
        }
    });
}

document.getElementById('loadTaskForm').addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const name = document.getElementById('taskName').value;
    const file = document.getElementById('wasmFile').files[0];
    
    if (!file) {
        showNotification('Please select a WASM file', 'error');
        return;
    }
    
    showNotification('Uploading...', 'info');
    
    const reader = new FileReader();
    reader.onload = async (event) => {
        try {
            const arrayBuffer = event.target.result;
            const wasmData = new Uint8Array(arrayBuffer);
            console.log('File:', file.name);
            console.log('Size:', wasmData.length, 'bytes');
            console.log('Magic:', Array.from(wasmData.slice(0, 4)).map(b => '0x' + b.toString(16).padStart(2, '0')).join(' '));
            
            const result = await API.loadTask(name, wasmData);
            console.log('API Result:', result);
            
            if (result.success) {
                showNotification('Task loaded: ' + result.data, 'success');
                document.getElementById('loadTaskForm').reset();
                refreshTasks();
            } else {
                console.error('API Error:', result.error);
                showNotification('Failed: ' + (result.error || 'Unknown error'), 'error');
            }
        } catch (error) {
            console.error('Upload error:', error);
            showNotification('Error: ' + error.message, 'error');
        }
    };
    reader.onerror = () => {
        showNotification('Failed to read file', 'error');
    };
    reader.readAsArrayBuffer(file);
});

async function startTask(id) {
    try {
        const result = await API.startTask(id);
        if (result.success) {
            showNotification('Task started', 'success');
            refreshTasks();
        } else {
            showNotification(result.error || 'Failed to start task', 'error');
        }
    } catch (error) {
        showNotification('Error starting task', 'error');
    }
}

async function pauseTask(id) {
    try {
        const result = await API.pauseTask(id);
        if (result.success) {
            showNotification('Task paused', 'success');
            refreshTasks();
        } else {
            showNotification(result.error || 'Failed to pause task', 'error');
        }
    } catch (error) {
        showNotification('Error pausing task', 'error');
    }
}

async function stopTask(id) {
    try {
        const result = await API.stopTask(id);
        if (result.success) {
            showNotification('Task stopped', 'success');
            refreshTasks();
        } else {
            showNotification(result.error || 'Failed to stop task', 'error');
        }
    } catch (error) {
        showNotification('Error stopping task', 'error');
    }
}

async function deleteTask(id) {
    if (!confirm('Are you sure you want to delete this task?')) return;
    
    try {
        const result = await API.deleteTask(id);
        if (result.success) {
            showNotification('Task deleted', 'success');
            refreshTasks();
        } else {
            showNotification(result.error || 'Failed to delete task', 'error');
        }
    } catch (error) {
        showNotification('Error deleting task', 'error');
    }
}

refreshTasks();
setInterval(refreshTasks, 3000);

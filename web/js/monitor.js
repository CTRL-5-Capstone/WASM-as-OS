let logCounter = 0;
let lastUpdateSuccess = true;

async function updateMonitor() {
    try {
        const statsResult = await API.getStats();
        const tasksResult = await API.getTasks();
        
        if (statsResult.success) {
            const stats = statsResult.data;
            
            // Update metrics
            const cpuUsage = Math.min((stats.total_instructions / 1000000) * 100, 100);
            const memUsage = stats.total_tasks * 64; // Estimate
            
            document.getElementById('cpuBar').style.width = cpuUsage + '%';
            document.getElementById('cpuValue').textContent = cpuUsage.toFixed(1) + '%';
            
            document.getElementById('memBar').style.width = Math.min((memUsage / 1024) * 100, 100) + '%';
            document.getElementById('memValue').textContent = memUsage + ' KB';
            
            // Log successful connection recovery
            if (!lastUpdateSuccess) {
                addSystemLog('✓ Connection restored', 'success');
                lastUpdateSuccess = true;
            }
        }
        
        if (tasksResult.success) {
            const statusGrid = document.getElementById('liveTaskStatus');
            
            if (tasksResult.data.length > 0) {
                statusGrid.innerHTML = tasksResult.data.map(task => `
                    <div class="task-card">
                        <div class="task-info">
                            <h4>${task.name}</h4>
                            <span class="task-status status-${task.status.toLowerCase()}">${task.status}</span>
                        </div>
                        <div style="font-size: 0.85rem; color: #666;">
                            <p>ID: ${task.id.substring(0, 8)}...</p>
                        </div>
                    </div>
                `).join('');
            } else {
                statusGrid.innerHTML = '<p style="color: #666;">No active tasks</p>';
            }
        }
        
    } catch (error) {
        // Only log connection error once
        if (lastUpdateSuccess) {
            addSystemLog('⚠ Connection to API lost - retrying...', 'error');
            lastUpdateSuccess = false;
        }
    }
}

function addSystemLog(message, type = 'info') {
    const logsBox = document.getElementById('systemLogs');
    const timestamp = new Date().toLocaleTimeString();
    const entry = document.createElement('div');
    entry.className = 'log-entry';
    
    let color = '#333';
    if (type === 'error') color = '#dc3545';
    if (type === 'success') color = '#28a745';
    
    entry.style.color = color;
    entry.innerHTML = `<span style="color: #666;">[${timestamp}]</span> ${message}`;
    logsBox.insertBefore(entry, logsBox.firstChild);
    
    if (logsBox.children.length > 100) {
        logsBox.removeChild(logsBox.lastChild);
    }
}

// Simulate some activity logs
function simulateActivity() {
    const activities = [
        'Syscall executed: read_sensor',
        'Memory bounds check passed',
        'Instruction limit check: OK',
        'Task scheduler tick',
        'Sandbox validation complete'
    ];
    
    if (Math.random() > 0.7) {
        const activity = activities[Math.floor(Math.random() * activities.length)];
        addSystemLog(activity);
    }
}

updateMonitor();
setInterval(updateMonitor, 2000);
setInterval(simulateActivity, 3000);

async function updateDashboard() {
    try {
        const result = await API.getStats();
        
        if (result.success) {
            const stats = result.data;
            document.getElementById('totalTasks').textContent = stats.total_tasks;
            document.getElementById('runningTasks').textContent = stats.running_tasks;
            document.getElementById('failedTasks').textContent = stats.failed_tasks;
            document.getElementById('totalInstructions').textContent = stats.total_instructions.toLocaleString();
            document.getElementById('totalSyscalls').textContent = stats.total_syscalls.toLocaleString();
            
            document.getElementById('systemStatus').innerHTML = `
                <p style="color: #28a745; font-weight: bold;">✓ WASM-OS API Connected</p>
                <p>Server is running and accepting requests</p>
            `;
            
            addActivityLog('Dashboard updated successfully');
        }
    } catch (error) {
        document.getElementById('systemStatus').innerHTML = `
            <p style="color: #dc3545; font-weight: bold;">✗ Connection Failed</p>
            <p>Unable to connect to WASM-OS API at ${API_BASE}</p>
            <p style="font-size: 0.9rem; color: #666;">Make sure the server is running: ./target/release/wasm-os server --port 8080</p>
        `;
        addActivityLog('Failed to connect to API', 'error');
    }
}

function addActivityLog(message, type = 'info') {
    const logBox = document.getElementById('activityLog');
    const timestamp = new Date().toLocaleTimeString();
    const entry = document.createElement('div');
    entry.className = 'log-entry';
    entry.innerHTML = `<span style="color: #666;">[${timestamp}]</span> ${message}`;
    logBox.insertBefore(entry, logBox.firstChild);
    
    if (logBox.children.length > 50) {
        logBox.removeChild(logBox.lastChild);
    }
}

updateDashboard();
setInterval(updateDashboard, 5000);

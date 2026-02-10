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
        document.getElementById('totalTasks').textContent = '0';
        document.getElementById('runningTasks').textContent = '0';
        document.getElementById('failedTasks').textContent = '0';
        document.getElementById('totalInstructions').textContent = '0';
        document.getElementById('totalSyscalls').textContent = '0';
        
        document.getElementById('systemStatus').innerHTML = `
            <div style="background: linear-gradient(135deg, #ffebee, #ffcdd2); padding: 1.5rem; border-radius: 12px; border-left: 5px solid #dc3545;">
                <p style="color: #d32f2f; font-weight: bold; font-size: 1.1rem; margin-bottom: 0.5rem;">⚠ Server Not Running</p>
                <p style="color: #666; margin-bottom: 1rem;">Failed to fetch: Cannot connect to API at ${API_BASE}</p>
                <div style="background: white; padding: 1rem; border-radius: 8px; margin-top: 1rem;">
                    <p style="font-weight: 600; margin-bottom: 0.5rem;">To start the server:</p>
                    <ol style="margin-left: 1.5rem; line-height: 1.8;">
                        <li>Install Rust: <a href="https://rustup.rs/" target="_blank" style="color: #667eea;">https://rustup.rs/</a></li>
                        <li>Run: <code style="background: #f8f9fa; padding: 0.25rem 0.5rem; border-radius: 4px;">build.bat</code></li>
                        <li>Run: <code style="background: #f8f9fa; padding: 0.25rem 0.5rem; border-radius: 4px;">run-server.bat</code></li>
                    </ol>
                </div>
            </div>
        `;
        addActivityLog('Failed to fetch: Server not running', 'error');
    }
}

function addActivityLog(message, type = 'info') {
    const logBox = document.getElementById('activityLog');
    const timestamp = new Date().toLocaleTimeString();
    const entry = document.createElement('div');
    entry.className = 'flex items-center gap-3 p-3 rounded-lg border border-border bg-muted/50 hover:bg-muted transition-colors';
    const color = type === 'error' ? 'text-red-600' : 'text-green-600';
    entry.innerHTML = `<span class="text-xs text-muted-foreground font-mono">${timestamp}</span><span class="text-sm ${color}">${message}</span>`;
    logBox.insertBefore(entry, logBox.firstChild);
    
    if (logBox.children.length > 50) {
        logBox.removeChild(logBox.lastChild);
    }
}

updateDashboard();
setInterval(updateDashboard, 5000);

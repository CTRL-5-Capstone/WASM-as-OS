async function updateDashboard() {
    try {
        const result = await API.getStats();

        if (result.success) {
            const stats = result.data;
            const el = (id) => document.getElementById(id);
            if (el('totalTasks')) el('totalTasks').textContent = stats.total_tasks || 0;
            if (el('runningTasks')) el('runningTasks').textContent = stats.running_tasks || 0;
            if (el('failedTasks')) el('failedTasks').textContent = stats.failed_tasks || 0;
            if (el('totalInstructions')) el('totalInstructions').textContent = (stats.total_instructions || 0).toLocaleString();
            if (el('totalSyscalls')) el('totalSyscalls').textContent = (stats.total_syscalls || 0).toLocaleString();

            const statusEl = el('systemStatus');
            if (statusEl) {
                statusEl.innerHTML = `
                    <span class="w-2 h-2 rounded-full bg-green-500"></span>
                    Connected
                `;
                statusEl.className = 'flex items-center gap-2 px-4 py-2 rounded-full bg-green-500/10 text-green-500 text-sm font-medium';
            }

            addActivityLog('Dashboard updated');
        }
    } catch (error) {
        const el = (id) => document.getElementById(id);
        const statusEl = el('systemStatus');
        if (statusEl) {
            statusEl.innerHTML = `
                <span class="w-2 h-2 rounded-full bg-red-500"></span>
                Disconnected
            `;
            statusEl.className = 'flex items-center gap-2 px-4 py-2 rounded-full bg-red-500/10 text-red-500 text-sm font-medium';
        }
        addActivityLog('Server unreachable', 'error');
    }
}

function addActivityLog(message, type = 'info') {
    const logBox = document.getElementById('activityLog');
    if (!logBox) return;
    const timestamp = new Date().toLocaleTimeString();
    const entry = document.createElement('div');
    entry.className = 'flex items-center gap-3 p-3 rounded-lg border border-border bg-muted/50 hover:bg-muted transition-colors';
    const color = type === 'error' ? 'text-red-400' : 'text-green-400';
    entry.innerHTML = `<span class="text-xs text-muted-foreground font-mono">${timestamp}</span><span class="text-sm ${color}">${message}</span>`;
    logBox.insertBefore(entry, logBox.firstChild);

    if (logBox.children.length > 50) {
        logBox.removeChild(logBox.lastChild);
    }
}

updateDashboard();
setInterval(updateDashboard, 5000);


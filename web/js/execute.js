document.getElementById('executeForm').addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const file = document.getElementById('execWasmFile').files[0];
    
    if (!file) {
        showNotification('Please select a WASM file', 'error');
        return;
    }
    
    showNotification('Uploading and executing...', 'info');
    
    const reader = new FileReader();
    reader.onload = async (event) => {
        try {
            const wasmData = new Uint8Array(event.target.result);
            console.log('Executing WASM file:', file.name, wasmData.length, 'bytes');
            
            // Load as temporary task
            const loadResult = await API.loadTask('temp-exec-' + Date.now(), wasmData);
            console.log('Load result:', loadResult);
            
            if (loadResult.success) {
                const taskId = loadResult.data; // API returns task ID directly
                console.log('Task loaded with ID:', taskId);
                
                // Start the task
                const startResult = await API.startTask(taskId);
                console.log('Start result:', startResult);
                
                if (startResult.success) {
                    showNotification('Execution completed', 'success');
                    
                    // Poll for task completion
                    let pollCount = 0;
                    const pollInterval = setInterval(async () => {
                        pollCount++;
                        const taskInfo = await API.getTask(taskId);
                        console.log('Task info:', taskInfo);
                        
                        if (taskInfo.success && taskInfo.data) {
                            const status = taskInfo.data.status.type || taskInfo.data.status;
                            
                            // Update display
                            displayResults(taskInfo.data);
                            
                            // Stop polling if completed or failed, or after 10 attempts
                            if (status === 'Completed' || status === 'Failed' || pollCount >= 10) {
                                clearInterval(pollInterval);
                                
                                // Clean up after a delay
                                setTimeout(async () => {
                                    await API.deleteTask(taskId);
                                }, 2000);
                            }
                        } else if (pollCount >= 10) {
                            clearInterval(pollInterval);
                            displayResults({ 
                                status: 'Completed',
                                id: taskId,
                                note: 'Task executed successfully'
                            });
                            await API.deleteTask(taskId);
                        }
                    }, 500); // Poll every 500ms
                } else {
                    showNotification('Execution failed: ' + startResult.error, 'error');
                    displayResults({ error: startResult.error });
                }
            } else {
                showNotification('Failed to load: ' + loadResult.error, 'error');
                displayResults({ error: loadResult.error });
            }
        } catch (error) {
            console.error('Execute error:', error);
            showNotification('Error: ' + error.message, 'error');
            displayResults({ error: error.message });
        }
    };
    reader.readAsArrayBuffer(file);
});

function displayResults(data) {
    const resultsBox = document.getElementById('executionResults');
    const metricsBox = document.getElementById('executionMetrics');
    
    if (data.error) {
        resultsBox.innerHTML = `
            <div class="bg-red-500/10 border border-red-500/20 rounded-lg p-4">
                <h4 class="text-red-500 font-semibold mb-2">Execution Error</h4>
                <p class="text-red-400">${data.error}</p>
            </div>
        `;
        metricsBox.innerHTML = '';
        return;
    }
    
    const execHistory = data.execution_history || [];
    const lastExec = execHistory.length > 0 ? execHistory[execHistory.length - 1] : null;
    
    resultsBox.innerHTML = `
        <div class="bg-green-500/10 border border-green-500/20 rounded-lg p-4">
            <h4 class="text-green-500 font-semibold mb-3">✓ Execution Completed</h4>
            <div class="space-y-2 text-sm">
                <p><span class="text-muted-foreground">Status:</span> <span class="text-foreground font-medium">${data.status || 'Completed'}</span></p>
                <p><span class="text-muted-foreground">Task ID:</span> <span class="text-foreground font-mono">${data.id || 'N/A'}</span></p>
                ${data.note ? '<p class="text-muted-foreground italic">' + data.note + '</p>' : ''}
                ${lastExec ? `
                    <p><span class="text-muted-foreground">Timestamp:</span> <span class="text-foreground">${lastExec.timestamp}</span></p>
                    <p><span class="text-muted-foreground">Duration:</span> <span class="text-foreground font-mono">${lastExec.duration_us}µs</span></p>
                ` : ''}
            </div>
        </div>
    `;
    
    const metrics = data.metrics || {};
    const lastMetrics = lastExec || {};
    
    metricsBox.innerHTML = `
        <div class="bg-secondary rounded-lg p-4 border border-border">
            <h4 class="text-sm text-muted-foreground mb-2">Instructions Executed</h4>
            <p class="text-2xl font-bold text-blue-500">
                ${(lastMetrics.instructions || metrics.total_instructions || 0).toLocaleString()}
            </p>
        </div>
        <div class="bg-secondary rounded-lg p-4 border border-border">
            <h4 class="text-sm text-muted-foreground mb-2">Syscalls Made</h4>
            <p class="text-2xl font-bold text-purple-500">
                ${(lastMetrics.syscalls || metrics.total_syscalls || 0).toLocaleString()}
            </p>
        </div>
        <div class="bg-secondary rounded-lg p-4 border border-border">
            <h4 class="text-sm text-muted-foreground mb-2">Total Runs</h4>
            <p class="text-2xl font-bold text-green-500">
                ${metrics.runs || 1}
            </p>
        </div>
    `;
}

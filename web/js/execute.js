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
            <div class="security-error">
                <h4>Execution Error</h4>
                <p>${data.error}</p>
            </div>
        `;
        metricsBox.innerHTML = '';
        return;
    }
    
    const execHistory = data.execution_history || [];
    const lastExec = execHistory.length > 0 ? execHistory[execHistory.length - 1] : null;
    
    resultsBox.innerHTML = `
        <div class="security-ok">
            <h4>✓ Execution Completed</h4>
            <p><strong>Status:</strong> ${data.status || 'Completed'}</p>
            <p><strong>Task ID:</strong> ${data.id || 'N/A'}</p>
            ${data.note ? '<p><em>' + data.note + '</em></p>' : ''}
            ${lastExec ? `
                <p><strong>Timestamp:</strong> ${lastExec.timestamp}</p>
                <p><strong>Duration:</strong> ${lastExec.duration_us}µs</p>
            ` : ''}
        </div>
    `;
    
    const metrics = data.metrics || {};
    const lastMetrics = lastExec || {};
    
    metricsBox.innerHTML = `
        <div class="metric-card">
            <h4>Instructions Executed</h4>
            <p style="font-size: 1.5rem; font-weight: bold; color: #667eea;">
                ${(lastMetrics.instructions || metrics.total_instructions || 0).toLocaleString()}
            </p>
        </div>
        <div class="metric-card">
            <h4>Syscalls Made</h4>
            <p style="font-size: 1.5rem; font-weight: bold; color: #667eea;">
                ${(lastMetrics.syscalls || metrics.total_syscalls || 0).toLocaleString()}
            </p>
        </div>
        <div class="metric-card">
            <h4>Total Runs</h4>
            <p style="font-size: 1.5rem; font-weight: bold; color: #667eea;">
                ${metrics.runs || 1}
            </p>
        </div>
    `;
}

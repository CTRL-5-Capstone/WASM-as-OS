const API_BASE = 'http://localhost:8080';

const API = {
    async getStats() {
        const response = await fetch(`${API_BASE}/stats`);
        return response.json();
    },

    async getTasks() {
        const response = await fetch(`${API_BASE}/tasks`);
        return response.json();
    },

    async getTask(id) {
        const response = await fetch(`${API_BASE}/tasks/${id}`);
        return response.json();
    },

    async loadTask(name, wasmData) {
        try {
            // Ensure wasmData is a Uint8Array and convert to regular array
            const dataArray = Array.from(new Uint8Array(wasmData));
            console.log('Sending', dataArray.length, 'bytes to API');
            
            const response = await fetch(`${API_BASE}/tasks`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ name, wasm_data: dataArray })
            });
            
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            
            return await response.json();
        } catch (error) {
            console.error('API loadTask error:', error);
            return { success: false, error: error.message };
        }
    },

    async startTask(id) {
        const response = await fetch(`${API_BASE}/tasks/${id}/start`, {
            method: 'POST'
        });
        return response.json();
    },

    async pauseTask(id) {
        const response = await fetch(`${API_BASE}/tasks/${id}/pause`, {
            method: 'POST'
        });
        return response.json();
    },

    async stopTask(id) {
        const response = await fetch(`${API_BASE}/tasks/${id}/stop`, {
            method: 'POST'
        });
        return response.json();
    },

    async deleteTask(id) {
        const response = await fetch(`${API_BASE}/tasks/${id}`, {
            method: 'DELETE'
        });
        return response.json();
    }
};

function showNotification(message, type = 'info') {
    const notification = document.createElement('div');
    notification.className = `notification notification-${type}`;
    notification.textContent = message;
    notification.style.cssText = `
        position: fixed;
        top: 20px;
        right: 20px;
        padding: 1rem 2rem;
        background: ${type === 'error' ? '#dc3545' : type === 'success' ? '#28a745' : '#667eea'};
        color: white;
        border-radius: 5px;
        box-shadow: 0 4px 6px rgba(0,0,0,0.2);
        z-index: 1000;
        animation: slideIn 0.3s ease;
    `;
    document.body.appendChild(notification);
    setTimeout(() => {
        notification.style.animation = 'slideOut 0.3s ease';
        setTimeout(() => notification.remove(), 300);
    }, 3000);
}

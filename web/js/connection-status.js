// Global connection status indicator
let connectionStatus = 'checking';

function createStatusBanner() {
    const banner = document.createElement('div');
    banner.id = 'connection-banner';
    banner.style.cssText = `
        position: fixed;
        top: 70px;
        left: 50%;
        transform: translateX(-50%);
        background: linear-gradient(135deg, #ffebee, #ffcdd2);
        color: #d32f2f;
        padding: 1rem 2rem;
        border-radius: 12px;
        box-shadow: 0 4px 20px rgba(220, 53, 69, 0.3);
        z-index: 9999;
        display: none;
        font-weight: 600;
        border: 2px solid #dc3545;
        animation: slideDown 0.3s ease;
    `;
    banner.innerHTML = `
        <span style="margin-right: 0.5rem;">⚠</span>
        <span>Failed to fetch: Server not running</span>
        <button onclick="window.location.reload()" style="margin-left: 1rem; background: white; color: #dc3545; border: none; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; font-weight: 600;">Retry</button>
    `;
    document.body.appendChild(banner);
    return banner;
}

function showConnectionError() {
    let banner = document.getElementById('connection-banner');
    if (!banner) {
        banner = createStatusBanner();
    }
    banner.style.display = 'block';
    connectionStatus = 'disconnected';
}

function hideConnectionError() {
    const banner = document.getElementById('connection-banner');
    if (banner) {
        banner.style.display = 'none';
    }
    connectionStatus = 'connected';
}

// Check connection on page load
async function checkConnection() {
    try {
        const response = await fetch(`${API_BASE}/stats`);
        if (response.ok) {
            hideConnectionError();
            return true;
        }
    } catch (error) {
        showConnectionError();
        return false;
    }
}

// Add CSS animation
const style = document.createElement('style');
style.textContent = `
    @keyframes slideDown {
        from { transform: translate(-50%, -100%); opacity: 0; }
        to { transform: translate(-50%, 0); opacity: 1; }
    }
`;
document.head.appendChild(style);

// Check connection every 5 seconds
setInterval(checkConnection, 5000);
checkConnection();

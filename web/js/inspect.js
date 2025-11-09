document.getElementById('inspectForm').addEventListener('submit', async (e) => {
    e.preventDefault();
    
    const file = document.getElementById('inspectWasmFile').files[0];
    
    if (!file) {
        showNotification('Please select a WASM file', 'error');
        return;
    }
    
    const reader = new FileReader();
    reader.onload = async (event) => {
        try {
            const wasmData = new Uint8Array(event.target.result);
            
            // Parse WASM module locally
            const module = parseWasmModule(wasmData);
            displayModuleInfo(module);
            analyzeSecurityRisks(module);
            
            showNotification('Module inspected successfully', 'success');
        } catch (error) {
            showNotification('Error inspecting module', 'error');
            document.getElementById('moduleInfo').innerHTML = `
                <div class="security-error">
                    <h4>Parse Error</h4>
                    <p>${error.message}</p>
                </div>
            `;
        }
    };
    reader.readAsArrayBuffer(file);
});

function parseWasmModule(data) {
    const view = new DataView(data.buffer);
    
    // Check magic number
    const magic = view.getUint32(0, true);
    if (magic !== 0x6d736100) {
        throw new Error('Invalid WASM magic number');
    }
    
    // Check version
    const version = view.getUint32(4, true);
    
    const module = {
        version,
        types: 0,
        functions: 0,
        imports: [],
        exports: [],
        memory: null
    };
    
    let pos = 8;
    while (pos < data.length) {
        const sectionId = data[pos++];
        const sectionSize = readLEB128(data, pos);
        pos += sectionSize.bytes;
        
        if (sectionId === 1) module.types++;
        else if (sectionId === 2) {
            // Parse imports
            const count = readLEB128(data, pos);
            module.imports.push(`${count.value} imports detected`);
        }
        else if (sectionId === 3) module.functions++;
        else if (sectionId === 5) module.memory = 'Present';
        else if (sectionId === 7) {
            // Parse exports
            const count = readLEB128(data, pos);
            module.exports.push(`${count.value} exports detected`);
        }
        
        pos += sectionSize.value;
    }
    
    return module;
}

function readLEB128(data, pos) {
    let result = 0;
    let shift = 0;
    let bytes = 0;
    
    while (pos + bytes < data.length) {
        const byte = data[pos + bytes];
        bytes++;
        result |= (byte & 0x7F) << shift;
        if ((byte & 0x80) === 0) break;
        shift += 7;
    }
    
    return { value: result, bytes };
}

function displayModuleInfo(module) {
    const infoBox = document.getElementById('moduleInfo');
    
    infoBox.innerHTML = `
        <h4>Module Structure</h4>
        <p><strong>Version:</strong> ${module.version}</p>
        <p><strong>Types:</strong> ${module.types}</p>
        <p><strong>Functions:</strong> ${module.functions}</p>
        <p><strong>Memory:</strong> ${module.memory || 'None'}</p>
        <p><strong>Imports:</strong> ${module.imports.join(', ') || 'None'}</p>
        <p><strong>Exports:</strong> ${module.exports.join(', ') || 'None'}</p>
    `;
}

function analyzeSecurityRisks(module) {
    const securityBox = document.getElementById('securityAnalysis');
    const risks = [];
    
    if (module.imports.length > 0) {
        risks.push({
            level: 'warning',
            message: 'Module imports external functions - verify syscall permissions'
        });
    }
    
    if (module.memory) {
        risks.push({
            level: 'info',
            message: 'Module uses linear memory - bounds checking enabled'
        });
    }
    
    if (module.functions > 10) {
        risks.push({
            level: 'info',
            message: `Large module with ${module.functions} functions - may require more resources`
        });
    }
    
    if (risks.length === 0) {
        risks.push({
            level: 'ok',
            message: 'No significant security risks detected'
        });
    }
    
    securityBox.innerHTML = risks.map(risk => `
        <div class="security-${risk.level}">
            <p>${risk.message}</p>
        </div>
    `).join('');
}

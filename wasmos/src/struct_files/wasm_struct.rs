use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize, Default, Debug)]
pub struct WasmMetrics {
    pub runs: u32,
    pub total_instructions: u64,
    pub total_syscalls: u64,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ExecutionRecord {
    pub timestamp: String,
    pub duration_us: u64,
    pub success: bool,
    pub error: Option<String>,
    pub instructions: u64,
    pub syscalls: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WasmFile //Struct for storing wasm file data
{
    pub name: String,
    pub path_to: String,
    //pub size: String,
    pub running: bool,
    pub metrics: WasmMetrics,
    pub execution_history: Vec<ExecutionRecord>,
}
//"Constructor"
impl WasmFile
{
    pub fn new_wasm(name:String, path_to: String) -> Self
    {
        Self
        {
            name,
            path_to,
            //size,
            running: false,
            metrics: WasmMetrics::default(),
            execution_history: Vec::new(),
        }
    }
}

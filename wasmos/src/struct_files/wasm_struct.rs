pub struct WasmFile //Struct for storing wasm file data
{
    pub name: String,
    pub path_to: String,
    //pub size: String,
    pub running: bool

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
            running: false
        }
    }
}
use std::{fs, path::Path};
pub fn wasm_engine(file_path: &Path) -> bool
{
    //execute wasm file.
    let wasm_binary:Vec<u8> = fs::read(file_path).expect("Wasm file could not be read");
    let magic_num: Vec<u8> = vec![0x00, 0x61, 0x73, 0x6D];
    let version: Vec<u8> = vec![0x01, 0x00, 0x00, 0x00];
    if wasm_binary.len() < 8
    {
        println!("Invalid file");
        return false;
    }
    if magic_num != wasm_binary[0..4] && version != wasm_binary[5..9]
    {
        println!("Invalid file");
        return false;
    }
    parser(wasm_binary);
    true

}
fn parser(wasm_binary: Vec<u8>)
{
    println!("To Parse"); //reached parse
    

}
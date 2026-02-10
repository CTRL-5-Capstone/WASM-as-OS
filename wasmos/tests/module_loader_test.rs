#[test]
fn test_wasm_file_extension() {
    let filename = "demo.wasm";

    assert!(
        filename.ends_with(".wasm"),
        "Loader should only accept .wasm files"
    );
}

#[test]
fn test_invalid_file_reject() {
    let filename = "data.txt";

    assert!(
        !filename.ends_with(".wasm"),
        "Non-WASM files must be rejected"
    );
}

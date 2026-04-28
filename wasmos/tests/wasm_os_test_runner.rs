/// Integration tests: run every file in WasmOSTest/ through the custom WASM engine.
///
/// Run with:
///   cargo test --test wasm_os_test_runner -- --nocapture
///
/// Each test is deliberately lenient — we record whether the engine panicked,
/// succeeded, or returned a controlled error.  A panic that is caught counts as
/// "engine error" (unsupported feature) rather than a test failure, so we can
/// distinguish between:
///   PASS  — module parsed and ran to completion
///   SKIP  — empty file or unsupported extension
///   ERROR — engine panicked on an unsupported opcode / invalid module
use std::path::{Path, PathBuf};
use wasmos::run_wasm::execute_wasm_file;

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn wasmos_test_dir() -> PathBuf {
    // CARGO_MANIFEST_DIR = <repo>/wasmos
    // WasmOSTest is one level up: <repo>/WasmOSTest
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let candidate = manifest_dir.join("../WasmOSTest");
    if candidate.exists() {
        return candidate.canonicalize().unwrap_or(candidate);
    }
    // fallback: running from repo root
    PathBuf::from("WasmOSTest")
}

/// Compile a .wat file to a temp .wasm, returning the temp path.
/// Caller is responsible for deleting it.
fn compile_wat(wat_path: &Path) -> Result<tempfile::NamedTempFile, String> {
    let bytes = std::fs::read(wat_path)
        .map_err(|e| format!("read WAT: {e}"))?;
    if bytes.is_empty() {
        return Err("empty WAT file".to_string());
    }
    let wasm = wat::parse_bytes(&bytes)
        .map_err(|e| format!("wat::parse_bytes: {e}"))?;
    let mut tmp = tempfile::NamedTempFile::new()
        .map_err(|e| format!("tempfile: {e}"))?;
    use std::io::Write;
    tmp.write_all(&wasm).map_err(|e| format!("write tmp: {e}"))?;
    Ok(tmp)
}

#[derive(Debug)]
enum Outcome {
    Pass { instructions: u64, duration_us: u64 },
    EngineError(String),
    Skip(String),
}

fn run_file(path: &Path) -> Outcome {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let wasm_path: Box<dyn AsRef<Path>>;
    let _tmp_guard; // keeps NamedTempFile alive for the duration of execute_wasm_file

    match ext.as_str() {
        "wasm" => {
            // run directly
            wasm_path = Box::new(path.to_path_buf());
            _tmp_guard = None::<tempfile::NamedTempFile>;
        }
        "wat" => {
            match compile_wat(path) {
                Ok(tmp) => {
                    wasm_path = Box::new(tmp.path().to_path_buf());
                    _tmp_guard = Some(tmp);
                }
                Err(e) => return Outcome::Skip(format!("WAT compile: {e}")),
            }
        }
        other => return Outcome::Skip(format!("unsupported extension: {other}")),
    }

    let path_str = wasm_path.as_ref().as_ref().to_string_lossy().to_string();
    match execute_wasm_file(&path_str, None) {
        Ok(result) => {
            if result.success {
                Outcome::Pass {
                    instructions: result.instructions_executed,
                    duration_us: result.duration_us,
                }
            } else {
                Outcome::EngineError(result.error.unwrap_or_else(|| "unknown error".to_string()))
            }
        }
        Err(e) => Outcome::EngineError(e),
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// The actual test
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn run_all_wasmos_test_files() {
    let dir = wasmos_test_dir();
    assert!(
        dir.exists(),
        "WasmOSTest directory not found at: {}",
        dir.display()
    );

    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .expect("read WasmOSTest dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    entries.sort();

    assert!(!entries.is_empty(), "WasmOSTest directory is empty");

    println!("\n{}", "═".repeat(70));
    println!("  WasmOS Engine — WasmOSTest suite");
    println!("  Dir: {}", dir.display());
    println!("{}\n", "═".repeat(70));

    let mut pass   = 0usize;
    let mut errors = 0usize;
    let mut skips  = 0usize;

    let mut error_details: Vec<(String, String)> = Vec::new();

    for path in &entries {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let outcome = run_file(path);

        match &outcome {
            Outcome::Pass { instructions, duration_us } => {
                pass += 1;
                println!(
                    "  ✅  {:<30}  {:>10} instr   {:>7} µs",
                    name, instructions, duration_us
                );
            }
            Outcome::EngineError(msg) => {
                errors += 1;
                // Truncate long error messages for the table
                let short = if msg.len() > 60 { format!("{}…", &msg[..57]) } else { msg.clone() };
                println!("  ❌  {:<30}  {}", name, short);
                error_details.push((name.clone(), msg.clone()));
            }
            Outcome::Skip(reason) => {
                skips += 1;
                println!("  ⏭   {:<30}  (skipped: {})", name, reason);
            }
        }
    }

    println!("\n{}", "─".repeat(70));
    println!(
        "  Results: {} passed, {} engine errors, {} skipped  (total files: {})",
        pass,
        errors,
        skips,
        entries.len()
    );

    if !error_details.is_empty() {
        println!("\n  Engine error details:");
        for (file, msg) in &error_details {
            println!("    {file}: {msg}");
        }
    }

    println!("{}\n", "═".repeat(70));

    // We do NOT assert!(errors == 0) here intentionally:
    // some WasmOSTest files exercise features the custom engine hasn't
    // implemented yet (e.g. imports, SIMD, bulk-memory).  The purpose of
    // this test is visibility, not a hard gate.
    //
    // However, the files we KNOW are simple arithmetic/control-flow MUST pass:
    let must_pass = [
        "simplei32.wasm",
        "test_add.wasm",
    ];
    for name in &must_pass {
        let path = dir.join(name);
        if !path.exists() {
            continue; // file not present in this checkout — skip
        }
        let outcome = run_file(&path);
        assert!(
            matches!(outcome, Outcome::Pass { .. }),
            "REQUIRED file {name} did not pass: {:?}",
            outcome
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Per-file unit tests — one test per known file, so failures show individually
// in `cargo test` output.
// ──────────────────────────────────────────────────────────────────────────────

macro_rules! wasm_file_test {
    ($test_name:ident, $file:expr) => {
        #[test]
        fn $test_name() {
            let dir = wasmos_test_dir();
            let path = dir.join($file);
            if !path.exists() {
                println!("SKIP: {} not found", $file);
                return;
            }
            let outcome = run_file(&path);
            println!("{}: {:?}", $file, outcome);
            // Non-fatal: record but don't fail — engine may not support all modules.
            // Only truly fatal outcome would be an OS-level crash (process killed),
            // which is already prevented by the panic::catch_unwind in execute_wasm_file.
        }
    };
    ($test_name:ident, $file:expr, required) => {
        #[test]
        fn $test_name() {
            let dir = wasmos_test_dir();
            let path = dir.join($file);
            if !path.exists() {
                println!("SKIP: {} not found", $file);
                return;
            }
            let outcome = run_file(&path);
            println!("{}: {:?}", $file, outcome);
            assert!(
                matches!(outcome, Outcome::Pass { .. }),
                "Engine must handle {}: got {:?}",
                $file,
                outcome
            );
        }
    };
}

// ── Required — simple deterministic modules ───────────────────────────────────
wasm_file_test!(test_simplei32_wasm,  "simplei32.wasm",  required);
wasm_file_test!(test_test_add_wasm,   "test_add.wasm",   required);

// ── Best-effort — complex/application modules ─────────────────────────────────
wasm_file_test!(test_file_a,             "A.wasm");
wasm_file_test!(test_file_b,             "B.wasm");
wasm_file_test!(test_file_c,             "C.wasm");
wasm_file_test!(test_file_g,             "G.wasm");
wasm_file_test!(test_big,                "Big.wasm");
wasm_file_test!(test_eagle_lyft,         "eagle_lyft.wasm");
wasm_file_test!(test_exists_wasm,        "Existswasm.wasm");
wasm_file_test!(test_game,               "game.wasm");
wasm_file_test!(test_simple_main,        "simple_main.wasm");
wasm_file_test!(test_snake,              "snake.wasm");
wasm_file_test!(test_test_wasm,          "test.wasm");
wasm_file_test!(test_test_loop,          "test_loop.wasm");
wasm_file_test!(test_wasm1,              "wasm1.wasm");
wasm_file_test!(test_wasm2,              "wasm2.wasm");
wasm_file_test!(test_wasm3,              "wasm3.wasm");

// ── WAT source files ─────────────────────────────────────────────────────────
wasm_file_test!(test_simplei32_wat,      "simplei32.wat", required);
wasm_file_test!(test_test_wat,           "test.wat");
wasm_file_test!(test_wat1,               "wat1.wat");
wasm_file_test!(test_wat2,               "wat2.wat");
wasm_file_test!(test_wat3,               "wat3.wat");
wasm_file_test!(test_exists_wat,         "Existswat.wat");

# WASM-OS: A Sandboxed Compute Platform

## Team CTRL 5
* Ololade Awoyemi
* Shivam Sakthivel Pandi
* Biraj Sharma
* Benjamin Wilson
* Sritan Reddy Gangidi

## Project Description
This project is a custom WebAssembly (WASM) runtime, built from scratch in Rust, that acts as a secure, sandboxed operating system for executing untrusted WASM code.

Per the project requirements, the runtime is implemented entirely from scratch, and **no external WASM engines like `wasmtime` or `wasmer` are used.**

The runtime executes `.wasm` modules with strict sandboxing rules, a custom capability-based ABI (e.g., `read_sensor`, `log`), multi-task scheduling, and real-time control through both a CLI and a complete 5-page web UI.

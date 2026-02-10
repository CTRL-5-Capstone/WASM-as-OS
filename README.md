# WASM-OS: A Sandboxed Compute Platform

## Team CTRL 5
* Ololade Awoyemi
* Benjamin Wilson
* Biraj Sharma
* Shivam Sakthivel Pandi
* Sritan Reddy Gangidi

## Project Description
This project is a custom WebAssembly (WASM) runtime, built from scratch in Rust, that acts as a secure, sandboxed operating system for executing untrusted WASM code.

Per the project requirements, the runtime is implemented entirely from scratch, and **no external WASM engines like `wasmtime` or `wasmer` are used.**

The runtime executes `.wasm` modules with strict sandboxing rules, a custom capability-based ABI (e.g., `read_sensor`, `log`), multi-task scheduling, and real-time control through both a CLI and a complete web UI.

## Architecture
The system consists of two main components running concurrently:
1.  **Rust Backend (`wasmos`)**:
    *   **Custom WASM Interpreter**: Parses and executes WASM binaries directly.
    *   **Actix Web Server**: Serves the frontend and provides REST APIs for task management.
    *   **CLI**: An interactive command-line interface for managing the system.
2.  **Web Frontend (`web`)**:
    *   A set of HTML/JS pages for monitoring and interacting with the system.

## Getting Started

### Prerequisites
*   [Rust](https://www.rust-lang.org/tools/install) (latest stable version)

### Running the Project
1.  Navigate to the backend directory:
    ```bash
    cd wasmos
    ```
2.  Run the application:
    ```bash
    cargo run
    ```
    This will start both the **Web Server** (background) and the **CLI Menu** (interactive).

## Usage Guide

### Command Line Interface (CLI)
Upon running the application, you will see the following menu:
*   **Load .Wasm File**: Register a new WASM file into the system.
*   **Remove .Wasm File**: Delete a registered file.
*   **Runtime Metrics**: View system performance stats.
*   **Start wasm**: Execute a loaded WASM module.
*   **Stop wasm**: Halt a running module.
*   **Prioritize Wasm's**: (Planned) Adjust scheduling priority.
*   **Save Machine State**: (Planned) Snapshot the system.
*   **Shutdown**: Gracefully stop the server and exit.

### Web Interface
Access the web dashboard at **[http://localhost:8080](http://localhost:8080)**.

*   **Dashboard**: Overview of system status.
*   **Tasks**: View, upload, and manage WASM tasks.
*   **Monitor**: Real-time metrics and logs.
*   **Execute**: Direct execution control.
*   **Inspect**: Analyze WASM binary structure.

## Project Structure
*   `wasmos/`: Rust source code.
    *   `src/run_wasm/`: Custom WASM interpreter and execution engine.
    *   `src/struct_files/`: Data structures for WASM files and lists.
    *   `src/server.rs`: Actix web server implementation.
    *   `src/main.rs`: Entry point, concurrency management, and CLI loop.
*   `web/`: Frontend assets (HTML, CSS, JS).

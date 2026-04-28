//! Syscall / import policy engine for the WASM runtime.
//!
//! The policy controls which host imports a WASM module is allowed to call.
//! Three built-in presets cover the most common use-cases; callers can also
//! supply explicit allow/block lists via `PolicyRequest` from the HTTP API.
//!
//! ## Usage (from build_runtime)
//!
//! let policy = SyscallPolicy::permissive();
//! // at import call site:
//! if let PolicyAction::Deny = policy.check("host_log") { /* block */ }
//! ```

use serde::{Deserialize, Serialize};

// ─── Violation record ────────────────────────────────────────────────────────

/// A single record of a blocked import call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallViolation {
    /// Name of the blocked import function.
    pub name: String,
    /// Module namespace (e.g. "env").
    pub module: String,
    /// Instruction counter at the time of the violation.
    pub instruction_index: u64,
    /// Human-readable explanation of why it was blocked.
    pub reason: String,
}

impl SyscallViolation {
    pub fn new(name: &str, module: &str, instruction_index: u64, reason: String) -> Self {
        Self {
            name: name.to_string(),
            module: module.to_string(),
            instruction_index,
            reason,
        }
    }
}

// ─── Policy action ───────────────────────────────────────────────────────────

/// The decision returned by `SyscallPolicy::check`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PolicyAction {
    /// Allow the import to proceed.
    Allow,
    /// Block the import and halt execution.
    Deny,
}

// ─── Policy preset ───────────────────────────────────────────────────────────

/// Named presets that select a pre-configured policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PolicyPreset {
    /// Allow all imports — useful for trusted/internal workloads.
    #[default]
    Permissive,
    /// Block every import not on the explicit allow list.
    Strict,
    /// Allow only the standard WasmOS ABI imports (host_log, read_sensor, send_alert).
    Sandbox,
}

// ─── Main policy struct ──────────────────────────────────────────────────────

/// Syscall policy applied during WASM execution.
///
/// Build with `SyscallPolicy::permissive()`, `SyscallPolicy::strict()`,
/// `SyscallPolicy::sandbox()`, or convert from a `PolicyRequest` received via
/// the HTTP API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyscallPolicy {
    /// Which named preset this policy was built from (informational).
    pub preset: PolicyPreset,
    /// Explicit allow-list (case-sensitive import names).  When non-empty,
    /// these names are always allowed regardless of preset.
    pub allowed: Vec<String>,
    /// Explicit block-list.  Names here are always denied regardless of preset.
    pub blocked: Vec<String>,
    /// Human-readable label shown in violation records and audit logs.
    pub label: String,
    /// The action taken for imports not matched by `allowed` or `blocked`.
    pub default_action: PolicyAction,
    /// How many violations to record before stopping (prevents log floods).
    pub max_violations: usize,
    /// Whether to log allowed-but-unknown import stubs to `stdout_log`.
    pub log_allowed: bool,
}

/// Standard WasmOS ABI imports that are always safe.
const STANDARD_ABI: &[&str] = &["host_log", "read_sensor", "send_alert"];

impl SyscallPolicy {
    // ─── Constructors ────────────────────────────────────────────────────────

    /// Allow every import — backward-compatible default.
    pub fn permissive() -> Self {
        Self {
            preset: PolicyPreset::Permissive,
            allowed: vec![],
            blocked: vec![],
            label: "permissive".to_string(),
            default_action: PolicyAction::Allow,
            max_violations: 100,
            log_allowed: false,
        }
    }

    /// Block every import unless explicitly in `allowed`.
    pub fn strict() -> Self {
        Self {
            preset: PolicyPreset::Strict,
            allowed: vec![],
            blocked: vec![],
            label: "strict".to_string(),
            default_action: PolicyAction::Deny,
            max_violations: 10,
            log_allowed: false,
        }
    }

    /// Only permit the standard WasmOS ABI imports.
    pub fn sandbox() -> Self {
        Self {
            preset: PolicyPreset::Sandbox,
            allowed: STANDARD_ABI.iter().map(|s| s.to_string()).collect(),
            blocked: vec![],
            label: "sandbox".to_string(),
            default_action: PolicyAction::Deny,
            max_violations: 10,
            log_allowed: true,
        }
    }

    // ─── Policy check ────────────────────────────────────────────────────────

    /// Decide whether `import_name` should be allowed or denied.
    ///
    /// Resolution order:
    /// 1. If the name is in `blocked`  → `Deny`
    /// 2. If the name is in `allowed`  → `Allow`
    /// 3. Fall back to `default_action`
    pub fn check(&self, import_name: &str) -> PolicyAction {
        let key = import_name.to_string();

        // Block-list wins over allow-list.
        if self.blocked.contains(&key) || self.blocked.iter().any(|b| b == import_name) {
            return PolicyAction::Deny;
        }

        if self.allowed.contains(&key) || self.allowed.iter().any(|a| a == import_name) {
            return PolicyAction::Allow;
        }

        self.default_action
    }

    /// Returns the policy's human-readable label.
    pub fn label(&self) -> &str {
        &self.label
    }
}

// ─── Default (required by build_runtime Deserialize derive) ─────────────────

impl Default for SyscallPolicy {
    fn default() -> Self {
        Self::permissive()
    }
}

// ─── HTTP API request ────────────────────────────────────────────────────────

/// JSON body sent by callers who want to override the default policy.
///
/// ```json
/// {
///   "preset": "sandbox",
///   "allowed": ["host_log"],
///   "blocked": ["dangerous_call"]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRequest {
    /// Named preset to start from.
    #[serde(default)]
    pub preset: PolicyPreset,
    /// Additional imports to explicitly allow (extends preset allow-list).
    #[serde(default)]
    pub allowed: Vec<String>,
    /// Additional imports to explicitly block (extends preset block-list).
    #[serde(default)]
    pub blocked: Vec<String>,
}

impl PolicyRequest {
    /// Convert this request into an executable `SyscallPolicy`.
    pub fn into_policy(self) -> SyscallPolicy {
        let mut base = match self.preset {
            PolicyPreset::Permissive => SyscallPolicy::permissive(),
            PolicyPreset::Strict => SyscallPolicy::strict(),
            PolicyPreset::Sandbox => SyscallPolicy::sandbox(),
        };
        // Merge caller overrides on top of the preset.
        base.allowed.extend(self.allowed);
        base.blocked.extend(self.blocked);
        base
    }
}


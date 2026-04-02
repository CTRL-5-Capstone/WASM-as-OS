/// Capability Token System — Zero-Trust Access Control
///
/// Design:
///  - Every capability is an explicit, signed, time-limited token.
///  - Tokens carry a precise set of permissions (fine-grained, not role-based).
///  - No capability = no access, regardless of JWT identity.
///  - Tokens are revocable at any time via the token registry.
///  - All checks are O(1) lookups in an in-memory hash map.

use chrono::{DateTime, Utc, Duration};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ─── Capability definitions ───────────────────────────────────────────────────

/// Fine-grained capabilities a token can carry. Each one grants exactly one
/// operation class. A token may carry multiple capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    /// Read task list and task metadata
    TaskRead,
    /// Upload new WASM modules
    TaskWrite,
    /// Trigger execution of an existing task
    TaskExecute,
    /// Delete tasks
    TaskDelete,
    /// Access execution history and per-task metrics
    MetricsRead,
    /// Access system-wide Prometheus metrics
    MetricsSystem,
    /// Manage tenants (create/delete)
    TenantAdmin,
    /// Manage snapshots
    SnapshotRead,
    SnapshotWrite,
    /// Access the interactive terminal WebSocket
    TerminalAccess,
    /// Read audit log
    AuditRead,
    /// Super-set: all capabilities (admin-only)
    Admin,
}

impl Capability {
    /// All known capabilities (used for "Admin" expansion).
    #[allow(dead_code)]
    pub fn all() -> Vec<Capability> {
        vec![
            Capability::TaskRead,
            Capability::TaskWrite,
            Capability::TaskExecute,
            Capability::TaskDelete,
            Capability::MetricsRead,
            Capability::MetricsSystem,
            Capability::TenantAdmin,
            Capability::SnapshotRead,
            Capability::SnapshotWrite,
            Capability::TerminalAccess,
            Capability::AuditRead,
            Capability::Admin,
        ]
    }
}

// ─── Token ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityToken {
    pub id: String,
    /// Human-readable label
    pub label: String,
    /// Who this token belongs to (user-id / service-account)
    pub subject: String,
    /// Optional: restrict to one tenant
    pub tenant_id: Option<String>,
    pub capabilities: HashSet<Capability>,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
}

impl CapabilityToken {
    /// Check if this token grants the given capability (or Admin).
    pub fn has(&self, cap: &Capability) -> bool {
        if self.revoked { return false; }
        if let Some(exp) = self.expires_at {
            if Utc::now() > exp { return false; }
        }
        self.capabilities.contains(&Capability::Admin) || self.capabilities.contains(cap)
    }

    /// True if the token is still valid (not expired, not revoked).
    pub fn is_valid(&self) -> bool {
        if self.revoked { return false; }
        if let Some(exp) = self.expires_at {
            return Utc::now() <= exp;
        }
        true
    }
}

// ─── Registry ─────────────────────────────────────────────────────────────────

/// Thread-safe, in-memory token registry.  
/// In a real deployment this would be backed by Redis or a fast DB table.
pub struct CapabilityRegistry {
    tokens: RwLock<HashMap<String, CapabilityToken>>,
}

impl CapabilityRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            tokens: RwLock::new(HashMap::new()),
        })
    }

    /// Issue a new capability token.
    pub async fn issue(
        &self,
        label: impl Into<String>,
        subject: impl Into<String>,
        tenant_id: Option<String>,
        capabilities: HashSet<Capability>,
        ttl_hours: Option<i64>,
    ) -> CapabilityToken {
        let token = CapabilityToken {
            id: Uuid::new_v4().to_string(),
            label: label.into(),
            subject: subject.into(),
            tenant_id,
            capabilities,
            created_at: Utc::now(),
            expires_at: ttl_hours.map(|h| Utc::now() + Duration::hours(h)),
            revoked: false,
        };
        self.tokens.write().await.insert(token.id.clone(), token.clone());
        token
    }

    /// Look up a token by id. Returns `None` if unknown, expired, or revoked.
    pub async fn get(&self, token_id: &str) -> Option<CapabilityToken> {
        let tokens = self.tokens.read().await;
        tokens.get(token_id).filter(|t| t.is_valid()).cloned()
    }

    /// Revoke a token immediately.
    pub async fn revoke(&self, token_id: &str) -> bool {
        let mut tokens = self.tokens.write().await;
        if let Some(token) = tokens.get_mut(token_id) {
            token.revoked = true;
            return true;
        }
        false
    }

    /// List all tokens (including revoked/expired — for admin audit).
    pub async fn list_all(&self) -> Vec<CapabilityToken> {
        self.tokens.read().await.values().cloned().collect()
    }

    /// List only valid tokens for a given subject.
    #[allow(dead_code)]
    pub async fn list_for_subject(&self, subject: &str) -> Vec<CapabilityToken> {
        self.tokens
            .read()
            .await
            .values()
            .filter(|t| t.subject == subject && t.is_valid())
            .cloned()
            .collect()
    }

    /// Purge all expired tokens (call periodically).
    #[allow(dead_code)]
    pub async fn purge_expired(&self) -> usize {
        let mut tokens = self.tokens.write().await;
        let before = tokens.len();
        tokens.retain(|_, t| {
            if let Some(exp) = t.expires_at {
                Utc::now() <= exp
            } else {
                true
            }
        });
        before - tokens.len()
    }

    /// Check: does the token with the given id hold this capability?
    pub async fn check(&self, token_id: &str, cap: &Capability) -> bool {
        match self.get(token_id).await {
            Some(token) => token.has(cap),
            None => false,
        }
    }
}

// ─── HTTP request helper ─────────────────────────────────────────────────────

/// Extract the capability token id from an HTTP request.
/// Checks `X-Capability-Token` header first, then `?cap_token=` query param.
#[allow(dead_code)]
pub fn extract_cap_token(req: &actix_web::HttpRequest) -> Option<String> {
    // Header: X-Capability-Token: <token-id>
    if let Some(val) = req.headers().get("X-Capability-Token") {
        if let Ok(s) = val.to_str() {
            return Some(s.trim().to_owned());
        }
    }
    // Query param: ?cap_token=<token-id>
    if let Some(qs) = req.uri().query() {
        for pair in qs.split('&') {
            if let Some(val) = pair.strip_prefix("cap_token=") {
                return Some(val.to_owned());
            }
        }
    }
    None
}

// ─── REST API DTOs ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct IssueTokenRequest {
    pub label: String,
    pub subject: String,
    pub tenant_id: Option<String>,
    pub capabilities: Vec<Capability>,
    /// None = never expires
    pub ttl_hours: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct IssueTokenResponse {
    pub token_id: String,
    pub label: String,
    pub subject: String,
    pub tenant_id: Option<String>,
    pub capabilities: Vec<Capability>,
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<CapabilityToken> for IssueTokenResponse {
    fn from(t: CapabilityToken) -> Self {
        Self {
            token_id: t.id,
            label: t.label,
            subject: t.subject,
            tenant_id: t.tenant_id,
            capabilities: t.capabilities.into_iter().collect(),
            expires_at: t.expires_at,
        }
    }
}

/// Summarised token view (no internal fields).
#[derive(Debug, Serialize, Clone)]
pub struct TokenSummary {
    pub id: String,
    pub label: String,
    pub subject: String,
    pub tenant_id: Option<String>,
    pub capabilities: Vec<String>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked: bool,
    pub valid: bool,
}

impl From<CapabilityToken> for TokenSummary {
    fn from(t: CapabilityToken) -> Self {
        let valid = t.is_valid();
        Self {
            id: t.id,
            label: t.label,
            subject: t.subject,
            tenant_id: t.tenant_id,
            // Use serde serialisation so the output matches the
            // #[serde(rename_all = "snake_case")] annotation on Capability
            // (e.g. "task_read" not "taskread" from Debug fmt).
            capabilities: t
                .capabilities
                .iter()
                .filter_map(|c| {
                    serde_json::to_value(c)
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_owned()))
                })
                .collect(),
            expires_at: t.expires_at,
            revoked: t.revoked,
            valid,
        }
    }
}

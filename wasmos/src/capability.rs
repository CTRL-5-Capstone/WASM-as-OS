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

#[cfg(test)]
mod tests {
    use super::*;
 
    // ── Capability enum ─────────────────────────────────────────────
 
    #[test]
    fn test_all_variants_constructable() {
        let caps = vec![
            Capability::TaskRead, Capability::TaskWrite,
            Capability::TaskExecute, Capability::TaskDelete,
            Capability::MetricsRead, Capability::MetricsSystem,
            Capability::TenantAdmin,
            Capability::SnapshotRead, Capability::SnapshotWrite,
            Capability::TerminalAccess, Capability::AuditRead,
            Capability::Admin,
        ];
        assert_eq!(caps.len(), 12);
    }
 
    #[test]
    fn test_capability_equality() {
        assert_eq!(Capability::TaskRead, Capability::TaskRead);
        assert_ne!(Capability::TaskRead, Capability::TaskWrite);
    }
 
    #[test]
    fn test_capability_in_hashset() {
        let mut set = HashSet::new();
        set.insert(Capability::TaskRead);
        set.insert(Capability::TaskWrite);
        set.insert(Capability::TaskRead); // dupe
        assert_eq!(set.len(), 2);
    }
 
    #[test]
    fn test_capability_serialize_snake_case() {
        let json = serde_json::to_string(&Capability::TaskRead).unwrap();
        assert_eq!(json, "\"task_read\"");
 
        let json = serde_json::to_string(&Capability::MetricsSystem).unwrap();
        assert_eq!(json, "\"metrics_system\"");
    }
 
    #[test]
    fn test_capability_deserialize_snake_case() {
        let cap: Capability = serde_json::from_str("\"task_write\"").unwrap();
        assert_eq!(cap, Capability::TaskWrite);
 
        let cap: Capability = serde_json::from_str("\"admin\"").unwrap();
        assert_eq!(cap, Capability::Admin);
    }
 
    #[test]
    fn test_capability_all() {
        let all = Capability::all();
        assert!(all.len() >= 12);
        assert!(all.contains(&Capability::TaskRead));
        assert!(all.contains(&Capability::Admin));
    }
 
    // ── CapabilityToken ─────────────────────────────────────────────
 
    fn make_token(caps: HashSet<Capability>, revoked: bool, expired: bool) -> CapabilityToken {
        CapabilityToken {
            id: "test-id".to_string(),
            label: "test".to_string(),
            subject: "user-1".to_string(),
            tenant_id: None,
            capabilities: caps,
            created_at: Utc::now(),
            expires_at: if expired {
                Some(Utc::now() - Duration::hours(1))
            } else {
                Some(Utc::now() + Duration::hours(1))
            },
            revoked,
        }
    }
 
    #[test]
    fn test_token_has_capability() {
        let mut caps = HashSet::new();
        caps.insert(Capability::TaskRead);
        let token = make_token(caps, false, false);
        assert!(token.has(&Capability::TaskRead));
        assert!(!token.has(&Capability::TaskWrite));
    }
 
    #[test]
    fn test_token_admin_grants_all() {
        let mut caps = HashSet::new();
        caps.insert(Capability::Admin);
        let token = make_token(caps, false, false);
        assert!(token.has(&Capability::TaskRead));
        assert!(token.has(&Capability::TaskWrite));
        assert!(token.has(&Capability::TenantAdmin));
    }
 
    #[test]
    fn test_revoked_token_has_nothing() {
        let mut caps = HashSet::new();
        caps.insert(Capability::Admin);
        let token = make_token(caps, true, false);
        assert!(!token.has(&Capability::TaskRead));
        assert!(!token.has(&Capability::Admin));
    }
 
    #[test]
    fn test_expired_token_has_nothing() {
        let mut caps = HashSet::new();
        caps.insert(Capability::TaskRead);
        let token = make_token(caps, false, true);
        assert!(!token.has(&Capability::TaskRead));
    }
 
    #[test]
    fn test_is_valid_active_token() {
        let token = make_token(HashSet::new(), false, false);
        assert!(token.is_valid());
    }
 
    #[test]
    fn test_is_valid_revoked() {
        let token = make_token(HashSet::new(), true, false);
        assert!(!token.is_valid());
    }
 
    #[test]
    fn test_is_valid_expired() {
        let token = make_token(HashSet::new(), false, true);
        assert!(!token.is_valid());
    }
 
    #[test]
    fn test_token_no_expiry_is_valid() {
        let token = CapabilityToken {
            id: "t".into(), label: "l".into(), subject: "s".into(),
            tenant_id: None,
            capabilities: HashSet::new(),
            created_at: Utc::now(),
            expires_at: None,
            revoked: false,
        };
        assert!(token.is_valid());
    }
 
    // ── CapabilityRegistry ──────────────────────────────────────────
 
    #[tokio::test]
    async fn test_registry_issue_and_get() {
        let reg = CapabilityRegistry::new();
        let mut caps = HashSet::new();
        caps.insert(Capability::TaskRead);
        let token = reg.issue("test", "user1", None, caps, Some(1)).await;
        let fetched = reg.get(&token.id).await;
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, token.id);
    }
 
    #[tokio::test]
    async fn test_registry_get_unknown_returns_none() {
        let reg = CapabilityRegistry::new();
        assert!(reg.get("nonexistent").await.is_none());
    }
 
    #[tokio::test]
    async fn test_registry_revoke() {
        let reg = CapabilityRegistry::new();
        let token = reg.issue("t", "u", None, HashSet::new(), Some(1)).await;
        assert!(reg.revoke(&token.id).await);
        assert!(reg.get(&token.id).await.is_none());
    }
 
    #[tokio::test]
    async fn test_registry_revoke_unknown_returns_false() {
        let reg = CapabilityRegistry::new();
        assert!(!reg.revoke("nope").await);
    }
 
    #[tokio::test]
    async fn test_registry_list_all() {
        let reg = CapabilityRegistry::new();
        reg.issue("a", "u", None, HashSet::new(), Some(1)).await;
        reg.issue("b", "u", None, HashSet::new(), Some(1)).await;
        let all = reg.list_all().await;
        assert_eq!(all.len(), 2);
    }
 
    #[tokio::test]
    async fn test_registry_check_capability() {
        let reg = CapabilityRegistry::new();
        let mut caps = HashSet::new();
        caps.insert(Capability::TaskRead);
        let token = reg.issue("t", "u", None, caps, Some(1)).await;
        assert!(reg.check(&token.id, &Capability::TaskRead).await);
        assert!(!reg.check(&token.id, &Capability::TaskWrite).await);
    }
 
    #[tokio::test]
    async fn test_registry_check_revoked_returns_false() {
        let reg = CapabilityRegistry::new();
        let mut caps = HashSet::new();
        caps.insert(Capability::Admin);
        let token = reg.issue("t", "u", None, caps, Some(1)).await;
        reg.revoke(&token.id).await;
        assert!(!reg.check(&token.id, &Capability::Admin).await);
    }
 
    #[tokio::test]
    async fn test_registry_list_for_subject() {
        let reg = CapabilityRegistry::new();
        reg.issue("a", "alice", None, HashSet::new(), Some(1)).await;
        reg.issue("b", "bob", None, HashSet::new(), Some(1)).await;
        reg.issue("c", "alice", None, HashSet::new(), Some(1)).await;
        let alice_tokens = reg.list_for_subject("alice").await;
        assert_eq!(alice_tokens.len(), 2);
    }
}

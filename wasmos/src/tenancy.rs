// tenancy.rs — in-memory tenant manager kept for legacy compatibility.
// All runtime quota enforcement is now DB-backed (see db/repository.rs and scheduler.rs).
// TenantManager is used in main.rs to initialise a default in-memory tenant at startup.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub quota: ResourceQuota,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    pub max_tasks: usize,
    pub max_memory_mb: usize,
    pub max_cpu_percent: u8,
    pub max_concurrent_executions: usize,
    pub max_wasm_size_mb: usize,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceUsage {
    pub task_count: usize,
    pub memory_mb: usize,
    pub cpu_percent: u8,
    pub concurrent_executions: usize,
}

pub struct TenantManager {
    tenants: HashMap<String, Tenant>,
    usage: HashMap<String, ResourceUsage>,
}

impl TenantManager {
    pub fn new() -> Self {
        Self {
            tenants: HashMap::new(),
            usage: HashMap::new(),
        }
    }

    pub fn create_tenant(&mut self, name: String, quota: ResourceQuota) -> Tenant {
        let tenant = Tenant {
            id: Uuid::new_v4().to_string(),
            name,
            quota,
            created_at: chrono::Utc::now(),
            active: true,
        };

        self.tenants.insert(tenant.id.clone(), tenant.clone());
        self.usage.insert(tenant.id.clone(), ResourceUsage::default());

        tenant
    }

    pub fn get_tenant(&self, tenant_id: &str) -> Option<&Tenant> {
        self.tenants.get(tenant_id)
    }

    pub fn can_create_task(&self, tenant_id: &str) -> bool {
        if let Some(tenant) = self.tenants.get(tenant_id) {
            if !tenant.active {
                return false;
            }

            let default_usage = ResourceUsage::default();
            let usage = self.usage.get(tenant_id).unwrap_or(&default_usage);
            usage.task_count < tenant.quota.max_tasks
        } else {
            false
        }
    }

    pub fn can_execute(&self, tenant_id: &str, estimated_memory_mb: usize) -> bool {
        if let Some(tenant) = self.tenants.get(tenant_id) {
            if !tenant.active {
                return false;
            }

            let default_usage = ResourceUsage::default();
            let usage = self.usage.get(tenant_id).unwrap_or(&default_usage);
            
            usage.concurrent_executions < tenant.quota.max_concurrent_executions &&
            usage.memory_mb + estimated_memory_mb <= tenant.quota.max_memory_mb
        } else {
            false
        }
    }

    pub fn increment_task_count(&mut self, tenant_id: &str) {
        if let Some(usage) = self.usage.get_mut(tenant_id) {
            usage.task_count += 1;
        }
    }

    pub fn decrement_task_count(&mut self, tenant_id: &str) {
        if let Some(usage) = self.usage.get_mut(tenant_id) {
            if usage.task_count > 0 {
                usage.task_count -= 1;
            }
        }
    }

    pub fn start_execution(&mut self, tenant_id: &str, memory_mb: usize) {
        if let Some(usage) = self.usage.get_mut(tenant_id) {
            usage.concurrent_executions += 1;
            usage.memory_mb += memory_mb;
        }
    }

    pub fn end_execution(&mut self, tenant_id: &str, memory_mb: usize) {
        if let Some(usage) = self.usage.get_mut(tenant_id) {
            if usage.concurrent_executions > 0 {
                usage.concurrent_executions -= 1;
            }
            if usage.memory_mb >= memory_mb {
                usage.memory_mb -= memory_mb;
            }
        }
    }

    pub fn get_usage(&self, tenant_id: &str) -> Option<&ResourceUsage> {
        self.usage.get(tenant_id)
    }

    pub fn deactivate_tenant(&mut self, tenant_id: &str) {
        if let Some(tenant) = self.tenants.get_mut(tenant_id) {
            tenant.active = false;
        }
    }

    pub fn activate_tenant(&mut self, tenant_id: &str) {
        if let Some(tenant) = self.tenants.get_mut(tenant_id) {
            tenant.active = true;
        }
    }

    pub fn list_tenants(&self) -> Vec<&Tenant> {
        self.tenants.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_quota(max_tasks: usize, max_memory_mb: usize, max_concurrent: usize) -> ResourceQuota {
        ResourceQuota {
            max_tasks,
            max_memory_mb,
            max_cpu_percent: 80,
            max_concurrent_executions: max_concurrent,
            max_wasm_size_mb: 50,
        }
    }

    // ── Tenant creation ────────────────────────────────────────────────────

    #[test]
    fn test_create_tenant() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Test Tenant".to_string(), make_quota(10, 1024, 5));
        assert_eq!(tenant.name, "Test Tenant");
        assert!(tenant.active);
    }

    #[test]
    fn test_unknown_tenant_cannot_create_task() {
        let manager = TenantManager::new();
        assert!(!manager.can_create_task("nonexistent-tenant-id"));
    }

    #[test]
    fn test_unknown_tenant_cannot_execute() {
        let manager = TenantManager::new();
        assert!(!manager.can_execute("nonexistent-tenant-id", 10));
    }

    // ── Task count quota ───────────────────────────────────────────────────

    #[test]
    fn test_task_count_quota_enforcement() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Test".to_string(), make_quota(2, 512, 1));

        assert!(manager.can_create_task(&tenant.id));
        manager.increment_task_count(&tenant.id);
        assert!(manager.can_create_task(&tenant.id));
        manager.increment_task_count(&tenant.id);
        assert!(!manager.can_create_task(&tenant.id), "quota exceeded: should block");
    }

    #[test]
    fn test_task_count_decrements_allow_new_tasks() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Test".to_string(), make_quota(1, 512, 1));

        manager.increment_task_count(&tenant.id);
        assert!(!manager.can_create_task(&tenant.id), "quota full");

        manager.decrement_task_count(&tenant.id);
        assert!(manager.can_create_task(&tenant.id), "after decrement should be allowed again");
    }

    #[test]
    fn test_task_count_never_underflows() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Test".to_string(), make_quota(5, 512, 1));

        // Decrement from zero — should not underflow
        manager.decrement_task_count(&tenant.id);
        manager.decrement_task_count(&tenant.id);
        let usage = manager.get_usage(&tenant.id).unwrap();
        assert_eq!(usage.task_count, 0);
    }

    // ── Concurrent execution quota ─────────────────────────────────────────

    #[test]
    fn test_concurrent_execution_quota_blocks_at_limit() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Exec".to_string(), make_quota(10, 512, 2));

        assert!(manager.can_execute(&tenant.id, 10));
        manager.start_execution(&tenant.id, 10);
        assert!(manager.can_execute(&tenant.id, 10));
        manager.start_execution(&tenant.id, 10);

        // At limit: concurrent = 2, max = 2
        assert!(!manager.can_execute(&tenant.id, 10), "should block at concurrent limit");
    }

    #[test]
    fn test_concurrent_execution_allows_after_end() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Exec".to_string(), make_quota(10, 512, 1));

        manager.start_execution(&tenant.id, 10);
        assert!(!manager.can_execute(&tenant.id, 10), "at limit");

        manager.end_execution(&tenant.id, 10);
        assert!(manager.can_execute(&tenant.id, 10), "slot freed");
    }

    // ── Memory quota ───────────────────────────────────────────────────────

    #[test]
    fn test_memory_quota_blocks_when_exceeded() {
        let mut manager = TenantManager::new();
        // max_memory_mb = 100, concurrent limit = 10 (not the bottleneck)
        let tenant = manager.create_tenant("Mem".to_string(), make_quota(10, 100, 10));

        manager.start_execution(&tenant.id, 80); // uses 80 MB
        // Requesting another 30 MB would push to 110 MB > 100 MB limit
        assert!(!manager.can_execute(&tenant.id, 30), "memory limit exceeded");
        // Requesting 20 MB is fine (80 + 20 = 100 MB exactly)
        assert!(manager.can_execute(&tenant.id, 20), "exactly at memory limit is allowed");
    }

    #[test]
    fn test_memory_usage_released_on_end() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Mem".to_string(), make_quota(10, 100, 10));

        manager.start_execution(&tenant.id, 90);
        assert!(!manager.can_execute(&tenant.id, 20), "memory full");

        manager.end_execution(&tenant.id, 90);
        assert!(manager.can_execute(&tenant.id, 20), "memory freed");
    }

    #[test]
    fn test_memory_never_underflows() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Mem".to_string(), make_quota(10, 100, 10));

        // end_execution called more than start_execution
        manager.end_execution(&tenant.id, 50);
        let usage = manager.get_usage(&tenant.id).unwrap();
        assert_eq!(usage.memory_mb, 0);
    }

    // ── Tenant activate / deactivate ───────────────────────────────────────

    #[test]
    fn test_inactive_tenant_cannot_create_task() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Inactive".to_string(), make_quota(10, 512, 5));

        manager.deactivate_tenant(&tenant.id);
        assert!(!manager.can_create_task(&tenant.id), "inactive tenant blocked");
    }

    #[test]
    fn test_inactive_tenant_cannot_execute() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Inactive".to_string(), make_quota(10, 512, 5));

        manager.deactivate_tenant(&tenant.id);
        assert!(!manager.can_execute(&tenant.id, 10), "inactive tenant blocked from execution");
    }

    #[test]
    fn test_reactivated_tenant_can_create_task() {
        let mut manager = TenantManager::new();
        let tenant = manager.create_tenant("Toggle".to_string(), make_quota(10, 512, 5));

        manager.deactivate_tenant(&tenant.id);
        assert!(!manager.can_create_task(&tenant.id));
        manager.activate_tenant(&tenant.id);
        assert!(manager.can_create_task(&tenant.id), "reactivated tenant should work");
    }

    // ── List and lookup ────────────────────────────────────────────────────

    #[test]
    fn test_list_tenants() {
        let mut manager = TenantManager::new();
        manager.create_tenant("A".to_string(), make_quota(5, 512, 2));
        manager.create_tenant("B".to_string(), make_quota(5, 512, 2));
        let tenants = manager.list_tenants();
        assert_eq!(tenants.len(), 2);
    }

    #[test]
    fn test_get_tenant_returns_correct_tenant() {
        let mut manager = TenantManager::new();
        let t = manager.create_tenant("FindMe".to_string(), make_quota(5, 512, 1));
        let found = manager.get_tenant(&t.id).expect("should find tenant");
        assert_eq!(found.name, "FindMe");
    }
}

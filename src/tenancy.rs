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

            let usage = self.usage.get(tenant_id).unwrap_or(&ResourceUsage::default());
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

            let usage = self.usage.get(tenant_id).unwrap_or(&ResourceUsage::default());
            
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

    #[test]
    fn test_create_tenant() {
        let mut manager = TenantManager::new();
        let quota = ResourceQuota {
            max_tasks: 10,
            max_memory_mb: 1024,
            max_cpu_percent: 80,
            max_concurrent_executions: 5,
            max_wasm_size_mb: 50,
        };

        let tenant = manager.create_tenant("Test Tenant".to_string(), quota);
        assert_eq!(tenant.name, "Test Tenant");
        assert!(tenant.active);
    }

    #[test]
    fn test_quota_enforcement() {
        let mut manager = TenantManager::new();
        let quota = ResourceQuota {
            max_tasks: 2,
            max_memory_mb: 512,
            max_cpu_percent: 80,
            max_concurrent_executions: 1,
            max_wasm_size_mb: 50,
        };

        let tenant = manager.create_tenant("Test".to_string(), quota);
        
        assert!(manager.can_create_task(&tenant.id));
        manager.increment_task_count(&tenant.id);
        assert!(manager.can_create_task(&tenant.id));
        manager.increment_task_count(&tenant.id);
        assert!(!manager.can_create_task(&tenant.id)); // Quota exceeded
    }
}

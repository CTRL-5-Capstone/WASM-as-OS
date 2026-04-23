use serde::Deserialize;
use std::net::IpAddr;

/// Mask the password in a postgres connection URL to prevent credential leakage in logs.
fn mask_db_url(url: &str) -> String {
    // Pattern: postgresql://user:PASSWORD@host/db → postgresql://user:****@host/db
    if let Some(at_pos) = url.find('@') {
        if let Some(colon_pos) = url[..at_pos].rfind(':') {
            let prefix = &url[..colon_pos + 1];
            let suffix = &url[at_pos..];
            return format!("{}****{}", prefix, suffix);
        }
    }
    url.to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub security: SecurityConfig,
    pub limits: ResourceLimits,
    pub logging: LoggingConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    #[serde(default = "default_host")]
    pub host: IpAddr,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_workers")]
    pub workers: usize,
    #[serde(default = "default_cors_origins")]
    pub cors_origins: Vec<String>,
}

#[derive(Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url")]
    pub url: String,
}

impl std::fmt::Debug for DatabaseConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Mask password in the URL to prevent credential leakage in logs.
        let masked = mask_db_url(&self.url);
        f.debug_struct("DatabaseConfig").field("url", &masked).finish()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    #[serde(default = "default_auth_enabled")]
    pub auth_enabled: bool,
    #[serde(default)]
    pub jwt_secret: String,
    #[serde(default = "default_jwt_expiry")]
    pub jwt_expiry_hours: i64,
    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,
    /// Admin key required to issue JWT tokens via /v1/auth/token
    #[serde(default = "default_admin_key")]
    pub admin_key: String,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct ResourceLimits {
    #[serde(default = "default_max_wasm_size")]
    pub max_wasm_size_mb: usize,
    #[serde(default = "default_max_memory")]
    pub max_memory_mb: usize,
    #[serde(default = "default_execution_timeout")]
    pub execution_timeout_secs: u64,
    #[serde(default = "default_max_instructions")]
    pub max_instructions: u64,
    #[serde(default = "default_max_stack_depth")]
    pub max_stack_depth: usize,
    #[serde(default = "default_max_concurrent_tasks")]
    pub max_concurrent_tasks: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_log_format")]
    pub format: String,
}

// Default values
fn default_host() -> IpAddr {
    // Bind to all interfaces by default so Docker/K8s and the Next.js frontend can reach the backend.
    // Override via WASMOS__SERVER__HOST=127.0.0.1 for local-only deployments.
    "0.0.0.0".parse().unwrap()
}

fn default_port() -> u16 {
    8080
}

fn default_workers() -> usize {
    num_cpus::get()
}

fn default_cors_origins() -> Vec<String> {
    vec!["*".to_string()]
}

fn default_database_url() -> String {
    "postgresql://postgres:postgres@localhost:5432/wasmos".to_string()
}

fn default_admin_key() -> String {
    "changeme".to_string()
}

fn default_auth_enabled() -> bool {
    false
}

fn default_jwt_expiry() -> i64 {
    24
}

fn default_rate_limit() -> u32 {
    60
}

fn default_max_wasm_size() -> usize {
    50
}

fn default_max_memory() -> usize {
    128
}

fn default_execution_timeout() -> u64 {
    30
}

fn default_max_instructions() -> u64 {
    10_000_000
}

fn default_max_stack_depth() -> usize {
    1024
}

fn default_max_concurrent_tasks() -> usize {
    100
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "json".to_string()
}

impl Config {
    pub fn load() -> Result<Self, config::ConfigError> {
        // Load .env file if it exists
        dotenvy::dotenv().ok();

        let config = config::Config::builder()
            // Start with default values
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.port", 8080)?
            .set_default("server.workers", num_cpus::get() as i64)?
            .set_default("server.cors_origins", vec!["*"])?
            .set_default("database.url", "postgresql://postgres:postgres@localhost:5432/wasmos")?
            .set_default("security.auth_enabled", false)?
            .set_default("security.jwt_secret", "")?
            .set_default("security.jwt_expiry_hours", 24)?
            .set_default("security.rate_limit_per_minute", 60)?
            .set_default("limits.max_wasm_size_mb", 50)?
            .set_default("limits.max_memory_mb", 128)?
            .set_default("limits.execution_timeout_secs", 30)?
            .set_default("limits.max_instructions", 10_000_000)?
            .set_default("limits.max_stack_depth", 1024)?
            .set_default("limits.max_concurrent_tasks", 100)?
            .set_default("logging.level", "info")?
            .set_default("logging.format", "json")?
            // Override with environment variables (with WASMOS_ prefix)
            .add_source(config::Environment::with_prefix("WASMOS").separator("__"))
            .build()?;

        let cfg: Config = config.try_deserialize()?;

        // Detect production mode: WASMOS_ENV=production or RUST_ENV=production
        let is_production = std::env::var("WASMOS_ENV")
            .or_else(|_| std::env::var("RUST_ENV"))
            .map(|v| v.eq_ignore_ascii_case("production"))
            .unwrap_or(false);

        let weak_jwt = cfg.security.jwt_secret.is_empty()
            || cfg.security.jwt_secret == "change-me-in-production-jwt-secret"
            || cfg.security.jwt_secret == "change-me-to-a-random-32-char-secret"
            || cfg.security.jwt_secret.len() < 32;

        let weak_admin = cfg.security.admin_key == "changeme"
            || cfg.security.admin_key.is_empty();

        if is_production {
            // Hard fail in production — insecure defaults must not reach prod.
            if weak_jwt {
                return Err(config::ConfigError::Message(
                    "WASMOS_ENV=production but WASMOS__SECURITY__JWT_SECRET is absent or \
                     too short (min 32 chars). Set a strong random secret.".into(),
                ));
            }
            if weak_admin {
                return Err(config::ConfigError::Message(
                    "WASMOS_ENV=production but WASMOS__SECURITY__ADMIN_KEY is absent or \
                     set to the default 'changeme'. Set a strong random key.".into(),
                ));
            }
            if !cfg.security.auth_enabled {
                eprintln!(
                    "[SECURITY WARNING] WASMOS_ENV=production but \
                     WASMOS__SECURITY__AUTH_ENABLED=false. All API endpoints are UNPROTECTED. \
                     Set WASMOS__SECURITY__AUTH_ENABLED=true in production."
                );
            }
            if cfg.server.cors_origins.contains(&"*".to_string()) {
                eprintln!(
                    "[SECURITY WARNING] WASMOS_ENV=production but \
                     WASMOS__SERVER__CORS_ORIGINS is \"*\" (allow all origins). \
                     Restrict to specific origins in production."
                );
            }
        } else {
            // Dev/CI: emit loud warnings but allow startup so developers aren't blocked.
            if weak_admin {
                eprintln!(
                    "[SECURITY WARNING] WASMOS__SECURITY__ADMIN_KEY is set to the default \
                     \"changeme\". Set it to a strong secret before deploying to production."
                );
            }
            if weak_jwt {
                eprintln!(
                    "[SECURITY WARNING] WASMOS__SECURITY__JWT_SECRET is not set or is insecure \
                     (< 32 chars). Set it to a long random secret before deploying to production."
                );
            }
            if cfg.server.cors_origins.contains(&"*".to_string()) {
                eprintln!(
                    "[SECURITY WARNING] WASMOS__SERVER__CORS_ORIGINS is \"*\" (allow all). \
                     Restrict to specific origins in production."
                );
            }
        }

        Ok(cfg)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
 
    // We can't test Config::load() directly because it reads env/files,
    // but we can test all the default functions and struct deserialization.
 
    #[test]
    fn test_default_host() {
        let host = default_host();
        assert_eq!(host.to_string(), "0.0.0.0");
    }
 
    #[test]
    fn test_default_port() {
        assert_eq!(default_port(), 8080);
    }
 
    #[test]
    fn test_default_workers_nonzero() {
        assert!(default_workers() > 0);
    }
 
    #[test]
    fn test_default_cors_origins() {
        let origins = default_cors_origins();
        assert_eq!(origins, vec!["*"]);
    }
 
    #[test]
    fn test_default_database_url() {
        let url = default_database_url();
        assert!(url.starts_with("postgresql://"));
    }
 
    #[test]
    fn test_default_admin_key() {
        assert_eq!(default_admin_key(), "changeme");
    }
 
    #[test]
    fn test_default_auth_disabled() {
        assert!(!default_auth_enabled());
    }
 
    #[test]
    fn test_default_jwt_expiry() {
        assert_eq!(default_jwt_expiry(), 24);
    }
 
    #[test]
    fn test_default_rate_limit() {
        assert_eq!(default_rate_limit(), 60);
    }
 
    #[test]
    fn test_default_max_wasm_size() {
        assert_eq!(default_max_wasm_size(), 50);
    }
 
    #[test]
    fn test_default_max_memory() {
        assert_eq!(default_max_memory(), 128);
    }
 
    #[test]
    fn test_default_execution_timeout() {
        assert_eq!(default_execution_timeout(), 30);
    }
 
    #[test]
    fn test_default_max_instructions() {
        assert_eq!(default_max_instructions(), 10_000_000);
    }
 
    #[test]
    fn test_default_max_stack_depth() {
        assert_eq!(default_max_stack_depth(), 1024);
    }
 
    #[test]
    fn test_default_max_concurrent_tasks() {
        assert_eq!(default_max_concurrent_tasks(), 100);
    }
 
    #[test]
    fn test_default_log_level() {
        assert_eq!(default_log_level(), "info");
    }
 
    #[test]
    fn test_default_log_format() {
        assert_eq!(default_log_format(), "json");
    }
}
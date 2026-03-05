use serde::Deserialize;
use std::net::IpAddr;

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

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    #[serde(default = "default_database_url")]
    pub url: String,
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
}

#[derive(Debug, Deserialize, Clone)]
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
    "127.0.0.1".parse().unwrap()
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
    "postgresql://postgres:postgres@localhost:5432/wasm_os".to_string()
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
            .set_default("server.host", "127.0.0.1")?
            .set_default("server.port", 8080)?
            .set_default("server.workers", num_cpus::get() as i64)?
            .set_default("server.cors_origins", vec!["*"])?
            .set_default("database.url", "postgresql://postgres:postgres@localhost:5432/wasm_os")?
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

        config.try_deserialize()
    }
}

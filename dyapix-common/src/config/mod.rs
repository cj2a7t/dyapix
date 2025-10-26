use anyhow::{anyhow, Context, Result};
use config::{Config, Environment, File};
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

static CONFIG: OnceCell<Arc<AppConfig>> = OnceCell::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSection,
    pub server: ServerSection,
    pub log: LogSection,
    pub data_source: DataSourceSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSection {
    pub data_source: String, // e.g., "mysql" or "etcd"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerSection {
    #[serde(default = "default_proxy_host")]
    pub proxy_host: String,
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogSection {
    pub log_level: String,
    pub log_dir: String,
    pub log_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceSection {
    pub mysql: Option<MySQLConfig>,
    pub etcd: Option<EtcdConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MySQLConfig {
    pub url: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_min_connections")]
    pub min_connections: u32,
    #[serde(default = "default_acquire_timeout_secs")]
    pub acquire_timeout_secs: u64,
    #[serde(default = "default_idle_timeout_secs")]
    pub idle_timeout_secs: u64,
    #[serde(default = "default_max_lifetime_secs")]
    pub max_lifetime_secs: u64,
    #[serde(default = "default_test_before_acquire")]
    pub test_before_acquire: bool,
}

fn default_max_connections() -> u32 {
    50
}

fn default_min_connections() -> u32 {
    5
}

fn default_acquire_timeout_secs() -> u64 {
    30
}

fn default_idle_timeout_secs() -> u64 {
    600 // 10 minutes
}

fn default_max_lifetime_secs() -> u64 {
    1800 // 30 minutes
}

fn default_test_before_acquire() -> bool {
    true
}

fn default_proxy_host() -> String {
    "0.0.0.0".to_string()
}

fn default_proxy_port() -> u16 {
    8080
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EtcdConfig {
    pub endpoints: Vec<String>,
    pub username: String,
    pub password: String,
}

impl AppConfig {
    /// Load configuration from environment variables
    ///
    /// Supported environment variables:
    /// - CONFIG_DIR: Configuration directory, defaults to "config"
    /// - RUN_MODE: Run mode (dev/prod/test), defaults to "dev"
    /// - APP__*: Environment variables with double underscore separator to override config
    ///
    /// Configuration loading priority (later overrides former):
    /// 1. config/default.toml
    /// 2. config/{RUN_MODE}.toml (optional)
    /// 3. Environment variables APP__*
    pub fn load_from_env() -> Result<Self> {
        let config_dir = std::env::var("CONFIG_DIR").unwrap_or_else(|_| "config".into());
        let run_mode = std::env::var("RUN_MODE").unwrap_or_else(|_| "dev".into());

        let builder = Config::builder()
            .add_source(File::with_name(&format!("{}/default", config_dir)))
            .add_source(File::with_name(&format!("{}/{}", config_dir, run_mode)).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"));

        builder
            .build()
            .context("Failed to build config")?
            .try_deserialize()
            .context("Failed to deserialize config")
    }
}

/// Initialize application configuration
///
/// This function loads the configuration from environment variables and
/// initializes the global configuration singleton.
///
/// # Errors
///
/// Returns an error if:
/// - The configuration files cannot be loaded
/// - The configuration has already been initialized
pub fn init_app_config() -> Result<()> {
    let config = Arc::new(AppConfig::load_from_env()?);
    CONFIG
        .set(config)
        .map_err(|_| anyhow!("AppConfig already initialized"))?;
    Ok(())
}

/// Get the application configuration
///
/// This function returns the global application configuration.
/// If the configuration has not been initialized yet, it will
/// automatically load it from environment variables.
///
/// # Errors
///
/// Returns an error if the configuration cannot be loaded.
pub fn get_app_config() -> Result<Arc<AppConfig>> {
    CONFIG
        .get_or_try_init(|| {
            let config = AppConfig::load_from_env()?;
            Ok(Arc::new(config))
        })
        .cloned()
}

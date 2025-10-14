use anyhow::{anyhow, Result};
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use tokio::sync::OnceCell;

use crate::config::get_app_config;

static MYSQL_POOL: OnceCell<MySqlPool> = OnceCell::const_new();

/// Get or initialize MySQL connection pool.
/// Reads database configuration from application config file.
pub async fn get_mysql_pool() -> Result<&'static MySqlPool> {
    MYSQL_POOL
        .get_or_try_init(|| async {
            let config = get_app_config()?;
            let mysql_config = config
                .data_source
                .mysql
                .as_ref()
                .ok_or_else(|| anyhow!("MySQL configuration not found in config file"))?;

            let database_url = &mysql_config.url;
            tracing::info!(
                "Initializing MySQL connection pool with config: max_connections={}, min_connections={}, acquire_timeout={}s, idle_timeout={}s, max_lifetime={}s",
                mysql_config.max_connections,
                mysql_config.min_connections,
                mysql_config.acquire_timeout_secs,
                mysql_config.idle_timeout_secs,
                mysql_config.max_lifetime_secs
            );

            let pool = MySqlPoolOptions::new()
                .max_connections(mysql_config.max_connections)
                .min_connections(mysql_config.min_connections)
                .acquire_timeout(std::time::Duration::from_secs(mysql_config.acquire_timeout_secs))
                .idle_timeout(Some(std::time::Duration::from_secs(mysql_config.idle_timeout_secs)))
                .max_lifetime(Some(std::time::Duration::from_secs(mysql_config.max_lifetime_secs)))
                .test_before_acquire(mysql_config.test_before_acquire)
                .connect(database_url)
                .await
                .map_err(|e| anyhow!("Failed to connect to MySQL: {}", e))?;

            tracing::info!(
                "MySQL pool initialized successfully (max: {}, min: {})",
                pool.options().get_max_connections(),
                pool.options().get_min_connections()
            );

            Ok(pool)
        })
        .await
}


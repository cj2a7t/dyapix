use anyhow::{anyhow, Result};
use async_trait::async_trait;

use crate::config::get_app_config;
use crate::cro::CRO;
use crate::datasource::interface::DataSource;
use crate::datasource::mysql::MysqlDataSource;

/// Dynamic datasource that wraps different datasource implementations
pub enum DynamicDataSource {
    Mysql(MysqlDataSource),
    // Etcd(EtcdDataSource),  // Future support
    // Redis(RedisDataSource), // Future support
}

impl DynamicDataSource {
    /// Create a datasource instance based on configuration
    ///
    /// # Example
    /// ```no_run
    /// use dyapix_common::datasource::factory::DynamicDataSource;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     // Automatically selects datasource based on config
    ///     let datasource = DynamicDataSource::from_config().await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn from_config() -> Result<Self> {
        let config = get_app_config()?;
        
        let datasource_type = &config.app.data_source;
        
        match datasource_type.to_lowercase().as_str() {
            "mysql" => {
                // Validate MySQL config exists
                if config.data_source.mysql.is_none() {
                    return Err(anyhow!(
                        "MySQL datasource selected but no MySQL configuration found"
                    ));
                }
                
                tracing::info!("Initializing MySQL datasource from config");
                Ok(DynamicDataSource::Mysql(MysqlDataSource))
            }
            "etcd" => {
                // Future implementation
                Err(anyhow!(
                    "Etcd datasource is not yet implemented. Use 'mysql' for now."
                ))
            }
            other => Err(anyhow!(
                "Unsupported datasource type: '{}'. Supported types: mysql, etcd",
                other
            )),
        }
    }
    
    /// Get the datasource type as string
    pub fn datasource_type(&self) -> &'static str {
        match self {
            DynamicDataSource::Mysql(_) => "mysql",
            // DynamicDataSource::Etcd(_) => "etcd",
        }
    }
}

#[async_trait]
impl DataSource for DynamicDataSource {
    async fn fetch_and_watch(&self) -> Result<()> {
        match self {
            DynamicDataSource::Mysql(ds) => ds.fetch_and_watch().await,
            // DynamicDataSource::Etcd(ds) => ds.fetch_and_watch().await,
        }
    }

    async fn put<T>(&self, resource: &T) -> Result<T>
    where
        T: CRO,
    {
        match self {
            DynamicDataSource::Mysql(ds) => ds.put(resource).await,
            // DynamicDataSource::Etcd(ds) => ds.put(resource).await,
        }
    }

    async fn get<T>(&self, id: &str) -> Result<T>
    where
        T: CRO,
    {
        match self {
            DynamicDataSource::Mysql(ds) => ds.get(id).await,
            // DynamicDataSource::Etcd(ds) => ds.get(id).await,
        }
    }

    async fn delete<T>(&self, id: &str) -> Result<bool>
    where
        T: CRO,
    {
        match self {
            DynamicDataSource::Mysql(ds) => ds.delete::<T>(id).await,
            // DynamicDataSource::Etcd(ds) => ds.delete::<T>(id).await,
        }
    }

    async fn get_all<T>(&self) -> Result<Vec<T>>
    where
        T: CRO,
    {
        match self {
            DynamicDataSource::Mysql(ds) => ds.get_all().await,
            // DynamicDataSource::Etcd(ds) => ds.get_all().await,
        }
    }
}

/// Global datasource instance
static GLOBAL_DATASOURCE: tokio::sync::OnceCell<DynamicDataSource> =
    tokio::sync::OnceCell::const_new();

/// Get or initialize the global datasource instance
///
/// This function ensures only one datasource instance exists across the application.
///
/// # Example
/// ```no_run
/// use dyapix_common::datasource::get_datasource;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     let datasource = get_datasource().await?;
///     // Use datasource...
///     Ok(())
/// }
/// ```
pub async fn get_datasource() -> Result<&'static DynamicDataSource> {
    GLOBAL_DATASOURCE
        .get_or_try_init(|| async {
            let ds = DynamicDataSource::from_config().await?;
            tracing::info!("Global datasource initialized: {}", ds.datasource_type());
            Ok(ds)
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datasource_type() {
        let mysql_ds = DynamicDataSource::Mysql(MysqlDataSource);
        assert_eq!(mysql_ds.datasource_type(), "mysql");
    }
}


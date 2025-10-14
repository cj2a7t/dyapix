mod crud;
mod pool;
mod route;
mod types;
mod upstream;
mod watcher;

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;

use crate::datasource::interface::DataSource;

// Re-export commonly used items
pub use pool::get_mysql_pool;
pub use types::DyapixDs;

/// MySQL implementation of DataSource trait
pub struct MysqlDataSource;

impl MysqlDataSource {
    /// Dispatch database record to appropriate cache based on ds_type
    async fn insert_into_cache(&self, record: &DyapixDs) -> bool {
        match record.ds_type.as_str() {
            "Route" => route::insert_route_into_cache(record).await,
            "Upstream" => upstream::insert_upstream_into_cache(record).await,
            _ => {
                tracing::error!(
                    "Unknown ds_type: {} for record id = {}",
                    record.ds_type,
                    record.id
                );
                false
            }
        }
    }
}

#[async_trait]
impl DataSource for MysqlDataSource {
    async fn fetch_and_watch(self: Arc<Self>) -> Result<()> {
        // Initial full load
        self.initial_load().await?;
        
        // Watch for pending records
        self.watch_pending().await
    }

    async fn put<T>(self: Arc<Self>, id: &str, value: &T) -> Result<T>
    where
        T: serde::Serialize + Clone + Send + Sync + 'static,
    {
        self.put(id, value).await
    }

    async fn get<T>(self: Arc<Self>, id: &str) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        self.get(id).await
    }

    async fn delete(self: Arc<Self>, id: &str) -> Result<bool> {
        self.delete(id).await
    }

    async fn get_all<T>(self: Arc<Self>) -> Result<Vec<T>>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        self.get_all().await
    }
}

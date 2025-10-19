mod crud;
mod extension;
mod handler;
mod health;
mod pool;
mod types;
mod watcher;

use anyhow::Result;
use async_trait::async_trait;

use crate::datasource::interface::DataSource;
use crate::cro::CRO;

// Re-export commonly used items
pub use handler::{CROEntity, CROHandler, CROHandlerRegistry};
pub use health::{DataSourceStats, HealthStatus};
pub use pool::get_mysql_pool;
pub use types::DyapixDs;
pub use watcher::{init_shutdown_channel, trigger_shutdown};

/// MySQL implementation of DataSource trait
pub struct MysqlDataSource;

impl MysqlDataSource {
    /// Check health status of the datasource
    pub async fn health_check() -> HealthStatus {
        HealthStatus::check().await
    }

    /// Get datasource statistics
    pub async fn get_stats() -> Result<DataSourceStats> {
        HealthStatus::get_statistics().await
    }

    /// Dispatch database record to appropriate cache based on entity type
    async fn insert_into_cache(&self, record: &DyapixDs) -> bool {
        // Get the handler for this entity type from the registry
        let handler = match CROHandlerRegistry::global().get(&record.ds_type) {
            Some(h) => h,
            None => {
                tracing::error!(
                    "No handler registered for entity type: {}",
                    record.ds_type
                );
                return false;
            }
        };

        // Parse the entity from the record
        let entity = match handler.parse_entity(&record.ds_json) {
            Ok(e) => e,
            Err(e) => {
                tracing::error!(
                    "Failed to parse entity for record id = {}: {}",
                    record.id,
                    e
                );
                return false;
            }
        };

        // Parse the previous entity if it exists (for updates)
        let prev_entity = if let Some(ref prev_json) = record.prev_ds_json {
            match handler.parse_entity(prev_json) {
                Ok(e) => Some(e),
                Err(e) => {
                    tracing::error!(
                        "Failed to parse previous entity for record id = {}: {}",
                        record.id,
                        e
                    );
                    return false;
                }
            }
        } else {
            None
        };

        // Insert into cache using the handler
        handler
            .insert_into_cache(&record.operation_type, entity, prev_entity)
            .await
    }
}

#[async_trait]
impl DataSource for MysqlDataSource {
    async fn fetch_and_watch(&self) -> Result<()> {
        // Initial full load
        self.initial_load().await?;

        // Watch for pending records
        self.watch_pending().await
    }

    async fn put<T>(&self, resource: &T) -> Result<T>
    where
        T: CRO,
    {
        self.put_internal(resource).await
    }

    async fn get<T>(&self, id: &str) -> Result<T>
    where
        T: CRO,
    {
        self.get_internal(id).await
    }

    async fn delete<T>(&self, id: &str) -> Result<bool>
    where
        T: CRO,
    {
        self.delete_internal::<T>(id).await
    }

    async fn get_all<T>(&self) -> Result<Vec<T>>
    where
        T: CRO,
    {
        self.get_all_internal().await
    }
}

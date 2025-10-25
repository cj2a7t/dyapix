mod crud;
mod extension;
mod handler;
mod health;
mod pool;
mod types;
mod watcher;

use anyhow::Result;
use async_trait::async_trait;

use crate::cro::CRO;
use crate::datasource::interface::DataSource;

// Re-export commonly used items
pub use handler::{CROEntity, CROHandler, CROHandlerRegistry};
pub use health::{DataSourceStats, HealthStatus};
pub use pool::get_mysql_pool;
pub use types::DyapixDs;

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
                tracing::error!("No handler registered for entity type: {}", record.ds_type);
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
    /// Perform initial full load of all MySQL datasource records
    ///
    /// This implementation loads all records from the `dyapix_ds` table using
    /// cursor-based pagination for better performance with large datasets.
    /// Records are processed in batches and inserted into the appropriate cache
    /// based on their entity type.
    ///
    /// # Process
    ///
    /// 1. Fetches records in batches of 100 using cursor-based pagination
    /// 2. For each record, determines the appropriate handler based on `ds_type`
    /// 3. Parses the JSON data into the appropriate entity type
    /// 4. Inserts the entity into the corresponding cache
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all records are successfully loaded, or an error if
    /// there are database connection issues or data parsing failures.
    async fn full_load(&self) -> Result<()> {
        self.initial_load().await
    }

    /// Load incremental changes from MySQL datasource
    ///
    /// This implementation processes only records with status 'pending' from the
    /// `dyapix_ds` table. It handles the complete lifecycle of processing pending
    /// records including status updates and error handling.
    ///
    /// # Process
    ///
    /// 1. Fetches all records with `ds_status = 'pending'`
    /// 2. Marks them as 'syncing' to prevent duplicate processing
    /// 3. Processes each record and updates the cache
    /// 4. Updates record status to 'synced' on success or back to 'pending' on failure
    ///
    /// # Error Handling
    ///
    /// - Database connection failures are retried with exponential backoff
    /// - Individual record processing failures don't stop the batch
    /// - Failed records are reset to 'pending' status for retry
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the incremental load completes successfully, or an error
    /// if there are critical database connection issues.
    async fn incremental_load(&self) -> Result<()> {
        self.process_pending_records().await
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

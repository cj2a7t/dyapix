use anyhow::Result;
use async_trait::async_trait;

use crate::cro::CRO;

#[async_trait]
pub trait DataSource: Send + Sync {
    /// Perform initial full load of all data from the datasource
    ///
    /// This method loads all existing records from the datasource into the cache.
    /// It should be called once during application startup to populate the cache
    /// with all current data.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the full load completes successfully, or an error if
    /// there are issues connecting to the datasource or processing the data.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dyapix_common::datasource::get_datasource;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let datasource = get_datasource().await?;
    ///     datasource.full_load().await?;
    ///     println!("Full load completed");
    ///     Ok(())
    /// }
    /// ```
    async fn full_load(&self) -> Result<()>;

    /// Load incremental changes (pending records) from the datasource
    ///
    /// This method processes only the records that have been marked as 'pending'
    /// since the last incremental load. It's designed to be called periodically
    /// to keep the cache synchronized with the latest changes.
    ///
    /// The method will:
    /// - Fetch all records with status 'pending'
    /// - Mark them as 'syncing' to prevent duplicate processing
    /// - Process each record and update the cache
    /// - Update the record status to 'synced' or back to 'pending' on failure
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the incremental load completes successfully, or an
    /// error if there are issues fetching or processing the pending records.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dyapix_common::datasource::get_datasource;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let datasource = get_datasource().await?;
    ///     datasource.incremental_load().await?;
    ///     println!("Incremental load completed");
    ///     Ok(())
    /// }
    /// ```
    async fn incremental_load(&self) -> Result<()>;

    /// Put (insert or update) a resource
    async fn put<T>(&self, resource: &T) -> Result<T>
    where
        T: CRO;

    /// Get a resource by id
    async fn get<T>(&self, id: &str) -> Result<T>
    where
        T: CRO;

    /// Delete a resource by id
    async fn delete<T>(&self, id: &str) -> Result<bool>
    where
        T: CRO;

    /// Get all resources of a specific type
    async fn get_all<T>(&self) -> Result<Vec<T>>
    where
        T: CRO;
}

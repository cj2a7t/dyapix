use anyhow::{anyhow, Result};

use super::pool::get_mysql_pool;
use super::types::DyapixDs;
use crate::datasource::mysql::MysqlDataSource;

impl MysqlDataSource {
    /// Perform initial full load of all datasource records
    ///
    /// Loads all records from the `dyapix_ds` table using cursor-based pagination
    /// for optimal performance. This method is designed to handle large datasets
    /// efficiently by processing records in batches.
    ///
    /// # Process Flow
    ///
    /// 1. **Pagination**: Uses cursor-based pagination with `id > last_id` to avoid
    ///    performance issues with large OFFSET values
    /// 2. **Batch Processing**: Processes records in batches of 100 for memory efficiency
    /// 3. **Cache Insertion**: Each record is parsed and inserted into the appropriate cache
    /// 4. **Progress Logging**: Logs progress for monitoring and debugging
    ///
    /// # Performance Characteristics
    ///
    /// - **Memory Efficient**: Processes records in small batches
    /// - **Database Friendly**: Uses indexed queries with cursor pagination
    /// - **Resumable**: Can be safely restarted if interrupted
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all records are successfully loaded, or an error if
    /// there are database connection issues or data processing failures.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dyapix_common::datasource::mysql::MysqlDataSource;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let ds = MysqlDataSource;
    ///     ds.initial_load().await?;
    ///     println!("Initial load completed");
    ///     Ok(())
    /// }
    /// ```
    pub(super) async fn initial_load(&self) -> Result<()> {
        let pool = get_mysql_pool().await?;
        const PAGE_SIZE: i64 = 100;

        tracing::info!("Starting initial full load of datasource records...");

        let mut last_id: i64 = 0;
        let mut total_loaded = 0;
        loop {
            // Use cursor-based pagination for better performance
            let rows: Vec<DyapixDs> = sqlx::query_as::<_, DyapixDs>(
                r#"
                SELECT id, `key`, ds_type, ds_json, prev_ds_json, ds_status, 
                       operation_type, is_deleted, create_time, update_time
                FROM dyapix_ds
                WHERE id > ?
                ORDER BY id ASC
                LIMIT ?
                "#,
            )
            .bind(last_id)
            .bind(PAGE_SIZE)
            .fetch_all(pool)
            .await?;

            if rows.is_empty() {
                tracing::info!(
                    "Initial load completed. Total records loaded: {}",
                    total_loaded
                );
                break;
            }

            let batch_size = rows.len();
            total_loaded += batch_size;
            tracing::info!(
                "Loaded {} records (last_id = {}, total = {})",
                batch_size,
                last_id,
                total_loaded
            );

            for record in &rows {
                tracing::debug!(
                    "Inserting record into cache: id = {}, key = {}, ds_type = {}",
                    record.id,
                    record.key,
                    record.ds_type
                );
                self.insert_into_cache(record).await;
            }

            // Update cursor to last record's id
            last_id = rows.last().unwrap().id;
        }

        Ok(())
    }

    /// Process pending records once
    ///
    /// This method processes all records with status 'pending' from the `dyapix_ds` table.
    /// It's designed to be called periodically by a BackgroundService to keep the cache
    /// synchronized with the latest changes.
    ///
    /// # Process Flow
    ///
    /// 1. **Fetch Pending**: Retrieves all records with `ds_status = 'pending'`
    /// 2. **Mark Syncing**: Updates status to 'syncing' to prevent duplicate processing
    /// 3. **Process Records**: Parses and inserts each record into the appropriate cache
    /// 4. **Update Status**: Sets status to 'synced' on success or back to 'pending' on failure
    ///
    /// # Error Handling
    ///
    /// - **Database Failures**: Retries with exponential backoff (max 3 retries)
    /// - **Individual Failures**: Failed records are reset to 'pending' for retry
    /// - **Batch Processing**: Individual record failures don't stop the batch
    ///
    /// # Performance
    ///
    /// - Processes up to 100 records per call
    /// - Uses batch status updates for efficiency
    /// - Returns early if no pending records found
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the processing completes successfully, or an error if
    /// there are critical database connection issues.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dyapix_common::datasource::mysql::MysqlDataSource;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let ds = MysqlDataSource;
    ///     ds.process_pending_records().await?;
    ///     println!("Pending records processed");
    ///     Ok(())
    /// }
    /// ```
    pub(super) async fn process_pending_records(&self) -> Result<()> {
        const PAGE_SIZE: i64 = 100;
        const MAX_RETRIES: u32 = 3;

        tracing::info!("Processing pending datasource records...");

        // Fetch pending records with retry logic
        let pending_rows = match Self::fetch_pending_with_retry(PAGE_SIZE, MAX_RETRIES).await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(
                    "Failed to fetch pending records after {} retries: {}",
                    MAX_RETRIES,
                    e
                );
                return Err(e);
            }
        };

        if pending_rows.is_empty() {
            tracing::debug!("No pending records found");
            return Ok(());
        }

        tracing::info!("Found {} pending records", pending_rows.len());

        // Batch update: Mark all pending records as syncing
        let ids: Vec<i64> = pending_rows.iter().map(|r| r.id).collect();
        if let Err(e) = Self::batch_update_status(&ids, "syncing").await {
            tracing::error!("Failed to mark records as syncing: {}", e);
            return Err(e);
        }

        // Process each record
        for record in pending_rows {
            tracing::debug!(
                "Syncing record into cache: id = {}, key = {}, ds_type = {}",
                record.id,
                record.key,
                record.ds_type
            );
            let ok = self.insert_into_cache(&record).await;
            let new_status = if ok { "synced" } else { "pending" };

            if ok {
                tracing::info!(
                    "✓ Synced record: id = {}, key = {}, ds_type = {}",
                    record.id,
                    record.key,
                    record.ds_type
                );
            } else {
                tracing::warn!(
                    "✗ Failed to sync record, resetting to pending: id = {}, key = {}, ds_type = {}",
                    record.id,
                    record.key,
                    record.ds_type
                );
            }

            // Update individual record status
            if let Err(e) = Self::update_single_status(record.id, new_status).await {
                tracing::error!(
                    "Failed to update status for record id = {}: {}",
                    record.id,
                    e
                );
            }
        }

        Ok(())
    }

    /// Fetch pending records with exponential backoff retry
    ///
    /// Retrieves records with status 'pending' from the `dyapix_ds` table with
    /// automatic retry logic for handling transient database connection issues.
    ///
    /// # Retry Strategy
    ///
    /// - **Exponential Backoff**: Delay increases exponentially (1s, 2s, 4s, ...)
    /// - **Maximum Delay**: Capped at 30 seconds to avoid excessive waits
    /// - **Retry Limit**: Configurable maximum number of retry attempts
    ///
    /// # Parameters
    ///
    /// - `page_size`: Maximum number of records to fetch per query
    /// - `max_retries`: Maximum number of retry attempts before giving up
    ///
    /// # Returns
    ///
    /// Returns a vector of `DyapixDs` records on success, or an error if all
    /// retry attempts are exhausted.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use dyapix_common::datasource::mysql::MysqlDataSource;
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let records = MysqlDataSource::fetch_pending_with_retry(100, 3).await?;
    ///     println!("Fetched {} pending records", records.len());
    ///     Ok(())
    /// }
    /// ```
    async fn fetch_pending_with_retry(page_size: i64, max_retries: u32) -> Result<Vec<DyapixDs>> {
        let mut retry_count = 0;
        let mut delay_secs = 1u64;

        loop {
            let pool = get_mysql_pool().await?;
            match sqlx::query_as::<_, DyapixDs>(
                r#"
                SELECT id, `key`, ds_type, ds_json, prev_ds_json, ds_status,
                       operation_type, is_deleted, create_time, update_time
                FROM dyapix_ds
                WHERE ds_status = 'pending'
                ORDER BY id ASC
                LIMIT ?
                "#,
            )
            .bind(page_size)
            .fetch_all(pool)
            .await
            {
                Ok(rows) => return Ok(rows),
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= max_retries {
                        return Err(anyhow!("Max retries exceeded: {}", e));
                    }
                    tracing::warn!(
                        "Failed to fetch pending records (attempt {}/{}): {}. Retrying in {}s...",
                        retry_count,
                        max_retries,
                        e,
                        delay_secs
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(delay_secs)).await;
                    delay_secs = (delay_secs * 2).min(30); // Exponential backoff, max 30s
                }
            }
        }
    }

    /// Batch update status for multiple records
    ///
    /// Efficiently updates the `ds_status` field for multiple records in a single
    /// database transaction. This is more efficient than updating records individually.
    ///
    /// # Parameters
    ///
    /// - `ids`: Slice of record IDs to update
    /// - `status`: New status value to set (e.g., "syncing", "synced", "pending")
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the batch update succeeds, or an error if there are
    /// database connection issues or SQL execution problems.
    ///
    /// # Performance
    ///
    /// - Uses a single SQL UPDATE statement with IN clause
    /// - More efficient than individual updates for large batches
    /// - Handles empty ID lists gracefully
    async fn batch_update_status(ids: &[i64], status: &str) -> Result<()> {
        if ids.is_empty() {
            return Ok(());
        }

        let pool = get_mysql_pool().await?;

        // Build placeholders for IN clause
        let placeholders = ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let query = format!(
            "UPDATE dyapix_ds SET ds_status = ? WHERE id IN ({})",
            placeholders
        );

        let mut q = sqlx::query(&query).bind(status);
        for id in ids {
            q = q.bind(id);
        }

        q.execute(pool).await?;
        Ok(())
    }

    /// Update status for a single record
    ///
    /// Updates the `ds_status` field for a single record. This is typically used
    /// after processing individual records to mark them as 'synced' or reset them
    /// to 'pending' on failure.
    ///
    /// # Parameters
    ///
    /// - `id`: The ID of the record to update
    /// - `status`: New status value to set
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the update succeeds, or an error if there are
    /// database connection issues or the record doesn't exist.
    ///
    /// # Usage
    ///
    /// This method is typically called after processing each record in a batch
    /// to update its status based on the processing result.
    async fn update_single_status(id: i64, status: &str) -> Result<()> {
        let pool = get_mysql_pool().await?;
        sqlx::query("UPDATE dyapix_ds SET ds_status = ? WHERE id = ?")
            .bind(status)
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }
}

use anyhow::{anyhow, Result};
use tokio::select;
use tokio::sync::broadcast;

use super::pool::get_mysql_pool;
use super::types::DyapixDs;
use crate::datasource::mysql::MysqlDataSource;

// Global shutdown channel
static SHUTDOWN_TX: std::sync::OnceLock<broadcast::Sender<()>> = std::sync::OnceLock::new();

/// Initialize shutdown channel
pub fn init_shutdown_channel() -> broadcast::Sender<()> {
    let (tx, _) = broadcast::channel(1);
    SHUTDOWN_TX.set(tx.clone()).ok();
    tx
}

/// Send shutdown signal to all watchers
pub fn trigger_shutdown() {
    if let Some(tx) = SHUTDOWN_TX.get() {
        let _ = tx.send(());
    }
}

impl MysqlDataSource {
    /// Perform initial full load of all datasource records
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

    /// Watch and sync pending records in a loop
    /// Supports graceful shutdown via shutdown_rx channel
    pub(super) async fn watch_pending(&self) -> Result<()> {
        const PAGE_SIZE: i64 = 100;
        const POLL_INTERVAL_SECS: u64 = 5;
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY_SECS: u64 = 10;

        tracing::info!("Entering watch loop for pending datasource records...");

        // Subscribe to shutdown signal
        let mut shutdown_rx = SHUTDOWN_TX
            .get_or_init(|| {
                let (tx, _) = broadcast::channel(1);
                tx
            })
            .subscribe();

        loop {
            // Check for shutdown signal
            select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("Received shutdown signal, stopping watcher...");
                    return Ok(());
                }
                _ = Self::process_pending_records(self, PAGE_SIZE, MAX_RETRIES, RETRY_DELAY_SECS, POLL_INTERVAL_SECS) => {}
            }
        }
    }

    /// Process pending records in one iteration
    async fn process_pending_records(
        &self,
        page_size: i64,
        max_retries: u32,
        retry_delay_secs: u64,
        poll_interval_secs: u64,
    ) {
        // Fetch pending records with retry logic
        let pending_rows = match Self::fetch_pending_with_retry(page_size, max_retries).await {
            Ok(rows) => rows,
            Err(e) => {
                tracing::error!(
                    "Failed to fetch pending records after {} retries: {}",
                    max_retries,
                    e
                );
                tracing::info!("Waiting {} seconds before retry...", retry_delay_secs * 2);
                tokio::time::sleep(std::time::Duration::from_secs(retry_delay_secs * 2)).await;
                return;
            }
        };

        if pending_rows.is_empty() {
            tracing::debug!(
                "No pending records found, sleeping {}s...",
                poll_interval_secs
            );
            tokio::time::sleep(std::time::Duration::from_secs(poll_interval_secs)).await;
            return;
        }

        tracing::info!("Found {} pending records", pending_rows.len());

        // Batch update: Mark all pending records as syncing
        let ids: Vec<i64> = pending_rows.iter().map(|r| r.id).collect();
        if let Err(e) = Self::batch_update_status(&ids, "syncing").await {
            tracing::error!("Failed to mark records as syncing: {}", e);
            tokio::time::sleep(std::time::Duration::from_secs(retry_delay_secs)).await;
            return;
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
    }

    /// Fetch pending records with exponential backoff retry
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

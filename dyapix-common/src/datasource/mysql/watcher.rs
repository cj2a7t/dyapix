use anyhow::Result;

use super::pool::get_mysql_pool;
use super::types::DyapixDs;
use crate::datasource::mysql::MysqlDataSource;

impl MysqlDataSource {
    /// Perform initial full load of all datasource records
    pub(super) async fn initial_load(&self) -> Result<()> {
        let pool = get_mysql_pool().await?;
        const PAGE_SIZE: i64 = 100;

        tracing::info!("Starting initial full load of datasource records...");

        let mut offset: i64 = 0;
        loop {
            let rows: Vec<DyapixDs> = sqlx::query_as::<_, DyapixDs>(
                r#"
                SELECT id, `key`, ds_type, ds_json, prev_ds_json, ds_status, 
                       operation_type, is_deleted, create_time, update_time
                FROM dyapix_ds
                ORDER BY id ASC
                LIMIT ? OFFSET ?
                "#,
            )
            .bind(PAGE_SIZE)
            .bind(offset)
            .fetch_all(pool)
            .await?;

            if rows.is_empty() {
                tracing::info!("Initial load completed.");
                break;
            }

            tracing::debug!("Loaded {} records (offset = {})", rows.len(), offset);

            for record in &rows {
                tracing::debug!(
                    "Inserting record into cache: id = {}, key = {}, ds_type = {}",
                    record.id,
                    record.key,
                    record.ds_type
                );
                self.insert_into_cache(record).await;
            }

            offset += PAGE_SIZE;
        }

        Ok(())
    }

    /// Watch and sync pending records in a loop
    pub(super) async fn watch_pending(&self) -> Result<()> {
        let pool = get_mysql_pool().await?;
        const PAGE_SIZE: i64 = 100;

        tracing::info!("Entering watch loop for pending datasource records...");

        loop {
            let pending_rows: Vec<DyapixDs> = sqlx::query_as::<_, DyapixDs>(
                r#"
                SELECT id, `key`, ds_type, ds_json, prev_ds_json, ds_status,
                       operation_type, is_deleted, create_time, update_time
                FROM dyapix_ds
                WHERE ds_status = 'pending'
                ORDER BY id ASC
                LIMIT ?
                "#,
            )
            .bind(PAGE_SIZE)
            .fetch_all(pool)
            .await?;

            if pending_rows.is_empty() {
                tracing::debug!("No pending records found, sleeping 5s...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }

            tracing::info!("Found {} pending records", pending_rows.len());

            // Mark all pending records as syncing
            for record in &pending_rows {
                tracing::debug!("Marking record as syncing: id = {}", record.id);
                sqlx::query("UPDATE dyapix_ds SET ds_status = 'syncing' WHERE id = ?")
                    .bind(record.id)
                    .execute(pool)
                    .await?;
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
                        "Synced record: id = {}, key = {}, ds_type = {}",
                        record.id,
                        record.key,
                        record.ds_type
                    );
                } else {
                    tracing::warn!(
                        "Failed to sync record, resetting to pending: id = {}, key = {}, ds_type = {}",
                        record.id,
                        record.key,
                        record.ds_type
                    );
                }

                sqlx::query("UPDATE dyapix_ds SET ds_status = ? WHERE id = ?")
                    .bind(new_status)
                    .bind(record.id)
                    .execute(pool)
                    .await?;
            }
        }
    }
}


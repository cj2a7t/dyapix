use anyhow::{anyhow, Result};
use chrono::Utc;
use tracing::warn;

use super::pool::get_mysql_pool;
use super::types::DyapixDs;
use crate::cro::CRO;
use crate::datasource::mysql::MysqlDataSource;

impl MysqlDataSource {
    /// Insert or update a record
    pub(super) async fn put_internal<T>(&self, value: &T) -> Result<T>
    where
        T: CRO,
    {
        let pool = get_mysql_pool().await?;
        let value_json = serde_json::to_string(value)?;
        let id = value.id();
        let ds_type = T::cro_kind();
        let now = Utc::now();

        // Use transaction to prevent race conditions
        let mut tx = pool.begin().await?;

        // Check if record already exists (including deleted ones) with row lock
        let existing: Option<(String, bool)> =
            sqlx::query_as("SELECT ds_json, is_deleted FROM dyapix_ds WHERE `key` = ? FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await?;

        let (operation_type, prev_ds_json) = match existing {
            Some((prev_json, is_deleted)) => {
                if is_deleted {
                    // Restore deleted record as a new create
                    ("create", None)
                } else {
                    // Update existing record
                    ("update", Some(prev_json))
                }
            }
            None => ("create", None),
        };

        sqlx::query(
            r#"
            INSERT INTO dyapix_ds (`key`, ds_type, ds_json, prev_ds_json, ds_status, operation_type, is_deleted, create_time, update_time)
            VALUES (?, ?, ?, ?, 'pending', ?, FALSE, ?, ?)
            ON DUPLICATE KEY UPDATE
                ds_type = VALUES(ds_type),
                ds_json = VALUES(ds_json),
                prev_ds_json = VALUES(prev_ds_json),
                ds_status = VALUES(ds_status),
                operation_type = VALUES(operation_type),
                is_deleted = VALUES(is_deleted),
                update_time = VALUES(update_time)
        "#,
        )
        .bind(id)
        .bind(ds_type)
        .bind(&value_json)
        .bind(prev_ds_json)
        .bind(operation_type)
        .bind(now)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // Commit transaction
        tx.commit().await?;

        Ok(value.clone())
    }

    /// Get a single record by key
    pub(super) async fn get_internal<T>(&self, id: &str) -> Result<T>
    where
        T: CRO,
    {
        let pool = get_mysql_pool().await?;
        let ds_type = T::cro_kind();

        let record: Option<DyapixDs> =
            sqlx::query_as::<_, DyapixDs>(
                "SELECT id, `key`, ds_type, ds_json, prev_ds_json, ds_status, operation_type, is_deleted, create_time, update_time 
                 FROM dyapix_ds 
                 WHERE `key` = ? AND ds_type = ? AND is_deleted = FALSE"
            )
                .bind(id)
                .bind(ds_type)
                .fetch_optional(pool)
                .await?;

        match record {
            Some(row) => {
                let ds = serde_json::from_str::<T>(&row.ds_json)
                    .map_err(|e| anyhow!("Failed to deserialize ds_json: {}", e))?;
                Ok(ds)
            }
            None => Err(anyhow!(
                "No record found for key `{}` with type `{}`",
                id,
                ds_type
            )),
        }
    }

    /// Soft delete a record
    pub(super) async fn delete_internal<T>(&self, id: &str) -> Result<bool>
    where
        T: CRO,
    {
        let pool = get_mysql_pool().await?;
        let ds_type = T::cro_kind();

        // Use transaction to ensure consistency
        let mut tx = pool.begin().await?;

        let record: Option<(String, String)> = sqlx::query_as(
            "SELECT ds_json, ds_status FROM dyapix_ds WHERE `key` = ? AND ds_type = ? AND is_deleted = FALSE FOR UPDATE",
        )
        .bind(id)
        .bind(ds_type)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((ds_json, ds_status)) = record {
            // If status is syncing, wait for it to complete and retry later
            if ds_status == "syncing" {
                tx.rollback().await?;
                return Err(anyhow!(
                    "Record is currently syncing, please retry deletion later"
                ));
            }

            let now = Utc::now();
            let result = sqlx::query(
                r#"
                UPDATE dyapix_ds 
                SET is_deleted = TRUE, 
                    prev_ds_json = ?,
                    operation_type = 'delete',
                    ds_status = 'pending',
                    update_time = ?
                WHERE `key` = ? AND ds_status != 'syncing'
                "#,
            )
            .bind(ds_json)
            .bind(now)
            .bind(id)
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;
            Ok(result.rows_affected() > 0)
        } else {
            tx.rollback().await?;
            Ok(false)
        }
    }

    /// Get all records of a specific type
    pub(super) async fn get_all_internal<T>(&self) -> Result<Vec<T>>
    where
        T: CRO,
    {
        let pool = get_mysql_pool().await?;
        let ds_type = T::cro_kind();

        let rows: Vec<DyapixDs> = sqlx::query_as::<_, DyapixDs>(
            r#"
            SELECT id, `key`, ds_type, ds_json, prev_ds_json, ds_status, operation_type, is_deleted, create_time, update_time
            FROM dyapix_ds
            WHERE ds_type = ? AND is_deleted = FALSE
            ORDER BY id ASC
            "#,
        )
        .bind(ds_type)
        .fetch_all(pool)
        .await?;

        let mut result = Vec::new();
        for row in rows {
            match serde_json::from_str::<T>(&row.ds_json) {
                Ok(ds) => result.push(ds),
                Err(e) => {
                    warn!(
                        "Failed to deserialize ds_json for key {} into type {}: {}",
                        row.key, ds_type, e
                    );
                }
            }
        }

        Ok(result)
    }
}

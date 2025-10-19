use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::pool::get_mysql_pool;

/// Health status of the datasource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    /// Is the datasource healthy
    pub healthy: bool,
    /// Connection pool status
    pub pool_status: PoolStatus,
    /// Pending records count
    pub pending_count: i64,
    /// Syncing records count
    pub syncing_count: i64,
    /// Last check time (as RFC3339 string)
    #[serde(with = "chrono::serde::ts_seconds")]
    pub last_check: DateTime<Utc>,
    /// Error message if unhealthy
    pub error: Option<String>,
}

/// Connection pool status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    /// Current connections in the pool
    pub size: u32,
    /// Maximum connections allowed
    pub max_size: u32,
    /// Idle connections
    pub idle: usize,
}

impl HealthStatus {
    /// Check health of the datasource
    pub async fn check() -> Self {
        let last_check = Utc::now();

        match Self::check_internal().await {
            Ok(status) => HealthStatus {
                healthy: true,
                last_check,
                error: None,
                ..status
            },
            Err(e) => HealthStatus {
                healthy: false,
                pool_status: PoolStatus {
                    size: 0,
                    max_size: 0,
                    idle: 0,
                },
                pending_count: 0,
                syncing_count: 0,
                last_check,
                error: Some(e.to_string()),
            },
        }
    }

    async fn check_internal() -> Result<HealthStatus> {
        let pool = get_mysql_pool().await?;

        // Get pool status
        let pool_status = PoolStatus {
            size: pool.size(),
            max_size: pool.options().get_max_connections(),
            idle: pool.num_idle(),
        };

        // Get pending count
        let (pending_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM dyapix_ds WHERE ds_status = 'pending'"
        )
        .fetch_one(pool)
        .await?;

        // Get syncing count
        let (syncing_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM dyapix_ds WHERE ds_status = 'syncing'"
        )
        .fetch_one(pool)
        .await?;

        Ok(HealthStatus {
            healthy: true,
            pool_status,
            pending_count,
            syncing_count,
            last_check: Utc::now(),
            error: None,
        })
    }

    /// Get statistics about datasource
    pub async fn get_statistics() -> Result<DataSourceStats> {
        let pool = get_mysql_pool().await?;

        let (total_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM dyapix_ds"
        )
        .fetch_one(pool)
        .await?;

        let (active_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM dyapix_ds WHERE is_deleted = FALSE"
        )
        .fetch_one(pool)
        .await?;

        let (deleted_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM dyapix_ds WHERE is_deleted = TRUE"
        )
        .fetch_one(pool)
        .await?;

        let (synced_count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM dyapix_ds WHERE ds_status = 'synced'"
        )
        .fetch_one(pool)
        .await?;

        Ok(DataSourceStats {
            total_count,
            active_count,
            deleted_count,
            synced_count,
        })
    }
}

/// Statistics about datasource records
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSourceStats {
    pub total_count: i64,
    pub active_count: i64,
    pub deleted_count: i64,
    pub synced_count: i64,
}


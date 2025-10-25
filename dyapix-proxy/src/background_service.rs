use anyhow::Result;
use async_trait::async_trait;
use dyapix_common::datasource::{get_datasource, interface::DataSource};
use pingora::{server::ShutdownWatch, services::background::BackgroundService};
use std::time::Duration;
use tokio::time::interval;

/// Dyapix Background Service for data synchronization
///
/// This service handles the synchronization of data between the datasource and cache.
/// It performs an initial full load on startup and then periodically processes
/// incremental changes to keep the cache up-to-date.
pub struct DyapixBackgroundService;

impl DyapixBackgroundService {
    /// Create a new instance of DyapixBackgroundService
    pub fn new() -> Self {
        Self
    }

    /// Perform initial full load of all data
    async fn perform_full_load(&self) -> Result<()> {
        tracing::info!("Starting initial full load of datasource...");

        let datasource = get_datasource().await?;
        datasource.full_load().await?;

        tracing::info!("Initial full load completed successfully");
        Ok(())
    }

    /// Perform incremental load of pending changes
    async fn perform_incremental_load(&self) -> Result<()> {
        tracing::debug!("Starting incremental load of pending changes...");

        let datasource = get_datasource().await?;
        datasource.incremental_load().await?;

        tracing::debug!("Incremental load completed successfully");
        Ok(())
    }

    /// Handle errors during data synchronization
    async fn handle_sync_error(&self, error: &anyhow::Error, operation: &str) {
        tracing::error!(
            "Error during {}: {}. Retrying in 30 seconds...",
            operation,
            error
        );

        // Wait before retry to avoid rapid error loops
        tokio::time::sleep(Duration::from_secs(30)).await;
    }
}

#[async_trait]
impl BackgroundService for DyapixBackgroundService {
    /// Start the background service
    ///
    /// This method performs the following operations:
    /// 1. Initial full load of all data from the datasource
    /// 2. Periodic incremental loads to sync pending changes
    /// 3. Graceful shutdown handling
    async fn start(&self, mut shutdown: ShutdownWatch) {
        tracing::info!("DyapixBackgroundService starting...");

        // Perform initial full load
        match self.perform_full_load().await {
            Ok(_) => {
                tracing::info!("Initial full load completed successfully");
            }
            Err(e) => {
                tracing::error!("Failed to perform initial full load: {}", e);
                // Continue with incremental loads even if full load fails
            }
        }

        // Set up periodic incremental loads
        let mut period = interval(Duration::from_secs(5)); // Every 5 seconds

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    tracing::info!("DyapixBackgroundService shutting down...");
                    break;
                }
                _ = period.tick() => {
                    // Perform incremental load
                    if let Err(e) = self.perform_incremental_load().await {
                        self.handle_sync_error(&e, "incremental load").await;
                    }
                }
            }
        }
    }
}

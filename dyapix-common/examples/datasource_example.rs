/// Dynamic Datasource Usage Example
///
/// This example demonstrates how to use the dynamic datasource
/// that automatically selects the appropriate implementation based on configuration.

use anyhow::Result;
use dyapix_common::datasource::{get_datasource, DynamicDataSource};
use dyapix_common::datasource::mysql::{init_shutdown_channel, MysqlDataSource};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    tracing::info!("=== Dynamic Datasource Example ===");

    // Initialize shutdown channel
    let _shutdown_tx = init_shutdown_channel();

    // Method 1: Get global datasource (Recommended) ⭐
    tracing::info!("\n--- Method 1: Global Datasource (Recommended) ---");
    let datasource = get_datasource().await?;
    tracing::info!("✓ Datasource type: {}", datasource.datasource_type());

    // Method 2: Create from config
    tracing::info!("\n--- Method 2: Create from Config ---");
    let datasource2 = DynamicDataSource::from_config().await?;
    tracing::info!("✓ Datasource type: {}", datasource2.datasource_type());

    // Method 3: Manually create specific datasource
    tracing::info!("\n--- Method 3: Manual Creation ---");
    let datasource3 = DynamicDataSource::Mysql(MysqlDataSource);
    tracing::info!("✓ Datasource type: {}", datasource3.datasource_type());

    // Example: CRUD Operations
    tracing::info!("\n--- CRUD Operations Example ---");

    // Note: These operations will fail if database is not configured
    // This is just to demonstrate the API usage

    // Create/Update a route
    tracing::info!("Example: Create/Update route");
    // let route = Route {
    //     id: "example-route".to_string(),
    //     // ... other fields
    // };
    // match datasource.put(&route).await {
    //     Ok(result) => tracing::info!("✓ Route saved: {}", result.id),
    //     Err(e) => tracing::error!("✗ Failed to save route: {}", e),
    // }

    // Get a route
    tracing::info!("Example: Get route by ID");
    // match datasource.get::<Route>("example-route").await {
    //     Ok(route) => tracing::info!("✓ Found route: {}", route.id),
    //     Err(e) => tracing::error!("✗ Route not found: {}", e),
    // }

    // Get all routes
    tracing::info!("Example: Get all routes");
    // match datasource.get_all::<Route>().await {
    //     Ok(routes) => tracing::info!("✓ Total routes: {}", routes.len()),
    //     Err(e) => tracing::error!("✗ Failed to get routes: {}", e),
    // }

    // Delete a route
    tracing::info!("Example: Delete route");
    // match datasource.delete::<Route>("example-route").await {
    //     Ok(true) => tracing::info!("✓ Route deleted"),
    //     Ok(false) => tracing::info!("○ Route not found"),
    //     Err(e) => tracing::error!("✗ Failed to delete: {}", e),
    // }

    // Example: Health Check
    tracing::info!("\n--- Health Check Example ---");
    let health = MysqlDataSource::health_check().await;
    if health.healthy {
        tracing::info!("✓ Datasource is healthy");
        tracing::info!("  - Pool: {}/{}", health.pool_status.size, health.pool_status.max_size);
        tracing::info!("  - Pending: {}", health.pending_count);
        tracing::info!("  - Syncing: {}", health.syncing_count);
    } else {
        tracing::warn!("✗ Datasource is unhealthy: {:?}", health.error);
    }

    // Example: Start watcher in background
    tracing::info!("\n--- Watcher Example ---");
    tracing::info!("To start the watcher, use:");
    tracing::info!("  tokio::spawn(async move {{");
    tracing::info!("      if let Err(e) = datasource.fetch_and_watch().await {{");
    tracing::info!("          tracing::error!(\"Watcher failed: {{}}\", e);");
    tracing::info!("      }}");
    tracing::info!("  }});");

    tracing::info!("\n=== Example Completed ===");
    tracing::info!("Note: Actual CRUD operations are commented out to avoid database dependencies");
    tracing::info!("Uncomment the operations above when your database is configured");

    Ok(())
}


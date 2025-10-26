use anyhow::Result;
use dyapix_common::{config::get_app_config, log::log::init_logging};
use dyapix_proxy::{background_service::DyapixBackgroundService, proxy::DyapixProxy};
use pingora::{
    proxy::http_proxy_service,
    server::{Server, configuration::Opt},
    services::background::background_service,
};
use tracing::info;

fn main() -> Result<()> {
    // Initialize configuration
    let config = get_app_config()?;

    // Initialize logging
    let _guard = init_logging()?;

    // Create server
    let opt = Some(Opt::default());
    let mut dyapix = Server::new(opt)?;
    dyapix.bootstrap();

    // Start background service
    let background_service =
        background_service("dyapix_background", DyapixBackgroundService::new());
    info!("Background service created");

    // Add services to server
    dyapix.add_service(background_service);

    // Start proxy service
    let mut proxy_service = http_proxy_service(&dyapix.configuration, DyapixProxy);
    let proxy_addr = format!("{}:{}", config.server.proxy_host, config.server.proxy_port);
    proxy_service.add_tcp(&proxy_addr);
    dyapix.add_service(proxy_service);
    info!("Proxy service created and listening on {}", proxy_addr);

    // Start the server
    info!("Starting Dyapix server...");
    dyapix.run_forever();
}

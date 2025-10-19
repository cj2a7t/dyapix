use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::OnceLock;

use crate::cro::{Route, Upstream, CRO_KIND_ROUTE, CRO_KIND_UPSTREAM};

/// Type-safe entity enum for CRO operations
/// Route is boxed to reduce enum size variance (Route is 424 bytes, Upstream is 152 bytes)
#[derive(Debug, Clone)]
pub enum CROEntity {
    Route(Box<Route>),
    Upstream(Upstream),
}

/// CRO (Core Resource Object) handler trait for processing cache operations
/// Each resource type (Route, Upstream, TlsCert, etc.) should implement this trait
#[async_trait]
pub trait CROHandler: Send + Sync {
    /// Parse CRO from JSON string into type-safe entity
    fn parse_entity(&self, json: &str) -> Result<CROEntity>;

    /// Insert CRO into cache
    ///
    /// # Arguments
    /// * `operation_type` - The operation type: "create", "update", or "delete"
    /// * `entity` - The current entity (parsed from ds_json)
    /// * `prev_entity` - The previous entity (parsed from prev_ds_json), used for update operations
    async fn insert_into_cache(
        &self,
        operation_type: &str,
        entity: CROEntity,
        prev_entity: Option<CROEntity>,
    ) -> bool;
}

/// Registry for CRO handlers
pub struct CROHandlerRegistry {
    handlers: HashMap<String, Box<dyn CROHandler>>,
}

impl CROHandlerRegistry {
    /// Create a new empty registry
    fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a handler for a specific CRO type
    pub fn register(&mut self, entity_type: &str, handler: Box<dyn CROHandler>) {
        self.handlers.insert(entity_type.to_string(), handler);
    }

    /// Get a handler for a specific CRO type
    pub fn get(&self, entity_type: &str) -> Option<&dyn CROHandler> {
        self.handlers.get(entity_type).map(|h| h.as_ref())
    }

    /// Get the global registry instance
    pub fn global() -> &'static CROHandlerRegistry {
        static REGISTRY: OnceLock<CROHandlerRegistry> = OnceLock::new();
        REGISTRY.get_or_init(|| {
            let mut registry = CROHandlerRegistry::new();

            // Register all built-in handlers
            // Use constants to ensure consistency with CRO implementations
            registry.register(
                CRO_KIND_ROUTE,
                Box::new(crate::datasource::mysql::extension::route_handler::RouteHandler),
            );
            registry.register(
                CRO_KIND_UPSTREAM,
                Box::new(crate::datasource::mysql::extension::upstream_handler::UpstreamHandler),
            );

            registry
        })
    }
}

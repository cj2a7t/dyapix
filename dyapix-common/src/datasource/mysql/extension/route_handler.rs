use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::any::Any;

use crate::cache::routes_cache::{self, RoutesCacheEvent};
use crate::cache::CacheEventType;
use crate::cro::Route;
use crate::datasource::mysql::handler::CROHandler;

/// Route CRO handler
pub struct RouteHandler;

#[async_trait]
impl CROHandler for RouteHandler {
    fn parse_entity(&self, json: &str) -> Result<Box<dyn Any + Send>> {
        let route: Route = serde_json::from_str(json)
            .map_err(|e| anyhow!("Failed to parse Route from JSON: {}", e))?;
        Ok(Box::new(route))
    }

    async fn insert_into_cache(
        &self,
        operation_type: &str,
        entity: Box<dyn Any + Send>,
        prev_entity: Option<Box<dyn Any + Send>>,
    ) -> bool {
        // Downcast to Route
        let route = match entity.downcast::<Route>() {
            Ok(r) => *r,
            Err(_) => {
                tracing::error!("Failed to downcast entity to Route");
                return false;
            }
        };

        let prev_route = prev_entity.and_then(|e| e.downcast::<Route>().ok().map(|r| *r));

        let event = match operation_type {
            "create" => RoutesCacheEvent {
                event_type: CacheEventType::Create,
                route: Some(route),
                history_route: None,
            },
            "update" => {
                if prev_route.is_none() {
                    tracing::error!("Update operation requires prev_route");
                    return false;
                }

                RoutesCacheEvent {
                    event_type: CacheEventType::Update,
                    route: Some(route),
                    history_route: prev_route,
                }
            }
            "delete" => RoutesCacheEvent {
                event_type: CacheEventType::Delete,
                route: Some(route),
                history_route: None,
            },
            _ => {
                tracing::error!("Unknown operation_type: {}", operation_type);
                return false;
            }
        };

        match routes_cache::inc_update(vec![event]) {
            Ok(_) => true,
            Err(e) => {
                tracing::error!("Failed to update routes cache: {}", e);
                false
            }
        }
    }
}


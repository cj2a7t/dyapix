use anyhow::{anyhow, Result};
use async_trait::async_trait;

use crate::cache::routes_cache::{self, RoutesCacheEvent};
use crate::cache::CacheEventType;
use crate::cro::Route;
use crate::datasource::mysql::handler::{CROEntity, CROHandler};

/// Route CRO handler
pub struct RouteHandler;

#[async_trait]
impl CROHandler for RouteHandler {
    fn parse_entity(&self, json: &str) -> Result<CROEntity> {
        let route: Route = serde_json::from_str(json)
            .map_err(|e| anyhow!("Failed to parse Route from JSON: {}", e))?;
        Ok(CROEntity::Route(Box::new(route)))
    }

    async fn insert_into_cache(
        &self,
        operation_type: &str,
        entity: CROEntity,
        prev_entity: Option<CROEntity>,
    ) -> bool {
        // Extract route from entity enum
        let route = match entity {
            CROEntity::Route(r) => *r,
            _ => {
                tracing::error!("Expected Route entity, got different type");
                return false;
            }
        };

        let prev_route = prev_entity.and_then(|e| match e {
            CROEntity::Route(r) => Some(*r),
            _ => None,
        });

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


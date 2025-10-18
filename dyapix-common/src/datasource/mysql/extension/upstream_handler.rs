use anyhow::{anyhow, Result};
use async_trait::async_trait;
use std::any::Any;

use crate::cache::upstreams_cache::{self, UpstreamsCacheEvent};
use crate::cache::CacheEventType;
use crate::cro::Upstream;
use crate::datasource::mysql::handler::CROHandler;

/// Upstream CRO handler
pub struct UpstreamHandler;

#[async_trait]
impl CROHandler for UpstreamHandler {
    fn parse_entity(&self, json: &str) -> Result<Box<dyn Any + Send>> {
        let upstream: Upstream = serde_json::from_str(json)
            .map_err(|e| anyhow!("Failed to parse Upstream from JSON: {}", e))?;
        Ok(Box::new(upstream))
    }

    async fn insert_into_cache(
        &self,
        operation_type: &str,
        entity: Box<dyn Any + Send>,
        _prev_entity: Option<Box<dyn Any + Send>>,
    ) -> bool {
        // Downcast to Upstream
        let upstream = match entity.downcast::<Upstream>() {
            Ok(u) => *u,
            Err(_) => {
                tracing::error!("Failed to downcast entity to Upstream");
                return false;
            }
        };

        let event = match operation_type {
            "create" => UpstreamsCacheEvent {
                event_type: CacheEventType::Create,
                upstream: Some(upstream),
            },
            "update" => {
                // For upstream, we don't need history_upstream like routes do
                // because upstreams cache doesn't have RadixRouter
                UpstreamsCacheEvent {
                    event_type: CacheEventType::Update,
                    upstream: Some(upstream),
                }
            }
            "delete" => UpstreamsCacheEvent {
                event_type: CacheEventType::Delete,
                upstream: Some(upstream),
            },
            _ => {
                tracing::error!("Unknown operation_type: {}", operation_type);
                return false;
            }
        };

        match upstreams_cache::inc_update(vec![event]) {
            Ok(_) => true,
            Err(e) => {
                tracing::error!("Failed to update upstreams cache: {}", e);
                false
            }
        }
    }
}


use anyhow::{anyhow, Result};
use async_trait::async_trait;

use crate::cache::upstreams_cache::{self, UpstreamsCacheEvent};
use crate::cache::CacheEventType;
use crate::cro::Upstream;
use crate::datasource::mysql::handler::{CROEntity, CROHandler};

/// Upstream CRO handler
pub struct UpstreamHandler;

#[async_trait]
impl CROHandler for UpstreamHandler {
    fn parse_entity(&self, json: &str) -> Result<CROEntity> {
        let upstream: Upstream = serde_json::from_str(json)
            .map_err(|e| anyhow!("Failed to parse Upstream from JSON: {}", e))?;
        Ok(CROEntity::Upstream(upstream))
    }

    async fn insert_into_cache(
        &self,
        operation_type: &str,
        entity: CROEntity,
        _prev_entity: Option<CROEntity>,
    ) -> bool {
        // Extract upstream from entity enum
        let upstream = match entity {
            CROEntity::Upstream(u) => u,
            _ => {
                tracing::error!("Expected Upstream entity, got different type");
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

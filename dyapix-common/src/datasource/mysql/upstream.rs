use anyhow::Result;

use crate::cache::upstreams_cache::{self, UpstreamsCacheEvent};
use crate::cache::CacheEventType;
use crate::cro::Upstream;
use crate::datasource::mysql::DyapixDs;

/// Insert Upstream record into upstreams cache
pub async fn insert_upstream_into_cache(record: &DyapixDs) -> bool {
    let upstream_result: Result<Upstream, _> = serde_json::from_str(&record.ds_json);
    let upstream = match upstream_result {
        Ok(u) => u,
        Err(e) => {
            tracing::error!(
                "Failed to parse upstream JSON for record id = {}: {}",
                record.id,
                e
            );
            return false;
        }
    };

    let event = match record.operation_type.as_str() {
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
            tracing::error!(
                "Unknown operation_type: {} for record id = {}",
                record.operation_type,
                record.id
            );
            return false;
        }
    };

    match upstreams_cache::inc_update(vec![event]) {
        Ok(_) => true,
        Err(e) => {
            tracing::error!(
                "Failed to update upstreams cache for record id = {}: {}",
                record.id,
                e
            );
            false
        }
    }
}


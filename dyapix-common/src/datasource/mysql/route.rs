use anyhow::Result;

use crate::cache::routes_cache::{self, RoutesCacheEvent};
use crate::cache::CacheEventType;
use crate::cro::Route;
use crate::datasource::mysql::DyapixDs;

/// Insert Route record into routes cache
pub async fn insert_route_into_cache(record: &DyapixDs) -> bool {
    let route_result: Result<Route, _> = serde_json::from_str(&record.ds_json);
    let route = match route_result {
        Ok(r) => r,
        Err(e) => {
            tracing::error!(
                "Failed to parse route JSON for record id = {}: {}",
                record.id,
                e
            );
            return false;
        }
    };

    let event = match record.operation_type.as_str() {
        "create" => RoutesCacheEvent {
            event_type: CacheEventType::Create,
            route: Some(route),
            history_route: None,
        },
        "update" => {
            let prev_route = if let Some(ref prev_json) = record.prev_ds_json {
                match serde_json::from_str::<Route>(prev_json) {
                    Ok(r) => Some(r),
                    Err(e) => {
                        tracing::error!(
                            "Failed to parse prev_ds_json for update: id = {}, error = {}",
                            record.id,
                            e
                        );
                        return false;
                    }
                }
            } else {
                tracing::error!("Update operation requires prev_ds_json: id = {}", record.id);
                return false;
            };

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
            tracing::error!(
                "Unknown operation_type: {} for record id = {}",
                record.operation_type,
                record.id
            );
            return false;
        }
    };

    match routes_cache::inc_update(vec![event]) {
        Ok(_) => true,
        Err(e) => {
            tracing::error!(
                "Failed to update routes cache for record id = {}: {}",
                record.id,
                e
            );
            false
        }
    }
}


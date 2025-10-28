use std::sync::Arc;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use once_cell::sync::OnceCell;

use crate::cache::CacheEventType;
use crate::cro::Route;
use anyhow::{anyhow, Result};
use router_radix::RadixRouter;

static ROUTES_CACHE: OnceCell<ArcSwap<RoutesCache>> = OnceCell::new();

pub struct RoutesCache {
    pub routes_map: Arc<DashMap<String, Route>>,
    pub routes_radix: Arc<RadixRouter>,
}

pub struct RoutesCacheEvent {
    pub event_type: CacheEventType,
    pub route: Option<Route>,
    /// The route state before update, required for RadixRouter incremental update
    pub history_route: Option<Route>,
}

pub fn local() -> Result<Arc<RoutesCache>> {
    // If cache is not initialized, try to initialize it automatically
    if ROUTES_CACHE.get().is_none() {
        init_cache()?;
    }

    // At this point, cache must be initialized
    match ROUTES_CACHE.get() {
        Some(cache_swap) => Ok(cache_swap.load_full().clone()),
        None => Err(anyhow!("Cache initialization failed")),
    }
}

pub fn init_cache() -> Result<()> {
    let init = RoutesCache {
        routes_map: Arc::new(DashMap::new()),
        routes_radix: Arc::new(RadixRouter::new().unwrap()),
    };

    ROUTES_CACHE
        .set(ArcSwap::new(Arc::new(init)))
        .map_err(|_| anyhow!("Cache already initialized"))?;

    Ok(())
}

pub fn full_build(routes: Vec<Route>) -> Result<()> {
    let map = DashMap::new();
    for r in &routes {
        map.insert(r.id.clone(), r.clone());
    }

    let mut radix_router = RadixRouter::new()?;
    radix_router.add_routes(routes.iter().map(|r| r.into()).collect())?;

    let new_cache = RoutesCache {
        routes_map: Arc::new(map),
        routes_radix: Arc::new(radix_router),
    };

    ROUTES_CACHE
        .get()
        .ok_or_else(|| anyhow!("Cache not initialized"))?
        .store(Arc::new(new_cache));

    Ok(())
}

pub fn inc_update(events: Vec<RoutesCacheEvent>) -> Result<()> {
    if events.is_empty() {
        return Ok(());
    }

    let current_cache = local()?;

    // Try to take ownership of DashMap and RadixRouter for zero-copy optimization
    let (new_map, mut new_radix) = match (
        Arc::try_unwrap(Arc::clone(&current_cache.routes_map)),
        Arc::try_unwrap(Arc::clone(&current_cache.routes_radix)),
    ) {
        (Ok(map), Ok(router)) => {
            // Best case: no other references, reuse both directly
            (map, router)
        }
        (Ok(map), Err(_)) => {
            // Reuse map, rebuild radix
            let mut router = RadixRouter::new()?;
            let all_routes: Vec<_> = map.iter().map(|entry| entry.value().into()).collect();
            router.add_routes(all_routes)?;
            (map, router)
        }
        (Err(_), Ok(router)) => {
            // Clone map, reuse radix (rare case)
            let new_map = DashMap::new();
            for entry in current_cache.routes_map.iter() {
                new_map.insert(entry.key().clone(), entry.value().clone());
            }
            (new_map, router)
        }
        (Err(_), Err(_)) => {
            // Worst case: clone both
            let new_map = DashMap::new();
            for entry in current_cache.routes_map.iter() {
                new_map.insert(entry.key().clone(), entry.value().clone());
            }
            let mut router = RadixRouter::new()?;
            let all_routes: Vec<_> = new_map.iter().map(|entry| entry.value().into()).collect();
            router.add_routes(all_routes)?;
            (new_map, router)
        }
    };

    // Process all events on the new map and radix
    for event in events {
        match event.event_type {
            CacheEventType::Create => {
                let route = event
                    .route
                    .ok_or_else(|| anyhow!("Route is required for Create event"))?;

                new_map.insert(route.id.clone(), route.clone());
                new_radix.add_route((&route).into())?;
            }
            CacheEventType::Update => {
                let route = event
                    .route
                    .ok_or_else(|| anyhow!("Route is required for Update event"))?;
                let history_route = event
                    .history_route
                    .ok_or_else(|| anyhow!("History route is required for Update event"))?;

                new_map.insert(route.id.clone(), route.clone());
                new_radix.update_route((&history_route).into(), (&route).into())?;
            }
            CacheEventType::Delete => {
                let route = event
                    .route
                    .ok_or_else(|| anyhow!("Route is required for Delete event"))?;

                new_map.remove(&route.id);
                new_radix.delete_route((&route).into())?;
            }
        }
    }

    // Create new cache with updated map and radix
    let new_cache = RoutesCache {
        routes_map: Arc::new(new_map),
        routes_radix: Arc::new(new_radix),
    };

    ROUTES_CACHE
        .get()
        .ok_or_else(|| anyhow!("Cache not initialized"))?
        .store(Arc::new(new_cache));

    Ok(())
}

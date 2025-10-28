use std::sync::Arc;

use arc_swap::ArcSwap;
use dashmap::DashMap;
use once_cell::sync::OnceCell;

use crate::cache::CacheEventType;
use crate::cro::Upstream;
use anyhow::{anyhow, Result};

static UPSTREAMS_CACHE: OnceCell<ArcSwap<UpstreamsCache>> = OnceCell::new();

pub struct UpstreamsCache {
    pub upstreams_map: Arc<DashMap<String, Upstream>>,
}

pub struct UpstreamsCacheEvent {
    pub event_type: CacheEventType,
    pub upstream: Option<Upstream>,
}

pub fn local() -> Result<Arc<UpstreamsCache>> {
    // If cache is not initialized, try to initialize it automatically
    if UPSTREAMS_CACHE.get().is_none() {
        init_cache()?;
    }

    // At this point, cache must be initialized
    match UPSTREAMS_CACHE.get() {
        Some(cache_swap) => Ok(cache_swap.load_full().clone()),
        None => Err(anyhow!("Cache initialization failed")),
    }
}

pub fn init_cache() -> Result<()> {
    let init = UpstreamsCache {
        upstreams_map: Arc::new(DashMap::new()),
    };

    UPSTREAMS_CACHE
        .set(ArcSwap::new(Arc::new(init)))
        .map_err(|_| anyhow!("Cache already initialized"))?;

    Ok(())
}

pub fn full_build(upstreams: Vec<Upstream>) -> Result<()> {
    let map = DashMap::new();
    for u in &upstreams {
        map.insert(u.id.clone(), u.clone());
    }

    let new_cache = UpstreamsCache {
        upstreams_map: Arc::new(map),
    };

    UPSTREAMS_CACHE
        .get()
        .ok_or_else(|| anyhow!("Cache not initialized"))?
        .store(Arc::new(new_cache));

    Ok(())
}

pub fn inc_update(events: Vec<UpstreamsCacheEvent>) -> Result<()> {
    if events.is_empty() {
        return Ok(());
    }

    let current_cache = local()?;

    // Try to take ownership of DashMap for zero-copy optimization
    let new_map = match Arc::try_unwrap(Arc::clone(&current_cache.upstreams_map)) {
        Ok(map) => {
            // Best case: no other references, reuse directly
            map
        }
        Err(_) => {
            // Clone the map
            let new_map = DashMap::new();
            for entry in current_cache.upstreams_map.iter() {
                new_map.insert(entry.key().clone(), entry.value().clone());
            }
            new_map
        }
    };

    // Process all events on the new map
    for event in events {
        match event.event_type {
            CacheEventType::Create => {
                let upstream = event
                    .upstream
                    .ok_or_else(|| anyhow!("Upstream is required for Create event"))?;

                new_map.insert(upstream.id.clone(), upstream);
            }
            CacheEventType::Update => {
                let upstream = event
                    .upstream
                    .ok_or_else(|| anyhow!("Upstream is required for Update event"))?;

                new_map.insert(upstream.id.clone(), upstream);
            }
            CacheEventType::Delete => {
                let upstream = event
                    .upstream
                    .ok_or_else(|| anyhow!("Upstream is required for Delete event"))?;

                new_map.remove(&upstream.id);
            }
        }
    }

    // Create new cache with updated map
    let new_cache = UpstreamsCache {
        upstreams_map: Arc::new(new_map),
    };

    UPSTREAMS_CACHE
        .get()
        .ok_or_else(|| anyhow!("Cache not initialized"))?
        .store(Arc::new(new_cache));

    Ok(())
}

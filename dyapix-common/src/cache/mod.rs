pub mod routes_cache;
pub mod upstreams_cache;

pub enum CacheEventType {
    Create,
    Update,
    Delete,
}
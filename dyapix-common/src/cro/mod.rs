mod route;
mod tls_cert;
mod upstream;

use serde::{de::DeserializeOwned, Serialize};

// Re-export resource types
pub use route::Route;
pub use tls_cert::TlsCert;
pub use upstream::{LoadBalanceType, Scheme, Timeout, Upstream, UpstreamNode};

// CRO kind constants - use these to avoid magic strings
pub const CRO_KIND_ROUTE: &str = "Route";
pub const CRO_KIND_UPSTREAM: &str = "Upstream";
pub const CRO_KIND_TLS_CERT: &str = "TlsCert";

/// Core Resource Object trait
/// All resources (Route, Upstream, TlsCert, etc.) must implement this trait
pub trait CRO: Serialize + DeserializeOwned + Clone + Send + Sync + 'static {
    /// Get the CRO kind/type name
    fn cro_kind() -> &'static str
    where
        Self: Sized;

    /// Get the resource ID
    fn id(&self) -> &str;
}

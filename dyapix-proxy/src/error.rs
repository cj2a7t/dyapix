pub const ERROR_ROUTE_NOT_FOUND: &str = "Route Not Found";
pub const ERROR_HOST_NOT_FOUND: &str = "Host Not Found";
pub const ERROR_UPSTREAM_NOT_FOUND: &str = "Upstream Not Found";
pub const ERROR_CACHE_NOT_INITIALIZED: &str = "Cache Not Initialized";

pub trait AnyhowResultExt<T> {
    fn to_pingora_result(self, message: &'static str) -> pingora::Result<T>;
}

impl<T> AnyhowResultExt<T> for anyhow::Result<T> {
    fn to_pingora_result(self, message: &'static str) -> pingora::Result<T> {
        self.map_err(|_| pingora::Error::new(pingora::ErrorType::Custom(message)))
    }
}

use anyhow::Result;
use async_trait::async_trait;

use crate::cro::CRO;

#[async_trait]
pub trait DataSource: Send + Sync {
    /// Fetch all data and start watching for changes
    async fn fetch_and_watch(&self) -> Result<()>;

    /// Put (insert or update) a resource
    async fn put<T>(&self, resource: &T) -> Result<T>
    where
        T: CRO;

    /// Get a resource by id
    async fn get<T>(&self, id: &str) -> Result<T>
    where
        T: CRO;

    /// Delete a resource by id
    async fn delete<T>(&self, id: &str) -> Result<bool>
    where
        T: CRO;

    /// Get all resources of a specific type
    async fn get_all<T>(&self) -> Result<Vec<T>>
    where
        T: CRO;
}

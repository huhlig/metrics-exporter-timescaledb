use crate::error::Error;
use crate::models::Metric;

#[async_trait::async_trait]
pub trait MetricRepository: Send + Sync {
    async fn insert(&self, metric: &Metric) -> Result<(), Error>;
    async fn insert_batch(&self, metrics: &[Metric]) -> Result<(), Error>;
    async fn query(
        &self,
        name: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Metric>, Error>;
    async fn list(&self) -> Result<Vec<String>, Error>;
    async fn get(&self, id: i64) -> Result<Option<Metric>, Error>;
    async fn update(&self, metric: &Metric) -> Result<(), Error>;
    async fn delete(&self, id: i64) -> Result<(), Error>;
    async fn delete_before(&self, timestamp: chrono::DateTime<chrono::Utc>) -> Result<u64, Error>;
}

pub mod timescaledb;

pub use timescaledb::TimescaleRepository;

use crate::models::Metric;
use crate::error::Error;

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
}

use crate::error::Error;
use crate::models::{Metric, MetricValue};
use crate::repository::MetricRepository;
use sqlx::PgPool;
use std::collections::HashMap;

#[derive(Clone)]
pub struct TimescaleRepository {
    pool: PgPool,
}

impl TimescaleRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn run_migrations(&self) -> Result<(), Error> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct MetricRow {
    pub id: i64,
    pub name: String,
    pub value: serde_json::Value,
    pub labels: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl MetricRow {
    pub fn into_metric(self) -> Result<Metric, Error> {
        let value: MetricValue = serde_json::from_value(self.value)?;
        let labels: HashMap<String, String> = serde_json::from_value(self.labels)?;

        Ok(Metric {
            id: Some(self.id),
            name: self.name,
            value,
            labels,
            timestamp: self.timestamp,
        })
    }
}

#[async_trait::async_trait]
impl MetricRepository for TimescaleRepository {
    async fn insert(&self, metric: &Metric) -> Result<(), Error> {
        let value_json = serde_json::to_value(&metric.value)?;
        let labels_json = serde_json::to_value(&metric.labels)?;

        sqlx::query(
            r#"
            INSERT INTO metrics (name, value, labels, timestamp)
            VALUES ($1, $2, $3, $4)
            "#,
        )
        .bind(&metric.name)
        .bind(&value_json)
        .bind(&labels_json)
        .bind(metric.timestamp)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn insert_batch(&self, metrics: &[Metric]) -> Result<(), Error> {
        if metrics.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        let mut values: Vec<String> = Vec::new();
        let mut param_idx = 1;

        for _ in metrics {
            values.push(format!(
                "(${}, ${}, ${}, ${})",
                param_idx,
                param_idx + 1,
                param_idx + 2,
                param_idx + 3
            ));
            param_idx += 4;
        }

        let query = format!(
            "INSERT INTO metrics (name, value, labels, timestamp) VALUES {}",
            values.join(", ")
        );

        let mut query_builder = sqlx::query(&query);
        for metric in metrics {
            let value_json = serde_json::to_value(&metric.value)?;
            let labels_json = serde_json::to_value(&metric.labels)?;
            query_builder = query_builder
                .bind(metric.name.clone())
                .bind(value_json)
                .bind(labels_json)
                .bind(metric.timestamp);
        }

        query_builder.execute(&mut *tx).await?;
        tx.commit().await?;

        Ok(())
    }

    async fn query(
        &self,
        name: &str,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<Metric>, Error> {
        let rows: Vec<MetricRow> = sqlx::query_as::<_, MetricRow>(
            r#"
            SELECT id, name, value, labels, timestamp
            FROM metrics
            WHERE name = $1 AND timestamp >= $2 AND timestamp <= $3
            ORDER BY timestamp ASC
            "#,
        )
        .bind(name)
        .bind(start)
        .bind(end)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(MetricRow::into_metric).collect()
    }

    async fn list(&self) -> Result<Vec<String>, Error> {
        let rows =
            sqlx::query_scalar::<_, String>("SELECT DISTINCT name FROM metrics ORDER BY name")
                .fetch_all(&self.pool)
                .await?;

        Ok(rows)
    }

    async fn get(&self, id: i64) -> Result<Option<Metric>, Error> {
        let row: Option<MetricRow> = sqlx::query_as::<_, MetricRow>(
            "SELECT id, name, value, labels, timestamp FROM metrics WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        row.map(MetricRow::into_metric).transpose()
    }

    async fn update(&self, metric: &Metric) -> Result<(), Error> {
        let id = metric
            .id
            .ok_or_else(|| Error::InvalidInput("Metric id is required for update".into()))?;
        let value_json = serde_json::to_value(&metric.value)?;
        let labels_json = serde_json::to_value(&metric.labels)?;

        sqlx::query(
            r#"
            UPDATE metrics
            SET name = $1, value = $2, labels = $3, timestamp = $4
            WHERE id = $5
            "#,
        )
        .bind(&metric.name)
        .bind(&value_json)
        .bind(&labels_json)
        .bind(metric.timestamp)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn delete(&self, id: i64) -> Result<(), Error> {
        sqlx::query("DELETE FROM metrics WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    async fn delete_before(&self, timestamp: chrono::DateTime<chrono::Utc>) -> Result<u64, Error> {
        let result = sqlx::query("DELETE FROM metrics WHERE timestamp < $1")
            .bind(timestamp)
            .execute(&self.pool)
            .await?;

        Ok(result.rows_affected())
    }
}

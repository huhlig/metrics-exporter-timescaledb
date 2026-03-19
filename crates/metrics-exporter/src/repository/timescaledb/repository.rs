use crate::models::Metric;
use crate::repository::MetricRepository;
use crate::error::Error;
use crate::models::MetricValue;
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
struct MetricRow {
    name: String,
    value: serde_json::Value,
    labels: serde_json::Value,
    timestamp: chrono::DateTime<chrono::Utc>,
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
        let mut tx = self.pool.begin().await?;

        for metric in metrics {
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
            .execute(&mut *tx)
            .await?;
        }

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
            SELECT name, value, labels, timestamp
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

        rows.into_iter().map(Metric::from_row).collect()
    }

    async fn list(&self) -> Result<Vec<String>, Error> {
        let rows = sqlx::query_scalar::<_, String>(
            "SELECT DISTINCT name FROM metrics ORDER BY name",
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
    }
}

impl Metric {
    fn from_row(row: MetricRow) -> Result<Self, Error> {
        let value: MetricValue = serde_json::from_value(row.value)?;
        let labels: HashMap<String, String> = serde_json::from_value(row.labels)?;

        Ok(Self {
            name: row.name,
            value,
            labels,
            timestamp: row.timestamp,
        })
    }
}

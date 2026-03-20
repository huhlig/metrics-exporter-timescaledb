use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AggregationInterval {
    OneMinute,
    FiveMinutes,
    OneHour,
}

impl AggregationInterval {
    pub fn as_str(&self) -> &'static str {
        match self {
            AggregationInterval::OneMinute => "1m",
            AggregationInterval::FiveMinutes => "5m",
            AggregationInterval::OneHour => "1h",
        }
    }

    pub fn view_name(&self) -> &'static str {
        match self {
            AggregationInterval::OneMinute => "metrics_1m",
            AggregationInterval::FiveMinutes => "metrics_5m",
            AggregationInterval::OneHour => "metrics_1h",
        }
    }

    pub fn time_bucket(&self) -> &'static str {
        match self {
            AggregationInterval::OneMinute => "1 minute",
            AggregationInterval::FiveMinutes => "5 minutes",
            AggregationInterval::OneHour => "1 hour",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetric {
    pub name: String,
    pub bucket: DateTime<Utc>,
    pub labels: serde_json::Value,
    pub metric_type: String,
    pub avg_value: Option<f64>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: DateTime<Utc>,
    pub value: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesData {
    pub name: String,
    pub interval: String,
    pub data: Vec<TimeSeriesPoint>,
}

pub struct AggregationRepository {
    pool: PgPool,
}

impl AggregationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn query_aggregated(
        &self,
        name: &str,
        interval: AggregationInterval,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<AggregatedMetric>, sqlx::Error> {
        let view_name = interval.view_name();

        let query = format!(
            r#"
            SELECT name, bucket, labels, metric_type,
                   avg_value, min_value, max_value, count
            FROM {}
            WHERE name = $1 AND bucket >= $2 AND bucket <= $3
            ORDER BY bucket ASC
            "#,
            view_name
        );

        let rows: Vec<AggregatedMetricRow> = sqlx::query_as(&query)
            .bind(name)
            .bind(start)
            .bind(end)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(AggregatedMetric::from).collect())
    }

    pub async fn query_timeseries(
        &self,
        name: &str,
        interval: AggregationInterval,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        field: &str,
    ) -> Result<TimeSeriesData, sqlx::Error> {
        let view_name = interval.view_name();
        let field_column = match field {
            "avg" => "avg_value",
            "min" => "min_value",
            "max" => "max_value",
            "count" => "count",
            _ => "avg_value",
        };

        let query = format!(
            r#"
            SELECT bucket AS timestamp, {} AS value
            FROM {}
            WHERE name = $1 AND bucket >= $2 AND bucket <= $3
            ORDER BY bucket ASC
            "#,
            field_column, view_name
        );

        let rows: Vec<TimeSeriesRow> = sqlx::query_as(&query)
            .bind(name)
            .bind(start)
            .bind(end)
            .fetch_all(&self.pool)
            .await?;

        Ok(TimeSeriesData {
            name: name.to_string(),
            interval: interval.as_str().to_string(),
            data: rows
                .into_iter()
                .map(|r| TimeSeriesPoint {
                    timestamp: r.timestamp,
                    value: r.value,
                })
                .collect(),
        })
    }

    pub async fn query_multiple_series(
        &self,
        names: &[String],
        interval: AggregationInterval,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        field: &str,
    ) -> Result<Vec<TimeSeriesData>, sqlx::Error> {
        let mut results = Vec::new();

        for name in names {
            let series = self
                .query_timeseries(name, interval, start, end, field)
                .await?;
            results.push(series);
        }

        Ok(results)
    }

    pub async fn get_metric_types(&self) -> Result<Vec<String>, sqlx::Error> {
        let rows: Vec<(String,)> = sqlx::query_as(
            r#"
            SELECT DISTINCT metric_type FROM metrics_1m ORDER BY metric_type
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|(v,)| v).collect())
    }

    pub async fn get_available_intervals(&self) -> Vec<AggregationInterval> {
        vec![
            AggregationInterval::OneMinute,
            AggregationInterval::FiveMinutes,
            AggregationInterval::OneHour,
        ]
    }
}

#[derive(Debug, sqlx::FromRow)]
struct AggregatedMetricRow {
    name: String,
    bucket: DateTime<Utc>,
    labels: serde_json::Value,
    metric_type: String,
    avg_value: Option<f64>,
    min_value: Option<f64>,
    max_value: Option<f64>,
    count: i64,
}

impl From<AggregatedMetricRow> for AggregatedMetric {
    fn from(row: AggregatedMetricRow) -> Self {
        Self {
            name: row.name,
            bucket: row.bucket,
            labels: row.labels,
            metric_type: row.metric_type,
            avg_value: row.avg_value,
            min_value: row.min_value,
            max_value: row.max_value,
            count: row.count,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct TimeSeriesRow {
    timestamp: DateTime<Utc>,
    value: Option<f64>,
}

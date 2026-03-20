use chrono::Utc;
use metrics_exporter::{
    AggregationInterval, AggregationRepository,
    models::{Metric, MetricValue},
    repository::TimescaleRepository,
};
use sqlx::postgres::PgPoolOptions;
use std::collections::HashMap;

fn test_pool() -> sqlx::PgPool {
    let database_url = std::env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5431/metrics_test".to_string());

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .expect("Failed to connect to test database")
}

#[sqlx::test]
async fn test_insert_and_list() {
    let pool = test_pool();
    let repo = TimescaleRepository::new(pool);

    let mut labels = HashMap::new();
    labels.insert("host".to_string(), "localhost".to_string());

    let metric = Metric {
        id: None,
        name: "test_metric".to_string(),
        value: MetricValue::Gauge(42.0),
        labels,
        timestamp: Utc::now(),
    };

    repo.insert(&metric).await.unwrap();

    let metrics = repo.list().await.unwrap();
    assert!(metrics.contains(&"test_metric".to_string()));

    sqlx::query("DELETE FROM metrics WHERE name = 'test_metric'")
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_insert_batch() {
    let pool = test_pool();
    let repo = TimescaleRepository::new(pool);

    let mut labels = HashMap::new();
    labels.insert("host".to_string(), "test_host".to_string());

    let metrics: Vec<Metric> = (0..5)
        .map(|i| Metric {
            id: None,
            name: "batch_metric".to_string(),
            value: MetricValue::Counter(i as i64),
            labels: labels.clone(),
            timestamp: Utc::now(),
        })
        .collect();

    repo.insert_batch(&metrics).await.unwrap();

    let list = repo.list().await.unwrap();
    assert!(list.contains(&"batch_metric".to_string()));

    sqlx::query("DELETE FROM metrics WHERE name = 'batch_metric'")
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_query_by_time_range() {
    let pool = test_pool();
    let repo = TimescaleRepository::new(pool);

    let mut labels = HashMap::new();
    labels.insert("service".to_string(), "api".to_string());

    let now = Utc::now();
    let metric = Metric {
        id: None,
        name: "query_test_metric".to_string(),
        value: MetricValue::Gauge(100.0),
        labels,
        timestamp: now,
    };

    repo.insert(&metric).await.unwrap();

    let results = repo
        .query(
            "query_test_metric",
            now - chrono::Duration::hours(1),
            now + chrono::Duration::hours(1),
        )
        .await
        .unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "query_test_metric");

    sqlx::query("DELETE FROM metrics WHERE name = 'query_test_metric'")
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_get_by_id() {
    let pool = test_pool();
    let repo = TimescaleRepository::new(pool);

    let metric = Metric {
        id: None,
        name: "get_by_id_metric".to_string(),
        value: MetricValue::Gauge(123.456),
        labels: HashMap::new(),
        timestamp: Utc::now(),
    };

    repo.insert(&metric).await.unwrap();

    let all = repo
        .query(
            "get_by_id_metric",
            Utc::now() - chrono::Duration::hours(1),
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();

    if let Some(inserted) = all.first() {
        if let Some(id) = inserted.id {
            let retrieved = repo.get(id).await.unwrap();
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().name, "get_by_id_metric");
        }
    }

    sqlx::query("DELETE FROM metrics WHERE name = 'get_by_id_metric'")
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_delete() {
    let pool = test_pool();
    let repo = TimescaleRepository::new(pool);

    let metric = Metric {
        id: None,
        name: "delete_test_metric".to_string(),
        value: MetricValue::Counter(999),
        labels: HashMap::new(),
        timestamp: Utc::now(),
    };

    repo.insert(&metric).await.unwrap();

    let all = repo
        .query(
            "delete_test_metric",
            Utc::now() - chrono::Duration::hours(1),
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();

    if let Some(metric_with_id) = all.first() {
        if let Some(id) = metric_with_id.id {
            repo.delete(id).await.unwrap();

            let after_delete = repo.get(id).await.unwrap();
            assert!(after_delete.is_none());
        }
    }
}

#[sqlx::test]
async fn test_update() {
    let pool = test_pool();
    let repo = TimescaleRepository::new(pool);

    let metric = Metric {
        id: None,
        name: "update_test_metric".to_string(),
        value: MetricValue::Gauge(10.0),
        labels: HashMap::new(),
        timestamp: Utc::now(),
    };

    repo.insert(&metric).await.unwrap();

    let all = repo
        .query(
            "update_test_metric",
            Utc::now() - chrono::Duration::hours(1),
            Utc::now() + chrono::Duration::hours(1),
        )
        .await
        .unwrap();

    if let Some(mut to_update) = all.first().cloned() {
        to_update.value = MetricValue::Gauge(20.0);
        repo.update(&to_update).await.unwrap();

        let updated = repo
            .query(
                "update_test_metric",
                Utc::now() - chrono::Duration::hours(1),
                Utc::now() + chrono::Duration::hours(1),
            )
            .await
            .unwrap();

        if let Some(new_value) = updated.first() {
            if let MetricValue::Gauge(val) = &new_value.value {
                assert_eq!(*val, 20.0);
            }
        }
    }

    sqlx::query("DELETE FROM metrics WHERE name = 'update_test_metric'")
        .execute(&pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn test_aggregation_repository() {
    let pool = test_pool();
    let agg_repo = AggregationRepository::new(pool);

    let intervals = agg_repo.get_available_intervals().await;
    assert_eq!(intervals.len(), 3);
    assert!(intervals.contains(&AggregationInterval::OneMinute));
    assert!(intervals.contains(&AggregationInterval::FiveMinutes));
    assert!(intervals.contains(&AggregationInterval::OneHour));
}

#[test]
fn test_aggregation_interval_as_str() {
    assert_eq!(AggregationInterval::OneMinute.as_str(), "1m");
    assert_eq!(AggregationInterval::FiveMinutes.as_str(), "5m");
    assert_eq!(AggregationInterval::OneHour.as_str(), "1h");
}

#[test]
fn test_aggregation_interval_view_name() {
    assert_eq!(AggregationInterval::OneMinute.view_name(), "metrics_1m");
    assert_eq!(AggregationInterval::FiveMinutes.view_name(), "metrics_5m");
    assert_eq!(AggregationInterval::OneHour.view_name(), "metrics_1h");
}

#[test]
fn test_metric_value_serialization() {
    use serde_json;

    let gauge = MetricValue::Gauge(42.5);
    let json = serde_json::to_string(&gauge).unwrap();
    assert!(json.contains("Gauge"));
    assert!(json.contains("42.5"));

    let counter = MetricValue::Counter(100);
    let json = serde_json::to_string(&counter).unwrap();
    assert!(json.contains("Counter"));
    assert!(json.contains("100"));
}

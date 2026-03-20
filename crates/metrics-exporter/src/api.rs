use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::trace::TraceLayer;
use utoipa::{OpenApi, ToSchema};

use crate::aggregation::AggregationInterval;
use crate::repository::{MetricRepository, TimescaleRepository};

#[derive(Clone)]
pub struct AppState {
    pool: Arc<PgPool>,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool: Arc::new(pool),
        }
    }

    pub fn repository(&self) -> TimescaleRepository {
        TimescaleRepository::new((*self.pool).clone())
    }

    pub fn aggregation_repository(&self) -> crate::aggregation::AggregationRepository {
        crate::aggregation::AggregationRepository::new((*self.pool).clone())
    }
}

pub fn router(pool: PgPool) -> Router {
    let state = AppState::new(pool);

    let api_router = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(list_metrics))
        .route("/metrics/{name}", get(get_metric))
        .route("/metrics/{name}/timeseries", get(get_timeseries))
        .route("/metrics/{name}/aggregated", get(get_aggregated))
        .route("/docs/openapi.json", get(openapi_json));

    Router::new()
        .nest("/api", api_router.with_state(state))
        .route("/index.html", get(index_html))
        .route("/", get(index_html))
        .layer(TraceLayer::new_for_http())
}

#[derive(OpenApi)]
#[openapi(
    paths(health, list_metrics, get_metric, get_timeseries, get_aggregated),
    components(schemas(
        MetricListResponse,
        MetricDetailResponse,
        TimeSeriesResponse,
        AggregatedResponse,
        TimeSeriesPoint,
        AggregateSummary,
        ErrorResponse,
        TimeRangeQuery,
        AggregatedQuery
    )),
    tags(
        (name = "metrics", description = "Metrics management endpoints"),
        (name = "health", description = "Health check endpoints")
    ),
    info(
        title = "Metrics Exporter API",
        version = "0.1.0",
        description = "REST API for querying metrics stored in TimescaleDB with automatic downsampling and aggregation."
    )
)]
pub struct ApiDoc;

#[utoipa::path(
    get,
    path = "/api/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy", body = String)
    )
)]
async fn health() -> &'static str {
    "OK"
}

#[utoipa::path(
    get,
    path = "/api/metrics",
    tag = "metrics",
    responses(
        (status = 200, description = "List of metric names", body = MetricListResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn list_metrics(
    State(state): State<AppState>,
) -> Result<Json<MetricListResponse>, crate::Error> {
    let repo = state.repository();
    let metrics = repo.list().await?;
    Ok(Json(MetricListResponse { metrics }))
}

#[utoipa::path(
    get,
    path = "/api/metrics/{name}",
    tag = "metrics",
    params(
        ("name" = String, Path, description = "Metric name")
    ),
    responses(
        (status = 200, description = "Metric details", body = MetricDetailResponse),
        (status = 404, description = "Metric not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn get_metric(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<MetricDetailResponse>, crate::Error> {
    let repo = state.repository();
    let end = Utc::now();
    let start = end - Duration::days(30);

    let metrics: Vec<crate::models::Metric> = repo.query(&name, start, end).await?;

    if metrics.is_empty() {
        return Err(crate::Error::NotFound(format!(
            "Metric '{}' not found",
            name
        )));
    }

    let count = metrics.len() as i64;
    let first_timestamp = metrics.first().map(|m| m.timestamp);
    let last_timestamp = metrics.last().map(|m| m.timestamp);

    Ok(Json(MetricDetailResponse {
        name,
        count,
        first_timestamp,
        last_timestamp,
    }))
}

#[utoipa::path(
    get,
    path = "/api/metrics/{name}/timeseries",
    tag = "metrics",
    params(
        ("name" = String, Path, description = "Metric name"),
        ("start" = Option<DateTime<Utc>>, Query, description = "Start time (ISO 8601)"),
        ("end" = Option<DateTime<Utc>>, Query, description = "End time (ISO 8601)")
    ),
    responses(
        (status = 200, description = "Time series data", body = TimeSeriesResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn get_timeseries(
    Path(name): Path<String>,
    Query(params): Query<TimeRangeQuery>,
    State(state): State<AppState>,
) -> Result<Json<TimeSeriesResponse>, crate::Error> {
    let end = params.end.unwrap_or_else(Utc::now);
    let start = params.start.unwrap_or_else(|| end - Duration::hours(24));

    let interval = params.interval_hint(&start, &end);

    let agg_repo = state.aggregation_repository();
    let time_series = agg_repo
        .query_timeseries(&name, interval, start, end, "avg")
        .await?;

    Ok(Json(TimeSeriesResponse {
        name,
        interval: interval.as_str().to_string(),
        start,
        end,
        data: time_series
            .data
            .into_iter()
            .map(|p| TimeSeriesPoint {
                timestamp: p.timestamp,
                value: p.value,
            })
            .collect(),
    }))
}

#[utoipa::path(
    get,
    path = "/api/metrics/{name}/aggregated",
    tag = "metrics",
    params(
        ("name" = String, Path, description = "Metric name"),
        ("start" = Option<DateTime<Utc>>, Query, description = "Start time (ISO 8601)"),
        ("end" = Option<DateTime<Utc>>, Query, description = "End time (ISO 8601)"),
        ("interval" = Option<String>, Query, description = "Aggregation interval: 1m, 5m, or 1h")
    ),
    responses(
        (status = 200, description = "Aggregated metric data", body = AggregatedResponse),
        (status = 400, description = "Invalid interval", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse)
    )
)]
async fn get_aggregated(
    Path(name): Path<String>,
    Query(params): Query<AggregatedQuery>,
    State(state): State<AppState>,
) -> Result<Json<AggregatedResponse>, crate::Error> {
    let end = params.end.unwrap_or_else(Utc::now);
    let start = params.start.unwrap_or_else(|| end - Duration::hours(24));

    let interval = match params.interval.as_deref() {
        Some("1m") | Some("1minute") | Some("minute") => AggregationInterval::OneMinute,
        Some("5m") | Some("5minute") | Some("5minutes") => AggregationInterval::FiveMinutes,
        Some("1h") | Some("1hour") | Some("hour") => AggregationInterval::OneHour,
        None => select_interval(&start, &end),
        Some(s) => return Err(crate::Error::BadRequest(format!("Invalid interval: {}", s))),
    };

    let agg_repo = state.aggregation_repository();
    let aggregated = agg_repo
        .query_aggregated(&name, interval, start, end)
        .await?;

    let bucket_count = aggregated.len();
    let aggregates = aggregated
        .into_iter()
        .map(|m| AggregateSummary {
            bucket: m.bucket,
            avg: m.avg_value,
            min: m.min_value,
            max: m.max_value,
            count: m.count,
        })
        .collect();

    Ok(Json(AggregatedResponse {
        name,
        interval: interval.as_str().to_string(),
        bucket_count,
        aggregates,
    }))
}

async fn openapi_json() -> Response {
    let json = ApiDoc::openapi().to_pretty_json().unwrap();
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(axum::body::Body::from(json))
        .unwrap()
}

async fn index_html() -> Response {
    let html = include_str!("../../../static/index.html");
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html")
        .body(html.into())
        .unwrap()
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({"start": "2024-01-01T00:00:00Z", "end": "2024-01-02T00:00:00Z"}))]
struct TimeRangeQuery {
    #[schema(value_type = Option<String>, example = "2024-01-01T00:00:00Z")]
    start: Option<DateTime<Utc>>,
    #[schema(value_type = Option<String>, example = "2024-01-02T00:00:00Z")]
    end: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
#[schema(example = json!({"start": "2024-01-01T00:00:00Z", "end": "2024-01-02T00:00:00Z", "interval": "1h"}))]
struct AggregatedQuery {
    #[schema(value_type = Option<String>, example = "2024-01-01T00:00:00Z")]
    start: Option<DateTime<Utc>>,
    #[schema(value_type = Option<String>, example = "2024-01-02T00:00:00Z")]
    end: Option<DateTime<Utc>>,
    #[schema(example = "1h")]
    interval: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({"metrics": ["cpu_usage", "memory_usage", "request_count"]}))]
struct MetricListResponse {
    metrics: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
struct MetricDetailResponse {
    name: String,
    count: i64,
    #[schema(value_type = Option<String>)]
    first_timestamp: Option<DateTime<Utc>>,
    #[schema(value_type = Option<String>)]
    last_timestamp: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, ToSchema)]
struct TimeSeriesResponse {
    name: String,
    interval: String,
    #[schema(value_type = String)]
    start: DateTime<Utc>,
    #[schema(value_type = String)]
    end: DateTime<Utc>,
    data: Vec<TimeSeriesPoint>,
}

#[derive(Debug, Serialize, ToSchema)]
struct AggregatedResponse {
    name: String,
    interval: String,
    bucket_count: usize,
    aggregates: Vec<AggregateSummary>,
}

#[derive(Debug, Serialize, ToSchema)]
struct TimeSeriesPoint {
    #[schema(value_type = String)]
    timestamp: DateTime<Utc>,
    value: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
struct AggregateSummary {
    #[schema(value_type = String)]
    bucket: DateTime<Utc>,
    avg: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    count: i64,
}

#[derive(Debug, Serialize, ToSchema)]
#[schema(example = json!({"error": "Resource not found"}))]
struct ErrorResponse {
    error: String,
}

impl IntoResponse for crate::Error {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            crate::Error::Database(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            crate::Error::Migration(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            crate::Error::Serialization(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            crate::Error::Anyhow(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
            crate::Error::InvalidInput(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            crate::Error::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            crate::Error::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
        };

        let body = Json(ErrorResponse { error: message });
        (status, body).into_response()
    }
}

fn select_interval(
    start: &chrono::DateTime<chrono::Utc>,
    end: &chrono::DateTime<chrono::Utc>,
) -> AggregationInterval {
    let duration = *end - *start;
    if duration <= Duration::hours(2) {
        AggregationInterval::OneMinute
    } else if duration <= Duration::days(2) {
        AggregationInterval::FiveMinutes
    } else {
        AggregationInterval::OneHour
    }
}

impl TimeRangeQuery {
    fn interval_hint(
        &self,
        start: &chrono::DateTime<chrono::Utc>,
        end: &chrono::DateTime<chrono::Utc>,
    ) -> AggregationInterval {
        let duration = *end - *start;
        if duration <= Duration::hours(2) {
            AggregationInterval::OneMinute
        } else if duration <= Duration::days(2) {
            AggregationInterval::FiveMinutes
        } else {
            AggregationInterval::OneHour
        }
    }
}

use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
struct AppState {
    repo: Arc<RwLock<Option<Arc<dyn metrics_exporter::MetricRepository>>>>,
}

pub fn router() -> Router {
    let state = AppState {
        repo: Arc::new(RwLock::new(None)),
    };

    Router::new()
        .route("/metrics", get(list_metrics))
        .route("/metrics/{name}", get(get_metric))
        .route("/metrics/{name}/timeseries", get(get_timeseries))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

#[derive(Debug, Deserialize)]
struct TimeRangeQuery {
    start: Option<chrono::DateTime<chrono::Utc>>,
    end: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize)]
struct MetricListResponse {
    metrics: Vec<String>,
}

async fn list_metrics(State(state): State<AppState>) -> Result<Json<MetricListResponse>, crate::Error> {
    let repo = state.repo.read().await;
    let repo = repo.as_ref().ok_or_else(|| crate::Error::Internal("Repository not initialized".into()))?;
    
    let metrics = repo.list().await
        .map_err(|e: metrics_exporter::Error| crate::Error::Internal(e.to_string()))?;
    
    Ok(Json(MetricListResponse { metrics }))
}

async fn get_metric(
    Path(name): Path<String>,
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, crate::Error> {
    Ok(Json(serde_json::json!({
        "name": name,
        "description": "Metric details endpoint"
    })))
}

async fn get_timeseries(
    Path(name): Path<String>,
    Query(params): Query<TimeRangeQuery>,
    _state: State<AppState>,
) -> Result<Json<serde_json::Value>, crate::Error> {
    let end = params.end.unwrap_or_else(chrono::Utc::now);
    let start = params.start.unwrap_or_else(|| end - chrono::Duration::hours(24));
    
    Ok(Json(serde_json::json!({
        "name": name,
        "start": start,
        "end": end,
        "data": []
    })))
}

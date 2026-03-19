use axum::Router;
use axum::routing::get;
use tower_http::cors::{Any, CorsLayer};

pub fn router() -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/health", get(health))
        .layer(cors)
}

async fn health() -> &'static str {
    "OK"
}

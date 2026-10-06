use crate::limiter::{Limiter, limit};
use axum::{Router, middleware::from_fn_with_state, routing::get};
use std::sync::Arc;

pub mod hammer;

async fn root() -> &'static str {
    "ok"
}

pub fn router(limiter: Arc<Limiter>) -> Router {
    Router::new()
        .route("/", get(root))
        .layer(from_fn_with_state(limiter, limit))
}

use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};

use crate::{handlers, state::AppState};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/upload", post(handlers::upload_file))
        .route("/upload/callback", post(handlers::qiniu_upload_callback))
        .route("/download/:id", get(handlers::download_file))
        .route("/files", get(handlers::list_files))
        .route("/files/:id", delete(handlers::delete_file))
        .route("/health", get(handlers::health_check))
    .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
        .with_state(state)
}

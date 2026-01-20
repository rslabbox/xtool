mod app;
mod handlers;
mod state;
mod storage;

use app::build_router;
use log::info;
use state::AppState;
use storage::init_temp_dir;
use std::env;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    dotenvy::dotenv().ok();

    info!("Starting transfer server...");

    init_temp_dir().expect("Failed to initialize temp directory");

    let state = AppState::new();
    let app = build_router(state);

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8080);
    let addr = format!("0.0.0.0:{}", port);
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

mod app;
mod handlers;
mod state;
mod storage;

use app::build_router;
use log::info;
use state::AppState;
use storage::init_temp_dir;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting transfer server...");

    init_temp_dir().expect("Failed to initialize temp directory");

    let state = AppState::new();
    let app = build_router(state);

    let addr = "0.0.0.0:3000";
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

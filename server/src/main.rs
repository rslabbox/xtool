mod app;
mod handlers;
mod state;
mod storage;
mod qiniu;

use app::build_router;
use log::{info, error};
use state::AppState;
use storage::init_temp_dir;
use std::{env, fs::OpenOptions};
use env_logger::Target;
use qiniu::QiniuClient;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let mut logger_builder = env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    );
    if let Ok(log_file) = env::var("LOG_FILE") {
        if !log_file.trim().is_empty() {
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file)
                .expect("Failed to open LOG_FILE");
            logger_builder.target(Target::Pipe(Box::new(file)));
        }
    }
    logger_builder.init();

    info!("Starting transfer server...");

    init_temp_dir().expect("Failed to initialize temp directory");

    let mut state = AppState::new();

    if let (Ok(ak), Ok(sk), Ok(domain), Ok(bucket)) = (
        env::var("QINIU_ACCESS_KEY"),
        env::var("QINIU_SECRET_KEY"),
        env::var("QINIU_DOMAIN"),
        env::var("QINIU_BUCKET"), // Changed from bucket_name to match likely env var
    ) {
        let scheme = env::var("QINIU_SCHEME").unwrap_or_else(|_| "http".to_string());
        let callback_url = env::var("QINIU_CALLBACK_URL")
            .unwrap_or_else(|_| "http://a.debin.cc:8080/upload/callback".to_string());
        
        info!("Qiniu configuration found. Bucket: {}", bucket);
        state.qiniu_config = Some(QiniuClient::new(ak, sk, domain, scheme, bucket, callback_url));
    } else {
        error!("Qiniu configuration missing (QINIU_ACCESS_KEY, QINIU_SECRET_KEY, QINIU_DOMAIN, QINIU_BUCKET)");
        // Depending on requirements, maybe we should panic or just run in memory mode?
        // User said "Upload to qiniu", so it is likely required.
        // But for development maybe optional?
    }

    // Spawn background cleanup task
    tokio::spawn(handlers::cleanup_expired_files_task(state.clone()));

    let app = build_router(state);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

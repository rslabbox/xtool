use axum::{
    body::Bytes,
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use log::{error, info};
use rand::Rng;
use std::{
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crate::{
    state::AppState,
    storage::{FileRecord, StorageType, ContentType},
};

const MAX_TEXT_SIZE: usize = 10 * 1024 * 1024; // 10MB for text
const MAX_FILE_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(serde::Serialize)]
pub struct UploadResponse {
    pub id: String,
    pub filename: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upload_url: Option<String>,
}

#[derive(serde::Serialize)]
pub struct DownloadResponse {
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub filename: Option<String>,
    pub content_type: ContentType,
}

#[derive(serde::Serialize)]
pub struct ListResponse {
    pub files: Vec<FileRecord>,
}

pub async fn health_check() -> &'static str {
    "OK"
}

pub async fn upload_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<UploadResponse>, StatusCode> {
    let upload_type = headers
        .get("x-upload-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("file"); // default to file

    let id = generate_token();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if upload_type == "text" {
        if body.len() > MAX_TEXT_SIZE {
            return Err(StatusCode::PAYLOAD_TOO_LARGE);
        }
        let content = String::from_utf8(body.to_vec()).map_err(|_| StatusCode::BAD_REQUEST)?;
        
        let mut files = state.files.lock().expect("State lock poisoned");
        files.insert(id.clone(), FileRecord {
            id: id.clone(),
            filename: None,
            content_type: ContentType::Text,
            storage: StorageType::Memory(content),
            uploaded_at: now,
        });
        
        info!("Text uploaded: id: {}", id);
        return Ok(Json(UploadResponse {
            id,
            filename: None,
            upload_token: None,
            upload_url: None,
        }));
    } else {
        // File upload - Qiniu
        let filename = headers
            .get("x-filename")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unnamed_file");

        let qiniu = state.qiniu_config.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
        
        let random_part = random_suffix();
        let save_as_name = format!("xtool_{}_{}_{}", id, random_part, now);
        let token_lifetime = Duration::from_secs(10 * 60);
        
        let upload_token = qiniu.generate_upload_token(&save_as_name, token_lifetime)
            .map_err(|e| {
                error!("Failed to generate qiniu token: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        info!("File upload prepared: {} (save_as: {})", filename, save_as_name);
        
        return Ok(Json(UploadResponse {
            id,
            filename: Some(filename.to_string()),
            upload_token: Some(upload_token),
            upload_url: None,
        }));
    }
}

#[derive(serde::Deserialize)]
pub struct QiniuCallbackPayload {
    pub key: String,
    pub fname: Option<String>,
}

pub async fn qiniu_upload_callback(
    State(state): State<AppState>,
    Form(payload): Form<QiniuCallbackPayload>,
) -> Result<Json<UploadResponse>, StatusCode> {
    let (id, filename) = parse_key_and_filename(&payload.key, payload.fname.as_deref());

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut files = state.files.lock().expect("State lock poisoned");
    files.insert(
        id.clone(),
        FileRecord {
            id: id.clone(),
            filename: Some(filename.clone()),
            content_type: ContentType::File,
            storage: StorageType::Qiniu(payload.key.clone()),
            uploaded_at: now,
        },
    );

    info!("Qiniu callback registered file: {} (id: {})", filename, id);

    Ok(Json(UploadResponse {
        id,
        filename: Some(filename),
        upload_token: None,
        upload_url: None,
    }))
}

pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, StatusCode> {
    let mut files = state.files.lock().expect("State lock poisoned");
    
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if let Some(record) = files.get(&id) {
        if now.saturating_sub(record.uploaded_at) > MAX_FILE_AGE.as_secs() {
            info!("File expired: {}", id);
            files.remove(&id);
            return Err(StatusCode::NOT_FOUND); 
        }
    }

    let record = files.get(&id).cloned().ok_or(StatusCode::NOT_FOUND)?;
    
    // Unlock early
    drop(files);

    match &record.storage {
        StorageType::Memory(content) => {
            let resp = DownloadResponse {
                url: None,
                content: Some(content.clone()),
                filename: None,
                content_type: record.content_type.clone(),
            };
            Ok(Json(resp).into_response())
        }
        StorageType::Qiniu(key) => {
             let qiniu = state.qiniu_config.as_ref().ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;
             let url = qiniu.get_download_url(key);
             
             let resp = DownloadResponse {
                url: Some(url),
                content: None,
                filename: record.filename.clone(),
                content_type: record.content_type.clone(),
            };
            Ok(Json(resp).into_response())
        }
    }
}

pub async fn list_files(State(state): State<AppState>) -> Json<ListResponse> {
    let files = state.files.lock().expect("State lock poisoned");
    let file_list: Vec<FileRecord> = files.values().cloned().collect();
    Json(ListResponse { files: file_list })
}

pub async fn delete_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let mut files = state.files.lock().expect("State lock poisoned");
    if files.remove(&id).is_some() {
        info!("File deleted: {}", id);
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

fn generate_token() -> String {
    let mut rng = rand::rng();
    let token: u32 = rng.random_range(100000..999999);
    token.to_string()
}

fn random_suffix() -> String {
    let mut rng = rand::rng();
    let token: u32 = rng.random_range(100000..999999);
    token.to_string()
}

fn sanitize_filename(filename: &str) -> String {
    let trimmed = filename.trim();
    if trimmed.is_empty() {
        return "file.bin".to_string();
    }
    trimmed
        .replace('/', "_")
        .replace('\\', "_")
}

fn parse_key_and_filename(key: &str, fallback_name: Option<&str>) -> (String, String) {
    let mut parts = key.split('_');
    let _prefix = parts.next();
    let id = parts.next().unwrap_or_default().to_string();

    let filename = fallback_name
        .map(|name| sanitize_filename(name))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "file.bin".to_string());

    let id = if id.is_empty() { generate_token() } else { id };

    (id, filename)
}

pub async fn cleanup_expired_files_task(state: AppState) {
    // Check every hour
    let mut interval = tokio::time::interval(Duration::from_secs(3600)); 
    
    // First tick completes immediately, so we might want to skip it or just let it run once at startup
    interval.tick().await;

    loop {
        interval.tick().await;
        info!("Running cleanup task...");
        
        // Use a block to ensure lock is dropped quickly
        let removed_count = {
            let mut files = match state.files.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    error!("State lock poisoned during cleanup");
                    poisoned.into_inner()
                }
            };

            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let initial_count = files.len();
            files.retain(|id, record| {
                let age = now.saturating_sub(record.uploaded_at);
                if age > MAX_FILE_AGE.as_secs() {
                    info!("Cleanup removing expired file: {} (age: {}s)", id, age);
                    false
                } else {
                    true
                }
            });
            initial_count - files.len()
        };
        
        if removed_count > 0 {
            info!("Cleanup task removed {} expired file(s)", removed_count);
        }
    }
}

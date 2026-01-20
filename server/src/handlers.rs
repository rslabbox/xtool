use axum::{
    body::{Body, Bytes},
    extract::{Path, State},
    http::{header, HeaderMap, StatusCode},
    response::Response,
    Json,
};
use log::{error, info};
use rand::Rng;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

use crate::{
    state::AppState,
    storage::{FileRecord, TEMP_DIR},
};

const MAX_FILE_SIZE: usize = 100 * 1024 * 1024;
const DEFAULT_DOWNLOADS: u8 = 1;
const MAX_DOWNLOADS: u8 = 10;
const MAX_FILE_AGE: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(serde::Serialize)]
pub struct UploadResponse {
    pub token: String,
    pub filename: String,
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
    if body.len() > MAX_FILE_SIZE {
        return Err(StatusCode::PAYLOAD_TOO_LARGE);
    }

    let content_type_value = headers
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let requested_name = headers
        .get("x-filename")
        .and_then(|value| value.to_str().ok())
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("file.bin");
    let download_limit = match parse_download_limit(&headers) {
        Ok(value) => value,
        Err(status) => return Err(status),
    };
    let (token, filename, file_path) =
        reserve_token(&state, content_type_value, requested_name.to_string(), download_limit);

    if let Err(err) = write_file(&file_path, &body).await {
        release_token(&state, &token);
        error!("Failed to write file: {}", err);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    info!("File uploaded: {} (token: {})", filename, token);

    Ok(Json(UploadResponse { token, filename }))
}

pub async fn download_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Response, StatusCode> {
    let (record, delete_after) = {
        let mut files = state.files.lock().expect("State lock poisoned");
        let record = match files.get_mut(&id) {
            Some(info) => info,
            None => {
                info!("File not found for token: {}", id);
                return Err(StatusCode::NOT_FOUND);
            }
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now.saturating_sub(record.uploaded_at) >= MAX_FILE_AGE.as_secs() {
            info!("File expired for token: {}", id);
            files.remove(&id);
            return Err(StatusCode::GONE);
        }

        if record.remaining_downloads == 0 {
            info!("Download limit reached for token: {}", id);
            return Err(StatusCode::GONE);
        }

        record.remaining_downloads = record.remaining_downloads.saturating_sub(1);
        let delete_after = record.remaining_downloads == 0;
        let cloned = record.clone();

        if delete_after {
            files.remove(&id);
        }

        (cloned, delete_after)
    };

    let file = match tokio::fs::File::open(&record.path).await {
        Ok(file) => file,
        Err(err) => {
            error!("Failed to open file: {}", err);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    if delete_after {
        if let Err(err) = fs::remove_file(&record.path) {
            error!("Failed to delete file after download: {}", err);
        }
    }

    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    info!("File downloaded: {} (token: {})", record.filename, id);

    let download_name = if record.filename.trim().is_empty() {
        "file.bin"
    } else {
        record.filename.as_str()
    };

    Ok(Response::builder()
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", download_name),
        )
        .header(header::CONTENT_TYPE, record.content_type)
        .body(body)
        .expect("Failed to build response"))
}

pub async fn list_files(State(state): State<AppState>) -> Json<ListResponse> {
    let files = state.files.lock().expect("State lock poisoned");
    let list: Vec<FileRecord> = files.values().cloned().collect();

    Json(ListResponse { files: list })
}

pub async fn delete_file(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let record = {
        let mut files = state.files.lock().expect("State lock poisoned");
        match files.remove(&id) {
            Some(info) => info,
            None => {
                info!("File not found for token: {}", id);
                return Err(StatusCode::NOT_FOUND);
            }
        }
    };

    if let Err(err) = fs::remove_file(&record.path) {
        error!("Failed to delete file: {}", err);
    }

    info!("File deleted: {} (token: {})", record.filename, id);

    Ok(StatusCode::NO_CONTENT)
}

fn reserve_token(
    state: &AppState,
    content_type: String,
    original_name: String,
    remaining_downloads: u8,
) -> (String, String, PathBuf) {
    let uploaded_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut rng = rand::rng();
    loop {
        let candidate = format!("{:06}", rng.random_range(0..=999_999));
        let storage_name = format!("file_{}", candidate);
        let file_path = PathBuf::from(TEMP_DIR).join(&storage_name);
        let record = FileRecord {
            id: candidate.clone(),
            filename: original_name.clone(),
            content_type: content_type.clone(),
            remaining_downloads,
            uploaded_at,
            path: file_path.clone(),
        };
        let mut files = state.files.lock().expect("State lock poisoned");
        if !files.contains_key(&candidate) {
            files.insert(candidate.clone(), record);
            return (candidate, original_name, file_path);
        }
    }
}

fn release_token(state: &AppState, token: &str) {
    let mut files = state.files.lock().expect("State lock poisoned");
    files.remove(token);
}

async fn write_file(path: &PathBuf, body: &Bytes) -> Result<(), std::io::Error> {
    let mut file = tokio::fs::File::create(path).await?;
    file.write_all(body).await
}

fn parse_download_limit(headers: &HeaderMap) -> Result<u8, StatusCode> {
    match headers.get("x-download-limit") {
        None => Ok(DEFAULT_DOWNLOADS),
        Some(value) => {
            let parsed = value
                .to_str()
                .ok()
                .and_then(|text| text.parse::<u8>().ok())
                .unwrap_or(0);
            if parsed == 0 || parsed > MAX_DOWNLOADS {
                Err(StatusCode::BAD_REQUEST)
            } else {
                Ok(parsed)
            }
        }
    }
}

use crate::file::archive::{compress_directory, MAX_FILE_SIZE, ZIP_CONTENT_TYPE};
use crate::file::{UploadResponse, MESSAGE_CONTENT_TYPE};
use anyhow::{Context, Result};
use log::info;
use std::{fs, path::Path, time::{SystemTime, UNIX_EPOCH}};

pub fn send_file(
    server: &str,
    filepath: Option<&Path>,
    dirpath: Option<&Path>,
    download_limit: u8,
    message: Option<&str>,
) -> Result<()> {
    let (data, filename, content_type, temp_path) = if let Some(text) = message {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(anyhow::anyhow!("Message cannot be empty"));
        }
        let data = trimmed.as_bytes().to_vec();
        if data.len() as u64 > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("Message exceeds 100MB limit"));
        }
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let filename = format!("message_{}.txt", nanos);
        (data, filename, MESSAGE_CONTENT_TYPE.to_string(), None)
    } else {
        match (filepath, dirpath) {
            (Some(path), None) => {
                let data = fs::read(path).with_context(|| {
                    format!("Failed to read file: {}", path.display())
                })?;
                if data.len() as u64 > MAX_FILE_SIZE {
                    return Err(anyhow::anyhow!("File exceeds 100MB limit"));
                }
                let filename = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("file.bin")
                    .to_string();
                (data, filename, "application/octet-stream".to_string(), None)
            }
            (None, Some(path)) => {
                let (zip_path, zip_name, size) = compress_directory(path)?;
                if size > MAX_FILE_SIZE {
                    let _ = fs::remove_file(&zip_path);
                    return Err(anyhow::anyhow!("Compressed file exceeds 100MB limit"));
                }
                let data = fs::read(&zip_path).with_context(|| {
                    format!("Failed to read archive: {}", zip_path.display())
                })?;
                (data, zip_name, ZIP_CONTENT_TYPE.to_string(), Some(zip_path))
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "Please provide either a file path or -d <dir> or -m <message>"
                ));
            }
        }
    };

    let client = reqwest::blocking::Client::new();
    let url = format!("{}/upload", normalize_server(server));
    let request = client
        .post(&url)
        .header("x-filename", filename)
        .header("x-download-limit", download_limit.to_string())
        .header(reqwest::header::CONTENT_TYPE, content_type)
        .body(data);

    let response = request
        .send()
        .context("Failed to send upload request")?;

    if let Some(path) = temp_path {
        let _ = fs::remove_file(path);
    }

    if response.status().is_success() {
        let upload_resp: UploadResponse = response
            .json()
            .context("Failed to parse upload response")?;
        info!("Upload success: token={}, name={}", upload_resp.token, upload_resp.filename);
        println!("{}", upload_resp.token);
        Ok(())
    } else {
        Err(anyhow::anyhow!("Upload failed: {}", response.status()))
    }
}

fn normalize_server(server: &str) -> String {
    server.trim_end_matches('/').to_string()
}

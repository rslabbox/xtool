use crate::file::archive::{compress_directory, MAX_FILE_SIZE};
use crate::file::UploadResponse;
use anyhow::{Context, Result};
use log::info;
use qiniu_sdk::upload::{AutoUploader, AutoUploaderObjectParams, UploadManager, UploadTokenSigner};
use qiniu_upload_token::StaticUploadTokenProvider;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

pub fn send_file(
    server: &str,
    filepath: Option<&Path>,
    dirpath: Option<&Path>,
    download_limit: u8,
    message: Option<&str>,
) -> Result<()> {
    let _ = download_limit;
    let client = reqwest::blocking::Client::new();
    let server = normalize_server(server);

    if let Some(text) = message {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(anyhow::anyhow!("Message cannot be empty"));
        }
        let data = trimmed.as_bytes().to_vec();
        if data.len() as u64 > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("Message exceeds {}MB limit", MAX_FILE_SIZE / 1024 / 1024));
        }
        let url = format!("{}/upload", server);
        let response = client
            .post(&url)
            .header("x-upload-type", "text")
            .body(trimmed.to_string())
            .send()
            .context("Failed to send text upload request")?;

        if response.status().is_success() {
            let upload_resp: UploadResponse = response
                .json()
                .context("Failed to parse upload response")?;
            info!("Upload success: id={}", upload_resp.id);
            println!("xtool file get {}", upload_resp.id);
            return Ok(());
        }

        return Err(anyhow::anyhow!("Upload text failed: {}", response.status()));
    }

    let (file_path, filename, temp_path) = resolve_upload_target(filepath, dirpath)?;
    let (upload_token, key) = request_file_upload(&client, &server, &filename)?;
    upload_to_qiniu(&file_path, &key, &upload_token)?;
    let id = complete_upload(&client, &server, &key, &filename)?;

    if let Some(path) = temp_path {
        let _ = fs::remove_file(path);
    }

    info!("Upload success: id={}, name={}", id, filename);
    println!("xtool file get {}", id);
    Ok(())
}

fn resolve_upload_target(
    filepath: Option<&Path>,
    dirpath: Option<&Path>,
) -> Result<(PathBuf, String, Option<PathBuf>)> {
    match (filepath, dirpath) {
        (Some(path), None) => {
            let metadata = fs::metadata(path)
                .with_context(|| format!("Failed to read file: {}", path.display()))?;
            if metadata.len() > MAX_FILE_SIZE {
                return Err(anyhow::anyhow!("File exceeds {}MB limit", MAX_FILE_SIZE / 1024 / 1024));
            }
            let filename = path
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("file.bin")
                .to_string();
            Ok((path.to_path_buf(), filename, None))
        }
        (None, Some(path)) => {
            eprintln!("Compressing directory: {}", path.display());
            let (zip_path, zip_name, size) = compress_directory(path)?;
            if size > MAX_FILE_SIZE {
                let _ = fs::remove_file(&zip_path);
                return Err(anyhow::anyhow!(
                    "Compressed file exceeds {}MB limit (current: {:.2}MB)",
                    MAX_FILE_SIZE / 1024 / 1024,
                    size as f64 / 1024.0 / 1024.0
                ));
            }
            Ok((zip_path.clone(), zip_name, Some(zip_path)))
        }
        _ => Err(anyhow::anyhow!(
            "Please provide either a file path or -d <dir> or -m <message>"
        )),
    }
}

fn request_file_upload(
    client: &reqwest::blocking::Client,
    server: &str,
    filename: &str,
) -> Result<(String, String)> {
    let url = format!("{}/upload", server);
    let response = client
        .post(&url)
        .header("x-upload-type", "file")
        .header("x-filename", filename)
        .send()
        .context("Failed to request upload token")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Request upload failed: {}",
            response.status()
        ));
    }

    let upload_resp: UploadResponse = response
        .json()
        .context("Failed to parse upload response")?;
    let token = upload_resp
        .upload_token
        .context("Missing upload token")?;
    let key = upload_resp.key.context("Missing upload key")?;
    Ok((token, key))
}

#[derive(Serialize)]
struct CompleteUploadRequest<'a> {
    key: &'a str,
    filename: &'a str,
}

fn complete_upload(
    client: &reqwest::blocking::Client,
    server: &str,
    key: &str,
    filename: &str,
) -> Result<String> {
    let url = format!("{}/upload/complete", server);
    let response = client
        .post(&url)
        .json(&CompleteUploadRequest { key, filename })
        .send()
        .context("Failed to complete upload")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Complete upload failed: {}",
            response.status()
        ));
    }

    let upload_resp: UploadResponse = response
        .json()
        .context("Failed to parse complete upload response")?;
    Ok(upload_resp.id)
}

fn upload_to_qiniu(file_path: &Path, key: &str, token: &str) -> Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let timer_flag = Arc::clone(&running);
    let start = Instant::now();
    let timer_handle = thread::spawn(move || {
        let mut seconds = 0u64;
        while timer_flag.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(1));
            seconds += 1;
            eprintln!("Uploading... elapsed {}s", seconds);
        }
    });

    let token_provider: StaticUploadTokenProvider = token
        .parse()
        .context("Failed to parse upload token")?;
    let upload_manager = UploadManager::builder(UploadTokenSigner::new_upload_token_provider(
        token_provider,
    ))
    .build();
    let uploader: AutoUploader = upload_manager.auto_uploader();

    let params = AutoUploaderObjectParams::builder()
        .object_name(key)
        .file_name(key)
        .build();

    uploader
        .upload_path(file_path, params)
        .context("Qiniu upload failed")?;

    running.store(false, Ordering::Relaxed);
    let _ = timer_handle.join();
    eprintln!("Upload finished in {:.2}s", start.elapsed().as_secs_f64());
    Ok(())
}

fn normalize_server(server: &str) -> String {
    server.trim_end_matches('/').to_string()
}

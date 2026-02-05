use crate::file::archive::{compress_path, encrypt_zip_file, MAX_FILE_SIZE};
use crate::file::UploadResponse;
use anyhow::{Context, Result};
use log::info;
use qiniu_sdk::upload::{AutoUploader, AutoUploaderObjectParams, UploadManager, UploadTokenSigner};
use qiniu_upload_token::StaticUploadTokenProvider;
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
    path: Option<&Path>,
    download_limit: u8,
    message: Option<&str>,
    key: Option<&str>,
) -> Result<()> {
    let _ = download_limit;
    let client = reqwest::blocking::Client::new();
    let server = normalize_server(server);

    if let Some(text) = message {
        return send_message(&client, &server, text);
    }

    send_archive(&client, &server, path, key)
}

fn send_message(client: &reqwest::blocking::Client, server: &str, text: &str) -> Result<()> {
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

    Err(anyhow::anyhow!("Upload text failed: {}", response.status()))
}

fn send_archive(
    client: &reqwest::blocking::Client,
    server: &str,
    path: Option<&Path>,
    key: Option<&str>,
) -> Result<()> {
    let (file_path, filename, temp_path) = resolve_upload_target(path)?;
    let result = (|| {
        maybe_encrypt(&file_path, key)?;
        let (upload_token, id) = request_file_upload(client, server, &filename)?;
        upload_to_qiniu(&file_path, &filename, &upload_token)?;
        info!("Upload success: id={}, name={}", id, filename);
        println!("xtool file get {}", id);
        Ok(())
    })();

    if let Some(path) = temp_path {
        let _ = fs::remove_file(path);
    }

    result
}

fn maybe_encrypt(file_path: &Path, key: Option<&str>) -> Result<()> {
    let Some(key) = key else { return Ok(()); };
    if key.trim().is_empty() {
        return Err(anyhow::anyhow!("Encryption key cannot be empty"));
    }
    let encrypted_size = encrypt_zip_file(file_path, key)?;
    if encrypted_size > MAX_FILE_SIZE {
        return Err(anyhow::anyhow!(
            "Encrypted file exceeds {}MB limit",
            MAX_FILE_SIZE / 1024 / 1024
        ));
    }
    Ok(())
}

fn resolve_upload_target(path: Option<&Path>) -> Result<(PathBuf, String, Option<PathBuf>)> {
    let path = path.ok_or_else(|| {
        anyhow::anyhow!("Please provide a file/dir path or -m <message>")
    })?;

    if path.is_dir() {
        eprintln!("Compressing directory: {}", path.display());
    } else {
        eprintln!("Compressing file: {}", path.display());
    }

    let (zip_path, zip_name, size) = compress_path(path)?;

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
    Ok((token, upload_resp.id))
}

fn upload_to_qiniu(file_path: &Path, filename: &str, token: &str) -> Result<()> {
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
        .file_name(filename)
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

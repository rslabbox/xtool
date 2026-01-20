use crate::file::archive::{
    is_zip_download, resolve_output_dir, resolve_output_path, unzip_to_dir, write_temp_zip,
    MAX_FILE_SIZE,
};
use crate::file::MESSAGE_CONTENT_TYPE;
use anyhow::{Context, Result};
use log::info;
use std::{fs, path::Path};

pub fn get_file(server: &str, token: &str, output: Option<&Path>) -> Result<()> {
    let client = reqwest::blocking::Client::new();
    let url = format!("{}/download/{}", normalize_server(server), token);
    let response = client
        .get(&url)
        .send()
        .context("Failed to send download request")?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!(
            "Download failed: {}",
            response.status()
        ));
    }

    let content_disposition = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let filename = parse_filename(content_disposition).unwrap_or_else(|| "file.bin".to_string());

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();

    let bytes = response.bytes().context("Failed to read response body")?;
    if is_message_download(&content_type) {
        if bytes.len() as u64 > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("Message exceeds 100MB limit"));
        }
        let text = String::from_utf8_lossy(&bytes);
        println!("{}", text);
        return Ok(());
    }
    if is_zip_download(&content_type, &filename) {
        let temp_path = write_temp_zip(&bytes)?;
        if bytes.len() as u64 > MAX_FILE_SIZE {
            let _ = fs::remove_file(&temp_path);
            return Err(anyhow::anyhow!("Archive exceeds 100MB limit"));
        }
        let output_dir = resolve_output_dir(output, &filename)?;
        let unzip_result = unzip_to_dir(&temp_path, &output_dir);
        let _ = fs::remove_file(&temp_path);
        unzip_result?;
        info!("Download success: {}", output_dir.display());
    } else {
        let output_path = resolve_output_path(output, &filename);
        if let Some(parent) = output_path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }
        }
        if bytes.len() as u64 > MAX_FILE_SIZE {
            return Err(anyhow::anyhow!("File exceeds 100MB limit"));
        }
        fs::write(&output_path, &bytes)
            .with_context(|| format!("Failed to write file: {}", output_path.display()))?;

        info!("Download success: {} ({} bytes)", output_path.display(), bytes.len());
    }

    Ok(())
}

fn parse_filename(header_value: &str) -> Option<String> {
    for part in header_value.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("filename=") {
            let trimmed = rest.trim().trim_matches('"');
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn normalize_server(server: &str) -> String {
    server.trim_end_matches('/').to_string()
}

fn is_message_download(content_type: &str) -> bool {
    content_type.eq_ignore_ascii_case(MESSAGE_CONTENT_TYPE)
}

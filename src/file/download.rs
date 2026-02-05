use crate::file::archive::{resolve_output_dir, resolve_output_path, unzip_to_dir, write_temp_zip, MAX_FILE_SIZE};
use crate::file::{ContentType, DownloadResponse};
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::info;
use std::{fs, io::Read, path::Path};

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

    let download_resp: DownloadResponse = response
        .json()
        .context("Failed to parse download response")?;

    match download_resp.content_type {
        ContentType::Text => {
            let content = download_resp
                .content
                .context("No content in response (is this a file?)")?;
            if content.len() as u64 > MAX_FILE_SIZE {
                return Err(anyhow::anyhow!("Message exceeds {}MB limit", MAX_FILE_SIZE / 1024 / 1024));
            }
            println!("{}", content);
        }
        ContentType::File => {
            let file_url = download_resp
                .url
                .context("No url in response (is this a text?)")?;
            let filename = download_resp
                .filename
                .unwrap_or_else(|| "file.bin".to_string());

            let mut file_response = client
                .get(&file_url)
                .send()
                .context("Failed to download file from storage")?;

            if !file_response.status().is_success() {
                return Err(anyhow::anyhow!(
                    "File download failed: {}",
                    file_response.status()
                ));
            }

            let total_size = file_response.content_length();
            let mut bytes: Vec<u8> = Vec::new();
            let mut downloaded: u64 = 0;

            let progress = match total_size {
                Some(total) if total > 0 => {
                    let pb = ProgressBar::new(total);
                    let style = ProgressStyle::with_template(
                        "{msg} {spinner:.green} {bytes}/{total_bytes} ({percent}%) [{bar:40.cyan/blue}] {eta}",
                    )
                    .unwrap()
                    .progress_chars("=>-");
                    pb.set_style(style);
                    pb.set_message(filename.clone());
                    pb
                }
                _ => {
                    let pb = ProgressBar::new_spinner();
                    pb.set_style(
                        ProgressStyle::with_template("{msg} {spinner:.green} {bytes} downloaded")
                            .unwrap(),
                    );
                    pb.set_message(filename.clone());
                    pb.enable_steady_tick(std::time::Duration::from_millis(120));
                    pb
                }
            };

            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = file_response
                    .read(&mut buffer)
                    .context("Failed to read file response")?;
                if read == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..read]);
                downloaded += read as u64;
                progress.inc(read as u64);

                if downloaded > MAX_FILE_SIZE {
                    progress.finish_and_clear();
                    return Err(anyhow::anyhow!(
                        "File exceeds {}MB limit",
                        MAX_FILE_SIZE / 1024 / 1024
                    ));
                }
            }

            progress.finish_and_clear();

            if filename.ends_with(".zip") {
                let temp_path = write_temp_zip(&bytes)?;
                let output_dir = resolve_output_dir(output, &filename)?;
                let unzip_result = unzip_to_dir(&temp_path, &output_dir);
                let _ = fs::remove_file(&temp_path);
                unzip_result?;
                info!("Download success: {}", output_dir.display());
            } else {
                let output_path = resolve_output_path(output, &filename);
                if let Some(parent) = output_path.parent()
                    && !parent.as_os_str().is_empty()
                {
                    fs::create_dir_all(parent).with_context(|| {
                        format!("Failed to create directory: {}", parent.display())
                    })?;
                }
                fs::write(&output_path, &bytes)
                    .with_context(|| format!("Failed to write file: {}", output_path.display()))?;

                info!(
                    "Download success: {} ({} bytes)",
                    output_path.display(),
                    bytes.len()
                );
            }
        }
    }

    Ok(())
}

fn normalize_server(server: &str) -> String {
    server.trim_end_matches('/').to_string()
}


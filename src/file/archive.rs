use anyhow::{Context, Result};
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;
pub const ZIP_CONTENT_TYPE: &str = "application/zip";

pub fn compress_directory(dir: &Path) -> Result<(PathBuf, String, u64)> {
    if !dir.exists() || !dir.is_dir() {
        return Err(anyhow::anyhow!("Directory not found: {}", dir.display()));
    }

    let zip_name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| format!("{}.zip", name))
        .unwrap_or_else(|| "archive.zip".to_string());

    let tmp = tempfile::Builder::new()
        .prefix("xtool_upload_")
        .suffix(".zip")
        .tempfile()
        .context("Failed to create temp file")?;
    let mut writer = zip::ZipWriter::new(tmp.as_file());
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    let base = dir.canonicalize().context("Failed to canonicalize path")?;

    for entry in WalkDir::new(&base) {
        let entry = entry.context("Failed to walk directory")?;
        let path = entry.path();
        let rel = path
            .strip_prefix(&base)
            .context("Failed to compute relative path")?;
        let name = rel.to_string_lossy().replace('\\', "/");
        if name.is_empty() {
            continue;
        }

        if path.is_dir() {
            writer
                .add_directory(name, options)
                .context("Failed to add directory to archive")?;
        } else if path.is_file() {
            writer
                .start_file(name, options)
                .context("Failed to add file to archive")?;
            let mut file = fs::File::open(path)
                .with_context(|| format!("Failed to open file: {}", path.display()))?;
            io::copy(&mut file, &mut writer)
                .context("Failed to write file to archive")?;
        }
    }

    writer.finish().context("Failed to finalize archive")?;
    tmp.as_file().sync_all().ok();

    let (file, path) = tmp.keep().context("Failed to keep temp file")?;
    let size = file
        .metadata()
        .context("Failed to read archive metadata")?
        .len();
    drop(file);

    Ok((path, zip_name, size))
}

pub fn write_temp_zip(bytes: &[u8]) -> Result<PathBuf> {
    let mut tmp = tempfile::Builder::new()
        .prefix("xtool_download_")
        .suffix(".zip")
        .tempfile()
        .context("Failed to create temp file")?;
    tmp.write_all(bytes)
        .context("Failed to write temp archive")?;
    let (_file, path) = tmp.keep().context("Failed to keep temp file")?;
    Ok(path)
}

pub fn unzip_to_dir(zip_path: &Path, output_dir: &Path) -> Result<()> {
    let file = fs::File::open(zip_path)
        .with_context(|| format!("Failed to open archive: {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("Failed to read archive")?;

    fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create directory: {}", output_dir.display()))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).context("Failed to read archive entry")?;
        let out_path = output_dir.join(entry.name());

        if entry.name().ends_with('/') {
            fs::create_dir_all(&out_path)
                .with_context(|| format!("Failed to create directory: {}", out_path.display()))?;
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create directory: {}", parent.display())
                })?;
            }
            let mut outfile = fs::File::create(&out_path)
                .with_context(|| format!("Failed to create file: {}", out_path.display()))?;
            io::copy(&mut entry, &mut outfile)
                .context("Failed to extract file")?;
        }
    }

    Ok(())
}

pub fn resolve_output_path(output: Option<&Path>, filename: &str) -> PathBuf {
    match output {
        Some(path) if path.exists() && path.is_dir() => path.join(filename),
        Some(path) => path.to_path_buf(),
        None => PathBuf::from(filename),
    }
}

pub fn resolve_output_dir(output: Option<&Path>, filename: &str) -> Result<PathBuf> {
    if let Some(path) = output {
        if path.exists() && path.is_file() {
            return Err(anyhow::anyhow!(
                "Output path must be a directory for archives"
            ));
        }
        return Ok(path.to_path_buf());
    }

    let stem = filename.trim_end_matches(".zip");
    if stem.is_empty() {
        Ok(PathBuf::from("xtool_download"))
    } else {
        Ok(PathBuf::from(stem))
    }
}

pub fn is_zip_download(content_type: &str, filename: &str) -> bool {
    content_type.eq_ignore_ascii_case(ZIP_CONTENT_TYPE) || filename.ends_with(".zip")
}

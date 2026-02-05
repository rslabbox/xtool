use anyhow::{Context, Result};
use aes_gcm::{aead::Aead, Aes256Gcm, KeyInit, Nonce};
use pbkdf2::pbkdf2_hmac;
use rand::RngCore;
use sha2::Sha256;
use std::{
    fs,
    io::{self, Cursor, Write},
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

pub const MAX_FILE_SIZE: u64 = 1000 * 1024 * 1024;
const ENCRYPT_MAGIC: &[u8] = b"XTOOLENC1";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const PBKDF2_ITERS: u32 = 100_000;
pub const XTOOL_FILE_SUFFIX: &str = ".xtool_file";
pub const XTOOL_DIR_SUFFIX: &str = ".xtool_dir";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArchiveHint {
    File,
    Dir,
    None,
}

pub fn compress_directory(dir: &Path) -> Result<(PathBuf, String, u64)> {
    if !dir.exists() || !dir.is_dir() {
        return Err(anyhow::anyhow!("Directory not found: {}", dir.display()));
    }

    let base_name = dir
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("archive");
    let zip_name = format!("{}{}", strip_xtool_suffix(base_name), XTOOL_DIR_SUFFIX);

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

pub fn compress_file(file_path: &Path) -> Result<(PathBuf, String, u64)> {
    if !file_path.exists() || !file_path.is_file() {
        return Err(anyhow::anyhow!("File not found: {}", file_path.display()));
    }

    let file_name = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file.bin")
        .to_string();
    let clean_name = strip_xtool_suffix(&file_name);
    let zip_name = format!("{}{}", clean_name, XTOOL_FILE_SUFFIX);

    let tmp = tempfile::Builder::new()
        .prefix("xtool_upload_")
        .suffix(".zip")
        .tempfile()
        .context("Failed to create temp file")?;

    let mut writer = zip::ZipWriter::new(tmp.as_file());
    let options = zip::write::FileOptions::<()>::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);

    writer
        .start_file(&file_name, options)
        .context("Failed to add file to archive")?;

    let mut file = fs::File::open(file_path)
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
    io::copy(&mut file, &mut writer).context("Failed to write file to archive")?;

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

pub fn compress_path(path: &Path) -> Result<(PathBuf, String, u64)> {
    if path.is_dir() {
        compress_directory(path)
    } else {
        compress_file(path)
    }
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

pub fn detect_archive_hint(filename: &str) -> (String, ArchiveHint) {
    if let Some(stripped) = filename.strip_suffix(XTOOL_FILE_SUFFIX) {
        return (stripped.to_string(), ArchiveHint::File);
    }
    if let Some(stripped) = filename.strip_suffix(XTOOL_DIR_SUFFIX) {
        return (stripped.to_string(), ArchiveHint::Dir);
    }
    (filename.to_string(), ArchiveHint::None)
}

pub fn unzip_single_from_bytes(bytes: &[u8], output_path: &Path) -> Result<()> {
    let cursor = Cursor::new(bytes);
    let mut archive = zip::ZipArchive::new(cursor).context("Failed to read archive")?;
    if archive.len() == 0 {
        return Err(anyhow::anyhow!("Archive is empty"));
    }
    let mut entry = archive.by_index(0).context("Failed to read archive entry")?;

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
    }

    let mut outfile = fs::File::create(output_path)
        .with_context(|| format!("Failed to create file: {}", output_path.display()))?;
    io::copy(&mut entry, &mut outfile).context("Failed to extract file")?;
    Ok(())
}

pub fn encrypt_zip_file(zip_path: &Path, key: &str) -> Result<u64> {
    let bytes = fs::read(zip_path)
        .with_context(|| format!("Failed to read archive: {}", zip_path.display()))?;
    let encrypted = encrypt_zip_bytes(&bytes, key)?;
    fs::write(zip_path, &encrypted)
        .with_context(|| format!("Failed to write encrypted archive: {}", zip_path.display()))?;
    Ok(encrypted.len() as u64)
}

pub fn is_encrypted_zip(bytes: &[u8]) -> bool {
    bytes.len() > ENCRYPT_MAGIC.len() + SALT_LEN + NONCE_LEN
        && bytes.starts_with(ENCRYPT_MAGIC)
}

pub fn decrypt_zip_bytes(bytes: &[u8], key: &str) -> Result<Vec<u8>> {
    if !is_encrypted_zip(bytes) {
        return Err(anyhow::anyhow!("Archive is not encrypted"));
    }

    let header_len = ENCRYPT_MAGIC.len();
    let salt_start = header_len;
    let salt_end = salt_start + SALT_LEN;
    let nonce_start = salt_end;
    let nonce_end = nonce_start + NONCE_LEN;

    let salt = &bytes[salt_start..salt_end];
    let nonce = &bytes[nonce_start..nonce_end];
    let ciphertext = &bytes[nonce_end..];

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(key.as_bytes(), salt, PBKDF2_ITERS, &mut key_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .context("Failed to initialize cipher")?;
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Decrypt failed (bad key or corrupted data)"))
}

fn encrypt_zip_bytes(bytes: &[u8], key: &str) -> Result<Vec<u8>> {
    let mut salt = [0u8; SALT_LEN];
    let mut rng = rand::rng();
    rng.fill_bytes(&mut salt);

    let mut key_bytes = [0u8; 32];
    pbkdf2_hmac::<Sha256>(key.as_bytes(), &salt, PBKDF2_ITERS, &mut key_bytes);

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .context("Failed to initialize cipher")?;

    let mut nonce_bytes = [0u8; NONCE_LEN];
    rng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let mut out = Vec::with_capacity(ENCRYPT_MAGIC.len() + SALT_LEN + NONCE_LEN + bytes.len() + 16);
    out.extend_from_slice(ENCRYPT_MAGIC);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, bytes)
        .map_err(|_| anyhow::anyhow!("Encrypt failed"))?;
    out.extend_from_slice(&ciphertext);
    Ok(out)
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

    let (mut stem, _) = detect_archive_hint(filename);
    stem = stem.trim_end_matches(".zip").to_string();
    if stem.is_empty() {
        Ok(PathBuf::from("xtool_download"))
    } else {
        Ok(PathBuf::from(stem))
    }
}

fn strip_xtool_suffix(name: &str) -> &str {
    if let Some(stripped) = name.strip_suffix(XTOOL_FILE_SUFFIX) {
        return stripped;
    }
    if let Some(stripped) = name.strip_suffix(XTOOL_DIR_SUFFIX) {
        return stripped;
    }
    name
}

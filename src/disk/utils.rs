use anyhow::{anyhow, bail, Result};
use dialoguer::Confirm;
use std::path::{Path, PathBuf};

use super::types::PathKind;

pub fn parse_size(input: &str) -> Result<u64> {
    let s = input.trim();
    if s.is_empty() {
        bail!("size is empty");
    }

    let (num_str, unit) = s.split_at(s.len().saturating_sub(1));
    let (value, multiplier) = match unit.to_ascii_lowercase().as_str() {
        "k" => (num_str, 1024u64),
        "m" => (num_str, 1024u64 * 1024),
        "g" => (num_str, 1024u64 * 1024 * 1024),
        _ => (s, 1u64),
    };

    let num: u64 = value
        .parse()
        .map_err(|_| anyhow!("invalid size: {input}"))?;
    Ok(num.saturating_mul(multiplier))
}

pub fn parse_u64_any(input: &str) -> Result<u64> {
    let s = input.trim();
    if let Some(hex) = s.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| anyhow!("invalid hex: {input}"))
    } else {
        s.parse::<u64>().map_err(|_| anyhow!("invalid number: {input}"))
    }
}

pub fn align_up(value: u64, align: u64) -> u64 {
    if align == 0 {
        return value;
    }
    value.div_ceil(align) * align
}

pub fn confirm_or_yes(yes: bool, prompt: &str) -> Result<()> {
    if yes {
        return Ok(());
    }
    let confirmed = Confirm::new()
        .with_prompt(prompt)
        .default(false)
        .interact()
        .map_err(|e| anyhow!("prompt failed: {e}"))?;
    if confirmed {
        Ok(())
    } else {
        bail!("aborted by user")
    }
}

pub fn path_kind(path: &str) -> PathKind {
    if path.starts_with("host:") {
        PathKind::Host
    } else {
        PathKind::Image
    }
}

pub fn host_path(path: &str) -> Result<PathBuf> {
    let p = path
        .strip_prefix("host:")
        .ok_or_else(|| anyhow!("host path required"))?;
    Ok(PathBuf::from(p))
}

pub fn normalize_image_path(path: &str) -> String {
    if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    }
}

pub fn iter_path_components(path: &str) -> Vec<String> {
    let clean = normalize_image_path(path);
    let mut cur = String::new();
    let mut out = Vec::new();
    for part in clean.split('/') {
        if part.is_empty() {
            continue;
        }
        cur.push('/');
        cur.push_str(part);
        out.push(cur.clone());
    }
    out
}

pub fn format_fat_label(label: &str) -> Result<[u8; 11]> {
    let mut out = [b' '; 11];
    let upper = label.trim().to_ascii_uppercase();
    if upper.is_empty() {
        return Ok(out);
    }
    if upper.len() > 11 {
        bail!("FAT label too long (max 11 chars)");
    }
    for (i, b) in upper.bytes().enumerate() {
        out[i] = b;
    }
    Ok(out)
}

pub fn remove_host_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

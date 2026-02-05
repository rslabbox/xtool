use anyhow::{anyhow, bail, Result};
use std::path::Path;
use std::path::PathBuf;

use super::super::fs::{copy_host_to_image, copy_image_to_host, copy_image_to_image, is_dir};
use super::super::types::{PartitionTarget, PathKind};
use super::super::utils::{host_path, normalize_image_path, path_kind};

pub fn cp(
    disk: &Path,
    target: &PartitionTarget,
    src: &str,
    dst: &str,
    recursive: bool,
    force: bool,
    _preserve: bool,
) -> Result<()> {
    let overwrite = force;
    let src_kind = path_kind(src);
    let dst_kind = path_kind(dst);

    match (src_kind, dst_kind) {
        (PathKind::Host, PathKind::Image) => {
            let host = host_path(src)?;
            let image = normalize_image_path(dst);
            let image = resolve_host_to_image_dst(disk, target, &host, &image)?;
            copy_host_to_image(disk, target, &host, &image, recursive, overwrite)?;
            println!("{}", image);
            Ok(())
        }
        (PathKind::Image, PathKind::Host) => {
            let image = normalize_image_path(src);
            let host = host_path(dst)?;
            let host = resolve_image_to_host_dst(&image, &host)?;
            copy_image_to_host(disk, target, &image, &host, recursive, overwrite)?;
            println!("{}", host.display());
            Ok(())
        }
        (PathKind::Image, PathKind::Image) => {
            let src_image = normalize_image_path(src);
            let dst_image = normalize_image_path(dst);
            let dst_image = resolve_image_to_image_dst(disk, target, &src_image, &dst_image)?;
            copy_image_to_image(disk, target, &src_image, &dst_image, recursive, overwrite)?;
            println!("{}", dst_image);
            Ok(())
        }
        _ => bail!("host -> host is not supported by xtool disk"),
    }
}

fn resolve_host_to_image_dst(
    disk: &Path,
    target: &PartitionTarget,
    host: &Path,
    image: &str,
) -> Result<String> {
    let mut is_dir_dst = image.ends_with('/');
    if !is_dir_dst {
        is_dir_dst = is_dir(disk, target, image).unwrap_or(false);
    }

    if image.ends_with('/') && !is_dir_dst {
        bail!("destination is not a directory");
    }

    if !is_dir_dst {
        return Ok(image.to_string());
    }

    let name = host
        .file_name()
        .and_then(|s| s.to_str())
        .ok_or_else(|| anyhow!("invalid host path"))?;

    if image == "/" {
        return Ok(format!("/{}", name));
    }
    Ok(format!("{}/{}", image.trim_end_matches('/'), name))
}

fn resolve_image_to_host_dst(image: &str, host: &Path) -> Result<PathBuf> {
    let mut is_dir_dst = host.as_os_str().to_string_lossy().ends_with('/');
    if !is_dir_dst && let Ok(meta) = std::fs::metadata(host) {
        is_dir_dst = meta.is_dir();
    }

    if host.as_os_str().to_string_lossy().ends_with('/') && !is_dir_dst {
        bail!("destination is not a directory");
    }

    if !is_dir_dst {
        return Ok(host.to_path_buf());
    }

    let image = image.trim_end_matches('/');
    let name = image
        .rsplit('/').next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("invalid image path"))?;

    if name == "." || name == ".." {
        bail!("invalid image path");
    }

    let dst = host.join(name);
    Ok(dst)
}

fn resolve_image_to_image_dst(
    disk: &Path,
    target: &PartitionTarget,
    src: &str,
    dst: &str,
) -> Result<String> {
    let mut is_dir_dst = dst.ends_with('/');
    if !is_dir_dst {
        is_dir_dst = is_dir(disk, target, dst).unwrap_or(false);
    }

    if dst.ends_with('/') && !is_dir_dst {
        bail!("destination is not a directory");
    }

    if !is_dir_dst {
        return Ok(dst.to_string());
    }

    let src = src.trim_end_matches('/');
    let name = src
        .rsplit('/').next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("invalid image path"))?;

    if name == "." || name == ".." {
        bail!("invalid image path");
    }

    if dst == "/" {
        return Ok(format!("/{}", name));
    }
    Ok(format!("{}/{}", dst.trim_end_matches('/'), name))
}

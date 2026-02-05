use anyhow::{anyhow, bail, Result};
use std::path::Path;

use super::super::fs::mv as fs_mv;
use super::super::types::{PartitionTarget, PathKind};
use super::super::utils::{confirm_or_yes, host_path, path_kind, remove_host_path};
use super::cp::cp;
use super::super::fs::is_dir as fs_is_dir;
use super::super::utils::normalize_image_path;

pub fn mv(disk: &Path, target: &PartitionTarget, src: &str, dst: &str, force: bool) -> Result<()> {
    let overwrite = force;
    let src_kind = path_kind(src);
    let dst_kind = path_kind(dst);

    match (src_kind, dst_kind) {
        (PathKind::Image, PathKind::Image) => {
            let src_image = normalize_image_path(src);
            let dst_image = normalize_image_path(dst);
            let dst_image = resolve_image_to_image_dst(disk, target, &src_image, &dst_image)?;
            fs_mv(disk, target, &src_image, &dst_image, overwrite)
        }
        (PathKind::Host, PathKind::Image) | (PathKind::Image, PathKind::Host) => {
            let prompt = "Move between host and image will copy then delete. Continue?";
            confirm_or_yes(false, prompt)?;
            cp(disk, target, src, dst, true, force, false)?;
            if src_kind == PathKind::Host {
                remove_host_path(&host_path(src)?)
            } else {
                super::rm::rm(disk, target, src, true, force, true)
            }
        }
        _ => bail!("host -> host is not supported by xtool disk"),
    }
}

fn resolve_image_to_image_dst(
    disk: &Path,
    target: &PartitionTarget,
    src: &str,
    dst: &str,
) -> Result<String> {
    let mut is_dir_dst = dst.ends_with('/');
    if !is_dir_dst {
        is_dir_dst = fs_is_dir(disk, target, dst).unwrap_or(false);
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

use anyhow::{Result, anyhow, bail};
use std::path::Path;
use std::{fs::OpenOptions, io::{Read, Seek, SeekFrom}};

mod ext4;
mod fat;

use super::types::{DirEntry, PartitionTarget};
use super::utils::normalize_image_path;

pub use ext4::mkfs_ext4;
pub use fat::mkfs_fat32;

pub trait FsOps {
    fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>>;
    fn read_file(&mut self, path: &str, offset: u64, bytes: Option<usize>) -> Result<Vec<u8>>;
    fn write_file(&mut self, path: &str, data: &[u8], force: bool) -> Result<()>;
    fn mkdir(&mut self, path: &str, parents: bool) -> Result<()>;
    fn rm(&mut self, path: &str, recursive: bool) -> Result<()>;
    fn mv(&mut self, src: &str, dst: &str, force: bool) -> Result<()>;
    fn is_dir(&mut self, path: &str) -> Result<bool>;
}

pub fn with_fs<R>(
    disk: &Path,
    target: &PartitionTarget,
    mut f: impl for<'a> FnMut(&'a mut dyn FsOps) -> Result<R>,
) -> Result<R> {
    if let Some(kind) = detect_fs_type(disk, target)? {
        return match kind {
            FsKind::Ext4 => ext4::with_ext4(disk, target, |mut ops| f(&mut ops)),
            FsKind::Fat => fat::with_fat(disk, target, |mut ops| f(&mut ops)),
        };
    }
    match ext4::with_ext4(disk, target, |mut ops| f(&mut ops)) {
        Ok(result) => Ok(result),
        Err(ext4_err) => match fat::with_fat(disk, target, |mut ops| f(&mut ops)) {
            Ok(result) => Ok(result),
            Err(fat_err) => Err(anyhow!(
                "mount failed: ext4: {ext4_err}; fat: {fat_err}"
            )),
        },
    }
}

enum FsKind {
    Ext4,
    Fat,
}

fn detect_fs_type(disk: &Path, target: &PartitionTarget) -> Result<Option<FsKind>> {
    let mut file = OpenOptions::new().read(true).open(disk)?;

    let ext_offset = target.offset_bytes + 1024 + 56;
    let mut ext_magic = [0u8; 2];
    if file.seek(SeekFrom::Start(ext_offset)).is_ok()
        && file.read_exact(&mut ext_magic).is_ok()
        && u16::from_le_bytes(ext_magic) == 0xEF53
    {
        return Ok(Some(FsKind::Ext4));
    }

    let mut boot = [0u8; 512];
    if file.seek(SeekFrom::Start(target.offset_bytes)).is_ok()
        && file.read(&mut boot).is_ok()
        && boot[510] == 0x55
        && boot[511] == 0xAA
        && (boot.get(82..87) == Some(b"FAT32")
            || boot.get(54..59) == Some(b"FAT16")
            || boot.get(54..59) == Some(b"FAT12"))
    {
        return Ok(Some(FsKind::Fat));
    }

    Ok(None)
}

pub fn list_dir(disk: &Path, target: &PartitionTarget, path: &str) -> Result<Vec<DirEntry>> {
    with_fs(disk, target, |fs| fs.list_dir(path))
}

pub fn read_file(
    disk: &Path,
    target: &PartitionTarget,
    path: &str,
    offset: u64,
    bytes: Option<usize>,
) -> Result<Vec<u8>> {
    with_fs(disk, target, |fs| fs.read_file(path, offset, bytes))
}

pub fn mkdir(disk: &Path, target: &PartitionTarget, path: &str, parents: bool) -> Result<()> {
    let image_path = normalize_image_path(path);
    with_fs(disk, target, |fs| fs.mkdir(&image_path, parents))
}

pub fn rm(disk: &Path, target: &PartitionTarget, path: &str, recursive: bool) -> Result<()> {
    let image_path = normalize_image_path(path);
    with_fs(disk, target, |fs| fs.rm(&image_path, recursive))
}

pub fn mv(disk: &Path, target: &PartitionTarget, src: &str, dst: &str, force: bool) -> Result<()> {
    let src_image = normalize_image_path(src);
    let dst_image = normalize_image_path(dst);
    with_fs(disk, target, |fs| fs.mv(&src_image, &dst_image, force))
}

pub fn is_dir(disk: &Path, target: &PartitionTarget, path: &str) -> Result<bool> {
    let image_path = normalize_image_path(path);
    with_fs(disk, target, |fs| fs.is_dir(&image_path))
}

pub fn write_file(
    disk: &Path,
    target: &PartitionTarget,
    path: &str,
    data: &[u8],
    force: bool,
) -> Result<()> {
    let image_path = normalize_image_path(path);
    with_fs(disk, target, |fs| fs.write_file(&image_path, data, force))
}

pub fn copy_host_to_image(
    disk: &Path,
    target: &PartitionTarget,
    src: &Path,
    dst: &str,
    recursive: bool,
    force: bool,
) -> Result<()> {
    if src.is_dir() {
        if !recursive {
            bail!("directory copy requires -r");
        }
        return copy_host_dir_to_image(disk, target, src, dst, force);
    }

    let data = std::fs::read(src).map_err(|e| anyhow!("read host file {}: {e}", src.display()))?;
    write_file(disk, target, dst, &data, force)
}

pub fn copy_image_to_host(
    disk: &Path,
    target: &PartitionTarget,
    src: &str,
    dst: &Path,
    recursive: bool,
    force: bool,
) -> Result<()> {
    let is_dir = with_fs(disk, target, |fs| fs.is_dir(src))?;
    if is_dir {
        if !recursive {
            bail!("directory copy requires -r");
        }
        std::fs::create_dir_all(dst)?;
        let entries = list_dir(disk, target, src)?;
        for entry in entries {
            let child_src = format!("{}/{}", src.trim_end_matches('/'), entry.name);
            let child_dst = dst.join(&entry.name);
            copy_image_to_host(disk, target, &child_src, &child_dst, recursive, force)?;
        }
        return Ok(());
    }

    if dst.exists() && !force {
        bail!("destination exists, use -f to overwrite");
    }
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let data = read_file(disk, target, src, 0, None)?;
    std::fs::write(dst, data)?;
    Ok(())
}

pub fn copy_image_to_image(
    disk: &Path,
    target: &PartitionTarget,
    src: &str,
    dst: &str,
    recursive: bool,
    force: bool,
) -> Result<()> {
    let is_dir = with_fs(disk, target, |fs| fs.is_dir(src))?;
    if is_dir {
        if !recursive {
            bail!("directory copy requires -r");
        }
        mkdir(disk, target, dst, true)?;
        let entries = list_dir(disk, target, src)?;
        for entry in entries {
            let child_src = format!("{}/{}", src.trim_end_matches('/'), entry.name);
            let child_dst = format!("{}/{}", dst.trim_end_matches('/'), entry.name);
            copy_image_to_image(disk, target, &child_src, &child_dst, recursive, force)?;
        }
        return Ok(());
    }

    let data = read_file(disk, target, src, 0, None)?;
    write_file(disk, target, dst, &data, force)?;
    Ok(())
}

fn copy_host_dir_to_image(
    disk: &Path,
    target: &PartitionTarget,
    src: &Path,
    dst: &str,
    force: bool,
) -> Result<()> {
    mkdir(disk, target, dst, true)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let child = format!("{}/{}", dst.trim_end_matches('/'), name);
        if path.is_dir() {
            copy_host_dir_to_image(disk, target, &path, &child, force)?;
        } else {
            let data = std::fs::read(&path)?;
            write_file(disk, target, &child, &data, force)?;
        }
    }
    Ok(())
}

use anyhow::{anyhow, bail, Result};
use crate::disk::fatfs::{self,
    FileSystem, FormatVolumeOptions, FsOptions, FatType, OemCpConverter, ReadWriteSeek, StdIoWrapper,
    TimeProvider,
};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use super::super::io::PartitionIo;
use super::super::types::{DirEntry, PartitionTarget};
use super::super::utils::{format_fat_label, iter_path_components, normalize_image_path};
use super::FsOps;

pub type FatFs = FileSystem<StdIoWrapper<PartitionIo>>;

pub struct FatOps<'a> {
    fs: &'a mut FatFs,
}

pub fn mkfs_fat32(disk: &Path, target: &PartitionTarget, label: Option<&str>) -> Result<()> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(disk)
        .map_err(|e| anyhow!("failed to open disk {}: {e}", disk.display()))?;

    let mut opts = FormatVolumeOptions::new().fat_type(FatType::Fat32);
    if let Some(label) = label {
        opts = opts.volume_label(format_fat_label(label)?);
    }

    let mut io = StdIoWrapper::new(PartitionIo::new(
        file,
        target.offset_bytes,
        target.size_bytes,
    ));
    fatfs::format_volume(&mut io, opts).map_err(|e| anyhow!("mkfs fat32 failed: {e}"))?;
    Ok(())
}

pub fn with_fat<R>(
    disk: &Path,
    target: &PartitionTarget,
    f: impl for<'a> FnOnce(FatOps<'a>) -> Result<R>,
) -> Result<R> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(disk)
        .map_err(|e| anyhow!("failed to open disk {}: {e}", disk.display()))?;
    let io = StdIoWrapper::new(PartitionIo::new(
        file,
        target.offset_bytes,
        target.size_bytes,
    ));
    let mut fs = FileSystem::new(io, FsOptions::new())
        .map_err(|e| anyhow!("mount fat failed: {e}"))?;

    let result = f(FatOps { fs: &mut fs })?;
    fs.unmount().map_err(|e| anyhow!("fat unmount failed: {e}"))?;
    Ok(result)
}

impl FsOps for FatOps<'_> {
    fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>> {
        let root = self.fs.root_dir();
        let dir = if path == "/" || path.is_empty() {
            root
        } else {
            root.open_dir(path).map_err(|e| anyhow!("open dir failed: {e}"))?
        };

        let mut out = Vec::new();
        for entry in dir.iter() {
            let entry = entry.map_err(|e| anyhow!("iter failed: {e:?}"))?;
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            out.push(DirEntry {
                name,
                is_dir: entry.is_dir(),
            });
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    fn read_file(&mut self, path: &str, offset: u64, bytes: Option<usize>) -> Result<Vec<u8>> {
        let root = self.fs.root_dir();
        let mut file = root
            .open_file(path)
            .map_err(|e| anyhow!("open file failed: {e}"))?;

        file.seek(SeekFrom::Start(offset))
            .map_err(|e| anyhow!("seek failed: {e}"))?;

        let mut data = Vec::new();
        if let Some(n) = bytes {
            let mut buf = vec![0u8; n];
            let read = file.read(&mut buf).map_err(|e| anyhow!("read failed: {e}"))?;
            buf.truncate(read);
            data.extend_from_slice(&buf);
        } else {
            file.read_to_end(&mut data)
                .map_err(|e| anyhow!("read failed: {e}"))?;
        }
        Ok(data)
    }

    fn write_file(&mut self, path: &str, data: &[u8], force: bool) -> Result<()> {
        let root = self.fs.root_dir();
        let mut file = match root.open_file(path) {
            Ok(mut f) => {
                if !force {
                    bail!("destination exists, use -f to overwrite");
                }
                f.truncate().map_err(|e| anyhow!("truncate failed: {e}"))?;
                f
            }
            Err(_) => root
                .create_file(path)
                .map_err(|e| anyhow!("create file failed: {e}"))?,
        };
        file.write_all(data)
            .map_err(|e| anyhow!("write failed: {e}"))?;
        Ok(())
    }

    fn mkdir(&mut self, path: &str, parents: bool) -> Result<()> {
        let root = self.fs.root_dir();
        if parents {
            for p in iter_path_components(path) {
                let _ = root.create_dir(&p);
            }
            return Ok(());
        }
        root.create_dir(path)
            .map_err(|e| anyhow!("mkdir failed: {e}"))?;
        Ok(())
    }

    fn rm(&mut self, path: &str, recursive: bool) -> Result<()> {
        let root = self.fs.root_dir();
        if recursive {
            return remove_fat_recursive(&root, path);
        }
        root.remove(path)
            .map_err(|e| anyhow!("remove failed: {e}"))?;
        Ok(())
    }

    fn mv(&mut self, src: &str, dst: &str, force: bool) -> Result<()> {
        let root = self.fs.root_dir();
        if !force {
            if root.open_file(dst).is_ok() || root.open_dir(dst).is_ok() {
                bail!("destination exists, use -f to overwrite");
            }
        } else {
            let _ = root.remove(dst);
        }
        root.rename(src, &root, dst)
            .map_err(|e| anyhow!("rename failed: {e}"))?;
        Ok(())
    }

    fn is_dir(&mut self, path: &str) -> Result<bool> {
        let root = self.fs.root_dir();
        let path = normalize_image_path(path);
        Ok(root.open_dir(&path).is_ok())
    }
}

fn remove_fat_recursive<IO, TP, OCC>(root: &fatfs::Dir<IO, TP, OCC>, path: &str) -> Result<()>
where
    IO: ReadWriteSeek,
    TP: TimeProvider,
    OCC: OemCpConverter,
{
    if let Ok(dir) = root.open_dir(path) {
        for entry in dir.iter() {
            let entry = entry.map_err(|e| anyhow!("iter failed: {e:?}"))?;
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let child = format!("{}/{}", path.trim_end_matches('/'), name);
            if entry.is_dir() {
                remove_fat_recursive(root, &child)?;
            } else {
                root.remove(&child)
                    .map_err(|e| anyhow!("remove failed: {e:?}"))?;
            }
        }
        root.remove(path)
            .map_err(|e| anyhow!("remove failed: {e:?}"))?;
        return Ok(());
    }
    root.remove(path)
        .map_err(|e| anyhow!("remove failed: {e:?}"))?;
    Ok(())
}

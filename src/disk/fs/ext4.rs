use anyhow::{anyhow, bail, Result};
use std::path::Path;

use rsext4::{
    entries::DirEntryIterator,
    file::{delete_dir, delete_file, read_file, rename, truncate, write_file},
    loopfile::{get_file_inode, resolve_inode_block_allextend},
    mkfs, Ext4FileSystem, Jbd2Dev, BLOCK_SIZE,
};

use super::super::io::PartitionBlockDev;
use super::super::types::{DirEntry, PartitionTarget};
use super::super::utils::{iter_path_components, normalize_image_path};
use super::FsOps;

pub struct Ext4Ops<'a> {
    jbd: &'a mut Jbd2Dev<PartitionBlockDev>,
    fs: &'a mut Ext4FileSystem,
}

pub fn mkfs_ext4(disk: &Path, target: &PartitionTarget, label: Option<&str>) -> Result<()> {
    if label.is_some() {
        eprintln!("ext4 label not supported, ignoring --label");
    }

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(disk)
        .map_err(|e| anyhow!("failed to open disk {}: {e}", disk.display()))?;

    let block_size = BLOCK_SIZE as u64;
    let usable = target.size_bytes - (target.size_bytes % block_size);
    if usable < block_size * 16 {
        bail!("partition too small for ext4");
    }

    let total_blocks = usable / block_size;
    let dev = PartitionBlockDev::new(file, target.offset_bytes, total_blocks, BLOCK_SIZE as u32);
    let mut jbd = Jbd2Dev::initial_jbd2dev(0, dev, false);
    mkfs(&mut jbd).map_err(|e| anyhow!("mkfs ext4 failed: {e:?}"))?;
    jbd.cantflush()
        .map_err(|e| anyhow!("flush failed: {e:?}"))?;
    Ok(())
}

pub fn with_ext4<R>(
    disk: &Path,
    target: &PartitionTarget,
    f: impl for<'a> FnOnce(Ext4Ops<'a>) -> Result<R>,
) -> Result<R> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(disk)
        .map_err(|e| anyhow!("failed to open disk {}: {e}", disk.display()))?;

    let block_size = BLOCK_SIZE as u64;
    let usable = target.size_bytes - (target.size_bytes % block_size);
    if usable < block_size * 2 {
        bail!("partition too small for ext4");
    }

    let total_blocks = usable / block_size;
    let dev = PartitionBlockDev::new(file, target.offset_bytes, total_blocks, BLOCK_SIZE as u32);
    let mut jbd = Jbd2Dev::initial_jbd2dev(0, dev, false);
    let mut fs = Ext4FileSystem::mount(&mut jbd)
        .map_err(|e| anyhow!("mount ext4 failed: {e:?}"))?;

    let result = f(Ext4Ops { jbd: &mut jbd, fs: &mut fs })?;

    fs.umount(&mut jbd)
        .map_err(|e| anyhow!("umount failed: {e:?}"))?;
    jbd.cantflush()
        .map_err(|e| anyhow!("flush failed: {e:?}"))?;
    Ok(result)
}

impl FsOps for Ext4Ops<'_> {
    fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>> {
        let (_, inode) = get_file_inode(self.fs, self.jbd, path)
            .map_err(|e| anyhow!("lookup failed: {e:?}"))?
            .ok_or_else(|| anyhow!("path not found"))?;
        if !inode.is_dir() {
            bail!("not a directory");
        }

        let mut inode = inode;
        let blocks = resolve_inode_block_allextend(self.fs, self.jbd, &mut inode)
            .map_err(|e| anyhow!("resolve dir blocks failed: {e:?}"))?;

        let mut entries = Vec::new();
        for phys in blocks.values() {
            let mut raw_entries = Vec::new();
            {
                let cached = self
                    .fs
                    .datablock_cache
                    .get_or_load(self.jbd, *phys)
                    .map_err(|e| anyhow!("load block failed: {e:?}"))?;
                let data = &cached.data[..BLOCK_SIZE];
                let iter = DirEntryIterator::new(data);
                for (entry, _) in iter {
                    if entry.is_dot() || entry.is_dotdot() {
                        continue;
                    }
                    let name = entry
                        .name_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| String::from_utf8_lossy(entry.name).to_string());
                    raw_entries.push((entry.inode, name));
                }
            }

            for (inode_num, name) in raw_entries {
                let child_inode = self
                    .fs
                    .get_inode_by_num(self.jbd, inode_num)
                    .map_err(|e| anyhow!("inode read failed: {e:?}"))?;
                entries.push(DirEntry {
                    name,
                    is_dir: child_inode.is_dir(),
                });
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    fn read_file(&mut self, path: &str, offset: u64, bytes: Option<usize>) -> Result<Vec<u8>> {
        let data = read_file(self.jbd, self.fs, path)
            .map_err(|e| anyhow!("read failed: {e:?}"))?
            .ok_or_else(|| anyhow!("file not found"))?;

        let start = offset as usize;
        if start >= data.len() {
            return Ok(Vec::new());
        }
        let end = bytes.map_or(data.len(), |n| (start + n).min(data.len()));
        Ok(data[start..end].to_vec())
    }

    fn write_file(&mut self, path: &str, data: &[u8], force: bool) -> Result<()> {
        let exists = get_file_inode(self.fs, self.jbd, path)
            .map_err(|e| anyhow!("lookup failed: {e:?}"))?
            .is_some();
        if exists {
            if !force {
                bail!("destination exists, use -f to overwrite");
            }
            truncate(self.jbd, self.fs, path, 0).map_err(|e| anyhow!("truncate failed: {e:?}"))?;
        } else {
            rsext4::mkfile(self.jbd, self.fs, path, None, None)
                .ok_or_else(|| anyhow!("mkfile failed"))?;
        }
        write_file(self.jbd, self.fs, path, 0, data)
            .map_err(|e| anyhow!("write failed: {e:?}"))?;
        Ok(())
    }

    fn mkdir(&mut self, path: &str, parents: bool) -> Result<()> {
        if parents {
            for p in iter_path_components(path) {
                let _ = rsext4::mkdir(self.jbd, self.fs, &p);
            }
            return Ok(());
        }
        rsext4::mkdir(self.jbd, self.fs, path).ok_or_else(|| anyhow!("mkdir failed"))?;
        Ok(())
    }

    fn rm(&mut self, path: &str, recursive: bool) -> Result<()> {
        let (_, inode) = get_file_inode(self.fs, self.jbd, path)
            .map_err(|e| anyhow!("lookup failed: {e:?}"))?
            .ok_or_else(|| anyhow!("path not found"))?;
        if inode.is_dir() {
            if !recursive {
                bail!("directory requires -r");
            }
            delete_dir(self.fs, self.jbd, path);
            return Ok(());
        }
        delete_file(self.fs, self.jbd, path);
        Ok(())
    }

    fn mv(&mut self, src: &str, dst: &str, force: bool) -> Result<()> {
        if !force
            && get_file_inode(self.fs, self.jbd, dst)
                .map_err(|e| anyhow!("lookup failed: {e:?}"))?
                .is_some()
        {
            bail!("destination exists, use -f to overwrite");
        }
        rename(self.jbd, self.fs, src, dst).map_err(|e| anyhow!("rename failed: {e:?}"))?;
        Ok(())
    }

    fn is_dir(&mut self, path: &str) -> Result<bool> {
        let (_, inode) = get_file_inode(self.fs, self.jbd, normalize_image_path(path).as_str())
            .map_err(|e| anyhow!("lookup failed: {e:?}"))?
            .ok_or_else(|| anyhow!("path not found"))?;
        Ok(inode.is_dir())
    }
}

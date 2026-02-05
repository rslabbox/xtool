use anyhow::{anyhow, bail, Result};
use std::path::Path;

use rsext4::{
    entries::DirEntryIterator,
    file::{delete_dir, delete_file, read_file, rename, truncate, write_file},
    loopfile::{get_file_inode, resolve_inode_block_allextend},
    mkfs, Ext4FileSystem, Jbd2Dev, BLOCK_SIZE,
};
// use rsext4::inode::Ext4Inode;
use rsext4::disknode::Ext4Inode;

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

impl<'a> Ext4Ops<'a> {
    fn get_dir_entries(&mut self, inode: &mut Ext4Inode) -> Result<Vec<(u32, String, bool)>> {
        let blocks = resolve_inode_block_allextend(self.fs, self.jbd, inode)
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
                    if entry.inode == 0 {
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
                entries.push((inode_num, name, child_inode.is_dir()));
            }
        }
        Ok(entries)
    }

    fn resolve_path(&mut self, path: &str) -> Result<Ext4Inode> {
         if path == "/" {
             let (_, root) = get_file_inode(self.fs, self.jbd, "/")
                 .map_err(|e| anyhow!("root lookup failed: {e:?}"))?
                 .ok_or_else(|| anyhow!("root not found"))?;
             return Ok(root);
         }

         let mut current_inode = {
             let (_, root) = get_file_inode(self.fs, self.jbd, "/")
                 .map_err(|e| anyhow!("root lookup failed: {e:?}"))?
                 .ok_or_else(|| anyhow!("root not found"))?;
             root
         };

         let normalized = normalize_image_path(path);
         let parts: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();

         for part in parts {
             if !current_inode.is_dir() {
                 bail!("not a directory");
             }
             
             let entries = self.get_dir_entries(&mut current_inode)?;
             let mut found_inode_num = None;
             
             for (inum, name, _) in entries {
                 if name == part {
                     found_inode_num = Some(inum);
                     break;
                 }
             }
             
             match found_inode_num {
                 Some(num) => {
                     current_inode = self
                    .fs
                    .get_inode_by_num(self.jbd, num)
                    .map_err(|e| anyhow!("inode read failed: {e:?}"))?;
                 }
                 None => bail!("path not found: {}", path),
             }
         }
         Ok(current_inode)
    }
}

impl FsOps for Ext4Ops<'_> {
    fn list_dir(&mut self, path: &str) -> Result<Vec<DirEntry>> {
        let mut inode = self.resolve_path(path)?;

        if !inode.is_dir() {
            bail!("not a directory");
        }

        let entries = self.get_dir_entries(&mut inode)?;
        let mut res = Vec::new();
        for (_, name, is_dir) in entries {
            res.push(DirEntry { name, is_dir });
        }
        res.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(res)
    }

    fn read_file(&mut self, path: &str, offset: u64, bytes: Option<usize>) -> Result<Vec<u8>> {
        // Verify file existence via manual resolution first
        let _ = self.resolve_path(path)?;
        
        let data = read_file(self.jbd, self.fs, path)
            .map_err(|e| anyhow!("read failed: {e:?}"))?
            .ok_or_else(|| anyhow!("file not found (read)"))?;

        let start = offset as usize;
        if start >= data.len() {
            return Ok(Vec::new());
        }
        let end = bytes.map_or(data.len(), |n| (start + n).min(data.len()));
        Ok(data[start..end].to_vec())
    }

    fn write_file(&mut self, path: &str, data: &[u8], force: bool) -> Result<()> {
        let exists = match self.resolve_path(path) {
            Ok(_) => true,
            Err(_) => false, // Assume not found if resolve failed
        };
        
        if exists {
            if !force {
                bail!("destination exists, use -f to overwrite");
            }
            truncate(self.jbd, self.fs, path, 0).map_err(|e| anyhow!("truncate failed: {e:?}"))?;
        } else {
            rsext4::mkfile(self.jbd, self.fs, path, None, None)
                .ok_or_else(|| anyhow!("mkfile failed for path: {}", path))?;
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
        let inode = self.resolve_path(path)?;
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
            && self.resolve_path(dst).is_ok()
        {
            bail!("destination exists, use -f to overwrite");
        }
        rename(self.jbd, self.fs, src, dst).map_err(|e| anyhow!("rename failed: {e:?}"))?;
        Ok(())
    }

    fn is_dir(&mut self, path: &str) -> Result<bool> {
        let inode = self.resolve_path(path)?;
        Ok(inode.is_dir())
    }
}

use anyhow::{anyhow, bail, Result};
use gpt::{disk::LogicalBlockSize, partition_types, GptConfig};
use std::path::Path;

use super::super::gpt::{
    align_partition_start, clamp_size_to_lba, lb_size_bytes, parse_parameter_file,
};
use super::super::utils::confirm_or_yes;

pub fn mkgpt(disk: &Path, param_file: &Path, align_bytes: u64, yes: bool) -> Result<()> {
    let disk_size = std::fs::metadata(disk)
        .map_err(|e| anyhow!("failed to stat disk {}: {e}", disk.display()))?
        .len();

    if disk_size < lb_size_bytes() * 34 {
        bail!("disk too small for GPT");
    }

    if !yes {
        let prompt = format!(
            "This will overwrite GPT on {}. Continue?",
            disk.display()
        );
        confirm_or_yes(false, &prompt)?;
    }

    let specs = parse_parameter_file(param_file)?;

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(disk)
        .map_err(|e| anyhow!("failed to open disk {}: {e}", disk.display()))?;

    let mut gdisk = GptConfig::new()
        .writable(true)
        .logical_block_size(LogicalBlockSize::Lb512)
        .create_from_device(file, None)
        .map_err(|e| anyhow!("failed to create GPT: {e}"))?;

    let header = gdisk.header();
    let usable_start_lba = header.first_usable;
    let usable_last_lba = header.last_usable;
    let usable_start_bytes = usable_start_lba * lb_size_bytes();
    let usable_end_bytes = (usable_last_lba + 1) * lb_size_bytes();

    let mut used_bytes = 0u64;
    let mut part_id: u32 = 1;
    for spec in specs {
        let mut start = align_partition_start(spec.offset_bytes, align_bytes);
        if start < usable_start_bytes {
            start = align_partition_start(usable_start_bytes, align_bytes);
        }

        let size = match spec.size_bytes {
            Some(sz) => sz,
            None => {
                if !spec.grow {
                    bail!("partition {} has no size and no grow flag", spec.name);
                }
                let remain = usable_end_bytes.saturating_sub(start);
                if remain == 0 {
                    bail!("partition {} has no space remaining", spec.name);
                }
                remain
            }
        };

        let size = clamp_size_to_lba(size);
        let start_lba = start / lb_size_bytes();
        let size_lba = size / lb_size_bytes();

        if start + size > usable_end_bytes {
            bail!("partition {} exceeds disk size", spec.name);
        }
        if start_lba < usable_start_lba || start_lba > usable_last_lba {
            bail!("partition {} start is outside usable LBA range", spec.name);
        }
        if start_lba + size_lba - 1 > usable_last_lba {
            bail!("partition {} exceeds usable LBA range", spec.name);
        }

        gdisk
            .add_partition_at(
                &spec.name,
                part_id,
                start_lba,
                size_lba,
                partition_types::LINUX_FS,
                0,
            )
            .map_err(|e| anyhow!("failed to add partition {}: {e}", spec.name))?;

        part_id = part_id.saturating_add(1);

        used_bytes = used_bytes.max(start + size);
    }

    let _ = gdisk
        .write()
        .map_err(|e| anyhow!("failed to write GPT: {e}"))?;

    if used_bytes > disk_size {
        bail!("GPT layout exceeds disk size after write");
    }
    Ok(())
}

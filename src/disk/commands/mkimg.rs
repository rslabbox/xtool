use anyhow::{bail, Context, Result};
use std::path::Path;

pub fn mkimg(path: &Path, size_bytes: u64, overwrite: bool) -> Result<()> {
    if path.exists() && !overwrite {
        bail!("image already exists, use --overwrite to replace");
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("failed to create parent directory: {}", parent.display())
        })?;
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(path)
        .with_context(|| format!("failed to create image {}", path.display()))?;
    file.set_len(size_bytes)
        .with_context(|| "failed to set image size".to_string())?;
    Ok(())
}

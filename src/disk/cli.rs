use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct DiskCli {
    /// Target disk image path
    #[arg(long, value_name = "PATH")]
    pub disk: PathBuf,

    /// Partition selector: index or name
    #[arg(long, value_name = "ID|NAME")]
    pub part: Option<String>,

    #[command(subcommand)]
    pub action: DiskAction,
}

#[derive(Subcommand, Debug)]
pub enum DiskAction {
    /// Create a blank disk image
    Mkimg {
        /// Image size (bytes or with K/M/G suffix)
        #[arg(long, value_name = "SIZE")]
        size: String,

        /// Allow overwrite existing file
        #[arg(long)]
        overwrite: bool,
    },

    /// Create GPT partition table using parameter.txt
    Mkgpt {
        /// Parameter file path (e.g. parameter.txt)
        #[arg(short = 'f', long, value_name = "PATH")]
        file: PathBuf,

        /// Alignment size (default 1M)
        #[arg(long, default_value = "1M", value_name = "SIZE")]
        align: String,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Format filesystem on partition or whole disk
    Mkfs {
        /// Filesystem type (ext4/fat32)
        #[arg(long, value_enum)]
        fstype: FsType,

        /// Volume label (optional)
        #[arg(long, value_name = "LABEL")]
        label: Option<String>,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// List files in directory
    Ls {
        /// Directory path inside image
        #[arg(value_name = "PATH", default_value = "/")]
        path: String,
    },

    /// Copy files between host and image
    Cp {
        #[arg(value_name = "SRC")]
        src: String,
        #[arg(value_name = "DST")]
        dst: String,

        /// Recursive copy for directories
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Overwrite existing destination
        #[arg(short = 'f', long)]
        force: bool,

        /// Preserve timestamps (best effort)
        #[arg(long)]
        preserve: bool,
    },

    /// Move/rename files between host and image
    Mv {
        #[arg(value_name = "SRC")]
        src: String,
        #[arg(value_name = "DST")]
        dst: String,

        /// Overwrite existing destination
        #[arg(short = 'f', long)]
        force: bool,
    },

    /// Remove file or directory inside image
    Rm {
        #[arg(value_name = "PATH")]
        path: String,

        /// Recursive remove for directories
        #[arg(short = 'r', long)]
        recursive: bool,

        /// Ignore missing target
        #[arg(short = 'f', long)]
        force: bool,

        /// Skip confirmation
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Create directory inside image
    Mkdir {
        #[arg(value_name = "PATH")]
        path: String,

        /// Create parent directories
        #[arg(short = 'p', long)]
        parents: bool,
    },

    /// Print file content inside image
    Cat {
        #[arg(value_name = "PATH")]
        path: String,

        /// Read only first N bytes
        #[arg(long, value_name = "N")]
        bytes: Option<usize>,

        /// Start offset
        #[arg(long, value_name = "N")]
        offset: Option<u64>,
    },

    /// Show disk and partition info
    Info {
        /// JSON output
        #[arg(long)]
        json: bool,
    },
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsType {
    Ext4,
    Fat32,
}

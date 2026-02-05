use anyhow::Result;

use super::cli::{DiskAction, DiskCli};
use super::gpt::resolve_partition_target;
use super::utils::parse_size;

mod cat;
mod cp;
mod info;
mod ls;
mod mkdir;
mod mkfs;
pub mod mkgpt;
pub mod mkimg;
mod mv;
mod rm;

pub fn run(cli: DiskCli) -> Result<()> {
    match cli.action {
        DiskAction::Mkimg { size, overwrite } => {
            let size_bytes = parse_size(&size)?;
            mkimg::mkimg(&cli.disk, size_bytes, overwrite)
        }
        DiskAction::Mkgpt { file, align, yes } => {
            let align_bytes = parse_size(&align)?;
            mkgpt::mkgpt(&cli.disk, &file, align_bytes, yes)
        }
        DiskAction::Mkfs { fstype, label, yes } => {
            let target = resolve_partition_target(&cli.disk, cli.part.as_deref())?;
            mkfs::mkfs(&cli.disk, &target, fstype, label.as_deref(), yes)
        }
        DiskAction::Ls { path } => {
            let target = resolve_partition_target(&cli.disk, cli.part.as_deref())?;
            ls::ls(&cli.disk, &target, &path)
        }
        DiskAction::Cp {
            src,
            dst,
            recursive,
            force,
            preserve,
        } => {
            let target = resolve_partition_target(&cli.disk, cli.part.as_deref())?;
            cp::cp(&cli.disk, &target, &src, &dst, recursive, force, preserve)
        }
        DiskAction::Mv { src, dst, force } => {
            let target = resolve_partition_target(&cli.disk, cli.part.as_deref())?;
            mv::mv(&cli.disk, &target, &src, &dst, force)
        }
        DiskAction::Rm {
            path,
            recursive,
            force,
            yes,
        } => {
            let target = resolve_partition_target(&cli.disk, cli.part.as_deref())?;
            rm::rm(&cli.disk, &target, &path, recursive, force, yes)
        }
        DiskAction::Mkdir { path, parents } => {
            let target = resolve_partition_target(&cli.disk, cli.part.as_deref())?;
            mkdir::mkdir(&cli.disk, &target, &path, parents)
        }
        DiskAction::Cat { path, bytes, offset } => {
            let target = resolve_partition_target(&cli.disk, cli.part.as_deref())?;
            cat::cat(&cli.disk, &target, &path, bytes, offset)
        }
        DiskAction::Info { json } => info::info(&cli.disk, json),
    }
}

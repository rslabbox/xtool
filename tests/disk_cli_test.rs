use std::fs;

use tempfile::TempDir;
use xtool::disk::{commands, fs as disk_fs, gpt as disk_gpt};

#[test]
fn disk_ext4_workflow() {
    let temp = TempDir::new().expect("temp dir");
    let disk = temp.path().join("disk.img");
    let hello = temp.path().join("hello.txt");
    fs::write(&hello, b"hello ext4").expect("write host file");

    commands::mkimg::mkimg(&disk, 32 * 1024 * 1024, false).expect("mkimg");

    let meta = fs::metadata(&disk).expect("disk exists");
    assert_eq!(meta.len(), 32 * 1024 * 1024);

    let target = disk_gpt::resolve_partition_target(&disk, None).expect("target");
    disk_fs::mkfs_ext4(&disk, &target, None).expect("mkfs ext4");

    disk_fs::mkdir(&disk, &target, "/etc", true).expect("mkdir");

    disk_fs::copy_host_to_image(&disk, &target, &hello, "/etc/hello.txt", false, false)
        .expect("copy host->image");

    let entries = disk_fs::list_dir(&disk, &target, "/etc").expect("ls");
    assert!(entries.iter().any(|e| e.name == "hello.txt"));

    let data = disk_fs::read_file(&disk, &target, "/etc/hello.txt", 0, None).expect("cat");
    assert_eq!(data, b"hello ext4");

    disk_fs::mv(&disk, &target, "/etc/hello.txt", "/etc/hi.txt", false).expect("mv");

    disk_fs::rm(&disk, &target, "/etc/hi.txt", false).expect("rm");

    let entries = disk_fs::list_dir(&disk, &target, "/etc").expect("ls");
    assert!(!entries.iter().any(|e| e.name == "hi.txt"));
}

#[test]
fn disk_gpt_fat32_workflow() {
    let temp = TempDir::new().expect("temp dir");
    let disk = temp.path().join("disk.img");
    let param = temp.path().join("parameter.txt");
    let hello = temp.path().join("hello.txt");
    fs::write(&hello, b"hello fat").expect("write host file");

    fs::write(
        &param,
        "CMDLINE: mtdparts=rk:0x04000000@0x00002000(boot),-@0x04002000(root:grow)\n",
    )
    .expect("write parameter file");

    commands::mkimg::mkimg(&disk, 256 * 1024 * 1024, false).expect("mkimg");

    commands::mkgpt::mkgpt(&disk, &param, 1024 * 1024, true).expect("mkgpt");

    let gdisk = disk_gpt::open_gpt(&disk, false).expect("open gpt");
    let parts = disk_gpt::map_partitions(&gdisk).expect("map partitions");
    assert_eq!(parts.len(), 2);

    let boot = disk_gpt::resolve_partition_target(&disk, Some("boot")).expect("part boot");
    disk_fs::mkfs_fat32(&disk, &boot, None).expect("mkfs fat32");

    disk_fs::mkdir(&disk, &boot, "/foo", false).expect("mkdir");

    disk_fs::copy_host_to_image(&disk, &boot, &hello, "/foo/hello.txt", false, false)
        .expect("copy host->image");

    let data = disk_fs::read_file(&disk, &boot, "/foo/hello.txt", 0, None).expect("cat");
    assert_eq!(data, b"hello fat");

    disk_fs::mv(&disk, &boot, "/foo/hello.txt", "/foo/hi.txt", false).expect("mv");

    disk_fs::rm(&disk, &boot, "/foo/hi.txt", false).expect("rm");

    let entries = disk_fs::list_dir(&disk, &boot, "/foo").expect("ls");
    assert!(!entries.iter().any(|e| e.name == "hi.txt"));
}
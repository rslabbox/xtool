use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use xtool::tftp::client::Client;
use xtool::tftp::client::config::ClientConfig;
use xtool::tftp::server::{Config, Server};

// Use serial_test to prevent port conflicts
use serial_test::serial;

fn setup_test_env() -> (PathBuf, PathBuf) {
    let _ = env_logger::builder().is_test(true).try_init();
    let test_dir = std::env::temp_dir().join(format!("tftp_test_{}", std::process::id()));
    let server_dir = test_dir.join("server");
    let client_dir = test_dir.join("client");

    fs::create_dir_all(&server_dir).unwrap();
    fs::create_dir_all(&client_dir).unwrap();

    (server_dir, client_dir)
}

fn cleanup_test_env(test_dir: &PathBuf) {
    let _ = fs::remove_dir_all(test_dir);
}

fn start_test_server(port: u16, root_dir: PathBuf) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let config =
            Config::default().merge_cli("127.0.0.1".to_string(), port, root_dir, false, false);
        let mut server = Server::new(&config).unwrap();
        server.listen();
    })
}

#[test]
#[serial]
fn test_file_download() {
    let (server_dir, client_dir) = setup_test_env();
    let test_dir = server_dir.parent().unwrap().to_path_buf();

    // Create test file
    let test_content = b"Hello TFTP World!";
    let server_file = server_dir.join("test.txt");
    let mut file = File::create(&server_file).unwrap();
    file.write_all(test_content).unwrap();
    drop(file);

    // Start server
    let port = 7000;
    let _server_handle = start_test_server(port, server_dir.clone());
    thread::sleep(Duration::from_millis(500));

    // Test download
    let config = ClientConfig::new("127.0.0.1".parse().unwrap(), port)
        .with_block_size(512)
        .with_timeout(Duration::from_secs(5));

    let client = Client::new(config).unwrap();
    let local_file = client_dir.join("downloaded.txt");
    let result = client.get("test.txt", &local_file);

    assert!(result.is_ok(), "Download failed: {:?}", result.err());

    // Verify content
    let downloaded_content = fs::read(&local_file).unwrap();
    assert_eq!(downloaded_content, test_content);

    cleanup_test_env(&test_dir);
}

#[test]
#[serial]
fn test_file_upload() {
    let (server_dir, client_dir) = setup_test_env();
    let test_dir = server_dir.parent().unwrap().to_path_buf();

    // Create test file
    let test_content = b"Upload Test Content";
    let client_file = client_dir.join("upload.txt");
    let mut file = File::create(&client_file).unwrap();
    file.write_all(test_content).unwrap();
    drop(file);

    // Start server
    let port = 7001;
    let _server_handle = start_test_server(port, server_dir.clone());
    thread::sleep(Duration::from_millis(500));

    // Test upload
    let config = ClientConfig::new("127.0.0.1".parse().unwrap(), port)
        .with_block_size(512)
        .with_timeout(Duration::from_secs(5));

    let client = Client::new(config).unwrap();
    let result = client.put(&client_file, "uploaded.txt");

    assert!(result.is_ok(), "Upload failed: {:?}", result.err());

    // Verify content on server
    thread::sleep(Duration::from_millis(200));
    let server_file = server_dir.join("uploaded.txt");
    let uploaded_content = fs::read(&server_file).unwrap();
    assert_eq!(uploaded_content, test_content);

    cleanup_test_env(&test_dir);
}

#[test]
#[serial]
fn test_large_file_transfer() {
    let (server_dir, client_dir) = setup_test_env();
    let test_dir = server_dir.parent().unwrap().to_path_buf();

    // Create large file (100KB)
    let test_content: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
    let client_file = client_dir.join("large.dat");
    let mut file = File::create(&client_file).unwrap();
    file.write_all(&test_content).unwrap();
    drop(file);

    // Start server
    let port = 7002;
    let _server_handle = start_test_server(port, server_dir.clone());
    thread::sleep(Duration::from_millis(500));

    // Test upload
    let config = ClientConfig::new("127.0.0.1".parse().unwrap(), port)
        .with_block_size(8192)
        .with_timeout(Duration::from_secs(10));

    let client = Client::new(config).unwrap();

    // Upload
    let result = client.put(&client_file, "large.dat");
    assert!(result.is_ok(), "Upload failed: {:?}", result.err());

    thread::sleep(Duration::from_millis(200));

    // Download
    let downloaded_file = client_dir.join("large_downloaded.dat");
    let result = client.get("large.dat", &downloaded_file);
    assert!(result.is_ok(), "Download failed: {:?}", result.err());

    // Verify content
    let downloaded_content = fs::read(&downloaded_file).unwrap();
    assert_eq!(downloaded_content.len(), test_content.len());
    assert_eq!(downloaded_content, test_content);

    cleanup_test_env(&test_dir);
}

#[test]
#[serial]
fn test_multiple_block_sizes() {
    let (server_dir, client_dir) = setup_test_env();
    let test_dir = server_dir.parent().unwrap().to_path_buf();

    // Create test file
    let test_content = b"Block Size Test Content";
    let server_file = server_dir.join("blocksize.txt");
    let mut file = File::create(&server_file).unwrap();
    file.write_all(test_content).unwrap();
    drop(file);

    // Start server
    let port = 7003;
    let _server_handle = start_test_server(port, server_dir.clone());
    thread::sleep(Duration::from_millis(500));

    // Test different block sizes
    for block_size in [512, 1024, 4096, 8192] {
        let config = ClientConfig::new("127.0.0.1".parse().unwrap(), port)
            .with_block_size(block_size)
            .with_timeout(Duration::from_secs(5));

        let client = Client::new(config).unwrap();
        let local_file = client_dir.join(format!("blocksize_{}.txt", block_size));
        let result = client.get("blocksize.txt", &local_file);

        assert!(
            result.is_ok(),
            "Download with block size {} failed: {:?}",
            block_size,
            result.err()
        );

        let downloaded_content = fs::read(&local_file).unwrap();
        assert_eq!(downloaded_content, test_content);
    }

    cleanup_test_env(&test_dir);
}

#[test]
#[serial]
fn test_nonexistent_file() {
    let (server_dir, client_dir) = setup_test_env();
    let test_dir = server_dir.parent().unwrap().to_path_buf();

    // Start server
    let port = 7004;
    let _server_handle = start_test_server(port, server_dir.clone());
    thread::sleep(Duration::from_millis(500));

    // Try to download non-existent file
    let config = ClientConfig::new("127.0.0.1".parse().unwrap(), port)
        .with_block_size(512)
        .with_timeout(Duration::from_secs(5));

    let client = Client::new(config).unwrap();
    let local_file = client_dir.join("nonexistent.txt");
    let result = client.get("nonexistent.txt", &local_file);

    assert!(
        result.is_err(),
        "Should fail when downloading non-existent file"
    );

    cleanup_test_env(&test_dir);
}

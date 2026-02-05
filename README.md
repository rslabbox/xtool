# xtool

A collection of amazing command-line tools, featuring TFTP server/client, HTTP static file server, serial port tools, file transfer service, and disk image utilities.

## Features

- **TFTP Server**: RFC-compliant TFTP server with support for read/write operations
- **TFTP Client**: Command-line client for downloading and uploading files
- **HTTP Server**: Static file server with directory listing
- **Serial Tools**: Serial port monitor, list, and network forwarding
- **File Transfer**: Upload/download files via token-based service with encryption support
- **Disk Utilities**: Create and manipulate disk images (GPT, ext4, FAT32)

## Installation

```bash
cargo install --path .
```

Or install from crates.io:

```bash
cargo install xtool
```

## Usage

### TFTP Server

Start a TFTP server:

```bash
# Basic usage - serve files from /var/tftp on port 69
xtool tftpd /var/tftp

# Specify IP and port
xtool tftpd -i 127.0.0.1 -p 6969 /path/to/directory

# Read-only mode
xtool tftpd -r /path/to/directory

# Single port mode (useful behind NAT)
xtool tftpd -s /path/to/directory
```

### TFTP Client

Download a file:

```bash
# Download file from server
xtool tftpc get 192.168.1.100 remote_file.txt

# Download to specific location
xtool tftpc get 192.168.1.100 remote_file.txt /path/to/local_file.txt

# Specify port and options
xtool tftpc get 192.168.1.100 remote_file.txt -p 6969 -b 8192 -t 10
```

Upload a file:

```bash
# Upload file to server
xtool tftpc put 192.168.1.100 local_file.txt

# Upload with different remote name
xtool tftpc put 192.168.1.100 local_file.txt remote_name.txt

# Specify options
xtool tftpc put 192.168.1.100 local_file.txt -p 6969 -b 8192 -t 10
```

### HTTP Server

Start a static file HTTP server:

```bash
# Basic usage - serve current directory on port 80
xtool http

# Specify port and directory
xtool http -p 8080 -d /path/to/serve
```

Features:
- Directory listing with HTML interface
- Automatic MIME type detection
- Serves `index.html` for directory requests

### Serial Console

List available serial ports:

```bash
xtool serial list
```

Monitor a serial port (interactive shell):

```bash
# Monitor COM1 with default baud rate (115200)
xtool serial monitor COM1

# Monitor with specific baud rate
xtool serial monitor COM1 -b 9600

# Use configuration file defaults (if port is set in .xtool.toml)
xtool serial monitor

# Interactive port selection if no port specified
xtool serial
```

Key bindings:
- `Ctrl + ]`: Exit monitor mode

Serial network forwarding (forward network to serial):

```bash
# Start server - forward TCP connections to serial port
xtool serial netd /dev/ttyUSB0 -b 115200 -p 5432

# Connect client - connect to serial server remotely
xtool serial netc -s 192.168.1.100 -p 5432
```

### File Transfer

Upload a file and get a token:

```bash
# Upload a file
xtool file send ./sample.txt

# Upload with download limit
xtool file send ./sample.txt --limit 3

# Upload directory (auto-compressed)
xtool file send ./myfolder

# Upload with encryption
xtool file send ./secret.txt -k mypassword

# Send a text message (no file)
xtool file send -m "Hello, World!"
```

Download a file by token:

```bash
# Download to current directory
xtool file get 081607

# Specify output path
xtool file get 081607 -o ./downloads/sample.txt

# Decrypt downloaded file
xtool file get 081607 -k mypassword
```

Specify custom server:

```bash
xtool file send ./sample.txt -s http://localhost:8080
xtool file get 081607 -s http://localhost:8080
```

### Disk Image Utilities

Create a blank disk image:

```bash
# Create 64MB disk image
xtool disk --disk disk.img mkimg --size 64M

# Create with overwrite
xtool disk --disk disk.img mkimg --size 1G --overwrite
```

Create GPT partition table:

```bash
# Create GPT from parameter.txt
xtool disk --disk disk.img mkgpt -f parameter.txt

# Skip confirmation
xtool disk --disk disk.img mkgpt -f parameter.txt -y
```

Format filesystem:

```bash
# Format partition as ext4
xtool disk --disk disk.img --part 1 mkfs --fstype ext4

# Format with label
xtool disk --disk disk.img --part 1 mkfs --fstype fat32 --label BOOT
```

List files in disk image:

```bash
# List root directory
xtool disk --disk disk.img ls

# List specific directory
xtool disk --disk disk.img --part 1 ls /boot
```

Copy files (host ↔ image):

```bash
# Copy from host to image
xtool disk --disk disk.img cp ./local.txt /dest.txt

# Copy from image to host
xtool disk --disk disk.img cp /remote.txt ./local.txt

# Recursive copy
xtool disk --disk disk.img cp -r ./mydir /destdir

# Force overwrite
xtool disk --disk disk.img cp -f ./local.txt /dest.txt
```

Move/rename files:

```bash
xtool disk --disk disk.img mv /oldname.txt /newname.txt
```

Remove files:

```bash
# Remove a file
xtool disk --disk disk.img rm /file.txt

# Remove directory recursively
xtool disk --disk disk.img rm -r /mydir

# Force remove (ignore errors)
xtool disk --disk disk.img rm -f /file.txt
```

Create directory:

```bash
# Create directory
xtool disk --disk disk.img mkdir /newdir

# Create parent directories
xtool disk --disk disk.img mkdir -p /a/b/c
```

Display file content:

```bash
# Cat a file
xtool disk --disk disk.img cat /boot/config.txt

# Read first N bytes
xtool disk --disk disk.img cat /large.bin --bytes 1024

# Read from offset
xtool disk --disk disk.img cat /file.bin --offset 512
```

Show disk info:

```bash
# Display disk and partition info
xtool disk --disk disk.img info

# JSON output
xtool disk --disk disk.img info --json
```

### Configuration File

Generate a configuration file (`.xtool.toml`):

```bash
# Generate config in current directory
xtool genconfig

# Force overwrite existing config
xtool genconfig --force
```

The configuration file supports settings for:
- TFTP server (ip, port, read_only, single_port)
- TFTP client (server, port, block_size, timeout)
- Serial (uart, baud, net_port, net_bind)

### Options

**TFTP Server Options:**
- `-i, --ip <IP>`: IP address to bind (default: 0.0.0.0)
- `-p, --port <PORT>`: Port to listen on (default: 69)
- `-r, --read-only`: Enable read-only mode (disable writes)
- `-s, --single-port`: Use single port mode

**TFTP Client Options:**
- `-p, --port <PORT>`: Server port (default: 69)
- `-b, --block-size <SIZE>`: Block size in bytes (default: 512, max: 65464)
- `-t, --timeout <SECONDS>`: Timeout in seconds (default: 5)

**HTTP Server Options:**
- `-p, --port <PORT>`: Port to listen on (default: 80)
- `-d, --path <PATH>`: Root directory to serve (default: current directory)

**Serial Options:**
- `-b, --baud <RATE>`: Baud rate (default: 115200)

**Disk Options:**
- `--disk <PATH>`: Target disk image path (required for all disk commands)
- `--part <ID|NAME>`: Select partition by index or name

## Examples

### Running Tests

```bash
# Run all tests
cargo test

# Run only integration tests
cargo test --test tftp_integration_test

# Run with output
cargo test -- --nocapture
```

### Example Session

**TFTP:**

Terminal 1 (Server):
```bash
$ xtool tftpd -p 6969 ./files
[14:20:00 INFO] Starting TFTP server on 0.0.0.0:6969
[14:20:00 INFO] Root directory: ./files
[14:20:00 INFO] TFTP server listening, press Ctrl+C to stop
```

Terminal 2 (Client):
```bash
$ xtool tftpc get 127.0.0.1 test.txt -p 6969
[14:20:15 INFO] Downloading test.txt from 127.0.0.1:6969
[14:20:15 INFO] Download completed successfully

$ xtool tftpc put 127.0.0.1 upload.txt -p 6969
[14:20:30 INFO] Uploading upload.txt to 127.0.0.1:6969
[14:20:30 INFO] Upload completed successfully
```

**HTTP Server:**
```bash
$ xtool http -p 8080 -d ./public
[14:20:00 INFO] HTTP server listening on http://0.0.0.0:8080
[14:20:00 INFO] Serving directory: /home/user/public
```

**Disk Image Operations:**
```bash
# Create and format a disk image
$ xtool disk --disk test.img mkimg --size 64M
$ xtool disk --disk test.img mkgpt -f parameter.txt -y
$ xtool disk --disk test.img --part 1 mkfs --fstype ext4

# Copy files to image
$ xtool disk --disk test.img cp ./boot.sh /boot.sh
$ xtool disk --disk test.img ls /
```

## License

Apache-2.0

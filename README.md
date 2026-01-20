# xtool

A collection of amazing command-line tools, currently featuring a full-featured TFTP (Trivial File Transfer Protocol) implementation.

## Features

- **TFTP Server**: RFC-compliant TFTP server with support for read/write operations
- **TFTP Client**: Command-line client for downloading and uploading files
- **Protocol Extensions**: Supports blocksize, timeout, transfer size, and window size options
- **Single Port Mode**: Optional mode for NAT-friendly operations

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
```

Key bindings:
- `Ctrl + ]`: Exit monitor mode

### File Transfer

Upload a file and get a token:

```bash
xtool file send ./sample.txt
# 输出: token
```

Download a file by token:

```bash
xtool file get 081607
```

Specify server and output path:

```bash
xtool file send ./sample.txt --server http://localhost:3000 --download-limit 2
xtool file get 081607 --server http://localhost:3000 --output ./downloads/sample.txt
```

### Options

**Server Options:**
- `-i, --ip <IP>`: IP address to bind (default: 0.0.0.0)
- `-p, --port <PORT>`: Port to listen on (default: 69)
- `-r, --read-only`: Enable read-only mode (disable writes)
- `-s, --single-port`: Use single port mode

**Client Options:**
- `-p, --port <PORT>`: Server port (default: 69)
- `-b, --block-size <SIZE>`: Block size in bytes (default: 512, max: 65464)
- `-t, --timeout <SECONDS>`: Timeout in seconds (default: 5)

**Serial Options:**
- `-b, --baud <RATE>`: Baud rate (default: 115200)

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

## License

Apache-2.0

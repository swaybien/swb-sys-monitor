# Resource Usage Monitor System

> English | [简体中文](README.md)

A minimal resource usage monitoring system designed specifically for embedded device monitoring scenarios. Supports 200+ high-concurrency client connections.

## Features

- **Minimal code implementation**: Pursuing code simplicity and maintainability
- **High performance, low memory usage**: Optimized for embedded devices
- **High concurrency access, lock-free algorithms**: Supports 200+ concurrent connections
- **Few dependencies, prefer system commands**: Reduces external dependencies
- **Data expires after 10 seconds**: No need to fetch data again when not expired
- **No fetching when no users access**: On-demand update strategy
- **No CSS, no JS**: Pure HTML implementation

## Quick Start

### Installation

#### Install pre-compiled binary package

```bash
cargo install swb-sys-monitor
```

#### Compile from source

```bash
# Clone repository
git clone https://github.com/swaybien/swb-sys-monitor.git
cd swb-sys-monitor

# Build release version
cargo build --release

# Or run directly
cargo run -- --help
```

### Usage

```bash
# Start with default configuration (listen on 0.0.0.0:8080, cache TTL 10 seconds)
./target/release/swb-sys-monitor

# Custom configuration
./target/release/swb-sys-monitor --address 127.0.0.1 --port 3000 --ttl 5

# Set log level
./target/release/swb-sys-monitor --log-level debug
```

### Access

After starting, visit `http://localhost:8080` in your browser to view system resource usage.

The page automatically refreshes every 10 seconds and displays the following information:

- CPU usage
- Memory usage (used, available, cached, free)
- Data acquisition timestamp

#### Health Check Endpoint

The system provides a health check endpoint at `http://localhost:8080/health` for monitoring service status:

```bash
curl http://localhost:8080/health
# Returns: OK
```

## Command Line Arguments

| Parameter     | Short Parameter | Default   | Description                                 |
| ------------- | --------------- | --------- | ------------------------------------------- |
| `--address`   | `-a`            | `0.0.0.0` | Server binding address                      |
| `--port`      | `-p`            | `8080`    | Server port                                 |
| `--ttl`       | `-t`            | `10`      | Cache TTL in seconds                        |
| `--log-level` | `-l`            | `info`    | Log level (trace, debug, info, warn, error) |
| `--help`      | `-h`            | -         | Show help information                       |

## Technical Architecture

### Core Components

1. **System Resource Acquisition Module**: Directly reads `/proc/stat` and `/proc/meminfo` to obtain system information
2. **Lock-free Data Caching Mechanism**: Uses `AtomicPtr` and `AtomicU64` to implement thread-safe caching
3. **High-concurrency Web Server**: High-performance HTTP server based on tokio + hyper
4. **Server-side HTML Rendering**: Pure HTML implementation, no CSS, no JS

### Performance Optimization

- **Lock-free algorithms**: Cache read/write uses atomic operations, supporting high-concurrency access
- **On-demand updates**: System information is updated only when data is expired and there are requests
- **Memory optimization**: Uses `String::with_capacity` to pre-allocate capacity, reducing reallocation
- **Function inlining**: Small functions use `#[inline]` attribute for performance optimization

## Development

### Build Requirements

- Rust 2024 Edition
- Linux system (currently only supports Linux)

### Development Commands

```bash
# Compile check
cargo check --all-targets

# Code style check
cargo clippy

# Run tests
cargo test

# Run benchmark tests
cargo bench

# Build release version
cargo build --release
```

### Test Coverage

The project includes comprehensive test coverage:

- Unit tests: caching mechanism, system information acquisition, HTTP server
- Integration tests: end-to-end functionality tests
- Performance benchmark tests: performance tests for key operations

## Deployment

### Automatic Installation (Recommended)

Use the provided installation script for automatic deployment to systemd systems:

```bash
# Clone repository
git clone https://github.com/swaybien/swb-sys-monitor.git
cd swb-sys-monitor

# Run installation script
sudo ./scripts/install.sh

# To uninstall
sudo ./scripts/uninstall.sh
```

For detailed deployment script usage instructions, please refer to [Deployment Guide](scripts/README.md).

### Manual Installation

#### System Service Configuration

Create a systemd service file `/etc/systemd/system/swb-sys-monitor.service`:

```ini
[Unit]
Description=Resource Status Monitor
After=network.target

[Service]
Type=simple
User=nobody
ExecStart=/usr/local/bin/swb-sys-monitor
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
sudo systemctl enable swb-sys-monitor
sudo systemctl start swb-sys-monitor
```

### Docker Deployment

#### Using Docker Compose (Recommended)

```bash
# Clone repository
git clone https://github.com/swaybien/swb-sys-monitor.git
cd swb-sys-monitor

# Start service
docker-compose up -d

# View logs
docker-compose logs -f swb-sys-monitor

# Stop service
docker-compose down
```

Access URL: http://localhost:18273

#### Using Docker Commands

```bash
# Build image
docker build -t swb-sys-monitor .

# Run container
docker run -d \
  --name swb-sys-monitor \
  --restart unless-stopped \
  -p 18273:8080 \
  --read-only \
  --tmpfs /tmp \
  --cap-drop ALL \
  --cap-add DAC_OVERRIDE \
  --security-opt no-new-privileges:true \
  swb-sys-monitor

# View logs
docker logs -f swb-sys-monitor

# Stop container
docker stop swb-sys-monitor
docker rm swb-sys-monitor
```

#### Docker Configuration Description

Docker deployment includes the following security features:

- **Read-only file system**: Prevents runtime modifications
- **Minimal privileges**: Removes all Linux capabilities, only adds necessary ones
- **Security options**: Enables `no-new-privileges` to prevent privilege escalation
- **Health check**: Automatically monitors service status
- **Non-root user**: Runs with unprivileged user inside the container

#### Custom Configuration

You can customize configuration through environment variables:

```yaml
# docker-compose.yml
services:
  swb-sys-monitor:
    build: .
    environment:
      - RUST_LOG=debug
    command:
      [
        "./swb-sys-monitor",
        "--address",
        "0.0.0.0",
        "--port",
        "8080",
        "--ttl",
        "5",
      ]
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

## Contributing

Issues and Pull Requests are welcome!
Please read the [Contributing Guide](CONTRIBUTING.md).

## Related Links

- [Design Document](docs/design.md)
- [Development Status](docs/status.md)
- [HTML Template Example](templates/index.html)

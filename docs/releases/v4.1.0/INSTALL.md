# SQLRustGo v4.1.0 — Installation Guide

> **Date**: 2026-10-23
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Purpose**: Complete installation instructions for v4.1.0.

## 1. System requirements

| Component | Minimum | Recommended |
|---|---|---|
| CPU | x86_64, 4 cores | x86_64, 16+ cores |
| RAM | 4 GB | 16 GB+ |
| Disk | 2 GB free | 10 GB+ (build + data) |
| OS | Linux 5.4+, macOS 12+, Windows 10+ | Linux 5.10+ |
| Rust | 1.75+ | 1.80+ |
| C compiler | gcc 9+ | gcc 12+ / clang 14+ |

## 2. Prerequisites

### 2.1 Linux (Debian/Ubuntu)

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    git \
    cargo \
    rustc \
    pkg-config \
    libssl-dev
```

### 2.2 Linux (RHEL/Fedora)

```bash
sudo dnf install -y \
    gcc \
    gcc-c++ \
    git \
    cargo \
    rust \
    openssl-devel
```

### 2.3 macOS (Homebrew)

```bash
brew install \
    rust \
    git \
    openssl
```

### 2.4 Windows (MSYS2)

```bash
pacman -S mingw-w64-x86_64-toolchain mingw-w64-x86_64-git
# Then install Rust via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## 3. Clone the repository

```bash
# Option A: from Gitea 252 (primary)
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout develop/v4.1.0

# Option B: from GitCode
git clone https://gitcode.com/BreavHeart/sqlrustgo.git
cd sqlrustgo
git checkout develop/v4.1.0

# Option C: from Gitee
git clone https://gitee.com/yinglichina/sqlrustgo.git
cd sqlrustgo
git checkout develop/v4.1.0
```

## 4. Build

```bash
# Release build (recommended for deployment)
cargo build --release --all-features

# Debug build (for development)
cargo build --all-features
```

Build time: ~5-15 minutes on 16-core machine (cold), ~2-5 minutes (warm cache).

## 5. Verify installation

```bash
# Check binary
cargo run --all-features -- --version

# Run a quick smoke test
cargo run --all-features -- cli "SELECT 1;"
# Expected output: 1

# Run unit tests (fast, ~2 min)
cargo test --all-features --lib
```

## 6. Configuration

The server reads configuration from:
1. Command-line arguments (highest priority)
2. Environment variables
3. Default values

### 6.1 Key command-line options

| Option | Default | Description |
|---|---|---|
| `--port` | 3306 | TCP port for MySQL wire protocol |
| `--host` | 0.0.0.0 | Bind address |
| `--data-dir` | ./data | Data directory for WAL and storage |
| `--log-level` | info | Log verbosity (trace/debug/info/warn/error) |
| `--workers` | 4 | Number of worker threads |
| `--max-connections` | 100 | Maximum concurrent connections |
| `--tls` | false | Enable TLS (requires cert files) |

### 6.2 Environment variables

| Variable | Description |
|---|---|
| `SQLRUSTGO_PORT` | Override port |
| `SQLRUSTGO_DATA_DIR` | Override data directory |
| `SQLRUSTGO_LOG_LEVEL` | Override log level |
| `SQLRUSTGO_WORKERS` | Override worker count |

## 7. Data directory setup

```bash
# Create data directory
mkdir -p /var/lib/sqlrustgo
chmod 750 /var/lib/sqlrustgo

# Start server with data dir
cargo run --all-features -- serve \
    --port 3306 \
    --data-dir /var/lib/sqlrustgo
```

The data directory contains:
- `wal/` — Write-Ahead Log files
- `tables/` — Table storage files
- `catalog.db` — Schema catalog
- `checkpoint.json` — Last checkpoint state

## 8. Run as a service

### 8.1 systemd (Linux)

```ini
# /etc/systemd/system/sqlrustgo.service
[Unit]
Description=SQLRustGo Database Server
After=network.target

[Service]
Type=simple
User=sqlrustgo
ExecStart=/usr/local/bin/sqlrustgo serve --port 3306 --data-dir /var/lib/sqlrustgo
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable sqlrustgo
sudo systemctl start sqlrustgo
```

### 8.2 launchd (macOS)

```xml
<!-- /Library/LaunchDaemons/com.sqlrustgo.server.plist -->
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.sqlrustgo.server</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/sqlrustgo</string>
        <string>serve</string>
        <string>--port</string>
        <string>3306</string>
        <string>--data-dir</string>
        <string>/var/lib/sqlrustgo</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>
```

## 9. Verify the server is running

```bash
# Check if port is listening
ss -tlnp | grep 3306

# Test connection
mysql -h 127.0.0.1 -P 3306 -u root -e "SELECT 1;"

# Check logs
journalctl -u sqlrustgo -f
```

## 10. Uninstall

```bash
# Stop service
sudo systemctl stop sqlrustgo
sudo systemctl disable sqlrustgo

# Remove binary
sudo rm /usr/local/bin/sqlrustgo

# Remove data (DANGEROUS — deletes all data)
sudo rm -rf /var/lib/sqlrustgo

# Remove config
sudo rm /etc/systemd/system/sqlrustgo.service
sudo systemctl daemon-reload
```

## 11. Troubleshooting

| Problem | Solution |
|---|---|
| `port already in use` | Kill existing process: `sudo lsof -i :3306` |
| `permission denied` on data dir | `sudo chown sqlrustgo:sqlrustgo /var/lib/sqlrustgo` |
| Build fails with `ring` error | Install C compiler: `apt install build-essential` |
| Server starts but no response | Check log level: `--log-level debug` |
| High memory usage | Reduce `--workers` or increase system RAM |

## 12. Security notes

- **Do not expose port 3306 to the public internet** without a firewall
- **Use TLS** for production: `--tls --cert /path/to/cert.pem --key /path/to/key.pem`
- **Set strong passwords** for users (not yet implemented in v4.1.0)
- **Restrict data directory** permissions to the service user only

## 13. References

- `docs/releases/v4.1.0/DEPLOYMENT_GUIDE.md` — production deployment
- `docs/releases/v4.1.0/QUICK_START.md` — 5-minute quick start
- `docs/releases/v4.1.0/ARCHITECTURE.md` — system architecture
- `docs/governance/GATE_CONDITIONS.md` — gate conditions

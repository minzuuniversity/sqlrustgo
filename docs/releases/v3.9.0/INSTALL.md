# Install — SQLRustGo v3.9.0

> This document covers every supported install path for v3.9.0: pre-built
> binaries, source build, Docker, package managers, cross-compilation,
> dependency list, and troubleshooting. If anything here disagrees with
> the in-repo [`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md), the deployment
> guide wins for production.

## 1. Quick start (TL;DR)

```bash
# macOS / Linux — Homebrew
brew install openclaw/tap/sqlrustgo
sqlrustgo --version   # → sqlrustgo 3.9.0

# Linux — pre-built binary
curl -L -o /tmp/sqlrustgo.tar.gz \
  https://github.com/openclaw/sqlrustgo/releases/download/v3.9.0/sqlrustgo-v3.9.0-linux-x86_64.tar.gz
tar xzf /tmp/sqlrustgo.tar.gz -C /tmp
sudo install -m 0755 /tmp/sqlrustgo-v3.9.0-linux-x86_64/bin/sqlrustgo /usr/local/bin/

# Docker
docker run --rm -p 5432:5432 -e SQLRUSTGO_PASSWORD=secret \
  openclaw/sqlrustgo:v3.9.0
```

## 2. Supported platforms

| Platform        | Pre-built | Source build | Package manager |
|-----------------|:---------:|:------------:|:---------------:|
| Linux x86_64    | ✅        | ✅           | apt, yum, dnf, apk |
| Linux aarch64   | ✅        | ✅           | apt, yum, dnf, apk |
| macOS x86_64    | ✅        | ✅           | brew, MacPorts |
| macOS aarch64 (M1/M2/M3) | ✅  | ✅           | brew           |
| Windows x86_64  | ✅        | ✅           | winget, chocolatey, scoop |
| Windows aarch64 | ❌        | ✅           | (build from source) |
| FreeBSD 13+     | ❌        | ✅           | (build from source) |

## 3. Pre-built binary

### 3.1 Download

Binaries for v3.9.0 are attached to the GitHub release
[openclaw/sqlrustgo v3.9.0](https://github.com/openclaw/sqlrustgo/releases/tag/v3.9.0).
Self-hosted Gitea mirror: `https://gitea-macmini:3000/openclaw/sqlrustgo/releases/tag/v3.9.0`.

Naming convention:

```
sqlrustgo-v3.9.0-<os>-<arch>.<ext>
```

| File | OS | Arch | Format |
|------|----|------|--------|
| `sqlrustgo-v3.9.0-linux-x86_64.tar.gz` | Linux | x86_64 | gzipped tar |
| `sqlrustgo-v3.9.0-linux-aarch64.tar.gz` | Linux | aarch64 | gzipped tar |
| `sqlrustgo-v3.9.0-darwin-x86_64.tar.gz` | macOS | x86_64 | gzipped tar |
| `sqlrustgo-v3.9.0-darwin-arm64.tar.gz` | macOS | aarch64 | gzipped tar |
| `sqlrustgo-v3.9.0-windows-x86_64.zip` | Windows | x86_64 | zip |
| `sqlrustgo-v3.9.0-windows-x86_64.msi` | Windows | x86_64 | MSI installer |

### 3.2 Verify the signature

Each binary is signed with the maintainer key
(`openclaw <openheart@gaoyuanyiyao.com>`, fingerprint
`8B17 5C29 7B5E 4A8C 9D3F  2E1A 6D44 8F0C 1A2B 3C4D`).

```bash
# Import the public key (one-time)
gpg --keyserver keys.openpgp.org --recv-keys 8B175C297B5E4A8C9D3F2E1A6D448F0C1A2B3C4D

# Verify
curl -L -O https://github.com/openclaw/sqlrustgo/releases/download/v3.9.0/sqlrustgo-v3.9.0-linux-x86_64.tar.gz
curl -L -O https://github.com/openclaw/sqlrustgo/releases/download/v3.9.0/sqlrustgo-v3.9.0-linux-x86_64.tar.gz.asc
gpg --verify sqlrustgo-v3.9.0-linux-x86_64.tar.gz.asc sqlrustgo-v3.9.0-linux-x86_64.tar.gz
# → "Good signature from \"openclaw <openheart@gaoyuanyiyao.com>\""
```

### 3.3 Install (Linux / macOS)

```bash
tar xzf sqlrustgo-v3.9.0-linux-x86_64.tar.gz
sudo install -m 0755 sqlrustgo-v3.9.0-linux-x86_64/bin/sqlrustgo /usr/local/bin/
sqlrustgo --version
```

The tarball also contains:

- `bin/sqlrustgo` — the server + CLI
- `share/man/man1/sqlrustgo.1` — manual page
- `share/sqlrustgo/examples/initdb.sql` — example bootstrap
- `etc/sqlrustgo/sqlrustgo.conf.default` — default config

To install the full payload:

```bash
sudo cp -r sqlrustgo-v3.9.0-linux-x86_64/* /
```

### 3.4 Install (Windows)

- **MSI**: double-click `sqlrustgo-v3.9.0-windows-x86_64.msi`, follow
  the wizard. Adds to PATH and registers the Windows service.
- **Zip**: extract anywhere, add the `bin\` directory to `PATH`:
  ```powershell
  Expand-Archive sqlrustgo-v3.9.0-windows-x86_64.zip C:\sqlrustgo
  [Environment]::SetEnvironmentVariable("Path", $env:Path + ";C:\sqlrustgo\bin", "User")
  sqlrustgo --version
  ```

## 4. Build from source

### 4.1 Prerequisites

| Tool | Version | Notes |
|------|---------|-------|
| Rust | 1.78+ (stable) | install via `rustup` |
| `cargo` | bundled with rustup | |
| `cc` | C99 | `apt install build-essential`, `xcode-select --install` |
| `pkg-config` | any | |
| `libssl-dev` | OpenSSL 1.1+ or 3.x | required for `rustls` TLS path |
| `cmake` | 3.20+ | only for Windows / cross-compile |
| `git` | 2.30+ | |

### 4.2 Build

```bash
git clone https://github.com/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout v3.9.0
cargo build --release --all-features
# Binary at target/release/sqlrustgo

# Smoke test
./target/release/sqlrustgo --version
./target/release/sqlrustgo repl <<< "SELECT 1+1;"
```

A full release build on a 16-core machine takes **6-8 minutes** for a
clean tree, **30-60 seconds** for an incremental rebuild.

### 4.3 Faster builds (dev cycle)

```bash
# Use the workspace shared target dir (saves ~1.2 GB per worktree)
export CARGO_TARGET_DIR=/var/tmp/sqlrustgo-target
cargo build --all-features

# Or limit the build to a single crate
cargo build -p executor --all-features
cargo test -p parser --all-features
```

### 4.4 Dependencies (high level)

| Category | Crates (top of tree) |
|----------|----------------------|
| Async runtime | `tokio 1.40+` (full features) |
| Parser | `nom 7.1`, `pest 2.7` (legacy lexer), `logos 0.14` |
| Storage | `sled 0.34` (legacy), custom heap pages in `crates/storage` |
| Network | `tokio-postgres 0.7`, `rustls 0.23`, `postgres-protocol 0.6` |
| Observability | `tracing 0.1`, `tracing-subscriber 0.3`, `metrics 0.23`, `prometheus 0.13` |
| Serialization | `serde 1.0`, `rmp-serde 1.3`, `bincode 1.3` |
| CLI | `clap 4.5`, `indicatif 0.17` |
| Crypto | `argon2 0.5`, `scrypt 0.11`, `sha2 0.10` |
| Testing | `proptest 1.4`, `criterion 0.5`, `rstest 0.21` |
| Cross-compile | `cross 0.2`, `cargo-zigbuild 0.19` |

The full workspace has 40+ crates (see `Cargo.toml`). Use
`cargo tree --workspace` to inspect the resolved graph.

## 5. Package managers

### 5.1 Homebrew (macOS, Linuxbrew)

```bash
brew tap openclaw/tap
brew install sqlrustgo
brew upgrade sqlrustgo   # when a new version is released
```

To install the latest unreleased:

```bash
brew install --HEAD sqlrustgo
```

### 5.2 apt (Debian 12+, Ubuntu 22.04+)

```bash
curl -fsSL https://openclaw.github.io/sqlrustgo/apt/openclaw.gpg \
  | sudo gpg --dearmor -o /etc/apt/keyrings/openclaw.gpg
echo "deb [signed-by=/etc/apt/keyrings/openclaw.gpg] https://openclaw.github.io/sqlrustgo/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/sqlrustgo.list
sudo apt update
sudo apt install sqlrustgo
```

### 5.3 yum / dnf (RHEL 9+, Rocky 9+, Fedora 39+)

```bash
sudo dnf config-manager --add-repo https://openclaw.github.io/sqlrustgo/yum/sqlrustgo.repo
sudo dnf install sqlrustgo
```

### 5.4 apk (Alpine 3.18+)

```bash
echo "https://openclaw.github.io/sqlrustgo/alpine/v3.18/main" \
  | sudo tee -a /etc/apk/repositories
sudo apk add sqlrustgo
```

### 5.5 Windows — winget

```powershell
winget install openclaw.sqlrustgo
```

### 5.6 Windows — chocolatey

```powershell
choco install sqlrustgo
```

### 5.7 Windows — scoop

```powershell
scoop bucket add openclaw https://github.com/openclaw/scoop-bucket
scoop install sqlrustgo
```

## 6. Docker

### 6.1 Official image

```bash
docker pull openclaw/sqlrustgo:v3.9.0
docker run --rm -p 5432:5432 \
  -e SQLRUSTGO_PASSWORD=secret \
  -v sqlrustgo-data:/var/lib/sqlrustgo \
  openclaw/sqlrustgo:v3.9.0
```

The image:

- Base: `debian:12-slim`
- User: `sqlrustgo` (uid 999), no root
- Exposed: `5432/tcp` (PostgreSQL v3 protocol)
- Healthcheck: `/_/health` every 10s
- Default config: `PG_MODE=warn`, `MAX_CONNECTIONS=200`,
  `STATEMENT_CACHE=1024`

### 6.2 docker-compose

See [`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md) §3 for the full compose
snippet. Minimal:

```yaml
services:
  sqlrustgo:
    image: openclaw/sqlrustgo:v3.9.0
    ports: ["5432:5432"]
    environment:
      SQLRUSTGO_PASSWORD: secret
      SQLRUSTGO_DATA_DIR: /var/lib/sqlrustgo
    volumes:
      - sqlrustgo-data:/var/lib/sqlrustgo
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:5432/_/health"]
      interval: 10s
      timeout: 3s
      retries: 5
volumes:
  sqlrustgo-data:
```

## 7. Cross-compilation

v3.9.0 supports cross-compilation via `cross` (which uses Docker) or
`cargo-zigbuild` (which uses Zig as the linker).

### 7.1 Linux x86_64 → Linux aarch64

```bash
rustup target add aarch64-unknown-linux-gnu
cross build --release --all-features --target aarch64-unknown-linux-gnu
# → target/aarch64-unknown-linux-gnu/release/sqlrustgo
```

### 7.2 Linux → Windows

```bash
rustup target add x86_64-pc-windows-gnu
cross build --release --all-features --target x86_64-pc-windows-gnu
```

### 7.3 macOS x86_64 → macOS aarch64

```bash
rustup target add aarch64-apple-darwin
cargo build --release --all-features --target aarch64-apple-darwin
# (requires Xcode + Apple Silicon SDK)
```

The release pipeline in
`.github/workflows/release.yml` (and the Gitea mirror equivalent)
produces all six targets via `cross` + `cargo-zigbuild`.

## 8. Windows-specific notes

- Long-path support: `git config --system core.longpaths true` is
  required for `cargo build` in deep workspace trees.
- Defender real-time scanning slows builds 2-3x; add the target
  directory to the exclusion list:
  ```powershell
  Add-MpPreference -ExclusionPath "$env:USERPROFILE\.cargo\target"
  ```
- The MSI installer registers a Windows service
  `SQLRustGo`; manage with `sc query SQLRustGo`,
  `sc start SQLRustGo`, etc.
- For WSL2, follow the Linux apt path inside the WSL distro.

## 9. macOS-specific notes

- Gatekeeper may quarantine the unsigned binary. Allow with:
  ```bash
  xattr -d com.apple.quarantine /usr/local/bin/sqlrustgo
  ```
- Apple Silicon: the pre-built `darwin-arm64` binary is universal-2
  and runs natively on M1/M2/M3.
- Code signing: official brew bottle and `.pkg` are signed with the
  openclaw Developer ID; the `brew` install path does not require
  `xattr -d`.

## 10. Verification

After install, run the bundled self-test:

```bash
sqlrustgo admin self-test
# → Running 8 checks...
#   [PASS] binary version
#   [PASS] data dir writable
#   [PASS] WAL directory
#   [PASS] max file handles >= 1024
#   [PASS] tcp listen on :5432
#   [PASS] SCRAM-SHA-256 auth
#   [PASS] TLS handshake (if certs present)
#   [PASS] TPC-H Q1 (in-process) <= 100ms
# All 8 checks passed.
```

## 11. Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| `sqlrustgo: command not found` | not in PATH | re-run `install` step or `export PATH=$PATH:/usr/local/bin` |
| `error while loading shared libraries: libssl.so.3` | missing OpenSSL 3 | `sudo apt install libssl3` (Debian 12) or `sudo dnf install openssl-libs` (RHEL 9) |
| `Too many open files` | ulimit too low | `ulimit -n 65535` in shell init; `LimitNOFILE=65535` in systemd unit |
| `permission denied: /var/lib/sqlrustgo` | wrong owner | `sudo chown -R sqlrustgo:sqlrustgo /var/lib/sqlrustgo` |
| `bind: address already in use` | port conflict | `ss -tlnp | grep 5432`; stop conflicting service or use `--port 5433` |
| `GLIBC not found` (Linux) | older glibc | use a static-musl build (not yet provided in v3.9.0 GA; plan: v3.9.1) |
| macOS: `bad CPU type` | wrong arch | download the `darwin-arm64` (M1/M2) or `darwin-x86_64` (Intel) variant |
| Windows: Defender blocks startup | real-time scan | add exclusion as in §8 |
| `cargo build` panics on tokio | sandbox CPU bug | `export RUSTFLAGS="-C target-cpu=native"` removed; use `target-cpu=x86-64-v3` |

If none of the above resolves, file an issue at
`https://gitea-macmini:3000/openclaw/sqlrustgo/issues` with the output
of:

```bash
sqlrustgo admin diagnostics --output /tmp/diag.json
# attach /tmp/diag.json to the issue
```

## 12. Upgrading from a prior install

See [`MIGRATION_GUIDE.md`](MIGRATION_GUIDE.md). In short: the
`sqlrustgo` binary location does not change between v3.8.0 and
v3.9.0; the data directory layout is compatible (no on-disk format
change); the wire protocol is forward-compatible (psql 12+ works
against v3.9.0 the same way as against v3.8.0).

## 13. Uninstall

- **Homebrew**: `brew uninstall sqlrustgo`
- **apt**: `sudo apt remove sqlrustgo && sudo apt autoremove`
- **yum / dnf**: `sudo dnf remove sqlrustgo`
- **MSI**: Settings → Apps → SQLRustGo → Uninstall
- **Manual**: `sudo rm /usr/local/bin/sqlrustgo`, then `rm -rf /var/lib/sqlrustgo` (data) and `/etc/sqlrustgo` (config)
- **Docker**: `docker stop sqlrustgo && docker rm sqlrustgo && docker rmi openclaw/sqlrustgo:v3.9.0`

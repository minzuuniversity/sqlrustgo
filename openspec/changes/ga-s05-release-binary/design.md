## Overview

固化构建流程，确保 Release Binary 的可重现性和完整性校验。

## Architecture

### Build Process

```
1. Check Environment
   - Verify Cargo.lock exists
   - Verify rust-toolchain.toml exists
   - Capture environment (RUSTC_VERSION, TARGET, BUILD_TIME)

2. Build Binary
   - cargo build --release --target <target>
   - Strip debug symbols for smaller binary

3. Generate Artifacts
   - Binary: sqlrustgo-{version}-{os}-{arch}
   - SHA256 checksum: sqlrustgo-{version}-{os}-{arch}.sha256
   - VERSION file: git commit, build time, rustc version

4. Verify Reproducibility (optional)
   - Compare checksum against known baseline
```

### Version Information

The `--version` flag outputs:
```
sqlrustgo 3.11.0 (git: abc123def, built: 2026-07-18T12:00:00Z, rustc: 1.80.0)
```

## Implementation Details

### 1. `scripts/build/release_binary.sh`

```bash
#!/bin/bash
VERSION=${1:-"3.11.0"}
TARGET=${2:-$(rustc -vV | grep host | cut -d' ' -f2)}
BUILD_TIME=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
GIT_COMMIT=$(git rev-parse HEAD)

# Build
cargo build --release --target "$TARGET"

# Strip (optional)
strip "target/$TARGET/release/sqlrustgo"

# Generate checksum
sha256sum "target/$TARGET/release/sqlrustgo" > "sqlrustgo-${VERSION}-${TARGET}.sha256"

# Generate VERSION file
echo "VERSION=${VERSION}" > VERSION
echo "GIT_COMMIT=${GIT_COMMIT}" >> VERSION
echo "BUILD_TIME=${BUILD_TIME}" >> VERSION
echo "RUSTC=$(rustc -V)" >> VERSION
```

### 2. Version Flag in sqlrustgo

Add `--version` to the main CLI:

```rust
fn main() {
    let matches = clap::Command::new("sqlrustgo")
        .version(env!("CARGO_PKG_VERSION"))
        .arg(clap::arg!(--version "Print version information"))
        .get_matches();
}
```

### 3. `scripts/verify_binary_checksum.sh`

```bash
#!/bin/bash
sha256sum -c sqlrustgo-*.sha256
```

## Dependencies

- `sha256sum` (Linux) or `shasum` (macOS)
- `rustc` and `cargo`
- `git`

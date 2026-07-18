## 1. Build Scripts

- [x] 1.1 Create `scripts/build/release_binary.sh`
- [x] 1.2 Implement environment verification (Cargo.lock, rust-toolchain.toml)
- [x] 1.3 Implement release build with `--release`
- [x] 1.4 Generate SHA256 checksum
- [x] 1.5 Generate VERSION file with git commit and build time

## 2. Verification Script

- [x] 2.1 Create `scripts/verify_binary_checksum.sh`
- [x] 2.2 Implement checksum verification

## 3. Version Flag

- [x] 3.1 Add `--version` flag to sqlrustgo CLI
- [x] 3.2 Output git commit hash in version string

## 4. Gate Script

- [x] 4.1 Create `scripts/gate/check_release_binary.sh`
- [x] 4.2 Verify build script exists and is valid bash
- [x] 4.3 Verify verification script exists
- [x] 4.4 Add to RC gate check list

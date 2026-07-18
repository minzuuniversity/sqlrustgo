## Why

GA 发布前必须确保 Release Binary 的可重现性和完整性校验。当前 CI 仅验证 `cargo build`，无法确保构建产物的一致性和可验证性。

## What Changes

- 新增 `scripts/build/release_binary.sh` — 可重现构建脚本
- 新增 `scripts/verify_binary_checksum.sh` — SHA 校验和验证脚本
- 修改 `sqlrustgo` 主程序 — 添加 `--version` 输出 git commit hash
- 新增 `Cargo.lock` 和 `rust-toolchain.toml` 版本固化

### 构建产物

1. **静态二进制**: `sqlrustgo-{version}-{os}-{arch}`
2. **SHA256 校验和**: `sqlrustgo-{version}-{os}-{arch}.sha256`
3. **Version 文件**: `VERSION` 包含 git commit hash、构建时间、RUSTC version

## Capabilities

### New Capabilities

- `release-binary`: Release Binary 可重现构建和校验能力

### Modified Capabilities

- None

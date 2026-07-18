# release-binary

Release Binary 可重现构建和校验能力。

## Functionality

### Core Features

1. **可重现构建**
   - 使用固定的 `Cargo.lock` 和 `rust-toolchain.toml`
   - 记录所有构建环境变量
   - 相同的源代码 + 相同的环境 = 相同的二进制

2. **SHA256 校验和**
   - 每个发布版本生成 `.sha256` 文件
   - 用户可验证二进制完整性

3. **Version 信息**
   - `--version` 输出 git commit hash、构建时间、RUSTC version

### User Interactions

- 下载发布版本
- 运行 `sha256sum -c sqlrustgo-*.sha256` 验证完整性
- 运行 `sqlrustgo --version` 查看版本信息

## Acceptance Criteria

- [ ] 构建脚本可重现
- [ ] SHA 校验和生成
- [ ] Version 信息完整

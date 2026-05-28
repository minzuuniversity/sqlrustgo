# Rustdoc CI 集成指南

> **版本**: v1.0
> **更新日期**: 2026-05-29
> **维护人**: hermes-z440

---

## 一、概述

Rustdoc CI 用于自动验证 SQLRustGo 代码库可生成完整文档，包括私有项。

**目标**: 确保 `cargo doc --all-features --no-deps` 执行无错误和警告。

---

## 二、配置说明

### 2.1 .cargo/config.toml

```toml
[build]
# Enable rustdoc documentation for private items
# Usage: cargo doc --all-features --no-deps
rustdocflags = ["--document-private-items"]
```

此配置确保 `cargo doc` 生成包含私有项的文档。

### 2.2 GitHub Actions Workflow

**文件**: `.github/workflows/rustdoc-ci.yml`

| 触发条件 | 说明 |
|----------|------|
| `push` 到 `main`, `develop/*` | 每次推送运行 |
| `pull_request` 到 `main`, `develop/*` | PR 检查 |

### 2.3 CI 执行步骤

1. **Checkout** - 克隆代码仓库
2. **Setup Rust** - 安装 Rust 工具链
3. **Cache** - 缓存 cargo registry/index/build
4. **Generate documentation** - 执行 `cargo doc --all-features --no-deps`
5. **Check for warnings** - 检查是否有 rustdoc 警告
6. **Upload artifact** - 仅 `main` 分支上传文档产物

---

## 三、本地验证

### 3.1 生成文档

```bash
# 生成完整文档
cargo doc --all-features --no-deps

# 本地查看
cargo doc --open

# 生成特定 crate 的文档
cargo doc -p sqlrustgo-network --no-deps
```

### 3.2 CI 脚本

```bash
# 使用 CI 脚本本地验证
bash scripts/ci/check_rustdoc.sh
```

### 3.3 检查警告

```bash
# 捕获警告
cargo doc --all-features --no-deps 2>&1 | grep "warning:"

# 详细警告
cargo doc --all-features --no-deps --message-format=short 2>&1
```

---

## 四、Artifacts

### 4.1 文档输出位置

| 环境 | 位置 |
|------|------|
| 本地构建 | `target/doc/` |
| CI Artifacts | `target/doc/` (压缩上传) |

### 4.2 访问文档

```bash
# 本地
open target/doc/index.html

# CI Artifacts (main 分支)
# 下载 artifacts 压缩包，解压后查看
```

---

## 五、门禁标准

| 检查项 | 要求 | 状态 |
|--------|------|------|
| 生成无错误 | `cargo doc` 退出码 0 | 必须 |
| 无警告 | 无 `warning:` 输出 | 必须 |
| 文档完整 | 包含所有 public/private 项 | 必须 |

---

## 六、相关文件

| 文件 | 说明 |
|------|------|
| `.cargo/config.toml` | rustdocflags 配置 |
| `.github/workflows/rustdoc-ci.yml` | GitHub Actions CI |
| `scripts/ci/check_rustdoc.sh` | 本地 CI 脚本 |
| `docs/api/README.md` | API 文档入口 |

---

## 七、故障排查

### 7.1 文档生成失败

```bash
# 检查依赖
cargo update

# 清理后重试
cargo clean
cargo doc --all-features --no-deps
```

### 7.2 警告过多

```bash
# 查看具体警告
cargo doc --all-features --no-deps 2>&1 | grep -E "^warning" | sort | uniq -c

# 修复后验证
cargo doc --all-features --no-deps 2>&1 | grep -c "warning:"
```

### 7.3 CI 超时

rustdoc 生成可能需要 10+ 分钟，考虑：
- 增加 CI timeout
- 使用 matrix 策略并行生成不同 crate
- 启用编译缓存
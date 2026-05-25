# 构建指南

> 版本：v3.4.0 GA | 日期：2026-05-25

---

## 环境要求

| 依赖 | 版本 | 说明 |
|------|------|------|
| Rust | 1.75+ | 推荐 stable |
| CMake | 3.20+ | 构建系统 |
| Python | 3.10+ | 测试框架 |
| LLVM/Clang | 可选 | 覆盖率测试 |

---

## 快速开始

```bash
# 克隆仓库
git clone https://github.com/openclaw/sqlrustgo.git
cd sqlrustgo

# Debug 构建
cargo build

# Release 构建
cargo build --release
```

---

## 构建选项

### 特性（Features）

```bash
# 默认特性
cargo build

# 所有特性
cargo build --all-features

# 特定特性
cargo build --features "wal,network"
```

### 构建类型

```bash
# Debug（快编译，低优化）
cargo build

# Release（慢编译，高优化）
cargo build --release

# 混合构建（部分优化）
cargo build -p sqlrustgo --profile release-lto
```

---

## 测试构建

```bash
# 单元测试
cargo test --lib

# 所有测试
cargo test --all-features

# 带覆盖率
cargo install cargo-llvm-cov
cargo llvm-cov test --all-features
```

---

## 性能构建（生产环境）

```bash
# LTO（链接时优化）
cargo build --release --profile release-lto

# 需要的 profile 配置（ Cargo.toml 中定义）
[profile.release-lto]
inherits = "release"
lto = true
codegen-units = 1
```

---

## 跨平台构建

### Linux

```bash
# 需要
- Rust 1.75+
- CMake 3.20+
- Python 3.10+
- GCC/Clang
```

### macOS

```bash
# Apple Silicon
brew install llvm cmake rustup
export CC=/opt/homebrew/opt/llvm/bin/clang
export CXX=/opt/homebrew/opt/llvm/bin/clang++
cargo build --release
```

### Windows

```bash
# 需要
- Visual Studio 2022+ (with C++ tools)
- CMake 3.20+
- Rust 1.75+
```

---

## 依赖卡点

常见问题：

**1. LLVM 未找到**
```bash
export LLVM_CONFIG=/path/to/llvm-config
cargo build
```

**2. 编译超时**
```bash
# 使用 cargo-chef 缓存
cargo chef prepare --recipe-path recipe.json
cargo chef cook --release
```

**3. 链接错误**
```bash
# Linux
apt install libssl-dev pkg-config
# macOS
brew install openssl
```

---

## 增量构建

```bash
# sccache 加速
cargo install sccache
export RUSTC_WRAPPER=sccache
cargo build --release
```

---

## 构建输出

```
target/
├── debug/          # Debug 构建
│   └── sqlrustgo    # 可执行文件
├── release/        # Release 构建
│   └── sqlrustgo   # 可执行文件（生产用）
└── doc/            # HTML 文档
```

---

## 验证构建

```bash
# 检查版本
./target/release/sqlrustgo --version

# 运行简单查询
./target/release/sqlrustgo "SELECT 1"

# TPC-H 测试（需要数据）
./target/release/sqlrustgo tpch-bench --queries all
```

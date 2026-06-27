# SQLRustGo v3.8.0 开发者指南

> **版本**: v3.8.0
> **发布日期**: 2026-06-04

---

## 快速开始

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable

# 克隆并编译
git clone http://192.168.0.252:3000/openclaw/sqlrustgo.git
cd sqlrustgo
git checkout develop/v3.8.0
cargo build --release

# 运行测试
cargo test --all-features
```

---

## 关键命令

```bash
# 编译
cargo build --release
cargo build --release -p sqlrustgo-mysql-server

# 测试
cargo test --all-features
cargo test --lib
cargo test --test integration_tests --all-features

# Lint
cargo clippy --all-features -- -D warnings
cargo fmt --check --all
```

---

## 门禁检查

```bash
# 快速检查
bash scripts/gate/check_docs_links.sh
bash scripts/gate/check_coverage.sh

# 完整检查
cargo clippy --all-features -- -D warnings
cargo fmt --check --all
cargo test --all-features
```

---

## 提交规范

```bash
git checkout -b fix/issue-NNNN-description
git commit -m "feat(executor): add merge join algorithm"
git push origin fix/issue-NNNN-description
```

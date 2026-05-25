# Contributing to SQLRustGo

SQLRustGo 是 Rust 语言实现的 SQL 数据库引擎。本指南帮助开发者参与贡献。

> 当前版本：v3.4.0 GA（2026-05-25）

---

## 开发环境

### 依赖

- **Rust**：1.75+
- **CMake**：3.20+
- **Python**：3.10+（测试框架）
- **cargo**：最新稳定版

### 推荐工具

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 安装 LLVM（覆盖率测试）
# macOS
brew install llvm
# Linux (Ubuntu/Debian)
apt-get install llvm clang
```

---

## 测试命令

```bash
# 快速测试（~5分钟）
make unit

# 完整测试（~30分钟）
make allunit

# 覆盖率报告
make coverage

# 性能基准测试（TPC-H SF=1）
make benchmark

# 格式化代码
make fmt

# 运行 clippy
make clippy
```

### Alpha/Beta/RC/GA 门禁

v3.4.0 使用四阶段发布门禁：

```bash
# Alpha 门禁
bash scripts/gate/check_alpha_v340.sh

# Beta 门禁
bash scripts/gate/check_beta_v340.sh

# RC 门禁
bash scripts/gate/check_rc_v340.sh

# GA 门禁（全部检查）
bash scripts/gate/check_ga_v340.sh
```

门禁检查项包括：
- G1: Build（cargo build --release --workspace）
- G2: Test（cargo test --all-features）
- G3: Clippy（cargo clippy --all-features）
- G4: Format（cargo fmt --all -- --check）
- G5: Coverage（覆盖率 >= 85%）
- G6: Security（cargo audit）
- G7: SQL Compatibility（SQL 兼容性测试）
- G8: GMP Features（GMP 功能验证）
- G9: TPC-H Benchmark（性能基准）
- G10: Stability（稳定性测试）
- G11: Documentation（文档检查）
- G12: MySQL Protocol（MySQL 协议兼容性）
- G13: Trust Infrastructure（Trust Infra 验证）

---

## Pull Request 流程

1. **Fork** 仓库并创建特性分支
2. **开发**：遵循代码规范
3. **测试**：确保 `make unit` 和 `make allunit` 通过
4. **提交**：使用语义化提交信息（Conventional Commits）
5. **PR**：创建 PR 并等待 CI 通过
6. **审查**：至少一名维护者审查通过后合并

### 代码规范

```bash
# 格式化
cargo fmt --all

# Clippy 检查
cargo clippy --all-features -- -D warnings

# 禁止直接 push 到 develop/v3.4.0 或 main
# 所有变更必须通过 PR
```

### 提交信息格式

```
<type>(<scope>): <description>

Types: feat, fix, docs, test, refactor, perf, chore
Scope: core, sql, executor, optimizer, storage, network, etc.
```

示例：
```
feat(sql): 新增 window function 支持
fix(executor): 修复聚合查询内存泄漏
docs(readme): 更新 v3.4.0 README
```

---

## 核心模块

| 模块 | 路径 | 说明 |
|------|------|------|
| Core Engine | src/core/ | 数据库核心 |
| SQL Parser | src/sql/ | SQL 解析 |
| Executor | src/executor/ | 查询执行 |
| Optimizer | src/optimizer/ | 查询优化 |
| Storage | src/storage/ | 存储引擎 |
| Network | src/network/ | 网络协议 |
| GMP | src/gmp/ | GMP 功能 |
| Trust | src/trust/ | Trust Infrastructure |

---

## 混沌测试

v3.4.0 引入了 Chaos Testing 用于验证故障恢复：

```bash
# OOM 测试（模拟内存耗尽）
make chaos-oom

# I/O Error 测试（模拟磁盘故障）
make chaos-io

# Crash 测试（模拟进程崩溃）
make chaos-crash
```

---

## 发布流程

v3.4.0 使用四阶段发布流程：

```
Alpha → Beta → RC → GA
```

每个阶段有对应的门禁检查，详见 [TESTING.md](docs/TESTING.md)。

---

## 相关文档

- [BUILD.md](BUILD.md) — 构建指南
- [TESTING.md](docs/TESTING.md) — 测试体系文档
- [docs/governance/GATE_SPEC_MASTER.md](docs/governance/GATE_SPEC_MASTER.md) — 门禁规范
- [docs/releases/v3.4.0/](docs/releases/v3.4.0/) — v3.4.0 版本文档

---

## 行为准则

- 尊重他人，保持专业
- 代码审查聚焦技术与质量
- 不得在代码中引入恶意内容

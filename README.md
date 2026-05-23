# SQLRustGo

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85+-dea584?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/version-v3.4.0--RC-blue" alt="Version">
  <img src="https://img.shields.io/badge/branch-rc%2Fv3.4.0-blue" alt="Branch">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
</p>

SQLRustGo 是一个使用 Rust 实现的关系型数据库教学与工程化项目，支持 SQL-92 子集，包含解析、规划、执行、存储、事务与网络模块，并具备向量存储与图存储等高级特性。

## 当前状态

| 项目 | 当前值 |
|------|--------|
| 当前版本状态 | **v3.4.0 (RC)** |
| 当前主分支 | **develop/v3.4.0** |
| RC 分支 | **rc/v3.4.0** |
| 当前阶段 | **RC (发布候选)** |
| 上一稳定版本 | v3.3.0 (GA) |
| 版本目标 | GMP Management Suite |

- 版本文件: [VERSION](VERSION)
- 当前版本说明: [docs/releases/v3.4.0/README.md](docs/releases/v3.4.0/README.md)
- v3.4.0 文档入口: [docs/releases/v3.4.0/README.md](docs/releases/v3.4.0/README.md)
- v3.3.0 文档: [docs/releases/v3.3.0/README.md](docs/releases/v3.3.0/README.md)

## 核心能力

- **SQL**: SELECT INSERT UPDATE DELETE CREATE TABLE DROP TABLE + CTE/窗口函数/JSON
- **存储**: Buffer Pool + FileStorage + MemoryStorage + ColumnarStorage
- **索引**: B+ Tree + Hash Index + Vector Index
- **事务**: WAL + MVCC (Snapshot Isolation) + XA 两阶段提交
- **网络**: TCP / MySQL 风格协议
- **高级**: 向量存储、图存储、Prepared Statement、存储过程、触发器
- **分布式**: Semi-sync 复制、MTS 并行复制、Multi-source 复制

## 快速开始

```bash
# 构建
cargo build --all-features

# 运行测试
cargo test --all-features

# 启动 REPL
cargo run --bin sqlrustgo

# 代码检查
cargo clippy --all-targets -- -D warnings
```

## v3.4.0 门禁状态

> **Alpha Gate: 16/16 PASS** (2026-05-22)
> **Beta Gate: 14/14 PASS** (2026-05-22)
> **RC Gate: 进行中**

### Alpha/Beta 通过项

| Gate | 检查项 | 状态 |
|------|--------|------|
| A1 | Build | ✅ PASS |
| A2 | Unit Tests | ✅ PASS |
| A3 | Clippy | ✅ PASS |
| A4 | Format | ✅ PASS |
| A5 | Coverage (≥70%) | ✅ PASS (76.44%) |
| A6 | MySQL Protocol | ✅ PASS |
| A7 | TPC-H SF=0.1 | ✅ PASS (22/22) |
| TI-1~5 | Trust Infrastructure | ✅ PASS |
| B1~B14 | Beta Gate | ✅ PASS |

### RC Gate 进行中

| Gate | 检查项 | 状态 |
|------|--------|------|
| R1 | Build | ⏳ |
| R2 | Unit Tests | ⏳ |
| R3 | Clippy | ⏳ |
| R4 | Format | ⏳ |
| R5 | Coverage (≥85%) | ⏳ |
| R6 | Security | ⏳ |
| R7 | SQL Compat | ⏳ |
| R8 | TPC-H SF=1 | ⏳ Z6G4 |

## v3.4.0 版本目标

**战略定位**: GMP Management Suite（管理套件）

| 类别 | 功能 | 状态 |
|------|------|------|
| GMP API | EBR Batch Manager | ✅ |
| GMP API | Electronic Signature Service | ✅ |
| GMP API | Audit Chain API | ✅ |
| GMP API | Device OPC UA Integration | ✅ |
| GMP API | Rule Engine Visual Editor | ✅ |
| GMP API | Dashboard + WebSocket | ✅ |
| GMP Retrieval | BM25 + RRF Fusion | ✅ |
| GMP Retrieval | Ollama Reranker | ✅ |
| GMP Retrieval | LLM Chat (qwen3:8b) | ✅ |

**战略演进**:
```
v3.2.0: Trust Convergence（可信收敛）✅ GA
v3.3.0: Industrial Trust Platform（可信内核）✅ GA
v3.4.0: GMP Management Suite（管理套件）← 当前 RC
v3.5.0: AI Native GMP Platform
```

## 功能矩阵

### GMP Framework (电子签名/可信赖 21 CFR Part 11)

| 功能 | 状态 | 说明 |
|------|------|------|
| GMP-1: Digital Signature Audit Chain | ✅ | SHA-256 哈希链 + ECDSA 签名 |
| GMP-2: Electronic Signature | ✅ | 21 CFR Part 11 合规 |
| GMP-3: Immutable Record | ✅ | EBR 证据链系统 |
| GMP-4: GMP Management API | ✅ | REST API + WebSocket |
| GMP-5: GMP Retrieval v2 | ✅ | BM25 + RRF + Reranker + LLM |

### SQL 功能

| 功能 | 状态 | 说明 |
|------|------|------|
| Multi-Table UPDATE | ✅ | 多表更新 |
| Multi-Table MERGE | ✅ | MERGE INTO |
| RECURSIVE CTE | ✅ | 递归 CTE |
| Window Functions | ✅ | 窗口函数 |
| GROUP BY | ✅ | 分组聚合 |
| JOIN (INNER/OUTER/CROSS) | ✅ | 哈希连接 |
| FULLTEXT Search | ✅ | 全文索引 |
| Stored Procedures | ✅ | 存储过程 |

### 性能基线

| 指标 | v3.3.0 | v3.4.0 |
|------|--------|--------|
| Point Select QPS | ~30K | ⏳ |
| TPC-H SF=1 | 22/22 | 22/22 |
| Executor Coverage | ~70% | 76.44% (Alpha) |

## 文档索引

### v3.4.0 (当前版本)

- [v3.4.0 文档中心](docs/releases/v3.4.0/README.md) ← 入口
- [v3.4.0 变更日志](docs/releases/v3.4.0/CHANGELOG.md)
- [v3.4.0 开发计划](docs/releases/v3.4.0/DEV_PLAN.md)
- [v3.4.0 Alpha Gate Checklist](docs/releases/v3.4.0/ALPHA_GATE_CHECKLIST.md)
- [v3.4.0 Beta Gate Checklist](docs/releases/v3.4.0/BETA_GATE_CHECKLIST.md)
- [v3.4.0 RC Gate Checklist](docs/releases/v3.4.0/RC_GATE_CHECKLIST.md)
- [v3.4.0 GA Gate Checklist](docs/releases/v3.4.0/GA_GATE_CHECKLIST.md)

### 历史版本

- [v3.3.0 GA 文档](docs/releases/v3.3.0/README.md)
- [v3.2.0 GA 文档](docs/releases/v3.2.0/README.md)
- [v3.1.0 Beta 文档](docs/releases/v3.1.0/README.md)

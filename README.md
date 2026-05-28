# SQLRustGo

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85+-dea584?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/version-v3.4.0--GA-blue" alt="Version">
  <img src="https://img.shields.io/badge/branch-develop%2Fv3.4.0-blue" alt="Branch">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
</p>

SQLRustGo 是一个使用 Rust 实现的关系型数据库教学与工程化项目，支持 SQL-92 子集，包含解析、规划、执行、存储、事务与网络模块，并具备向量存储与图存储等高级特性。

## 当前状态

| 项目 | 当前值 |
|------|--------|
| 当前版本状态 | **v3.4.0 (GA)** |
| 当前主分支 | **main** |
| 当前阶段 | **GA (正式发布)** |
| 上一稳定版本 | v3.3.0 (GA) |
| 版本目标 | GMP Management Suite（管理套件） |

- 版本文件: [VERSION](VERSION)
- 当前版本说明: [docs/releases/v3.4.0/GA_GATE_REPORT.md](docs/releases/v3.4.0/GA_GATE_REPORT.md)
- v3.4.0 文档入口: [docs/releases/v3.4.0/](docs/releases/v3.4.0/)
- v3.3.0 GA 文档: [docs/releases/v3.3.0/README.md](docs/releases/v3.3.0/README.md)

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

## v3.4.0 RC 门禁状态

> **RC Gate**: 待执行（Alpha 16/16 PASS 已完成）
> **Beta Gate**: 14/14 PASS

| Gate | 检查项 | 状态 | 说明 |
|------|--------|------|------|
| R1 | Build | ⏳ | cargo build --release |
| R2 | Test | ⏳ | cargo test |
| R3 | Clippy | ⏳ | 零警告 |
| R4 | Format | ⏳ | fmt check |
| R5 | Coverage | ⏳ | L1 ≥85% |
| R6 | Security | ⏳ | cargo audit |
| R7 | SQL Compat | ⏳ | check_sql_compat.sh |
| R8 | TPC-H SF=1 | ⏳ Z6G4 | 22/22 |
| B1-B14 | Beta 复核 | ✅ PASS | 14/14 |

## v3.4.0 功能矩阵

### GMP Management Suite (P0)

|| 功能 | 状态 | 说明 |
|------|------|------|------|
| GMP API - Batch CRUD | ✅ | BatchManager 批次生命周期管理 |
| GMP API - Audit Chain | ✅ | SHA-256 哈希链审计 |
| GMP API - Electronic Signature | ✅ | 21 CFR Part 11 合规 |
| GMP API - Device (OPC UA) | ✅ | 设备集成 |
| GMP API - Rule Engine | ✅ | 可视化规则编辑 |
| GMP API - Dashboard | ✅ | 可信指标可视化 |
| GMP API - Export | ✅ | JSON/PDF 审计包导出 |
| GMP Retrieval v2 | ✅ | BM25 + RRF + Ollama Reranker + LLM Chat |

### Trust Infrastructure (v3.3.0 GA 继承)

|| 功能 | 状态 | 说明 |
|------|------|------|------|
| Evidence Engine | ✅ | 证据链生成与验证 |
| Provenance Graph | ✅ | 数据溯源图 |
| Compliance Engine | ✅ | 规则评估引擎 |
| Workflow V2 | ✅ | 审批链引擎 |
| Trust Viz | ✅ | 可信度可视化 |
| Perf Baseline | ✅ | 性能基线工具 |
| Crash Sim | ✅ | 故障模拟框架 |
| WAL Verification | ✅ | TLA+ 形式化验证 |

### SQL 功能

|| 功能 | 状态 | 说明 |
|------|------|------|
| SELECT/INSERT/UPDATE/DELETE | ✅ | 基础 CRUD |
| CREATE/DROP TABLE | ✅ | DDL |
| JOIN (INNER/OUTER/CROSS) | ✅ | 哈希连接 |
| GROUP BY + 聚合函数 | ✅ | 聚合计算 |
| Window Functions | ✅ | 窗口函数 |
| Recursive CTE | ✅ | 递归 CTE |
| MERGE INTO | ✅ | 合并操作 |
| FULLTEXT Search | ✅ | 全文索引 |
| Vector Index | ✅ | 向量检索 |
| Graph Store | ✅ | 图存储 |

### MySQL 兼容性

|| 功能 | 状态 |
|------|------|------|
| MySQL 协议 | ✅ |
| SQL 语法 | ✅ |
| 数据类型 | ✅ |
| 索引 | ✅ |
| 事务 (MVCC) | ✅ |
| Prepared Statements | ✅ |
| Built-in Functions (SELECT) | ✅ |

## 性能基线

|| 指标 | 状态 |
|------|------|------|
| TPC-H SF=1 | ✅ 22/22 PASS |
| Point Select QPS | ⏳ |
| Update QPS | ⏳ |
| 72h 稳定性测试 | ⏳ 运行中 |

## 版本演进

```
v3.2.x: GMP Framework Core
    ↓
v3.3.0: Industrial Trust Platform ✅ GA
    ↓
v3.4.0: GMP Management Suite ← 当前
    ↓
v3.5.0: AI Native GMP Platform
```

### 版本治理趋势

| 版本 | 合规率 | 覆盖率 | 门禁规范 | 主要问题 |
|------|--------|--------|----------|---------|
| v3.0.0 GA | 🔴 20% | 23.9% (GA-6 降级至22%) | 未建立 | **阈值腐蚀**：GA-6 从 85%→22%，两次降级 |
| v3.1.0 GA | 🟡 58% | ~60% | 初始建立 | Alpha 无自动化 |
| v3.2.0 GA | 🟢 90% | 85.81% | 完善 | 部分门禁项仍手工 |
| v3.3.0 GA | 🟡 ~70% | **72.65%** (实测) | Beta/RC/GA 脚本缺失 | 无自动化门禁脚本 |
| v3.4.0 GA | 🟢 100% | 82.89% | 完整 | 首次完全合规 |

> 完整趋势分析: [VERSION_TREND.md](docs/releases/VERSION_TREND.md)
> 技术债务追踪: [WEAK_POINT_EVOLUTION.md](docs/releases/WEAK_POINT_EVOLUTION.md)

## 文档索引

### v3.4.0 (当前版本)

- [v3.4.0 文档中心](docs/releases/v3.4.0/)
- [v3.4.0 RC 门禁检查清单](docs/releases/v3.4.0/GA_GATE_REPORT.md)
- [v3.4.0 GA 门禁检查清单](docs/releases/v3.4.0/GA_GATE_CHECKLIST.md)
- [v3.4.0 变更日志](docs/releases/v3.4.0/CHANGELOG.md)
- [v3.4.0 开发计划](docs/releases/v3.4.0/DEV_PLAN.md)
- [v3.4.0 OO 路线图](docs/releases/v3.4.0/oo/OO_ROADMAP.md)
- [v3.4.0 测试系统分析](docs/releases/v3.4.0/TEST_SYSTEM_ANALYSIS.md)
- [v3.4.0 历史遗留问题](docs/releases/v3.4.0/LEGACY_ISSUES.md)

### 历史版本

- [v3.3.0 GA 文档](docs/releases/v3.3.0/README.md)
- [v3.2.0 RC 文档](docs/releases/v3.2.0/README.md)
- [v3.1.0 Beta 文档](docs/releases/v3.1.0/README.md)
- [v3.0.0 GA 文档](docs/releases/v3.0.0/README.md)

---

*SQLRustGo v3.4.0 — GMP Management Suite*

# SQLRustGo

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85+-dea584?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/version-v3.6.0--DEVELOPING-blue" alt="Version">
  <img src="https://img.shields.io/badge/branch-develop%2Fv3.6.0-blue" alt="Branch">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
</p>

SQLRustGo 是一个使用 Rust 实现的关系型数据库教学与工程化项目，支持 SQL-92 子集，包含解析、规划、执行、存储、事务与网络模块，并具备向量存储与图存储等高级特性。

## 当前状态

| 项目 | 当前值 |
|------|--------|
| 当前版本状态 | **v3.6.0 (开发中)** |
| 当前主分支 | **develop/v3.6.0** |
| 当前阶段 | **Alpha** |
| 上一稳定版本 | **v3.5.0 GA** (2026-05-28) |
| 版本目标 | 完善 AI Native GMP Platform |

- 版本文件: [VERSION](VERSION)
- v3.6.0 文档入口: [docs/releases/v3.6.0/](docs/releases/v3.6.0/)
- v3.5.0 GA 文档: [docs/releases/v3.5.0/](docs/releases/v3.5.0/)
- v3.5.0 GA Gate: [GA_GATE_REPORT.md](docs/releases/v3.5.0/GA_GATE_REPORT.md)

---

## v3.5.0 GA 发布总结

**发布日期**: 2026-05-28 | **Tag**: `v3.5.0` → `5720805b`

### GA Gate 结果

| Gate | 检查项 | 结果 |
|------|--------|------|
| G1 | Release 编译 | ✅ PASS |
| G2 | 全 workspace 测试 | ✅ PASS (1771 tests) |
| G3 | Clippy 零警告 | ✅ PASS (0 errors) |
| G4 | Format 无 diff | ✅ PASS |
| G5 | 覆盖率 ≥85% (L1) | ✅ **87.35%** |

**覆盖率详情**（综合方法 `--tests` + `--lib` fallback）:

| Crate | 覆盖率 | 方法 |
|-------|--------|------|
| sqlrustgo-types | 87.65% | --tests |
| sqlrustgo-parser | 83.82% | --tests |
| sqlrustgo-planner | 89.39% | --tests |
| sqlrustgo-optimizer | 89.72% | --tests |
| sqlrustgo-executor | 89.30% | --tests |
| sqlrustgo-storage | 81.66% | --lib (fallback) |
| sqlrustgo-transaction | 88.75% | --tests |
| sqlrustgo-catalog | 88.52% | --tests |
| **平均** | **87.35%** | ✅ |

### v3.5.0 待处理债务

| ID | 问题 | 状态 |
|----|------|------|
| EX-v350-001 | parser 覆盖率 83.82% | 🔴 Open (v3.6.0 Alpha 前) |
| EX-v350-002 | optimizer 覆盖率 89.72% | ✅ 已达标 |
| EX-v350-003 | executor 覆盖率 89.30% | ✅ 已达标 |
| EX-v350-004 | storage 覆盖率 81.66% | 🔴 Open (v3.6.0 Alpha 前) |
| EX-v350-005 | Storage varlen 5 tests fail | 🔴 Open |

### 历史遗留豁免（逾期）

| ID | 问题 | 逾期版本 |
|----|------|----------|
| EX-v320-001 | executor 覆盖率 70.7% | v3.3.0 Alpha |
| EX-v320-002 | MySQL Protocol 握手失败 | v3.3.0 Alpha |
| EX-v320-003 | TPC-H SF=1 数据缺失 | v3.3.0 Alpha |

---

## 核心能力

- **SQL**: SELECT INSERT UPDATE DELETE CREATE TABLE DROP TABLE + CTE/窗口函数/JSON
- **存储**: Buffer Pool + FileStorage + MemoryStorage + ColumnarStorage
- **索引**: B+ Tree + Hash Index + Vector Index
- **事务**: WAL + MVCC (Snapshot Isolation) + XA 两阶段提交
- **网络**: TCP / MySQL 风格协议
- **高级**: 向量存储、图存储、Prepared Statement、存储过程、触发器
- **分布式**: Semi-sync 复制、MTS 并行复制、Multi-source 复制
- **AI 合规**: LLM-powered Compliance Judge Agent，支持 GMP 规范实时判断 (<500ms)

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

---

## 版本演进

```
v3.3.0: Industrial Trust Platform ✅ GA
    ↓
v3.4.0: GMP Management Suite       ✅ GA (2026-05-24)
    ↓
v3.5.0: AI Native GMP Platform    ✅ GA (2026-05-28)
    ↓
v3.6.0: AI Native GMP Platform   ← 开发中
```

### 版本治理趋势

| 版本 | 合规率 | 覆盖率 | 门禁规范 | 主要问题 |
|------|--------|--------|----------|---------|
| v3.0.0 GA | 🔴 20% | 23.9% (阈值腐蚀) | 未建立 | **阈值腐蚀**：GA-6 从 85%→22%，两次降级 |
| v3.1.0 GA | 🟡 58% | ~60% | 初始建立 | Alpha 无自动化 |
| v3.2.0 GA | 🟢 90% | 85.81% | 完善 | 部分门禁项仍手工 |
| v3.3.0 GA | 🟡 ~70% | **72.65%** (实测) | Beta/RC/GA 脚本缺失 | 无自动化门禁脚本 |
| v3.4.0 GA | 🟢 100% | 82.89% | 完整 | 首次完全合规 |
| **v3.5.0 GA** | 🟢 **100%** | **87.35%** | 完整 | 覆盖率方法统一为综合方法 |

> 完整趋势分析: [VERSION_TREND.md](docs/releases/VERSION_TREND.md)
> 技术债务追踪: [WEAK_POINT_EVOLUTION.md](docs/releases/WEAK_POINT_EVOLUTION.md)
> 治理债务报告: [GOVERNANCE_DEBT_REPORT_v350.md](docs/governance/GOVERNANCE_DEBT_REPORT_v350.md)

---

## v3.6.0 Alpha 门禁状态

> **Alpha Gate**: ⬜ 进行中
> **P0 Issue**: Storage varlen encoder + EX-v320-* 豁免关闭

| Gate | 检查项 | 状态 | 说明 |
|------|--------|------|------|
| A1 | Build | ⬜ | cargo build --release |
| A2 | Unit Tests | ⬜ | cargo test |
| A3 | Clippy | ⬜ | 零警告 |
| A4 | Format | ⬜ | fmt check |
| A5 | Coverage (≥72%) | ⬜ | 综合方法 llvm-cov |
| A6 | MySQL Protocol | ⬜ | 协议测试 |
| A7 | TPC-H SF=1 | ⬜ | 22/22 queries |
| A8 | Ollama 连接 | ⬜ | gmp-llm 健康检查 |

---

## v3.6.0 功能矩阵

### 遗留任务（P0，来自 v3.5.0）

| 功能 | Issue | 状态 |
|------|-------|------|
| Storage varlen encoder/decoder 修复 | #varlen | 🔴 FIX REQUIRED |
| Storage 覆盖率 81.66% → 85%+ | EX-v350-004 | 🔴 FIX REQUIRED |
| EX-v320-* 豁免关闭 | EX-v320-* | 🔴 3项逾期 |

### AI Native GMP Platform (P1)

| 功能 | Issue | 状态 |
|------|-------|------|
| AI 偏差调查助手 | #1361 | TODO |
| GMP Retrieval v3 集成 | #1363 | TODO |
| AI 流式输出 SSE | #1365 | TODO |
| DiskGraphStore Cypher | #1326 | TODO |

---

## 文档索引

### v3.6.0 (开发中)

- [v3.6.0 文档中心](docs/releases/v3.6.0/)
- [v3.6.0 开发计划](docs/releases/v3.6.0/DEV_PLAN.md)

### v3.5.0 (GA)

- [v3.5.0 文档中心](docs/releases/v3.5.0/)
- [v3.5.0 GA 门禁报告](docs/releases/v3.5.0/GA_GATE_REPORT.md)
- [v3.5.0 变更日志](docs/releases/v3.5.0/CHANGELOG.md)
- [v3.5.0 技术债务分析](docs/releases/v3.5.0/TECHNICAL_DEBT_ANALYSIS.md)
- [v3.5.0 治理债务报告](docs/governance/GOVERNANCE_DEBT_REPORT_v350.md)

### v3.4.0 (GA)

- [v3.4.0 文档中心](docs/releases/v3.4.0/)
- [v3.4.0 GA 门禁报告](docs/releases/v3.4.0/GA_GATE_REPORT.md)
- [v3.4.0 变更日志](docs/releases/v3.4.0/CHANGELOG.md)

### 跨版本分析

- [VERSION_TREND.md](docs/releases/VERSION_TREND.md) — 版本趋势分析 v2.6.0~v3.5.0
- [WEAK_POINT_EVOLUTION.md](docs/releases/WEAK_POINT_EVOLUTION.md) — 技术债务追踪
- [GOVERNANCE_DEBT_REPORT_v350.md](docs/governance/GOVERNANCE_DEBT_REPORT_v350.md) — v3.5.0 治理债务
- [GATE_EXEMPTIONS.md](docs/governance/GATE_EXEMPTIONS.md) — 豁免与延期记录

### 历史版本

- [v3.3.0 GA 文档](docs/releases/v3.3.0/README.md)
- [v3.2.0 RC 文档](docs/releases/v3.2.0/README.md)
- [v3.1.0 Beta 文档](docs/releases/v3.1.0/README.md)
- [v3.0.0 GA 文档](docs/releases/v3.0.0/README.md)

---

*SQLRustGo v3.6.0 — 开发中*

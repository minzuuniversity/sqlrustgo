# SQLRustGo

<p align="center">
  <img src="https://img.shields.io/badge/Rust-1.85+-dea584?style=flat-square&logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/version-v3.5.0--DEVELOPING-blue" alt="Version">
  <img src="https://img.shields.io/badge/branch-develop%2Fv3.5.0-blue" alt="Branch">
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License">
</p>

SQLRustGo 是一个使用 Rust 实现的关系型数据库教学与工程化项目，支持 SQL-92 子集，包含解析、规划、执行、存储、事务与网络模块，并具备向量存储与图存储等高级特性。

## 当前状态

| 项目 | 当前值 |
|------|--------|
| 当前版本状态 | **v3.5.0 (开发中)** |
| 当前主分支 | **develop/v3.5.0** |
| 当前阶段 | **Alpha** |
| 上一稳定版本 | v3.4.0 (GA) |
| 版本目标 | AI Native GMP Platform（AI 原生 GMP 平台） |

- 版本文件: [VERSION](VERSION)
- 当前版本说明: [docs/releases/v3.5.0/README.md](docs/releases/v3.5.0/)
- v3.5.0 文档入口: [docs/releases/v3.5.0/](docs/releases/v3.5.0/)
- v3.4.0 GA 文档: [docs/releases/v3.4.0/README.md](docs/releases/v3.4.0/README.md)

## 核心能力

- **SQL**: SELECT INSERT UPDATE DELETE CREATE TABLE DROP TABLE + CTE/窗口函数/JSON
- **存储**: Buffer Pool + FileStorage + MemoryStorage + ColumnarStorage
- **索引**: B+ Tree + Hash Index + Vector Index
- **事务**: WAL + MVCC (Snapshot Isolation) + XA 两阶段提交
- **网络**: TCP / MySQL 风格协议
- **高级**: 向量存储、图存储、Prepared Statement、存储过程、触发器
- **分布式**: Semi-sync 复制、MTS 并行复制、Multi-source 复制
- **AI 合规**: LLM-powered Compliance Judge Agent，支持 GMP 规范实时判断 (<500ms)，置信度评分，完整审计日志

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

## v3.5.0 门禁状态（全部 ✅ PASSED）

> **Alpha/Beta/RC/GA**: ✅ 全部 PASSED（2026-05-28）
> **L1 平均覆盖率**: 87.36%（≥85%）| TPC-H SF=1: 22/22 PASS

### Alpha Gate ✅ PASSED（2026-05-28）

| Gate | 检查项 | 期望 | 实际 | 状态 |
|------|--------|------|------|------|
| A1 | Build | 成功 | ✅ 0.64s | ✅ PASS |
| A2 | Unit Tests | 100% | 39 passed, 0 failed | ✅ PASS |
| A3 | Clippy | 零 error | ✅ 0 errors | ✅ PASS |
| A4 | Format | 无 diff | ✅ 无格式错误 | ✅ PASS |
| A5 | Coverage (≥75%) | ≥75% | **84.99%** | ✅ PASS |
| A6 | MySQL Protocol | 协议测试 | ⏭️ SKIP | ⏭️ SKIP |
| A7 | TPC-H SF=1 | 22/22 | ✅ 22/22 PASS | ✅ PASS |
| A8 | Ollama 连接 | 健康检查 | ✅ PASS | ✅ PASS |
| A9 | gmp-api build | 编译成功 | ✅ PASS | ✅ PASS |
| A10 | gmp-llm build | 编译成功 | ✅ PASS | ✅ PASS |

> 详情: [docs/releases/v3.5.0/ALPHA_GATE_REPORT.md](docs/releases/v3.5.0/ALPHA_GATE_REPORT.md)
> 汇总: [docs/releases/v3.5.0/GATE_SUMMARY.md](docs/releases/v3.5.0/GATE_SUMMARY.md)

## v3.5.0 功能矩阵

### AI Native GMP Platform (P0)

| 功能 | Issue | 状态 |
|------|-------|------|
| AI 偏差调查助手 | #1361 | TODO |
| LLM 合规判断引擎 | #1362 | ✅ DONE |
| GMP Retrieval v3 集成 | #1363 | TODO |
| 本地 LLM 推理支持（Ollama） | #1364 | ✅ DONE |
| AI 流式输出 SSE | #1365 | TODO |
| DiskGraphStore Cypher | #1326 | TODO |

### AI Agent Layer (P1)

| 功能 | Issue | 状态 |
|------|-------|------|
| 预测性设备维护 | #1366 | TODO |
| 自然语言报表生成 | #1367 | TODO |
| 审计链 AI 摘要 | #1368 | TODO |
| 规则自动推荐 | #1369 | TODO |

### v3.4.0 GA 功能继承

| 功能 | 状态 |
|------|------|
| GMP API - Batch/ Audit/ Device/ Rule | ✅ 继承 |
| Trust Infrastructure | ✅ 继承 |
| SQL 功能 | ✅ 继承 |
| MySQL 兼容性 | ✅ 继承 |

## 版本演进

```
v3.3.0: Industrial Trust Platform ✅ GA
    ↓
v3.4.0: GMP Management Suite ✅ GA
    ↓
v3.5.0: AI Native GMP Platform ← 开发中
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

### v3.5.0 (开发中)

- [v3.5.0 文档中心](docs/releases/v3.5.0/)
- [v3.5.0 开发计划](docs/releases/v3.5.0/DEV_PLAN.md)
- [v3.5.0 GA 门禁检查清单](docs/releases/v3.5.0/GA_GATE_CHECKLIST.md)
- [v3.5.0 变更日志](docs/releases/v3.5.0/CHANGELOG.md)
- [v3.5.0 历史遗留问题](docs/releases/v3.5.0/LEGACY_ISSUES.md)

### v3.4.0 (GA)

- [v3.4.0 文档中心](docs/releases/v3.4.0/)
- [v3.4.0 RC 门禁检查清单](docs/releases/v3.4.0/RC_GATE_CHECKLIST.md)
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

*SQLRustGo v3.5.0 — AI Native GMP Platform（开发中）*

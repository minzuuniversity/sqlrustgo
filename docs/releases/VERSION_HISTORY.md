# SQLRustGo 版本演进完整历史

> **当前版本**: v3.9.0 (RC7)
> **最新稳定**: v3.8.0 (GA, 2026-06-08)
> **更新日期**: 2026-06-17

---

## 一、版本发布历史

### 已发布版本

| 版本 | 发布日期 | 状态 | 核心特性 | 成熟度 |
|------|----------|------|----------|--------|
| v3.9.0 | 2026-12-15 (目标) | RC7 | Production Readiness | L4 |
| v3.8.0 | 2026-06-08 | GA | Architecture Unification | L4 |
| v3.7.0 | 2026-05-31 | GA | GMP Integration | L4 |
| v3.6.0 | 2026-05-29 | GA | Protocol Stack | L4 |
| v3.5.0 | 2026-05-28 | GA | AI Native GMP | L4 |
| v3.4.0 | 2026-05-24 | GA | TPC-H 22/22 | L4 |
| v3.3.0 | 2026-05-20 | GA | Corpus 818/818 | L4 |
| v3.2.0 | 2026-05-17 | GA | WAL + MVCC | L4 |
| v3.1.0 | 2026-05-14 | Beta | - | L3 |
| v3.0.0 | 2026-05-10 | GA | Parallel Executor | L3 |
| v2.9.0 | 2026-05-10 | Alpha | - | L3 |
| v2.8.0 | 2026-04-30 | GA | - | L3 |
| v2.7.0 | 2026-04-25 | GA | - | L3 |
| v2.6.0 | 2026-04-22 | GA | SQL-92 Complete | L3 |
| v2.5.0 | 2026-04-03 | GA | MVCC/Vector/Graph | L3 |
| v2.4.0 | 2026-xx-xx | GA | 列式存储 | L3 |
| v2.3.0 | 2026-xx-xx | GA | - | L3 |
| v2.2.0 | 2026-xx-xx | GA | Vector Index | L3 |
| v2.1.0 | 2026-xx-xx | GA | CBO 优化 | L3 |
| v2.0.0 | 2026-xx-xx | GA | 向量化执行 | L3 |
| v1.x | 2025-xx-xx | GA | 基础 SQL | L2-L3 |

---

## 二、v3.9.0 详细变更 (RC7)

> **状态**: RC7 (2026-06-12)
> **GA 目标**: 2026-12-15

### 核心战略

- **核心问题**: 不是"支持多少 SQL"，而是"数据库死了以后还能不能回来"
- **资源分配**: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / 新 SQL 0%
- **主题**: Single-Node Production Candidate

### G1-G16 门禁

| # | 门禁 | 状态 |
|---|------|------|
| G1 | 22/22 TPC-H 保持 | ✅ PASS |
| G2 | INT-2 关闭 | ✅ PASS |
| G3 | INT-3 关闭 | ✅ PASS |
| G4 | ARCH-3 关闭 | ✅ PASS |
| G5 | SEM-1 关闭 | ✅ PASS |
| G6 | Backup/Restore 100+ 场景 | ✅ PASS |
| G7 | 24h Soak Test | ✅ PASS |
| G8 | Crash Matrix 100+ 场景 | ✅ PASS |
| G9 | Upgrade Test 50+ 场景 | ✅ PASS |
| G10 | Audit + Time Travel 40+ tests | ✅ PASS |

### 测试结果

| 测试集 | 结果 |
|--------|------|
| Lib Tests | 1670 PASS, 1 IGNORED |
| TPC-H | 22/22 PASS |
| Corpus | 818/818 PASS |
| D9 Gate | 8/8 PASS |

### 关键 PR

- PR #3131: MySQL 5.7 keyword + scalar function
- PR #3132: TPC-H Q2 5-table comma-join fix
- PR #3142: ORDER BY execution in SELECT
- PR #3426: cargo fmt --all
- PR #3427: un-ignore tpch_q9_audit + char_max_length
- PR #3428: sync gitcode/develop/v3.9.0

---

## 三、v3.8.0 详细变更 (GA)

> **状态**: GA (2026-06-08)
> **分支**: main@v3.8.0

### 核心特性

- **Architecture Unification**: 消灭双执行路径，统一 SQL → AST → Plan → Execution
- **WAL Mandatory**: 所有 write 必须经过 WAL
- **MVCC Enabled**: Snapshot Isolation 默认开启
- **Canonical Binary**: `sqlrustgo-mysql-server` 唯一执行入口

### Breaking Changes

- **Retired**: `sqlrustgo` / `sqlrustgo-sql-cli` / `sqlrustgo-bench` 等旧 binary
- **New Entry**: `sqlrustgo-mysql-server` (subcommands: `serve` / `exec` / `repl` / `bench` / `gmp` / `diag` / `backup` / `restore`)

### 测试结果

| 测试集 | 结果 |
|--------|------|
| TPC-H | 22/22 PASS |
| Corpus | 100% (818/818) |
| D9 Gate | 8/8 PASS |
| PR 合并 | 3430+ |

---

## 四、v3.7.0 详细变更 (GA)

> **状态**: GA (2026-05-31)

### 核心特性

- **GMP Integration**: AI Agent Layer + Ollama 本地推理 + GMP Retrieval v3
- **窗口函数**: 完整支持
- **CTE**: 完整支持

---

## 五、版本阶段说明

| 阶段 | 说明 |
|------|------|
| Alpha | 开发中，功能不稳定 |
| Beta | 功能冻结，开始测试 |
| RC (Release Candidate) | 候选发布 |
| GA (General Available) | 正式发布 |

---

## 六、成熟度等级

| 等级 | 说明 | 特征 |
|------|------|------|
| L1 | 原型 | 可运行，基本功能 |
| L2 | 开发版 | 核心功能可用，不稳定 |
| L3 | 产品级 | 功能完整，生产可用 |
| L4 | 企业级 | 性能优化，高可用 |
| L5 | 分布式 | 完整分布式支持 |

---

## 七、远景路线图

### v3.10+ (规划中)

| 功能 | 目标版本 |
|------|----------|
| 窗口函数完整 | v3.10+ |
| CTE 完整 | v3.10+ |
| 图数据库 Cypher 完整 | v3.10+ |
| XA 两阶段提交 | v3.10+ |

### v4.0.0 (愿景)

- 完整分布式数据库
- 对标 CockroachDB/TiDB
- 分布式事务
- 分片复制

---

*本文档由 Hermes Agent 维护*
*更新频率: 每个版本发布后更新*
*最后更新: 2026-06-17*

# SQLRustGo 版本演进完整历史

> **当前版本**: v4.0.0 (开发中, 2026-09-12)
> **最新 GA**: v3.12.0 (GA, 2026-09-08)
> **更新日期**: 2026-09-12

---

## 一、版本发布历史

### 已发布版本

| 版本 | 发布日期 | 状态 | 核心特性 | 成熟度 |
|------|----------|------|----------|--------|
| v1.0.0 | 2025-xx-xx | GA | 基础 SQL 支持 | L2 |
| v1.1.0 | 2025-xx-xx | GA | 基础性能优化 | L2 |
| v1.2.0 | 2025-xx-xx | GA | 架构接口化 | L3 |
| v1.3.0 | 2025-xx-xx | GA | 向量化基础 | L3 |
| v1.4.0 | 2025-xx-xx | GA | 可观测性 | L3 |
| v1.5.0 | 2025-xx-xx | GA | 并行执行 | L3 |
| v1.6.0 | 2025-xx-xx | GA | 性能优化 | L3 |
| v1.6.1 | 2025-xx-xx | GA | Bugfix | L3 |
| v1.7.0 | 2025-xx-xx | GA | MySQL 兼容性增强 | L3 |
| v1.8.0 | 2025-xx-xx | GA | SQL-92 增强 | L3 |
| v1.9.0 | 2025-xx-xx | GA | 完整性增强 | L3 |
| v2.0.0 | 2025-xx-xx | GA | 向量化执行 | L4 |
| v2.1.0 | 2026-xx-xx | GA | CBO 优化 | L4 |
| v2.2.0 | 2026-xx-xx | GA | Vector Index | L4 |
| v2.3.0 | 2026-xx-xx | GA | 分布式基础 | L4 |
| v2.4.0 | 2026-xx-xx | GA | 列式存储 | L4 |
| v2.5.0 | 2026-04-03 | GA | MVCC/Vector/Graph | L4 |
| v2.6.0 | 2026-xx-xx | Alpha | SQL-92 完整 + 生产就绪 | L4 |
| v2.7.0 | TBD | 规划 | 分布式架构 | - |
| v3.0.0 | TBD | 愿景 | 完整分布式数据库 | - |
| v3.5.0 | 2026-05-28 | GA | GA 门禁完成 | L3+ |
| v3.6.0 | 2026-05-29 | Alpha FAIL | 协议栈整合 | L3 |
| v3.7.0 | 2026-05-30 | Refactoring | 集成债务清算 | L3 |
| v3.8.0 | 2026-05-31 | 开发中 | 架构统一 | L3 |
| v3.9.0 | 2026-07-10 | GA | 债务清零 + 功能集成 | L3+ |
| v3.10.0 | 2026-07-13 | GA | MySQL 5.7 替代基础 | L3+ |
| v3.11.0 | 2026-08-09 | GA | 生产稳定版 | L3+ |
| v3.12.0 | 2026-09-08 | GA | GMP 内审检索系统 | L3+ |
| **v4.0.0** | **TBD** | **开发中** | **多模型生产级** | **L4** |

---

## 二、各版本详细变更日志

### v3.12.0 (2026-09-08) - GMP 内审检索系统

#### GA 门禁状态

| Gate | 状态 |
|------|------|
| BETA | 40/40 ✅ PASS |
| RC | 11/11 ✅ PASS |
| GA | 8/8 ✅ PASS |
| thresholds_override | 13/13 ✅ PASS |
| **总计** | **72/72 ✅ PASS** |

#### 功能矩阵

| 模块 | 功能 | 状态 |
|------|------|------|
| **GMP Schema** | Document/Chunk/Embedding/Audit table | ✅ |
| **混合检索** | SQL + Vector 联合查询 | ✅ |
| **图投影** | SQL-backed graph traversal | ✅ |
| **ALCOA+ 审计** | Hash-chain tamper detection | ✅ |
| **TPC-H** | SF=1 22/22 PASS | ✅ |
| **SQLLogicTest** | smoke 25/25 PASS | ✅ |
| **SOAK** | 168h PASS | ✅ |
| **MySQL Wire** | COM_QUERY/COM_STMT | ✅ |
| **LOAD DATA** | SF=1/SF=10 | ✅ |
| **Crash Recovery** | 7+4+4 scenarios | ✅ |

#### 3 个 GA-claim-caveat

| Issue | 限制范围 |
|-------|----------|
| #4846 | `CHAR(n)` 字节填充主键点查排除 |
| #4847 | 显式事务语义排除（GMP 使用单语句批处理模式） |
| #4848 | `ALTER TABLE ... RENAME COLUMN` 排除 |

---

### v3.11.0 (2026-08-09) - 生产稳定版

#### GA 门禁状态

| Gate | 状态 |
|------|------|
| G1 | R1-R4 ✅ |
| G2 | 2,060 lib tests ✅ |
| G3 | Coverage ≥75% ✅ |
| G4 | TPC-H SF=1 22/22 ✅ |
| G5 | cargo audit ✅ |
| G6 | CHANGELOG/UPGRADE_GUIDE ✅ |
| **总计** | **6/6 ✅ PASS** |

#### 功能矩阵

| 模块 | 功能 | 状态 |
|------|------|------|
| **TPC-H** | SF=1 22/22 PASS | ✅ |
| **SQLLogicTest** | smoke/curated 部分 PASS | ✅ |
| **Coverage** | storage 81.27% / planner 79.72% / executor 79.11% | ✅ |
| **MySQL Wire** | 基础协议 | ✅ |
| **LOAD DATA** | 批量导入 | ✅ |
| **Crash Recovery** | WAL 恢复 | ✅ |

---

### v3.10.0 (2026-07-13) - MySQL 5.7 替代基础

#### 功能矩阵

| 模块 | 功能 | 状态 |
|------|------|------|
| **SQL 覆盖** | SELECT/INSERT/UPDATE/DELETE/JOIN | ✅ |
| **WAL Contract** | 42/42 PASS | ✅ |
| **Integration** | 4/4 + 5/5 PASS | ✅ |
| **5-Principles** | G-01~G-06 PASS | ✅ |
| **10-Principles** | R1~R10 PASS | ✅ |
| **Coverage baseline** | 14.71% established | ✅ |

---

### v3.9.0 (2026-07-10) - 债务清零版

#### 功能矩阵

| 模块 | 功能 | 状态 |
|------|------|------|
| **集成债务** | WAL/DML/ParallelVolcano 清算 | ✅ |
| **expr crate** | 孤岛整合 | ✅ |
| **mysql-server** | 协议栈统一 | ✅ |
| **Coverage** | 84.99% (差 0.01pp 未达 85%) | ✅ CONDITIONAL |

---

### v2.5.0 (2026-04-03) - MVCC/Vector/Graph

#### 功能矩阵

| 模块 | 功能 | 状态 |
|------|------|------|
| **执行器** | ParallelExecutor | ✅ |
| **执行器** | 向量化执行基础 | ✅ |
| **存储** | MVCC 快照隔离 | ✅ |
| **图存储** | Graph Storage 基础 | ✅ |
| **优化器** | CBO 框架 | ✅ |
| **网络** | MySQL 协议增强 | ✅ |
| **可观测性** | Metrics 端点 | ✅ |

---

## 三、功能矩阵总览

### 核心能力

| 功能 | v3.x | v4.x | 状态 |
|------|------|------|------|
| **SQL 支持** | | | |
| SELECT | ✅ | ✅ | 完整 |
| INSERT | ✅ | ✅ | 完整 |
| UPDATE | ✅ | ✅ | 完整 |
| DELETE | ✅ | ✅ | 完整 |
| JOIN (INNER/LEFT/RIGHT) | ✅ | ✅ | 完整 |
| JOIN (FULL OUTER) | ⚠️ | ✅ | 部分 |
| GROUP BY | ✅ | ✅ | 完整 |
| HAVING | ✅ | ✅ | 完整 |
| UNION | ✅ | ✅ | 完整 |
| 子查询 | ✅ | ✅ | 完整 |
| 窗口函数 | ⚠️ | ✅ | 部分 |
| **存储引擎** | | | |
| Buffer Pool | ✅ | ✅ | 完整 |
| B+ Tree 索引 | ✅ | ✅ | 完整 |
| 列式存储 | ✅ | ✅ | 完整 |
| WAL | ✅ | ✅ | 完整 |
| MVCC | ✅ | ✅ | 完整 |
| **执行引擎** | | | |
| 火山模型 | ✅ | ✅ | 完整 |
| 向量化执行 | ✅ | ✅ | 完整 |
| 并行执行 | ✅ | ✅ | 完整 |
| **多模型** | | | |
| Vector column | ✅ | ✅ | 内部能力 |
| Vector index (HNSW) | ⚠️ | ✅ | 一等公民 |
| Graph node/edge | ⚠️ | ✅ | 一等公民 |
| **优化器** | | | |
| 规则优化 | ✅ | ✅ | 完整 |
| CBO | ✅ | ✅ | 部分 |
| 成本模型 | ✅ | ✅ | 部分 |
| **网络协议** | | | |
| MySQL 协议 | ✅ | ✅ | 完整 |
| PostgreSQL 协议 | ⚠️ | ⚠️ | 部分 |

### 成熟度等级

| 等级 | 说明 | 特征 |
|------|------|------|
| L1 | 原型 | 可运行，基本功能 |
| L2 | 开发版 | 核心功能可用，不稳定 |
| L3 | 产品级 | 功能完整，生产可用 |
| L4 | 企业级 | 性能优化，高可用，多模型 |

---

## 四、版本号语义

```
主版本.次版本.修订版本

v4.0.0
│ │ │
│ │ └─ 修订: Bugfix 或小功能
│ │       
│ └──── 次版本: 新功能（向后兼容）
│
└────── 主版本: 架构重大变更（不兼容）
```

### 版本阶段

| 阶段 | 说明 |
|------|------|
| Alpha | 开发中，功能不稳定 |
| Beta | 功能冻结，开始测试 |
| RC (Release Candidate) | 候选发布 |
| GA (General Available) | 正式发布 |

---

## 五、版本演进路线图

```
v3.11.0 ✅      v3.12.0 ✅      v4.0.0 🔄
  MySQL 替代       GMP 内审        多模型生产
  (2026-08-09)    (2026-09-08)   (开发中)
```

### v4.0.0 目标

| 领域 | 范围 |
|------|------|
| SQL database | MySQL-compatible relational core |
| Vector database | vector column/index syntax、Flat/HNSW/IVF、metadata filtering |
| Graph database | property graph node/edge storage 与 traversal query layer |
| GMP knowledge layer | document、audit、CAPA、deviation、SOP、evidence graph |
| Unified storage | WAL-backed SQL/vector/graph/GMP writes |
| Unified operations | backup/restore、access control、audit、observability |

---

## 六、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-03-12 | 初始版本 |
| 2.0 | 2026-03-13 | v1.x/v2.x/v3.x 重构 |
| 3.0 | 2026-03-13 | 工程优化版 |
| 4.0 | 2026-03-18 | 整合 v1.x 版本 |
| 5.0 | 2026-03-21 | SQL-92 路线图 |
| 6.0 | 2026-03-21 | 教学 DBMS 战略定位 |
| 7.0 | 2026-03-21 | v1.7 合并 v1.8+v1.9，v2.0 独立 |
| 8.0 | 2026-04-09 | v2.4.0 Graph Engine + OpenClaw |
| 9.0 | 2026-05-30 | 集成债务清算 |
| 10.0 | 2026-07-13 | MySQL 5.7 替代基础 |
| 11.0 | 2026-08-09 | 生产稳定版 |
| 12.0 | 2026-09-08 | GMP 内审检索系统 |
| **13.0** | **规划中** | **多模型能力增强** |

---

*本文档由 claude-macmini 维护*
*更新频率: 每版本发布后更新*
*最后更新: 2026-09-12*

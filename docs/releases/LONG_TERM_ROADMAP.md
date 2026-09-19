# SQLRustGo 长期版本演进路线图

> **版本**: 2.0
> **制定日期**: 2026-03-06
> **更新日期**: 2026-09-12
> **状态**: 规划中

---

## 一、版本演进总览

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SQLRustGo 版本演进路线                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   v3.12.0 ✅ (2026-09-08)                                                 │
│   └── 架构: GMP 内审检索系统                                                 │
│   └── 状态: GA (72/72 gate PASS)                                            │
│                                                                              │
│   v4.0.0 🔄 (开发中)                                                        │
│   └── 架构: 生产级多模型数据库（SQL + Vector + Graph + GMP）                   │
│   └── 目标: L4 企业级                                                       │
│                                                                              │
│   v5.0 📋 (规划中)                                                          │
│   └── 架构: 完整分布式数据库原型                                              │
│   └── 目标: 对标 CockroachDB, TiDB                                         │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 二、各版本核心目标

### 2.1 v3.12.0: GMP 内审检索系统（已完成）

| 项目 | 值 |
|------|-----|
| **版本号** | v3.12.0 |
| **代号** | GMP Internal Audit Retrieval |
| **核心目标** | SQL + Vector + Graph 内部能力，GMP 合规性内审 |
| **成熟度** | L3+ GA |
| **目录结构** | crates/ workspace |
| **发布日期** | 2026-09-08 |

> 当前版本状态：`GA`
> 当前开发分支：`develop/v3.12.0`
> HEAD: `6d23f3658` (2026-09-12)

**已完成的核心能力**:

- ✅ GMP Schema (Document/Chunk/Embedding/Audit table)
- ✅ 混合检索 (SQL + Vector 联合查询)
- ✅ 图投影 (SQL-backed graph traversal)
- ✅ ALCOA+ 审计 (Hash-chain tamper detection)
- ✅ TPC-H SF=1 22/22 PASS
- ✅ SQLLogicTest smoke 25/25 PASS
- ✅ 168h SOAK PASS
- ✅ MySQL Wire 协议
- ✅ LOAD DATA 批量导入
- ✅ Crash Recovery

### 2.2 v4.0.0: 多模型生产级数据库（开发中）

| 项目 | 值 |
|------|-----|
| **版本号** | v4.0.0 |
| **核心目标** | 一等公民多模型能力（SQL + Vector + Graph + GMP） |
| **成熟度** | L4 开发中 |
| **预计时间** | v3.12.0 GA 后 |
| **开发分支** | `develop/v4.0.0` |

**核心功能**:

| ID | 工作包 | 优先级 | 退出证据 |
|---|---|---|---|
| V400-01 | Vector column 与 vector index SQL syntax | P0 | parser/executor E2E PASS |
| V400-02 | WAL-backed vector storage 与 rebuild | P0 | crash/rebuild tests |
| V400-03 | Graph crate revival 或 rewrite | P0 | storage and traversal tests |
| V400-04 | Graph query surface | P0 | bounded path query tests |
| V400-05 | Cross-model transaction semantics | P0 | rollback and crash tests |
| V400-06 | Unified backup/restore | P0 | restore SQL/vector/graph/GMP equality |
| V400-07 | Unified ACL and audit | P0 | permission bypass tests fail |
| V400-08 | Multi-model optimizer 与 metadata filters | P1 | query plan evidence |
| V400-09 | Multi-model SOAK | P0 | 168h mixed report |

**发布里程碑**:

| 里程碑 | 目标 | 必需证据 |
|---|---|---|
| Alpha | vector SQL + storage prototype | vector E2E tests |
| Beta | graph store + traversal prototype | graph E2E tests |
| RC | unified transaction/backup/security | cross-model gates |
| GA | multi-model production | 168h SOAK + full GA report |

### 2.3 v5.0: 完整分布式数据库（规划中）

| 项目 | 值 |
|------|-----|
| **版本号** | v5.0 |
| **核心目标** | 完整分布式数据库 |
| **架构** | 对标 CockroachDB / TiDB |
| **目标** | 生产级分布式事务 |

**核心功能**:

- Raft 共识
- 分布式执行
- 分布式优化
- 分区表
- 地理分布复制

---

## 三、版本依赖关系

```
v3.12.0 GA
    │
    ├── GMP Schema ────────────────────────────────────┐
    ├── Hybrid Retrieval ───────────────────────────────┤
    ├── Graph Projection ───────────────────────────────┤
    └── ALCOA+ Audit ─────────────────────────────────┤
                                                             ├──→ v4.0.0
                                                                 │
    ┌────────────────────────────────────────────────────┴───────────┐
    │                                                              │
    ▼                                                              ▼
┌────────────┐     ┌────────────┐     ┌────────────┐     ┌────────────┐
│ Vector DB  │     │ Graph DB   │     │ WAL Unified│     │ ACL Unified│
│ V400-01   │     │ V400-03   │     │ V400-02   │     │ V400-07   │
└────────────┘     └────────────┘     └────────────┘     └────────────┘
         │                   │                   │                   │
         └───────────────────┴───────────────────┴───────────────────┘
                             │
                             ▼
                      v4.0.0 GA
                             │
                             │ v5.0 依赖
                             ▼
    ┌────────────┐     ┌────────────┐     ┌────────────┐
    │ Raft 共识  │     │ 分布式执行 │     │ 分区表    │
    └────────────┘     └────────────┘     └────────────┘
                             │
                             ▼
                      v5.0 (完整分布式)
```

---

## 四、阶段门禁规则

详见 `docs/governance/STAGE_CONFIG.yaml`

| 阶段 | 门禁要求 | 允许提交类型 |
|------|----------|--------------|
| **Draft** | 编译通过 | 架构、目录、接口设计 |
| **Alpha** | 测试通过率 ≥ 80% | 新功能、新模块 |
| **Beta** | 测试通过率 ≥ 95%、Clippy 零警告 | Bug 修复、性能优化 |
| **RC** | 测试 100% 通过、CI 全绿 | 仅 Critical Bug 修复 |
| **GA** | 所有门禁通过 | 禁止修改 |

---

## 五、分支管理规范

### 5.1 分支命名规范

| 类型 | 命名格式 | 示例 |
|------|----------|------|
| 开发分支 | `develop/vX.Y.Z` | `develop/v4.0.0` |
| 功能分支 | `feature/vX.Y.Z-<功能名>` | `feature/v4.0.0-vector-index` |
| 修复分支 | `fix/vX.Y.Z-<问题描述>` | `fix/v4.0.0-wal-crash` |
| 文档分支 | `docs/vX.Y.Z-<文档类型>` | `docs/v4.0.0-release-notes` |
| 维护分支 | `release/vX.Y.Z` | `release/v4.0.0` |

### 5.2 分支保护规则

| 分支 | 保护规则 |
|------|----------|
| `main` | 禁止直接 push，必须通过 PR + 2人审核 |
| `develop/vX.Y.Z` | 禁止直接 push，必须通过 PR + 1人审核 |
| `feature/*`, `fix/*` | 允许直接 push，但需要 CI 通过 |

---

## 六、文档索引

### 6.1 v4.0.0 文档

| 文档 | 说明 |
|------|------|
| [VERSION_PLAN.md](./v4.0.0/VERSION_PLAN.md) | 版本计划 |
| [TEST_PLAN.md](./v4.0.0/TEST_PLAN.md) | 测试计划 |
| [DEV_PLAN.md](./v4.0.0/DEV_PLAN.md) | 开发计划 |
| [GMP_PLATFORM_REQUIREMENTS.md](./v4.0.0/GMP_PLATFORM_REQUIREMENTS.md) | GMP 平台需求 |
| [CHANGELOG.md](./v4.0.0/CHANGELOG.md) | 变更日志 |

### 6.2 v3.12.0 文档

| 文档 | 说明 |
|------|------|
| [STAGE.yaml](./v3.12.0/STAGE.yaml) | 阶段状态 |
| [GA_GATE_REPORT.md](./v3.12.0/GA_GATE_REPORT.md) | GA 门禁报告 |
| [COMPREHENSIVE_ASSESSMENT_REPORT.md](./v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md) | 综合评估报告 |
| [COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md](./v3.12.0/COMPREHENSIVE_TEST_FRAMEWORK_AND_COVERAGE_BASELINE.md) | 测试框架 |

---

## 七、版本发布历史

| 版本 | 日期 | 状态 | 核心变化 |
|------|------|------|----------|
| v3.11.0 | 2026-08-09 | ✅ GA | 生产稳定版 |
| v3.12.0 | 2026-09-08 | ✅ GA | GMP 内审检索系统（72/72 gate PASS） |
| v4.0.0 | TBD | 🔄 开发中 | 多模型生产级（SQL + Vector + Graph + GMP） |
| v5.0 | TBD | 📋 规划 | 完整分布式数据库 |

---

## 八、成熟度演进

```
L1 (Toy)   →   L2 (Query Engine)   →   L3 (Mini DBMS)   →   L4 (Multi-Model DB)
                   v1.7                    v3.x                v4.0
               MySQL 教学替代          GMP 内审检索        多模型生产级
```

| 等级 | 说明 | 特征 |
|------|------|------|
| L1 | 原型 | 可运行，基本功能 |
| L2 | 开发版 | 核心功能可用，不稳定 |
| L3 | 产品级 | 功能完整，生产可用 |
| L4 | 企业级 | 性能优化，高可用，多模型 |

---

## 九、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-03-06 | 创建统一版本演进路线图 |
| 2.0 | 2026-09-12 | 更新到 v4.0.0 开发状态，v3.12.0 GA 完成 |

---

*本文档由 claude-macmini 维护*
*统一管理 SQLRustGo 所有版本规划*
*最后更新: 2026-09-12*

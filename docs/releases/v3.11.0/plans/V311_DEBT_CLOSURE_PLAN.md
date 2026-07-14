# v3.11.0 债务闭环计划 — 23 项 OPEN 项 100% 清零

> **创建日期**: 2026-07-13
> **创建人**: openclaw
> **SSOT**: `docs/governance/debt/debt-registry.yaml`
> **目标**: v3.11.0 GA 时 `debt-registry.yaml` 状态为 **0 OPEN**
> **关联**: [`V311_DEVELOPMENT_PLAN.md`](V311_DEVELOPMENT_PLAN.md) (任务 ↔ 债务 ID 映射)

---

## 0. v3.10.0 → v3.11.0 债务基线

### 0.1 v3.10.0 状态 (2026-07-12 snapshot)

| 状态 | 数量 | 占比 |
| --- | --- | --- |
| CLOSED | 17 | 38% |
| VERIFIED (待集成) | 8 | 18% |
| IN_PROGRESS | 2 | 4% |
| PARTIAL | 2 | 4% |
| DEFERRED | 4 | 9% |
| SCOPE_DEFERRED | 8 | 18% |
| SCOPE_INTERNAL | 1 | 2% |
| SUPERSEDED | 2 | 4% |
| FROZEN | 1 | 2% |
| **合计** | **45** | **100%** |

### 0.2 v3.11.0 目标状态

| 状态 | 数量 | 备注 |
| --- | --- | --- |
| CLOSED | 45 | **100%** |
| **合计** | **45** | **100%** |

**核心 KPI**: 23 项 OPEN/IN_PROGRESS/VERIFIED/DEFERRED/PARTIAL/SCOPE_DEFERRED/FROZEN → 全部 CLOSED

---

## 1. 债务清零详细计划 (按债务 ID)

### 1.1 SEM (Semantic Debt) — 2 项 → CLOSED

| ID | 标题 | v3.10.0 状态 | v3.11.0 任务 | 闭环证据 |
| --- | --- | --- | --- | --- |
| **SEM-3** | ALTER TABLE 不完整 (C-4b RENAME / C-4c RENAME COLUMN / C-4d MODIFY) | IN_PROGRESS | **V311-13** (20h) | C-4b/c/d 全部 PASS |
| **SEM-4** | Coverage 测量差异 (Z6G4 82% vs Z440 32%) | IN_PROGRESS | **V311-14** (60h) | 3 crate ≥85% |

**任务合计**: 80h

### 1.2 F-XX ISOLATED → MAIN_PATH — 8 项 → CLOSED

| ID | 标题 | v3.10.0 状态 | v3.11.0 任务 | 集成证据 |
| --- | --- | --- | --- | --- |
| **F-23** | Clustered Index | VERIFIED | **V311-01** (80h) | `ClusteredIndex::insert/delete` 集成到 `ExecutionEngine` |
| **F-24** | Adaptive Hash Index | VERIFIED | **V311-02** (60h) | `AdaptiveHashIndex::lookup` 集成到 `BTreeIndex::scan` |
| **F-25** | Change Buffer | VERIFIED | **V311-03** (40h) | `ChangeBuffer::apply` 集成到 `BTreeIndex::write` |
| **F-26** | Double-Write Buffer | VERIFIED | **V311-04** (50h) | `DoubleWriteBuffer::write_page` 集成到 `FileStorage` |
| **F-27** | Table Compression | VERIFIED | **V311-12** (50h) | LZ4/zstd 集成到 `FileStorage::write_page` |
| **F-29** | Row-Level Security | VERIFIED | **V311-05** (40h) | `Policy::filter` 集成到 VolcanoExecutor DML 算子 |
| **F-31** | Performance Schema | VERIFIED | **V311-06** (30h) | `InstrumentationHook` 集成到 8 个核心算子 |
| **F-35** | Password Rotation | VERIFIED | **V311-08** (20h) | `AuthManager::check_expiration` 集成到连接握手 |

**任务合计**: 370h

### 1.3 F-XX NOT_IMPLEMENTED → IMPLEMENTED — 3 项 → CLOSED

| ID | 标题 | v3.10.0 状态 | v3.11.0 任务 | 实现证据 |
| --- | --- | --- | --- | --- |
| **F-03** | GIS 空间数据类型 | DEFERRED | **V311-11** (80h) | `Point` type + `ST_WITHIN` 函数 + R-Tree index |
| **F-30** | CREATE SEQUENCE | DEFERRED | **V311-10** (20h) | `Sequence` catalog + `nextval`/`currval` 函数 |
| **F-36** | 列级权限 | DEFERRED | **V311-09** (40h) | `GRANT SELECT(col)` 语法 + `AuthManager` 检查 |

**任务合计**: 140h

### 1.4 F-XX PARTIAL → CLOSED — 1 项

| ID | 标题 | v3.10.0 状态 | v3.11.0 任务 | 闭环证据 |
| --- | --- | --- | --- | --- |
| **F-32** | MySQL Admin | PARTIAL | **V311-07 + V311-19** (30h) | sqlrustgo-admin ↔ mysql-server 通过 wire protocol 集成 |

### 1.5 Extension Crate 决策 — 11 项 → 解决

| Crate | v3.10.0 状态 | v3.11.0 决策 | 实施 | 工作量 |
| --- | --- | --- | --- | --- |
| `agentsql` | SCOPE_DEFERRED | **DELETED** | V311-19 | 8h |
| `gmp` | SCOPE_DEFERRED | **ARCHIVED** (`archive/v3.11/`) | V311-19 | 4h |
| `rag` | SCOPE_DEFERRED | **DELETED** (依赖 gmp) | V311-19 | 4h |
| `distributed` | FROZEN | **DELETED** | V311-19 | 16h |
| `graph` | SCOPE_DEFERRED | **ARCHIVED** (Cypher v3.12+) | V311-19 | 8h |
| `qmd-bridge` | SCOPE_DEFERRED | **DELETED** | V311-19 | 2h |
| `evidence-graph` | SCOPE_DEFERRED | **DELETED** | V311-19 | 2h |
| `admin` | PARTIAL | **MAIN_PATH** (集成 mysql-server) | V311-07 | 30h |
| `unified-query` | SCOPE_DEFERRED | **DELETED** | V311-19 | 4h |
| `unified-storage` | SCOPE_DEFERRED | **DELETED** | V311-19 | 4h |
| `vector` | SCOPE_INTERNAL | **RETAINED** (storage 内部 HNSW/IVF/PQ) | — | 0h |

**实施合计**: 84h

**debt-registry.yaml 变更**:
- 5 SCOPE_DEFERRED → DELETED (agentsql/rag/qmd-bridge/evidence-graph/unified-query/unified-storage)
- 3 SCOPE_DEFERRED → ARCHIVED (gmp/graph + ?)
- 1 FROZEN → DELETED (distributed)
- 1 PARTIAL → MAIN_PATH (admin)
- 1 SCOPE_INTERNAL → CLOSED (vector，标记为已完成能力)

### 1.6 GA-P0 Long-Running Tasks — 2 项

| ID | 标题 | v3.10.0 状态 | v3.11.0 状态 | 任务 |
| --- | --- | --- | --- | --- |
| **#3423** | TPC-H SF=1.0 baseline | SUPERSEDED | **CLOSED** | V311-20 (80h) |
| **#3648** | TPC-H 混合负载 SOAK | SUPERSEDED | **CLOSED** | V311-21 (Hermes 协作) |

### 1.7 其他 — 1 项

| ID | 标题 | v3.10.0 状态 | v3.11.0 状态 | 任务 |
| --- | --- | --- | --- | --- |
| **#3136** | check_cross_version_debt.sh 升级 | DEFERRED | **CLOSED** | V311-19 (8h, 决策实施时同步升级 gate script) |

---

## 2. v3.11.0 新增债务 (Q4 优化相关)

基于 Issue #3792 v3.10.0 实测发现的新债务：

| ID | 标题 | 来源 | v3.11.0 状态 | 任务 |
| --- | --- | --- | --- | --- |
| **PERF-1** | Q4 相关子查询占 96% 总时间 | Issue #3792 实测 | **CLOSED** | V311-15 (80h) Hash Semi Join |
| **PERF-2** | EXISTS / NOT EXISTS 无优化路径 | Issue #3792 | **CLOSED** | V311-17 (40h) Hash Anti Join |
| **PERF-3** | CTE 不物化导致重复计算 | Issue #3792 推测 | **CLOSED** | V311-18 (30h) CTE 物化 |
| **PERF-4** | 相关子查询无法去相关 | Issue #3792 | **CLOSED** | V311-16 (60h) Decorrelation |

**新增 4 项, 全部 v3.11.0 闭环**

---

## 3. v3.11.0 debt-registry.yaml 变更摘要

### 3.1 v3.11.0 GA 时债务清单（45 项）

| 状态 | 数量 | 比例 | 备注 |
| --- | --- | --- | --- |
| CLOSED | 45 | **100%** | v3.11.0 GA 目标 |
| **合计** | **45** | **100%** | — |

### 3.2 关键变化 (vs v3.10.0)

| 类别 | v3.10.0 OPEN | v3.11.0 闭环 |
| --- | --- | --- |
| SEM IN_PROGRESS | 2 (SEM-3, SEM-4) | 2 → CLOSED |
| F-XX VERIFIED (待集成) | 8 (F-23/24/25/26/27/29/31/35) | 8 → CLOSED |
| F-XX DEFERRED | 3 (F-03/30/36) | 3 → CLOSED |
| F-XX PARTIAL | 1 (F-32) | 1 → CLOSED |
| Extension SCOPE_DEFERRED | 8 | 8 → CLOSED (5 DELETED + 3 ARCHIVED) |
| Extension FROZEN | 1 (distributed) | 1 → CLOSED (DELETED) |
| Extension SCOPE_INTERNAL | 1 (vector) | 1 → CLOSED |
| GA-P0 SUPERSEDED | 2 | 2 → CLOSED |
| DEFERRED (#3136) | 1 | 1 → CLOSED |
| 新增 PERF (Q4 优化) | 4 (PERF-1/2/3/4) | 4 → CLOSED |
| **小计变化** | **31 项 → CLOSED** | — |

---

## 4. Gate Scripts 更新

v3.11.0 必须新增/更新以下 gate script：

| Script | 用途 | v3.11.0 改动 |
| --- | --- | --- |
| `check_cross_version_debt.sh` | 验证 debt-registry 0 OPEN | 升级：解析 YAML 检查 0 OPEN |
| `check_fxx_main_path.sh` (新) | 验证 10 项 F-XX 已集成主路径 | 新增 |
| `check_extension_crates.sh` (新) | 验证 8 项删除/归档完成 | 新增 |
| `check_perf_q4.sh` (新) | 验证 Q4 < 5 分钟 @ SF=3 | 新增 |
| `check_coverage.sh` | 覆盖率 ≥85% | 更新阈值 80 → 85 |
| `check_tpch_sf1.sh` | TPC-H SF=1 22/22 | 更新 |

**实施**: V311-19 (8h) 含 gate script 升级

---

## 5. 风险评估

| 风险 | 概率 | 影响 | 缓解 |
| --- | --- | --- | --- |
| **F-23 Clustered Index 集成超预算** | 中 | 高 | 80h 中 16h 为 spike 验证；如不可行，延后 v3.12 |
| **PERF-1 Hash Semi Join 性能不达预期** | 中 | 高 | 60h spike + 60h fallback（decorrelation + filter push-down） |
| **Extension 删除破坏现有依赖** | 低 | 中 | 删除前 grep 所有引用；archived 不删除，仅 exclude from workspace |
| **TPC-H SF=1 fixture 准备时间超预算** | 中 | 中 | 使用 dbgen 自动化 |
| **新 gate script 与现有冲突** | 低 | 低 | ALPHA 阶段先 dry-run 验证 |

---

## 6. v3.11.0 GA 时 debt-registry.yaml 应有的最终状态

```yaml
version: "3.11.0-ga-snapshot"
baseline: develop/v3.11.0
last_updated: "2026-10-01T00:00:00Z"

# 全部 45 项均为 CLOSED
# 验证方法:
#   $ grep "state: OPEN\|state: IN_PROGRESS\|state: VERIFIED\|state: DEFERRED\|state: PARTIAL\|state: SCOPE_DEFERRED\|state: FROZEN\|state: SUPERSEDED" debt-registry.yaml | wc -l
#   0

items:
  - id: SEM-3
    state: CLOSED
    closed_in: v3.11.0
    closed_by: V311-13
    evidence: "ALTER TABLE RENAME/MODIFY 完整实现"
  # ... 全部 45 项 CLOSED
```

---

## 7. 验收清单

### 7.1 必达 (GO/NO-GO)

- [ ] debt-registry.yaml 0 OPEN
- [ ] F-XX 10/10 集成（VERIFIED → CLOSED）
- [ ] F-XX NOT_IMPL 5/5 闭环（DEFERRED → CLOSED）
- [ ] Extension Crate 0 SCOPE_DEFERRED
- [ ] Q4 TPC-H @ SF=3 < 5 分钟
- [ ] 覆盖率 3 crate 均 ≥85%
- [ ] TPC-H SF=1 22/22 PASS

### 7.2 加分 (Optional)

- [ ] PERF-1/2/3/4 全闭环（默认必达）
- [ ] F-27 LZ4 压缩生效
- [ ] Wired-SOAK sysbench prepare/run 完整闭环
- [ ] TPC-H SF=10 实测完成

---

## 8. 关联资源

- `docs/governance/debt/debt-registry.yaml` — SSOT
- `docs/governance/DEBT_TRACKING.md` — 跨版本债务追踪机制
- `docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md` — 上版本报告
- `docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md` — F-XX 集成设计
- `docs/releases/v3.10.0/PARALLEL_EXECUTOR_OPTIMIZATION.md` — Q4 性能瓶颈背景

---

*Created: 2026-07-13 (DRAFT stage init)*
*Author: openclaw*

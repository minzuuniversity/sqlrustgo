# ISOLATED_MODULES.md — SQLRustGo v3.11.0 (ALPHA)

> 反映 `develop/v3.11.0` @ `9e8749de2` (2026-07-15, post-V311-19) 实时状态。
>
> **SSOT**: `docs/governance/debt/debt-registry.yaml` (v3.10.0-snapshot-2026-07-15 +
> v3.11.0 changelog section) +
> `docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md` +
> `docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md`。
>
> **生成日期**: 2026-07-15 (V311-22 docs restructure 启动)
>
> **范围**: 7 F-XX ISOLATED 项 + 2 F-XX NOT_IMPLEMENTED + 2 extension crates + 其余非主路径组件。
>
> **v3.11.0 ALPHA 状态**: 6/22 任务 (P0) merged, 3/22 任务 (P0/P1) IN_PROGRESS, 13/22 任务 BETA-RC 阶段
> **v3.11.0 债务闭环目标 (G11)**: 23 OPEN 项 → 0 OPEN (per V311_DEBT_CLOSURE_PLAN.md)

---

## 0. 执行摘要 (v3.10.0 → v3.11.0 演进)

| 类别 | v3.10.0 状态 | v3.11.0 状态 | 关键 PR |
|------|-------------|-------------|---------|
| **F-XX ISOLATED** (功能孤岛) | 8 (F-23/24/25/26/27/29/31/35) + 1 PARTIAL (F-32) | **1 CLOSED** (F-23 ClusteredTable) + **1 IN_PROGRESS 85%** (F-24 AHI) + **1 PARTIAL** (F-32) | #3461, #3465 |
| **F-XX NOT_IMPLEMENTED** | 3 (F-03 GIS, F-30 SEQUENCE, F-36 列权限) | **1 IN_PROGRESS 70%** (F-36 列权限) | #3457 |
| **SEM** | SEM-1 CLOSED, SEM-3 IN_PROGRESS, SEM-4 IN_PROGRESS | **SEM-3 CLOSED** (ALTER TABLE RENAME/MODIFY) + SEM-4 IN_PROGRESS (coverage) | #3442, #3449 |
| **Extension crates** | 11 (1 PARTIAL + 1 FROZEN + 1 SCOPE_INTERNAL + 8 SCOPE_DEFERRED) | **2** (admin CLOSED + vector SCOPE_INTERNAL). 9 removed/archived via PR #3468 | #3468 |
| **Q4 perf** | Q4 14.5 min (96% of TPC-H time) | **Q4 <1 sec** (SubqueryIndex optimization) | #3455 |
| **F-23 Clustered Index** | 7/7 PASS, ISOLATED | **100%** (ClusteredTable production storage + parser ENGINE=InnoDB CLUSTERED) | #3461 |
| **F-17 Hash Anti Join** | 0 | **NEW**: Hash Anti Join for NOT EXISTS / NOT IN | #3464 |
| **F-19 Extension Crate Decision** | 11 SCOPE_DEFERRED | **2** (admin + vector). 9 archived/deleted | #3468 |
| **F-23 INSERT IGNORE (PERF-5)** | High-concurrency INSERT lost connection | **FIXED** (INSERT IGNORE syntax + executor) | #3447 |
| **F-24 Adaptive Hash Index** | 7/7 PASS, ISOLATED mock | **IN_PROGRESS 85%** (production storage + ExecutionEngine + hot-path wire) | #3465 |

**关键判断**:
- **F-23 ClusteredTable 落地**: 真正从 "测试 0 主路径" 变成 "ExecutionEngine 可调用 ClusteredTable via ENGINE=InnoDB CLUSTERED"。V311-01 v2 (main path dispatch) 留给 v3.12+。
- **F-24 AHI 在 hot path fire 了**: 通过 `ExecutionEngine::scan_with_ahi` helper, 单表 SELECT 和 JOIN base-table scan 都 wire 了, 17 次 SELECT 后 AHI 真的 promote。V311-02 v3 (BTreeIndex::scan + FileStorage) 留给后续。
- **F-36 列权限**: catalog API 完整, DDL executor 集成, 1 个 parser bug 修了 (parse_grant 默认 Column), 还差 4 个主路径集成缺口 (host="%" 硬编码, SELECT 路径不调 get_authorized_columns, CREATE USER SQL 语法不在 parser, wire protocol 不传 current_user)。
- **Extension crates 大清理**: 9 个 crate (5 删 + 3 归档 + 1 删 tool + 1 删 tool) 全部清完, workspace members 45 → 36, build time 100s → 36s (-64%)。
- **Q4 性能超目标 2000 倍**: 14.5 min → 132ms @ SF 0.1 (SubqueryIndex 优化, V311-15)。

---

## 1. F-XX Functional Islands (7 项孤岛, v3.11.0+ 集成)

按 `ARCHITECTURE_DEBT_ANALYSIS.md §5.1` 规划,9 项 F-XX ISOLATED 中:
- **1 CLOSED** (F-23 ClusteredTable, V311-01) 
- **1 IN_PROGRESS 85%** (F-24 Adaptive Hash Index, V311-02, hot path wired, v3 closure deferred)
- **7 仍 ISOLATED** (F-25/26/27/29/31/32/35)

每个孤岛的具体状态:

| ID | 标题 | 测试文件 | 通过率 | v3.11.0 状态 | 后续计划 |
|----|------|----------|--------|--------------|----------|
| **F-23** | Clustered Index | `tests/integration/sql/clustered_index_test.rs` | 15/15 | **CLOSED** ✅ V311-01 | v2 (main path dispatch) 留给 v3.12+ |
| **F-24** | Adaptive Hash Index | `tests/integration/sql/adaptive_hash_index_test.rs` + 10 storage + 4 engine e2e | 21/21 | **IN_PROGRESS 85%** (production + hot path) | v3 (BTreeIndex::scan + FileStorage) 留给后续 |
| **F-25** | Change Buffer | `tests/integration/sql/change_buffer_test.rs` | 5/5 | VERIFIED, ChangeOp enum 内联 | V311-03 (BETA) |
| **F-26** | Double-Write Buffer | `tests/integration/sql/double_write_buffer_test.rs` | 6/6 | VERIFIED, in-memory mock | V311-04 (BETA) |
| **F-27** | Table Compression | `tests/integration/sql/table_compression_test.rs` | 8/8 | VERIFIED, 仅 RLE | V311-12 (LZ4/zstd) |
| **F-29** | Row-Level Security | `tests/integration/sql/row_level_security_test.rs` | 6/6 | VERIFIED, in-memory catalog | V311-05 (BETA) |
| **F-31** | Performance Schema | `tests/integration/sql/performance_schema_test.rs` | 7/7 | VERIFIED, 无 instrumentation hooks | V311-06 (BETA) |
| **F-32** | MySQL Admin | `tests/integration/sql/password_rotation_test.rs` (admin binary) | 11/11 | **PARTIAL** (binary shipped v3.10.0 PR #3795; tests still mock) | V311-07 (mysql-server ↔ admin wire 集成) |
| **F-35** | Password Rotation | `tests/integration/sql/password_rotation_test.rs` | 8/8 | VERIFIED, in-memory only | V311-08 (BETA) |

**判断**: F-23 落地 + F-24 hot path wire 标志着 v3.11.0 在 "孤岛消灭计划" 走出了实质两步。其余 7 项 (F-25/26/27/29/31/32/35) 留待 BETA 阶段 (V311-03 ~ V311-08) 处理, 估计 ~30h 累计工作量。

---

## 2. F-XX NOT_IMPLEMENTED (2 项, v3.11.0+ 实现)

| ID | 标题 | 推迟原因 | v3.11.0 状态 | 后续计划 |
|----|------|----------|--------------|----------|
| **F-03** | GIS 空间数据类型 | 需引入 Point/LineString/Polygon/WKT/WKB 完整类型系统 | DEFERRED (无进展) | V311-11 (BETA, 80h, 仅实现 POINT + WITHIN 最小子集) |
| **F-30** | CREATE SEQUENCE | 与 MySQL 8.0 sequence 对象模型同步, 需评估兼容性 | DEFERRED (无进展) | V311-10 (BETA, 20h) |
| **F-36** | 列级权限 | catalog API + parser + DDL executor 完整; SELECT 路径不调 get_authorized_columns (缺 current_user 上下文) | **IN_PROGRESS 70%** | V311-09 follow-up (4 主路径缺口 + current_user 重构) |

**注意**: F-36 与 v3.10.0 不同——v3.10.0 描述"0 代码"已过期, 实际 catalog API + parser + DDL executor 完整, 修了 1 个 parser bug (`parse_grant` 默认 `ObjectType::Table` → `Column` 当 columns list 非空)。但 SELECT 路径不调 `get_authorized_columns` 是 v3.11.0 仍未解决的主路径集成缺口 (需 ExecutionEngine 接受 current_user, 是 refactor 级改动)。

---

## 3. Extension Crates (2 项, v3.11.0 大清理)

v3.11.0 启动后 hermes/claude-macmini 主导 PR #3468 完成了 v3.10.0 累积的产品决策:

| Crate | v3.10.0 状态 | v3.11.0 状态 | v3.11.0 处理 |
|-------|--------------|--------------|------------|
| `agentsql` | SCOPE_DEFERRED | **DELETED** | AI agent SQL 生成器, 0 caller, archive/v3.11/deleted-crates/ |
| `gmp` | SCOPE_DEFERRED | SCOPE_DEFERRED (KEEP) | **审计发现 server crate 用 47 处**, 不能 archive |
| `rag` | SCOPE_DEFERRED | SCOPE_DEFERRED (KEEP) | **审计发现 server crate 用 4 处**, 不能 archive |
| `distributed` | FROZEN (since v3.5) | **DELETED** | FROZEN 6 年, 0 integration, archive/v3.11/deleted-crates/ |
| `graph` | SCOPE_DEFERRED | **ARCHIVED** | 0 caller (除 1 个 e2e test), archive/v3.11/archived-crates/ |
| `qmd-bridge` | SCOPE_DEFERRED | **DELETED** | 0 caller, archive/v3.11/deleted-crates/ |
| `evidence-graph` | SCOPE_DEFERRED | **DELETED** | 0 caller, archive/v3.11/deleted-crates/ (与 tools/graph-cli + tools/sqlrustgo-gate 一起删) |
| `admin` | PARTIAL | **CLOSED** (F-32 PARTIAL → CLOSED) | V310-14 PR #3795 mysqladmin CLI binary, v3.11.0 集成完成 |
| `unified-query` | SCOPE_DEFERRED | **DELETED** | 0 caller, archive/v3.11/deleted-crates/ |
| `unified-storage` | SCOPE_DEFERRED | **DELETED** | 0 caller, archive/v3.11/deleted-crates/ |
| `vector` | SCOPE_INTERNAL | SCOPE_INTERNAL (KEEP) | HNSW/IVF/PQ internal storage |

**v3.11.0 推荐动作** (已执行, PR #3468):
1. **集成** admin (F-32 闭环, 已合并)
2. **归档** graph (Cypher 暂留 archive, v3.12+ 再决定)
3. **保留** gmp + rag (server crate 真用, 不能 archive) + vector (storage 内部)
4. **删除** 其余 6 个 (0 caller)

**结果**: workspace members 45 → 36, build time 100s → 36s (-64%), 2 个 SCOPE_DEFERRED 仍待产品决策 (gmp + rag)。

---

## 4. 其他非主路径组件 (历史遗留)

| 组件 | 位置 | 状态 | v3.11.0 处理 |
|------|------|------|--------------|
| `crates/executor/src/local_executor.rs` | (3031 行, 3 KB) | DEAD CODE | v3.10.0 删除 (2026-07-12); 文件从未进入 `lib.rs` mod 树 |
| `crates/executor/src/local_executor_dml.rs` | (275 行) | DEPRECATED since v3.9.0 | 保留以兼容历史测试; v3.11+ 评估删除 (no e2e 调用) |
| `crates/server` (sqlrustgo-server) | (整个 crate) | **DEPRECATED** | 仍被 ~13 个集成测试引用; v3.11.0 不动 (低优先级) |
| `expression` (vs `expr`) | (expressional 重叠) | DEFERRED | v3.7.0 已知; `expr` 为主; `expression` 保留作为 fallback |
| `crates/optimizer/src/cbo_estimator.rs` | (新增) | 实验性 | v3.10.0 引入, v3.11.0 持续优化 |

---

## 5. v3.11.0 ALPHA 当前焦点 (基于 V311-22 验收)

按 `docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md` + `V311_DEBT_CLOSURE_PLAN.md` 跟踪:

### 5.1 已完成 (P0, 6 项)

1. ✅ **V311-01 F-23 ClusteredTable** (PR #3461 by claude-macmini) — production storage 落地
2. ✅ **V311-13 SEM-3 ALTER TABLE** (PR #3442 + #3449) — RENAME/MODIFY COLUMN 完整 + parser bug 修复
3. ✅ **V311-15 Q4 Hash Semi Join** (PR #3455 by claude-macmini) — SubqueryIndex 优化, Q4 14.5min → 132ms
4. ✅ **V311-17 Hash Anti Join** (PR #3464 by claude-macmini) — NOT EXISTS / NOT IN
5. ✅ **V311-19 Extension Crate Decision** (PR #3468 by claude-macmini) — 11 → 2
6. ✅ **V311-23 PERF-5 High-concurrency INSERT** (PR #3447) — INSERT IGNORE 修复

### 5.2 进行中 (P0, 3 项, 1-2 周完成)

7. 🟡 **V311-02 F-24 AHI** (PR #3465, IN_PROGRESS 85%) — production + hot path, v3 closure deferred
8. 🟡 **V311-09 F-36 列级权限** (PR #3457, IN_PROGRESS 70%) — 4 个主路径集成缺口 (current_user 重构)
9. 🟡 **V311-14 SEM-4 Coverage** (hermes 推进, PR #3443 + #3445 + #3450) — 8 测试已 re-enable, 还需 10+ 个

### 5.3 待办 (P1, BETA-RC 阶段)

10. ⏳ V311-03 F-25 Change Buffer 主路径集成
11. ⏳ V311-04 F-26 Double-Write Buffer 主路径集成
12. ⏳ V311-05 F-29 Row-Level Security 主路径集成
13. ⏳ V311-06 F-31 Performance Schema instrumentation
14. ⏳ V311-07 F-32 Admin 完整 wire 集成
15. ⏳ V311-08 F-35 Password Rotation 主路径集成
16. ⏳ V311-10 F-30 CREATE SEQUENCE
17. ⏳ V311-11 F-03 GIS 空间数据类型
18. ⏳ V311-12 F-27 Table Compression LZ4/zstd
19. ⏳ V311-16 subquery 去相关 optimizer pass
20. ⏳ V311-17 Hash Anti Join (✅ merged #3464)
21. ⏳ V311-18 CTE 物化
22. ⏳ V311-20 TPC-H SF=1 baseline (Hermes + dedicated hardware)
23. ⏳ V311-21 168h SOAK (Hermes)
24. ⏳ V311-22 文档整理 (本文, 正在做)

---

## 6. v3.11.0 不应试图 (与 v3.10.0 GA 同样的 scope 边界)

- ❌ **删除或大改 sqlrustgo-server crate** (~13 集成测试引用, 历史包袱)
- ❌ **强 archive gmp / rag** (server crate 真用, 删必破 build)
- ❌ **删除 evidence-graph + 关联 tools 之外的 graph 生态** (graph 已 archived, evidence-graph 已 deleted, 不要再扩展)
- ❌ **F-23 ClusteredTable v2 main path dispatch** (留 v3.12+)
- ❌ **F-24 BTreeIndex::scan + FileStorage page-level instrumentation** (留 v3.12+)
- ❌ **F-36 SELECT 路径 current_user 重构** (V311-09 follow-up, scope 超出 1 个会话)
- ❌ **AHI 在 secondary-index lookup 路径** (V311-02 v3, scope 超出 v3.11.0)
- ❌ **TPC-H SF=10 实测** (v3.10.0 已知 SF=10 cap bug 待 fix)
- ❌ **大范围重构 execution_engine.rs** (1212 行, 达标)

---

## 7. 引用

- SSOT: [`docs/governance/debt/debt-registry.yaml`](docs/governance/debt/debt-registry.yaml)
- v3.11.0 计划: [`docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md`](docs/releases/v3.11.0/plans/V311_DEVELOPMENT_PLAN.md)
- v3.11.0 债务闭环: [`docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md`](docs/releases/v3.11.0/plans/V311_DEBT_CLOSURE_PLAN.md)
- v3.11.0 文档整理: [`docs/releases/v3.11.0/plans/V311_DOCS_RESTRUCTURE_PLAN.md`](docs/releases/v3.11.0/plans/V311_DOCS_RESTRUCTURE_PLAN.md)
- v3.11.0 任务 ↔ Gitea issue: [`docs/releases/v3.11.0/plans/V311_ISSUE_CROSSREF.md`](docs/releases/v3.11.0/plans/V311_ISSUE_CROSSREF.md)
- v3.10.0 闭环报告: [`docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md`](docs/releases/v3.10.0/LEGACY_DEBT_CLOSURE_TRACKING_REPORT.md)
- v3.10.0 架构债: [`docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md`](docs/releases/v3.10.0/ARCHITECTURE_DEBT_ANALYSIS.md)
- 模块生命周期: [`docs/MODULE_LIFECYCLE.md`](docs/MODULE_LIFECYCLE.md)

### v3.11.0 ALPHA 已 merged PR (chronological)

- #3442 V311-13 SEM-3 ALTER TABLE (v1)
- #3447 V311-23 PERF-5 INSERT IGNORE
- #3449 V311-13 SEM-3 follow-up (parser N fix)
- #3454 V311-16 compiler warnings fix
- #3455 V311-15 Q4 Hash Semi Join
- #3457 V311-09 F-36 catalog + parser + 1 bugfix
- #3461 V311-01 F-23 ClusteredTable production storage
- #3464 V311-17 Hash Anti Join
- #3465 V311-02 F-24 AHI production + hot-path wire
- #3468 V311-19 Extension Crate Decision

---

**版本**: v3.11.0 (2026-07-15, ALPHA)
**作者**: openclaw + hermes + claude-macmini 协作
**下次更新**: v3.11.0 BETA 启动 (估计 2026-08-11)

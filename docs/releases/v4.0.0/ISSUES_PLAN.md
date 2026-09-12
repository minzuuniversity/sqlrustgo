# v4.0.0 Issue 计划

> **版本**: v4.0.0
> **创建日期**: 2026-09-12
> **状态**: Phase 0 完成

---

## Milestone: v4.0.0

**描述**: 生产级多模型数据库（SQL + Vector + Graph + GMP）

**截止日期**: 2026-12-31

---

## Issue 列表

### Phase 1 - Alpha

| Issue | 标题 | 状态 | Owner |
|-------|------|------|-------|
| V400-00 | V400-00 文件治理 gate | ✅ 已完成 | devops |
| V400-01 | Vector SQL syntax (parser + executor) | 🟡 待开始 | parser |
| V400-02 | WAL-backed vector storage + rebuild | 🟡 待开始 | storage |
| V400-03 | Graph first-class storage | 🟡 待开始 | graph |
| V400-04 | Graph query surface | 🟡 待开始 | graph |
| V400-05 | Cross-model transaction | 🟡 待开始 | transaction |
| V400-06 | Unified backup/restore | 🟡 待开始 | ops |
| V400-07 | Unified ACL + audit | 🟡 待开始 | security |
| V400-08 | Multi-model optimizer | 🟡 待开始 | optimizer |
| V400-09 | 168h multi-model SOAK | 🟡 待开始 | ops |
| V400-10 | GMP-Platform consumer regression | 🟡 待开始 | release |
| WP-A | v3.12.0 parser 遗留问题 | 🟡 待开始 | parser |
| WP-B | v3.12.0 type/function 遗留问题 | 🟡 待开始 | types |
| WP-C | v3.12.0 DDL/integrity 遗留问题 | 🟡 待开始 | storage |
| WP-D | v3.12.0 join/subquery 遗留问题 | 🟡 待开始 | executor |
| WP-E | v3.12.0 transaction 遗留问题 | 🟡 待开始 | transaction |
| WP-F | v3.12.0 schema migration 遗留问题 | 🟡 待开始 | storage |
| WP-G | v3.12.0 type/comparison 遗留问题 | 🟡 待开始 | types |
| WP-H | v3.13/defer 重新评估 | 🟡 待开始 | mixed |

---

## 详细 Issue 描述

### V400-00: 文件治理 gate ✅ 已完成

**状态**: 已完成

**完成内容**:
- `scripts/gate/check_no_log_tbl_json.sh`
- `.gitignore` 更新

---

### V400-01: Vector SQL syntax

**标题**: Vector column 与 vector index SQL syntax

**描述**:
实现 Vector SQL 语法支持，包括：
- `VECTOR(dimensions, dtype)` 列类型
- `CREATE VECTOR INDEX ... USING HNSW/IVF`
- `distance()` 函数
- parser + executor E2E tests

**依赖**: 无

**估计工时**: 4 周

**Owner**: parser

**验收标准**:
- `crates/parser/tests/v400_vector_parse.rs` — 100+ tests
- `crates/executor/tests/v400_vector_exec.rs` — 50+ tests

---

### V400-02: WAL-backed vector storage

**标题**: WAL-backed vector storage 与 rebuild

**描述**:
实现向量存储的 WAL 支持，包括：
- WAL entry for vector insert/delete/update
- WAL replay rebuild vector index (HNSW/IVF)
- crash recovery tests

**依赖**: V400-01

**估计工时**: 6 周

**Owner**: storage

**验收标准**:
- `crates/storage/tests/v400_vector_wal.rs` — crash test (≥ 5 scenarios)
- `crates/storage/tests/v400_vector_rebuild.rs` — index rebuild correctness

---

### V400-03: Graph first-class storage

**标题**: Graph crate revival 或 rewrite

**描述**:
把 `crates/graph/` 从 internal library 升级为 first-class subsystem：
- 公开 API: create_node, create_edge, traversal
- WAL-backed graph writes
- catalog: graph_nodes, graph_edges system tables

**依赖**: V400-02

**估计工时**: 4 周

**Owner**: graph

**验收标准**:
- `crates/graph/tests/v400_first_class.rs`
- `docs/architecture/v400_graph_storage.md`

---

### V400-04: Graph query surface

**标题**: Graph query surface

**描述**:
实现图查询语法：
- `GRAPH MATCH ... RETURN` Cypher subset
- Bounded path queries (max 5 hops)
- 集成 `crates/graph/src/cypher/`

**依赖**: V400-03

**估计工时**: 6 周

**Owner**: graph

**验收标准**:
- `crates/parser/tests/v400_graph_match.rs`
- `crates/executor/tests/v400_graph_traversal.rs`

---

### V400-05: Cross-model transaction

**标题**: Cross-model transaction semantics

**描述**:
实现跨模型事务：
- SQL + vector + graph + audit 同一事务边界
- COMMIT 时 all-or-nothing
- ROLLBACK 时 all-or-nothing
- 解决 v3.12.0 遗留 #4847, #4626

**依赖**: V400-02, V400-03

**估计工时**: 8 周

**Owner**: transaction

**验收标准**:
- `crates/transaction/tests/v400_cross_model.rs` — 50+ tests
- crash test: 10 scenarios

---

### V400-06: Unified backup/restore

**标题**: Unified backup/restore

**描述**:
实现统一备份恢复：
- `BACKUP DATABASE` 包含 SQL + vector + graph + audit data
- `RESTORE DATABASE` 重建全部 index
- incremental backup support

**依赖**: V400-05

**估计工时**: 4 周

**Owner**: ops

**验收标准**:
- `crates/backup/tests/v400_unified.rs`
- `docs/evidence/v4.0.0/backup_restore_equality_report.md`

---

### V400-07: Unified ACL + audit

**标题**: Unified ACL and audit

**描述**:
实现统一访问控制：
- ACL policy 覆盖 SQL + vector + graph
- Audit chain: ALCOA+
- Permission bypass tests 必须 fail closed

**依赖**: V400-03, V400-04

**估计工时**: 6 周

**Owner**: security

**验收标准**:
- `crates/security/tests/v400_unified_acl.rs` — 100+ tests
- `crates/audit/tests/v400_alcoa_chain.rs`

---

### V400-08: Multi-model optimizer

**标题**: Multi-model optimizer 与 metadata filters

**描述**:
实现多模型优化器：
- cost model 扩展: vector index cost, graph traversal cost
- metadata filter pushdown
- hybrid query plan

**依赖**: V400-01, V400-03

**估计工时**: 8 周

**Owner**: optimizer

**验收标准**:
- `crates/optimizer/tests/v400_multi_model.rs`

---

### V400-09: 168h multi-model SOAK

**标题**: Multi-model SOAK

**描述**:
运行 168 小时混合负载测试：
- 60% OLTP SQL
- 20% vector search
- 10% graph traversal
- 10% cross-model txn

**依赖**: V400-05, V400-06, V400-07

**估计工时**: 1 周准备 + 168h 运行

**Owner**: ops

**验收标准**:
- `docs/evidence/v4.0.0/168h_soak_report.md`
- 0 P0 incident open

---

### V400-10: GMP-Platform consumer regression

**标题**: GMP-Platform consumer regression

**描述**:
维护 GMP-Platform consumer contract：
- v1.5/v1.6 compile + smoke tests
- REST `/healthz`/`/api/stats`/`/api/search`
- 408 regression
- Audit/WebUI parity

**依赖**: 所有 Phase 1/2 功能

**估计工时**: 每阶段 1 周 + 按需运行

**Owner**: release

**验收标准**:
- `docs/releases/v4.0.0/evidence/gmp-platform-consumer/summary.md`

---

## Legacy Issue 工作包

### WP-A: v3.12.0 parser 遗留问题

**Issues**: #4708, #4696, #4710, #4720 及更多

**Owner**: parser

**估计工时**: 4 周

---

### WP-B: v3.12.0 type/function 遗留问题

**Issues**: #4721, #4674, #4716, #4676, #4675, #4670

**Owner**: types

**估计工时**: 3 周

---

### WP-C: v3.12.0 DDL/integrity 遗留问题

**Issues**: #4652, #4672, #4682, #4669, #4703

**Owner**: storage

**估计工时**: 4 周

---

### WP-D: v3.12.0 join/subquery 遗留问题

**Issues**: #4668, #4656, #4649, #4636

**Owner**: executor

**估计工时**: 4 周

---

### WP-E: v3.12.0 transaction 遗留问题

**Issues**: #4847, #4626

**Owner**: transaction

**估计工时**: 4 周

---

### WP-F: v3.12.0 schema migration 遗留问题

**Issues**: #4848

**Owner**: storage

**估计工时**: 2 周

---

### WP-G: v3.12.0 type/comparison 遗留问题

**Issues**: #4846

**Owner**: types

**估计工时**: 2 周

---

### WP-H: v3.13/defer 重新评估

**Issues**: #4717, #4707, #4701, #4692, #4699, #4688, #4671, #4639

**Owner**: mixed

**估计工时**: 8 周

---

## 创建 Issue 的 Gitea API 命令

```bash
# 创建 Milestone
curl -X POST "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/milestones" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"v4.0.0","description":"生产级多模型数据库","due_on":"2026-12-31T00:00:00Z"}'

# 创建 Issue 示例
curl -X POST "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/issues" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "title":"V400-01: Vector SQL syntax",
    "body":"实现 Vector SQL 语法支持\n\n依赖: 无\n估计: 4 周",
    "milestone":1,
    "labels":["v4.0.0","Phase-1"]
  }'
```

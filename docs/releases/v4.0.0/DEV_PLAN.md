# SQLRustGo v4.0.0 Development Plan

> **版本**: v4.0.0
> **状态**: draft (2026-09-08)
> **目标**: 落地 18 个 WP,每个 WP 都有 owner + exit evidence + 测试要求
> **配合**: `ROADMAP.md` (阶段), `LEGACY_ISSUES.md` (issue 来源), `TEST_PLAN.md` (gate), `VERSION_PLAN.md` (产品)

---

## 1. 编码与提交规则 (强制)

### 1.1 提交文件排除 (用户 2026-09-08)

**禁止** 提交以下类型文件:

- `*.log` (server.log, cargo.log, sysbench.log, ...)
- `*.tbl` (数据表 dump)
- `*.json` (大 evidence JSON, > 1MB)

**例外**:
- 小 JSON 配置 (`Cargo.toml`, `package.json` 类 metadata)
- GitHub Actions 工作流 `.json`
- `cargo audit` / `cargo clippy` 等工具输出必须 redirect 到 stdout,不入仓

**机制**:

1. `.gitignore` 加入:
   ```
   *.log
   *.tbl
   *.json
   !Cargo.toml
   !package.json
   !tsconfig.json
   !*.config.json
   !docs/governance/*.json
   ```
2. `scripts/gate/check_no_log_tbl_json.sh` 在每个 PR 上跑
3. CI 失败 if new commit 引入这些类型

### 1.2 文件大小

- 单文件 ≤ 100MB (GitHub 兼容)
- 大 evidence → `*.json.gz` 或 external object storage (S3-like)

### 1.3 Commit message

```
<type>(<area>/<milestone>): <subject>

[body: evidence + reasoning]

[footer: refs #issue-number]
```

type ∈ {`feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `perf`, `revert`}
area ∈ {`v400`, `v400-vec`, `v400-graph`, `v400-txn`, `v400-sec`, `v400-opt`, `v400-soak`}

---

## 2. 工作包详细分解

### V400-00 — log/tbl/json gate + .gitignore

**Phase**: 0
**Owner**: devops
**依赖**: 无
**估计**: 1 周

**任务**:

1. 写 `scripts/gate/check_no_log_tbl_json.sh`:
   - 遍历 `git diff --name-only HEAD~1..HEAD`
   - 失败 if any path matches `*.log`, `*.tbl`, `*.json` (with allowlist)
   - 失败 if any path size > 100MB
2. 写 `.gitignore` additions (repo root):
   ```
   # v4.0.0 rule (2026-09-08): no log/tbl/json files
   *.log
   *.tbl
   *.json
   !Cargo.toml
   !package.json
   !tsconfig.json
   !.gitignore
   !.gitattributes
   !.config.json
   !docs/governance/*.json
   !docs/releases/*/manifest.json
   !docs/releases/*/evidence/*/summary.json
   ```
3. `git rm --cached -r` 所有 tracked log/tbl/json files (历史 commit 保留,但 working tree 不再 track)
4. CI 在 `docs/governance/ci/gate-no-log-tbl-json.yml` 跑
5. 文档: `docs/governance/FILE_GOVERNANCE.md`

**Exit evidence**:
- gate script dry-run PASS
- CI failure on intentional `*.log` push
- 现有 repo 中 log/tbl/json 文件列表 + cleanup 计划

---

### V400-01 — vector SQL syntax

**Phase**: 1
**Owner**: parser
**依赖**: 无
**估计**: 4 周

**任务**:

1. SQL syntax:
   ```sql
   CREATE TABLE t(
     id INT PRIMARY KEY,
     embedding VECTOR(384, FLOAT32),   -- new
     meta JSON
   );

   CREATE VECTOR INDEX idx_emb ON t USING HNSW (embedding) WITH (m=16, ef_construction=200);
   CREATE VECTOR INDEX idx_ivf ON t USING IVF (embedding, nlist=100);

   SELECT id, distance(embedding, '[0.1, 0.2, ...]') AS d
   FROM t ORDER BY d LIMIT 10;
   ```

2. `crates/parser/` 增加 token + AST 节点
3. `crates/executor/` 增加 vector 物理 plan
4. `crates/types/` 增加 `VectorType { dim, dtype }`
5. Tests: parser 100+ case,executor 50+ case

**Exit evidence**:
- `crates/parser/tests/v400_vector_parse.rs` — 100+ tests
- `crates/executor/tests/v400_vector_exec.rs` — 50+ tests
- docs: `docs/architecture/v400_vector_sql.md`

---

### V400-02 — WAL-backed vector storage

**Phase**: 1
**Owner**: storage
**依赖**: V400-01
**估计**: 6 周

**任务**:

1. WAL entry for vector insert/delete/update
2. WAL replay rebuild vector index (HNSW/IVF)
3. crash recovery: kill -9 → restart → verify index + data equality
4. benchmark: insert throughput, search recall@10

**Exit evidence**:
- `crates/storage/tests/v400_vector_wal.rs` — crash test (≥ 5 crash scenarios)
- `crates/storage/tests/v400_vector_rebuild.rs` — index rebuild correctness
- `docs/evidence/v4.0.0/vector_wal_recovery_report.md`

---

### V400-03 — sqlrustgo-graph first-class storage

**Phase**: 2
**Owner**: graph
**依赖**: sqlrustgo-graph M1-M5 (✅ 已完成,v3.12.0 develop)
**估计**: 4 周

**任务**:

1. 把 `crates/graph/` (sqlrustgo-graph) 从 internal library 升级为 first-class subsystem
2. 公开 API:
   - `GraphStore::create_node(NodeLabel, props)`
   - `GraphStore::create_edge(src, dst, EdgeLabel, props)`
   - `GraphStore::traversal(Spec)`
3. WAL-backed graph writes (复用 V400-02 框架)
4. catalog: `graph_nodes`, `graph_edges`, `graph_node_props`, `graph_edge_props` system tables
5. ACL: SQL `CREATE ROLE ... WITH GRAPH_PERMS = ...`

**Exit evidence**:
- `crates/graph/tests/v400_first_class.rs`
- docs: `docs/architecture/v400_graph_storage.md`

---

### V400-04 — graph query surface

**Phase**: 2
**Owner**: graph
**依赖**: V400-03
**估计**: 6 周

**任务**:

1. SQL extension:
   ```sql
   SELECT * FROM GRAPH MATCH (n:Person)-[r:KNOWS]->(m:Person)
   WHERE n.age > 25
   RETURN n, r, m;
   ```
2. 解析器增加 `GRAPH MATCH` syntax
3. Cypher subset: `MATCH ... RETURN`, `WHERE`, `LIMIT`, simple patterns
4. Bounded path queries (max 5 hops)
5. 集成 `crates/graph/src/cypher/` (✅ 已实现 M4)

**Exit evidence**:
- `crates/parser/tests/v400_graph_match.rs`
- `crates/executor/tests/v400_graph_traversal.rs`
- docs: `docs/architecture/v400_graph_query.md`

---

### V400-05 — cross-model transaction

**Phase**: 2
**Owner**: transaction
**依赖**: V400-02, V400-03
**估计**: 8 周

**任务**:

1. 事务 boundary 覆盖 SQL + vector + graph + audit
2. COMMIT 时:all-or-nothing;ROLLBACK 时:all-or-nothing
3. crash during multi-model write → 启动后 verify 状态一致性
4. 解决 v3.12.0 遗留 #4847 (batch 注释行 / ROLLBACK 误回滚事务外 DML) — WP-E
5. 解决 v3.12.0 遗留 #4626 (SELECT FOR UPDATE 后 ROLLBACK) — WP-E

**Exit evidence**:
- `crates/transaction/tests/v400_cross_model.rs` — 50+ tests
- crash test: 10 scenarios
- rollback test: 20 scenarios
- `docs/evidence/v4.0.0/cross_model_txn_report.md`

---

### V400-06 — unified backup/restore

**Phase**: 3
**Owner**: ops
**依赖**: V400-05
**估计**: 4 周

**任务**:

1. `BACKUP DATABASE` 包含 SQL + vector + graph + audit data
2. `RESTORE DATABASE` 重建全部 index (vector HNSW/IVF, graph adjacency)
3. 验证:counts、hashes、indexes 三者一致
4. incremental backup support

**Exit evidence**:
- `crates/backup/tests/v400_unified.rs`
- `docs/evidence/v4.0.0/backup_restore_equality_report.md`

---

### V400-07 — unified ACL + audit

**Phase**: 3
**Owner**: security
**依赖**: V400-03, V400-04
**估计**: 6 周

**任务**:

1. ACL policy 覆盖:
   - SQL tables/columns
   - vector columns
   - graph node labels / edge labels
2. Audit chain:ALCOA+ (Attributable, Legible, Contemporaneous, Original, Accurate, Complete, Consistent, Enduring, Available)
3. Permission bypass tests 必须 fail closed
4. 解决 v3.12.0 遗留 #4847 (transaction 误回滚) — WP-E 协同

**Exit evidence**:
- `crates/security/tests/v400_unified_acl.rs` — 100+ tests
- `crates/audit/tests/v400_alcoa_chain.rs`
- `docs/evidence/v4.0.0/acl_audit_report.md`

---

### V400-08 — multi-model optimizer

**Phase**: 3 (skeleton in Phase 1)
**Owner**: optimizer
**依赖**: V400-01, V400-03
**估计**: 8 周

**任务**:

1. cost model 扩展:vector index cost, graph traversal cost
2. metadata filter pushdown
3. hybrid query: SQL + vector search + graph traversal 单 plan
4. EXPLAIN 包含 vector/graph 节点

**Exit evidence**:
- `crates/optimizer/tests/v400_multi_model.rs`
- docs: `docs/architecture/v400_optimizer.md`

---

### V400-09 — 168h multi-model SOAK

**Phase**: 3
**Owner**: ops
**依赖**: V400-05, V400-06, V400-07
**估计**: 168h (≈ 7 天) 实际运行 + 1 周准备

**任务**:

1. 准备 workload mix:
   - 60% OLTP SQL
   - 20% vector search
   - 10% graph traversal
   - 10% cross-model txn
2. 跑 168h,持续监控 crash / consistency / latency
3. 失败 case 必须 reproduce + fix

**Exit evidence**:
- `docs/evidence/v4.0.0/168h_soak_report.md`
- 所有 crash incident 有 issue + PR
- 0 P0 incident open

---

### WP-A — v3.12.0 parser 必修

**Phase**: 1
**Owner**: parser
**依赖**: 无
**估计**: 4 周
**Issues**: #4708 #4696 #4710 #4720 (17 issues per LEGACY_ISSUES §3.3)

**任务**: 每个 issue 一个 PR,带 regression test。

**Exit evidence**:
- 17 issues closed via merged PR
- `crates/parser/tests/wp_a_legacy.rs` 包含每个 issue 的 regression

---

### WP-B — v3.12.0 type/function 必修

**Phase**: 1
**Owner**: types
**依赖**: WP-A
**估计**: 3 周
**Issues**: #4721 #4674 #4716 #4676 #4675 #4670

**Exit evidence**: 7 issues closed

---

### WP-C — v3.12.0 DDL/integrity 必修

**Phase**: 2
**Owner**: storage
**依赖**: WP-A
**估计**: 4 周
**Issues**: #4652 #4672 #4682 #4669 #4703

**Exit evidence**: 5 issues closed + metadata table integration tests

---

### WP-D — v3.12.0 join/subquery 必修

**Phase**: 1
**Owner**: executor
**依赖**: WP-A
**估计**: 4 周
**Issues**: #4668 #4656 #4649 #4636

**Exit evidence**: 4 issues closed + Oracle comparison tests

---

### WP-E — v3.12.0 transaction 必修

**Phase**: 2
**Owner**: transaction
**依赖**: WP-C
**估计**: 4 周
**Issues**: #4847 #4626 (必),可能增加 #4722 #4709 #4721 等的 cross-effect

**Exit evidence**: 2 issues closed + cross-model rollback tests

---

### WP-F — schema migration 必修

**Phase**: 1
**Owner**: storage
**依赖**: WP-A
**估计**: 2 周
**Issues**: #4848

**Exit evidence**: 1 issue closed + ALTER TABLE RENAME COLUMN test

---

### WP-G — type/comparison 必修

**Phase**: 1
**Owner**: types
**依赖**: WP-A
**估计**: 2 周
**Issues**: #4846

**Exit evidence**: 1 issue closed + CHAR padding comparison test

---

### WP-H — v3.13/defer 重新评估

**Phase**: 2/3
**Owner**: mixed
**依赖**: V400-04 (graph) 才能 #4699 recursive CTE
**估计**: 8 周 (跨 Phase)
**Issues**: #4717 #4707 #4701 #4692 #4699 #4688 #4671 #4639

**Exit evidence**: 8 issues triage 完成,每个有 owner + scope decision

---

## 3. 工作包依赖图

```
WP-A (parser) ── WP-B (types) ── WP-C (DDL/integrity) ──┐
   │                                                   │
   ├─ WP-D (join/subq) ──────────────────────────────► WP-E (txn) ──┐
   │                                                                  │
   ├─ WP-F (schema migration) ──────────────────────────────────────┐ │
   │                                                                  │
   ├─ WP-G (CHAR) ───────────────────────────────────────────────┐  │ │
   │                                                             │  │ │
V400-01 (vector SQL) ── V400-02 (WAL vec) ── V400-03 (graph) ── V400-04 (graph qry) ── V400-05 (cross txn)
                                                                     │                  │
                                                                     └─ V400-08 (optimizer) ┘
                                                                                │
                                                                  V400-06 + V400-07
                                                                                │
                                                                          V400-09 (168h SOAK)
                                                                                │
                                                                              GA gate
```

---

## 4. CI/CD 与 Gate

### 4.1 每个 PR 必跑

- `cargo fmt --check`
- `cargo clippy -- -D warnings`
- `cargo build --workspace --all-targets`
- `cargo test --workspace`
- `scripts/gate/check_no_log_tbl_json.sh` (V400-00)
- file size check (`*.log/*.tbl/*.json` < 1MB;`*.anything` < 100MB)

### 4.2 每晚

- `cargo bench` regression
- `docs/releases/v4.0.0/evidence/` 自动生成 manifest

### 4.3 Phase 退出

- alpha: V400-G1..G6 (6 gates)
- beta: B-G1..G6
- rc: RC-G1..G5
- ga: GA-G1..G6

每 gate 必有 `*.json` evidence file。

---

## 5. 测试矩阵

按 TEST_PLAN.md 的 10 个 gate 实施。每个 gate 一个或多个 `crates/*/tests/v400_*.rs`。

---

## 6. 风险与执行注意

### 6.1 GMP-Platform integration

- PR #207 仍 blocked by self-approval
- 解决:Phase 0 增加 `merge_whitelist_usernames = ["openclaw", "hermes-agent"]`,或 split PR
- 影响:GMP-Platform regression suite 必须能跑 (Phase 1 entry criteria)

### 6.2 llama.cpp embeddings

- 当前 /v1/embeddings 501
- Phase 0 任务:让 .252 llama.cpp 加 `--embeddings` flag
- 替代:Phase 1 用 sqlrustgo 内部模型 (sentence-transformers-rs)

### 6.3 历史大文件

- v3.12.0 GA 历史 commit 含 251MB server.log
- Phase 0 必须 git filter-branch 重写或转 LFS,否则 GA push GitHub 阻塞
- 沿用 v4.0.0 file-extension 规则

### 6.4 资源约束

- .250 + .252 是仅有的 Gitea,GitHub 尚未 unblock
- 168h SOAK 需要 7×24 持续运行,需提前申请资源窗口

---

## 7. 估算与排期

| WP | 工作量 (人周) | Phase |
|---|---|---|
| V400-00 | 1 | 0 |
| V400-01 | 4 | 1 |
| V400-02 | 6 | 1 |
| V400-03 | 4 | 2 |
| V400-04 | 6 | 2 |
| V400-05 | 8 | 2 |
| V400-06 | 4 | 3 |
| V400-07 | 6 | 3 |
| V400-08 | 8 | 3 |
| V400-09 | 168h 实际运行 + 1 周准备 | 3 |
| WP-A | 4 | 1 |
| WP-B | 3 | 1 |
| WP-C | 4 | 2 |
| WP-D | 4 | 1 |
| WP-E | 4 | 2 |
| WP-F | 2 | 1 |
| WP-G | 2 | 1 |
| WP-H | 8 | 2/3 |

**总工作量**: ~76 人周 (~19 人月)

并行假设:
- parser / types / executor 三组并行
- storage + transaction 紧密耦合
- ops + security 独立

---

## 8. Owner 矩阵

| Role | WP |
|---|---|
| parser 负责人 | V400-01, WP-A, WP-D |
| types 负责人 | WP-B, WP-G |
| storage 负责人 | V400-02, V400-03, WP-C, WP-F |
| graph 负责人 | V400-03, V400-04, WP-H (#4699 recursive CTE) |
| transaction 负责人 | V400-05, WP-E, WP-C (#4652) |
| executor 负责人 | WP-D, WP-H |
| security 负责人 | V400-07 |
| optimizer 负责人 | V400-08 |
| ops 负责人 | V400-00, V400-06, V400-09 |
| docs 负责人 | LEGACY_ISSUES / ROADMAP / TEST_PLAN / VERSION_PLAN / CHANGELOG 维护 |

---

## 9. Phase 0 退出 checklist (本次 draft 完成目标)

- [x] `develop/v4.0.0` branch created at `9febebb255` and pushed to 3 remotes
- [x] LEGACY_ISSUES.md (21+12+8)
- [x] ROADMAP.md (4 phases)
- [x] DEV_PLAN.md (本文,18 WP)
- [ ] CHANGELOG.md 更新 draft phase
- [ ] Gitea branch protection: develop/v4.0.0 (草稿期)
- [ ] `scripts/gate/check_no_log_tbl_json.sh` 完成
- [ ] `.gitignore` 加入 log/tbl/json 排除
- [ ] milestone `v4.0.0` 创建
- [ ] 至少 3 个 v4.0.0 issue 创建 (V400-01, V400-02, V400-00)
- [ ] GMP-Platform PR #207 合并或 split

---

## 10. 下一步

完成 Phase 0 后进入 Phase 1 Alpha:
1. 写 V400-01 issue + spec
2. 开始 vector SQL syntax 设计
3. 启动 .252 llama.cpp `--embeddings` flag (ops)
4. 每周 stand-up 同步 phase exit evidence
# v3.11.0 扩展 Crate 归档说明

## 1. 文档定位

本文件记录 v3.11.0 对 extension crates 的产品决策：哪些保留、哪些归档、哪些进入主路径。该文档用于追溯架构边界，不能单独证明某个 crate 已具备生产能力。

## 2. 决策原则

| 原则 | 说明 |
|---|---|
| 主路径优先 | 只有进入 parser/planner/executor/storage/network 主路径并有 gate 的能力，才可对外作为产品功能声明 |
| 归档不等于删除历史 | archived crate 可保留历史价值，但不得作为当前生产依赖 |
| crate 存在不等于生产可用 | 需要 build/test/e2e/soak/recovery 证据 |
| GMP/RAG/Vector/Graph 分层 | v3.11 保留原型或内部能力，v3.12/4.0 再决定生产级边界 |

## 3. v3.12 影响

v3.12.0 应基于本文件继续清理 extension crate 语义漂移：

- 明确 `vector`、`graph`、`rag`、`gmp` 哪些是生产路径，哪些仍是实验路径。
- 对进入 GMP 内审检索路径的 crate 增加 gate 和 evidence bundle。
- 对归档 crate 防止被误引用为生产能力。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# V311-19 Extension Crates Archive (v3.11.0)

This document records 9 extension crates removed from the workspace in
V311-19. Crates are classified by action taken: **DELETED** (pure dead
code) or **ARCHIVED** (revivable; preserved for history).

## Summary Table

| Crate | LOC | Action | Location | Reason |
|-------|-----|--------|----------|--------|
| `distributed` | 17,266 | DELETED | `archive/v3.11/deleted-crates/distributed/` | 0 callers in production |
| `agentsql` | 4,390 | DELETED | `archive/v3.11/deleted-crates/agentsql/` | 0 callers in production |
| `qmd-bridge` | 1,033 | DELETED | `archive/v3.11/deleted-crates/qmd-bridge/` | 0 callers (orphan bridge) |
| `evidence-graph` | 926 | DELETED | `archive/v3.11/deleted-crates/evidence-graph/` | gate-tool self-contained |
| `unified-query` | 1,616 | DELETED | `archive/v3.11/deleted-crates/unified-query/` | only refs `unified-storage` |
| `unified-storage` | 983 | DELETED | `archive/v3.11/deleted-crates/unified-storage/` | only deps on archived `graph` |
| `graph` | 6,429 | ARCHIVED | `archive/v3.11/archived-crates/graph/` | 1 test + dead-code crates used it |
| ~~gmp~~ | — | KEPT | (workspace member) | 47 refs in `crates/server/src/openclaw_endpoints.rs` |
| ~~rag~~ | — | KEPT | (workspace member) | 4 refs in `crates/server/src/openclaw_endpoints.rs` |

Also archived:
- `tools/graph-cli` (depends on `evidence-graph`)
- `tools/sqlrustgo-gate` (depends on `evidence-graph`)
- `tests/integration/sql/graph_cypher_integration_test.rs` (depends on `graph`)

## DELETED (6 crates, ~36K LOC)

Pure dead code — no callers anywhere in active source. Removal eliminates:
- ~36K LOC to maintain
- 6 workspace members to compile
- Transitive dependencies (rusqlite, tokio, reqwest, etc.)

These were `SCOPE_DEFERRED` debt items that never integrated. Archiving
preserves git history; reviving is possible via `git mv` from archive.

## ARCHIVED (1 crate + 2 tools + 1 test, ~7K LOC)

`graph` (and its dependent test + tools) was kept for potential revival:
- 1 test (`graph_cypher_integration_test.rs`) actively uses it
- 2 tools (`graph-cli`, `sqlrustgo-gate`) built against it

Future revival is straightforward — see below.

## KEPT (2 crates)

**`gmp`** (5,732 LOC): actively used by `crates/server/src/openclaw_endpoints.rs` for:
- Audit reporting (`sqlrustgo_gmp::report::*`)
- Compliance checking (`sqlrustgo_gmp::compliance::*`)
- Audit log access (`sqlrustgo_gmp::audit::*`)

**`rag`** (2,022 LOC): actively used by `crates/server/src/openclaw_endpoints.rs` for:
- Document retrieval (`sqlrustgo_rag::Document`)
- OpenClaw client integration (`sqlrustgo_rag::OpenClawClient`)

These are core to the server's compliance/RAG features and remain
active workspace members.

## Revival Procedure

To revive an archived crate:

```bash
# 1. Move the crate back into the source tree
git mv archive/v3.11/{deleted,archived}-crates/<name> crates/<name>

# 2. Add it back to [workspace] members in /Cargo.toml
# Find the [workspace] section and add the entry:
#     "crates/<name>",

# 3. Add it back to [workspace.dependencies] if other crates depend on it
# Find the [workspace.dependencies] section and add:
#   sqlrustgo-<name> = { path = "crates/<name>" }

# 4. Restore any cross-deps that were modified for removal
# (e.g., gmp's Cargo.toml no longer references sqlrustgo-graph)

# 5. Verify the build
cargo build -p sqlrustgo-<name>
cargo test -p sqlrustgo-<name>

# 6. Update this document to remove the revival entry
```

## Audit Trail (V311-19 Implementation)

| Step | Action | Verification |
|------|--------|--------------|
| 1 | Moved 6 crates to `archive/v3.11/deleted-crates/` via `git mv` | git log --follow shows continuity |
| 2 | Moved 1 crate + 1 test to `archive/v3.11/archived-crates/` | git log --follow shows continuity |
| 3 | Updated `Cargo.toml` `[workspace] members` (7 deletions) | `cargo build --workspace` succeeds |
| 4 | Updated `Cargo.toml` workspace.dependencies (4 removals) | `cargo build --release` succeeds |
| 5 | Removed `sqlrustgo-graph` dep from `crates/gmp/Cargo.toml` | gmp still builds |
| 6 | Archived 2 tools (`graph-cli`, `sqlrustgo-gate`) | workspace excludes them |
| 7 | All 25 V311-15/17/01 tests still PASS | `cargo test --release ...` green |

Result:
- Workspace members: 45 → 36 (–9 crates, –2 tools = **11 total removals**)
- `cargo build --workspace`: 25.80s (down from prior runs)
- `cargo build --release`: 8.44s
- All 25 prior integration tests: **25/25 PASS**

## References

- `openspec/changes/v311-19-extension-crates-decision/` — full proposal/design/tasks
- `docs/governance/debt/debt-registry.yaml` — debt items transitioned DELETED/ARCHIVED
- `Cargo.toml` — workspace member list (current)

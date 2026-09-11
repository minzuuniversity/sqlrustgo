# SQLRustGo v4.0.0 测试计划

> **版本**: v4.0.0
> **状态**: 规划中
> **日期**: 2026-08-08
> **目标**: 生产级 SQL + Vector + Graph + GMP 多模型数据库
> **本次整改**: 2026-09-11,补齐 legacy issue 与 GMP-Platform consumer contract

## 1. 测试矩阵

| Gate | 领域 | 方法 | 阈值 |
|---|---|---|---|
| V400-G1 | SQL 回归 | workspace build/test/fmt/clippy | 退出码 0，0 warning |
| V400-G2 | Vector SQL | parser + executor + storage E2E | vector insert/search/delete 通过 |
| V400-G3 | 向量恢复 | vector writes 期间 crash | WAL replay 恢复 index/data |
| V400-G4 | 图存储 | node/edge CRUD + traversal tests | graph 结果确定 |
| V400-G5 | 图恢复 | graph writes 期间 crash | WAL replay 恢复 nodes/edges |
| V400-G6 | 跨模型事务 | SQL + vector + graph + audit 同一事务 | commit/rollback 原子化 |
| V400-G7 | 备份恢复 | full multi-model restore | counts、hashes、indexes 相等 |
| V400-G8 | 安全 | ACL 和 audit 绕过测试 | fail closed |
| V400-G9 | 检索质量 | GMP SQL + vector + graph hybrid set | citation 和 path evidence 完整 |
| V400-G10 | 多模型长稳 | 168h mixed production workload | 0 crash，无一致性破坏 |
| V400-G11 | 历史遗留问题 | WP-A..WP-H regression matrix | must-fix issue 100% 有 regression |
| V400-G12 | GMP-Platform consumer | v1.5/v1.6 compile + REST/WebUI/audit/408 smoke | 通过或失败归因明确 |

## 1.1 Work Package 测试映射

| WP | 覆盖范围 | 必需测试 | 阈值 | Evidence |
|---|---|---|---|---|
| V400-00 | file governance | `scripts/gate/check_no_log_tbl_json.sh` + intentional violation | clean repo PASS,故意提交 FAIL | `evidence/v400-00-file-gate.md` |
| V400-01 | vector SQL syntax | parser 100+ cases,executor 50+ cases | 100% pass | `crates/parser/tests/v400_vector_parse.rs` |
| V400-02 | WAL-backed vector storage | crash/rebuild >=5 scenarios,embedding count check | index/data equality | `vector_wal_recovery_report.md` |
| V400-03 | first-class graph storage | node/edge CRUD,WAL replay,snapshot open | deterministic graph equality | `v400_graph_storage.md` |
| V400-04 | graph query surface | `GRAPH MATCH`,Cypher subset,bounded path | 100% expected rows | `v400_graph_query.md` |
| V400-05 | cross-model transaction | SQL row + vector + graph + audit commit/rollback/crash | all-or-nothing | `cross_model_txn_report.md` |
| V400-06 | backup/restore | full restore counts/hashes/indexes | exact equality | `backup_restore_equality_report.md` |
| V400-07 | unified ACL + audit | bypass attempts,ALCOA+ chain,tamper tests | fail closed | `acl_audit_report.md` |
| V400-08 | optimizer/filter | hybrid SQL + vector + graph plan,EXPLAIN evidence | expected plan nodes | `v400_optimizer.md` |
| V400-09 | SOAK | 24h beta,168h RC mixed workload | 0 crash,0 consistency break | `168h_soak_report.md` |
| V400-10 | GMP-Platform consumer | see §1.3 | see §1.3 | `gmp-platform-consumer/summary.md` |
| WP-A | parser legacy | #4708/#4696/#4710/#4720 plus mapped parser issues | every issue has regression | `wp_a_legacy.rs` |
| WP-B | type/function legacy | #4721/#4674/#4716/#4676/#4675/#4670 | every issue has regression | `wp_b_type_fn.rs` |
| WP-C | DDL/integrity legacy | #4652/#4672/#4682/#4669/#4703/#4709 | metadata/integrity pass | `wp_c_ddl_integrity.rs` |
| WP-D | join/subquery legacy | #4668/#4656/#4649/#4636 | oracle comparison pass | `wp_d_join_subquery.rs` |
| WP-E | transaction legacy | #4847/#4626 | rollback/abort semantics pass | `wp_e_txn_legacy.rs` |
| WP-F | schema migration | #4848 | rename-column pass | `issue_4848_alter_rename_column_test.rs` |
| WP-G | CHAR comparison | #4846 | MySQL/SQLite-compatible subset pass | `issue_4846_char_pad_space_test.rs` |
| WP-H | deferred issues | #4717/#4707/#4701/#4692/#4699/#4688/#4671/#4639 | owner + scope decision + tests if in-scope | `wp_h_triage.md` |

## 1.2 Legacy Issue 回归规则

每个 `LEGACY_ISSUES.md` §3 的 must-fix issue 必须满足:

1. 有一个命名包含 issue id 的 regression test 或 sqllogictest case。
2. 测试必须在修复前能复现失败,修复后通过;如果无法保留 red/green 证据,需在 PR body 写明原因。
3. issue close 前必须记录 affected crate、test command、observed failure before fix、pass evidence after fix。
4. `LEGACY_ISSUES.md` §4 的 continued caveat 不需要修复,但必须在 GA claim manifest 里继续列出。
5. `LEGACY_ISSUES.md` §5 的 re-evaluate item 必须在 RC 前给出 in-scope / out-of-scope / defer-to-v4.1 决策。

## 1.3 GMP-Platform Consumer Gate

`docs/releases/v4.0.0/GMP_PLATFORM_REQUIREMENTS.md` 是本 gate 的合同来源。最小验收:

| Check | Command / method | Threshold |
|---|---|---|
| Consumer compile | GMP-Platform path deps 指向当前 SQLRustGo worktree;`cargo build -p gmp-storage -p gmp-server` | 0 errors |
| Graph/RAG unit subset | `cargo test -p gmp-storage cypher_engine`;RAG/tokenizer/search subset | all pass |
| Embedding load | GMP embedding fixture 12,531 rows | count matches;startup/rebuild time recorded |
| REST smoke | `/healthz`, `/api/stats`, `/api/search` | ok;docs/chunks/embeddings counts plausible;hits non-empty |
| Upload immediate search | upload md/txt fixture then search unique phrase | hit without restart |
| 408 regression | GMP 408 evaluator | >=99.0% or every fail classified as corpus/LLM/GMP/SQLRustGo |
| Audit degraded mode | no-rag/no-llm and rag mode audit runs | deterministic verdict;timeout behavior documented |
| WebUI parity | Track 0/A/B/C coverage statement | documented gap;automated A/B/C tests pass |
| CJK safety | Chinese identifier/LIKE/SUBSTRING/string functions | no panic;expected rows |
| Contention smoke | mixed REST/search/audit workload | no runaway process;latency/cpu recorded |

失败归因必须使用四类之一: `SQLRustGo bug`, `GMP-Platform bug`, `corpus gap`, `LLM/backend dependency`。不能把 Ollama/llama.cpp crash 算作 SQLRustGo pass/fail。

## 2. 必需 fixtures

| Fixture | 用途 |
|---|---|
| GMP 语料 | document/chunk/audit workload |
| 向量语料 | ANN correctness、recall、latency |
| 图语料 | traversal correctness 和 path expansion |
| 跨模型事务 fixture | SQL row + vector + graph + audit atomicity |
| 崩溃/恢复 fixture | WAL and backup verification |
| GMP-Platform v1.5 fixture | 1760 docs / 12531 chunks / 12531 embeddings / 408 scenarios |
| GMP-Platform v1.6 audit fixture | sample BPR,rule set,PDF/audit/WebUI parity inputs |
| CJK fixture | Chinese identifier,Chinese LIKE,SUBSTRING boundary,UTF-8 multi-byte text |
| LLM backend fixture | OpenAI-compatible and Ollama failure classification stubs |

## 3. 拒绝规则

只要出现以下任一情况，v4.0.0 不得进入 GA：

- vector storage 不是 WAL-backed。
- graph storage 不是 WAL-backed。
- SQL/vector/graph writes 不能参与同一个 transaction boundary。
- backup/restore 不能重建 vector 和 graph indexes。
- access control 只覆盖 SQL，不覆盖 vector/graph 路径。
- graph query support 只是 archived crate，且没有 production revalidation。
- 168h multi-model SOAK 缺失或不完整。
- `LEGACY_ISSUES.md` §3 的 must-fix issue 没有 regression test。
- GMP-Platform consumer gate 缺失,或 408/WebUI/audit 失败未归因。
- CJK 文本路径存在 panic 或 silent data corruption。
- 把 LLM backend crash 当作 SQLRustGo gate PASS 证据。

## 4. Evidence 布局

所有 v4.0.0 测试 evidence 统一进入:

```
docs/releases/v4.0.0/evidence/
  alpha_gate.json
  beta_gate.json
  rc_gate.json
  ga_gate.json
  legacy-regression/
  gmp-platform-consumer/
  soak/
```

大 JSON 不直接入仓;按 `DEV_PLAN.md` 的 file governance 写成 `.json.gz` 或 external artifact,仓库内只保留 summary/manifest。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v4.0.0 Test Plan

> **Version**: v4.0.0
> **Status**: PLANNED
> **Date**: 2026-08-08
> **Target**: production multi-model SQL + Vector + Graph + GMP database

## 1. Test Matrix

| Gate | Area | Method | Threshold |
|---|---|---|---|
| V400-G1 | SQL regression | workspace build/test/fmt/clippy | exit 0, 0 warnings |
| V400-G2 | Vector SQL | parser + executor + storage E2E | vector insert/search/delete 通过 |
| V400-G3 | Vector recovery | crash during vector writes | WAL replay restores index/data |
| V400-G4 | 图存储 | node/edge CRUD + traversal tests | graph 结果确定 |
| V400-G5 | Graph recovery | crash during graph writes | WAL replay restores nodes/edges |
| V400-G6 | 跨模型事务 | SQL + vector + graph + audit 同一事务 | commit/rollback 原子化 |
| V400-G7 | Backup/restore | full multi-model restore | counts, hashes, indexes equal |
| V400-G8 | 安全 | ACL 和 audit 绕过测试 | fail closed |
| V400-G9 | 检索质量 | GMP SQL + vector + graph hybrid set | citation 和 path evidence 完整 |
| V400-G10 | Multi-model SOAK | 168h mixed production workload | 0 crash, no consistency break |

## 2. Required Fixtures

| Fixture | Purpose |
|---|---|
| GMP 语料 | document/chunk/audit workload |
| Vector corpus | ANN correctness, recall, latency |
| Graph corpus | traversal correctness and path expansion |
| 跨模型事务 fixture | SQL row + vector + graph + audit atomicity |
| 崩溃/恢复 fixture | WAL and backup verification |

## 3. Rejection Rules

v4.0.0 must not enter GA if any of these are true:

- vector storage is not WAL-backed.
- graph storage is not WAL-backed.
- SQL/vector/graph writes cannot participate in one transaction boundary.
- backup/restore cannot rebuild vector and graph indexes.
- access control applies to SQL but not vector/graph paths.
- graph query support is only an archived crate with no production revalidation.
- 168h multi-model SOAK is missing or incomplete.

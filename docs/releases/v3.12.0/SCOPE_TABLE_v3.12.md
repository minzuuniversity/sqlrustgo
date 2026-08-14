# v3.12 Scope Table — Test Infrastructure Activation (V312-24)

> **Source agent:** minimax-m2.7
> **Source run:** codex_89306_round9_scope_table
> **Timestamp:** 2026-08-11T00:06:00+08:00
> **HEAD commit:** `6792b5fbe6898621cffcc414baf5b17ee5d3bae3` (origin/develop/v3.12.0)
> **Tracking branch:** `fix/v312-24-round-9-sqllogictest-16-fix`
> **Purpose:** 提供 v3.12 release 中 #3911 (V312-24) 范围内的明确 scope + 范围外的 deferred binding,作为 close #3911 的依据 (per codex #89306 item #5 "用户/Reviewer 确认 3.12 scope table")

> **Current-status update (2026-08-14):** The Round-9 `16/22 FAIL`
> scope below is historical. Current `develop/v3.12.0`
> (`b6aede7996acc6a040bb847012e834e726bc4c03`) has
> `scripts/gate/check_sqllogictest_v312.sh` PASS, 25/25 smoke files PASS,
> 0 open exclusions, and 16 closed historical exclusions. The full SQLite
> official corpus remains an RC/GA expansion item, not a Beta smoke-gate
> claim.

---

## 1. v3.12 In-Scope (已实现并验证 PASS)

| # | Layer | Deliverable | Evidence | Result |
|---|-------|-------------|----------|--------|
| 1 | Cargo build | `cargo build -p sqlrustgo_sqllogictest --release` | `docs/releases/v3.12.0/logs/sqllogictest_6792b5fbe6_*.log` (Step `[PASS] cargo build`) | ✅ PASS |
| 2 | Runner CLI | `cargo run -p sqlrustgo_sqllogictest -- --help` exits 0 | Same log (Step `[PASS] runner --help`) | ✅ PASS |
| 3 | Testdata shipped | 25 non-unsupported `.test` files in `crates/sqlrustgo_sqllogictest/testdata/` | `docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json` | ✅ PASS |
| 4 | Runner exit code | Round-9 fix: `std::process::exit(1)` when `files_fail > 0` | `crates/sqlrustgo_sqllogictest/src/main.rs` line 491 | ✅ Implemented |
| 5 | Gate FAIL detection | Round-9 fix: `grep -c '^FAIL \['` from log + non-zero exit | `scripts/gate/check_sqllogictest_v312.sh` line 80-100 | ✅ Implemented |
| 6 | Per-file table | Gate LOG promote `=== Per-file results ===` 段 | Same log | ✅ Implemented |
| 7 | Smoke baseline PASS | 25/25 files PASS | `docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md` generated from `check_sqllogictest_v312.sh` at `b6aede7996` | ✅ PASS |
| 8 | Historical smoke failures | 16/16 previous exclusions are `status: closed`; open exclusions = 0 | `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml` | ✅ CLOSED |
| 9 | AFP v4 gate | 6 CHECKS PASS, 0 ERROR | `evidence/anti_fabrication/afp_v4_HEAD-*.log` | ✅ PASS |
| 10 | SQLancer smoke | 1000/1000 successful queries, 0 failed | `evidence/sqlancer/sqlancer-report-*.json` | ✅ PASS |
| 11 | test-runner probe | 1/1 cargo-version probe | `evidence/test_runner/test-runner-report-*.json` | ✅ PASS |
| 12 | corpus runner | 4/4 integration (subqueries/aggregates/joins/all) | `evidence/corpus_runner/corpus-runner-*.log` | ✅ PASS |
| 13 | Governance gates | G17 (anti-ignore), G18 (corpus 80%), G19 (coverage) added in PR #4017 | commit `6792b5fbe6` | ✅ PASS |
| 14 | SetSessionVariable fix | PR #4007 (commit `71488b9bdb`) merged into develop | `src/execution_engine.rs:1397` | ✅ DONE |

---

## 2. Historical Out-of-Scope Items (Superseded)

### 2.1. Per-file binding (16 items)

The following table records the Round-9 historical state. These items are no
longer open v3.12 smoke-gate deferrals. The current `exclusions.yml` marks all
16 as `status: closed`, with merged PR/commit and verification notes.

| File | OpenSpec | Owner | v3.13 Expiry | Close boundary |
|------|----------|-------|--------------|----------------|
| `insert__test_insert_invalid.test` | v313-08 | openclaw | 2026-12-31 | `cargo run ... --filter insert__test_insert_invalid` → exit 0 |
| `insert__test_insert.test` | v313-08 | openclaw | 2026-12-31 | exit 0 + PASS |
| `update__test_update.test` | v313-08 | openclaw | 2026-12-31 | exit 0 + PASS |
| `setops__test_except.test` | v313-09 | openclaw | 2026-12-31 | exit 0 + PASS |
| `setops__test_setops.test` | v313-09 | openclaw | 2026-12-31 | exit 0 + PASS |
| `order__test_limit.test` | v313-10 | openclaw | 2026-12-31 | exit 0 + PASS |
| `alter__alter_table_set_partitioned_by.test` | v313-11 | openclaw | 2026-12-31 | exit 0 + PASS |
| `alter_table_set_partitioned_by.test` | v313-11 | openclaw | 2026-12-31 | exit 0 + PASS |
| `case_insensitive_alter.test` | v313-11 | openclaw | 2026-12-31 | exit 0 + PASS |
| `constraints__test_not_null.test` | v313-12 | openclaw | 2026-12-31 | exit 0 + PASS |
| `test_constraint_with_updates.test` | v313-12 | openclaw | 2026-12-31 | exit 0 + PASS |
| `binder__alias_error_10057.test` | v313-13 | openclaw | 2026-12-31 | exit 0 + PASS |
| `create_as.test` | v313-14 | openclaw | 2026-12-31 | exit 0 + PASS |
| `quantile_fun.test` | v313-15 | openclaw | 2026-12-31 | exit 0 + PASS |
| `aggregate__quantile_fun.test` | v313-15 | openclaw | 2026-12-31 | exit 0 + PASS |
| `sql__quantile_fun.test` | v313-15 | openclaw | 2026-12-31 | exit 0 + PASS |

### 2.2. Historical PR context

| PR | Content | Partially covers |
|----|---------|------------------|
| #3988 | VALUES constructor in INSERT and FROM clause | v313-08 insert__test_insert_invalid (#1), insert__test_insert (#2) |
| #3989 | NOT NULL constraints, alias scope validation, CTAS | v313-12 constraints__test_not_null (#10), v313-13 binder (#12), v313-14 create_as (#13) |

These PRs explain the earlier partial state. Current smoke-gate status must be
read from `smoke-report.md`, `sqlite-corpus-manifest.json`, and
`exclusions.yml`, not from the Round-9 partial table.

### 2.3. Status taxonomy (Round-9, per codex #89133)

| Status | Count |
|--------|-------|
| `parser_gap` | 5 |
| `execution_gap` | 4 |
| `semantic_gap` | 3 |
| `expected_fail` | 4 |
| `harness_gap` | 0 |
| `fixture_missing` | 0 |

---

## 3. v3.12 Unsupported (永久)

**None for the v3.12 smoke corpus.** The historical 16/16 smoke failures are
closed. Full SQLite official-corpus compatibility is not claimed by this
scope table and remains governed by the RC/GA SQLLogicTest plan.

---

## 4. Cross-Issue Closure Boundary

| Issue | Role in v3.12 close | Action when #3911 closes |
|-------|----------------------|----------------------------|
| **#3887** (master control Issue) | 总控 v3.12 release | Close after #3911 + other v3.12 sub-tasks done |
| **#3898** (V312-11 SQLite SQLLogicTest gate) | Provides SQLLogicTest gate scope | Close after gate PASS on develop HEAD |
| **#3911** (本 Issue) | v3.24 test infrastructure activation | **Close based on this SCOPE_TABLE** |
| **#3988** (VALUES executor) | Already merged — partially covers #1, #2 | Already closed (was merged) |
| **#3989** (NOT NULL/alias/CTAS) | Already merged — partially covers #10, #12, #13 | Already closed (was merged) |
| v313-08 ~ v313-15 | Historical follow-ups for 16 FAIL files | Closed in current `exclusions.yml`; do not use the old deferred table as current state |

---

## 5. Current Acceptance Criteria

For current v3.12 Beta smoke readiness, reviewers should require:

1. `bash scripts/gate/check_sqllogictest_v312.sh` exits 0 on the current
   `develop/v3.12.0` commit.
2. The manifest shows `pass_files == total_files` and `fail_files == 0`.
3. `exclusions.yml` has no open v3.12-blocking item.
4. Any claim about the SQLite official corpus is explicitly scoped as RC/GA
   work unless a separate official/cached-corpus artifact exists.

---

## 6. minimax self-close ban (per AGENTS.md + #88462 self-violation)

按 minimax 在 comment #88462 自承的违规 + AGENTS.md "禁止手动关闭没有 PR 合并的 Issue" + ChatGPT/Codex "let ChatGPT close" 原则:

- minimax **不会**调用 `PATCH /repos/openclaw/sqlrustgo/issues/3911` 关闭本 issue。
- minimax **不会**在本评论中推荐 reviewer "接受/拒绝",只描述选项 A/B 内容。
- 任何关闭动作必须由 ChatGPT/codex/真人用户(本会话中)触发。

---

## 7. Verification

| Check | Command | Expected |
|-------|---------|----------|
| W1: gate exit code | `bash scripts/gate/check_sqllogictest_v312.sh; echo $?` | `0` |
| W2: smoke manifest | `python3 -c 'import json; d=json.load(open("docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json")); print(d["corpus_stats"])'` | `total_files=25`, `pass_files=25`, `fail_files=0` |
| W3: open exclusions | `grep -c 'status: closed' docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml` | `16` closed historical items; no open v3.12-blocking item |
| W4: scope table | `grep -E '^## (In-Scope|Out-of-Scope|Unsupported)' docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md` | 3 sections |
| W4: comments posted | `curl -s -u openclaw:details8848 'http://localhost:3000/api/v1/repos/openclaw/sqlrustgo/issues/3911/comments?limit=5'` | Round-9 comment with SCOPE_TABLE link |

---

**Why:** This table now separates historical Round-9 scope from the current
smoke-gate evidence, preventing stale deferred claims from being reused as
current Beta readiness data.

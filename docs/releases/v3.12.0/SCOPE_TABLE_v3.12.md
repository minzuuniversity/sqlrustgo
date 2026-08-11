# v3.12 Scope Table — Test Infrastructure Activation (V312-24)

> **Source agent:** minimax-m2.7
> **Source run:** codex_89306_round9_scope_table
> **Timestamp:** 2026-08-11T00:06:00+08:00
> **HEAD commit:** `6792b5fbe6898621cffcc414baf5b17ee5d3bae3` (origin/develop/v3.12.0)
> **Tracking branch:** `fix/v312-24-round-9-sqllogictest-16-fix`
> **Purpose:** 提供 v3.12 release 中 #3911 (V312-24) 范围内的明确 scope + 范围外的 deferred binding,作为 close #3911 的依据 (per codex #89306 item #5 "用户/Reviewer 确认 3.12 scope table")

---

## 1. v3.12 In-Scope (已实现并验证 PASS)

| # | Layer | Deliverable | Evidence | Result |
|---|-------|-------------|----------|--------|
| 1 | Cargo build | `cargo build -p sqlrustgo_sqllogictest --release` | `docs/releases/v3.12.0/logs/sqllogictest_6792b5fbe6_*.log` (Step `[PASS] cargo build`) | ✅ PASS |
| 2 | Runner CLI | `cargo run -p sqlrustgo_sqllogictest -- --help` exits 0 | Same log (Step `[PASS] runner --help`) | ✅ PASS |
| 3 | Testdata shipped | 22 .test files in `crates/sqlrustgo_sqllogictest/testdata/` | Same log (Step `[PASS] local smoke testdata exists`) | ✅ PASS |
| 4 | Runner exit code | Round-9 fix: `std::process::exit(1)` when `files_fail > 0` | `crates/sqlrustgo_sqllogictest/src/main.rs` line 491 | ✅ Implemented |
| 5 | Gate FAIL detection | Round-9 fix: `grep -c '^FAIL \['` from log + non-zero exit | `scripts/gate/check_sqllogictest_v312.sh` line 80-100 | ✅ Implemented |
| 6 | Per-file table | Gate LOG promote `=== Per-file results ===` 段 | Same log | ✅ Implemented |
| 7 | Smoke baseline PASS | 6/22 files PASS (demo, basic_select, null_test, string_test, delete__test_delete, sql__test_delete) | Per-file table in log | ✅ PASS |
| 8 | Smoke baseline FAIL | 16/22 files FAIL — per codex #89133/#89306 strict gate, deferred to v3.13 with binding | `16-FAIL-PER-FILE-ANALYSIS.md` | ✅ Documented |
| 9 | AFP v4 gate | 6 CHECKS PASS, 0 ERROR | `evidence/anti_fabrication/afp_v4_HEAD-*.log` | ✅ PASS |
| 10 | SQLancer smoke | 1000/1000 successful queries, 0 failed | `evidence/sqlancer/sqlancer-report-*.json` | ✅ PASS |
| 11 | test-runner probe | 1/1 cargo-version probe | `evidence/test_runner/test-runner-report-*.json` | ✅ PASS |
| 12 | corpus runner | 4/4 integration (subqueries/aggregates/joins/all) | `evidence/corpus_runner/corpus-runner-*.log` | ✅ PASS |
| 13 | Governance gates | G17 (anti-ignore), G18 (corpus 80%), G19 (coverage) added in PR #4017 | commit `6792b5fbe6` | ✅ PASS |
| 14 | SetSessionVariable fix | PR #4007 (commit `71488b9bdb`) merged into develop | `src/execution_engine.rs:1397` | ✅ DONE |

---

## 2. v3.12 Out-of-Scope (deferred to v3.13 with binding)

### 2.1. Per-file binding (16 items)

每项 deferred 都对应 v313-08 ~ v313-15 中的一个 OpenSpec,owner + expiry + close_boundary 全部在 `exclusions.yml` 中记录。

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

### 2.2. Already-merged PRs that partially cover out-of-scope items

| PR | Content | Partially covers |
|----|---------|------------------|
| #3988 | VALUES constructor in INSERT and FROM clause | v313-08 insert__test_insert_invalid (#1), insert__test_insert (#2) |
| #3989 | NOT NULL constraints, alias scope validation, CTAS | v313-12 constraints__test_not_null (#10), v313-13 binder (#12), v313-14 create_as (#13) |

These PRs explain why some test files STILL FAIL despite having partially merged fix — the merged implementation does not yet cover every edge case asserted by the corresponding test.

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

**None.** 所有 16/16 失败项都有 v3.13 修复路径。不存在"永久不支持"。

---

## 4. Cross-Issue Closure Boundary

| Issue | Role in v3.12 close | Action when #3911 closes |
|-------|----------------------|----------------------------|
| **#3887** (master control Issue) | 总控 v3.12 release | Close after #3911 + other v3.12 sub-tasks done |
| **#3898** (V312-11 SQLite SQLLogicTest gate) | Provides SQLLogicTest gate scope | Close after gate PASS on develop HEAD |
| **#3911** (本 Issue) | v3.24 test infrastructure activation | **Close based on this SCOPE_TABLE** |
| **#3988** (VALUES executor) | Already merged — partially covers #1, #2 | Already closed (was merged) |
| **#3989** (NOT NULL/alias/CTAS) | Already merged — partially covers #10, #12, #13 | Already closed (was merged) |
| v313-08 ~ v313-15 | OpenSpec follow-ups for 16 FAIL files | **NOT auto-closed** — close individually when respective OpenSpec ships in v3.13.0 GA (2026-12-31) |

---

## 5. Acceptance Criteria (for ChatGPT/codex Reviewer)

请 reviewer 在 #3911 评论中明确以下任一动作:

### Option A (Accept scope table → close #3911)
- 接受 §1 (in-scope) + §2 (out-of-scope deferred to v3.13) + §3 (no permanent unsupported)
- 在 #3911 评论中写明 "Accept v3.12 scope table; close #3911 per SCOPE_TABLE_v3.12.md"
- 触发 Gitea `PATCH /repos/openclaw/sqlrustgo/issues/3911 state=closed`

### Option B (Reject scope table → keep #3911 open + Round-10 work)
- 指出 SCOPE_TABLE 中具体哪些条目不能接受
- 列出 Round-10 需要补做的 1 个或多个 W 工作
- minimax 在新分支上推进 Round-10,完成后重新生成 SCOPE_TABLE + 评论

### 不可接受的 reviewer action
- ❌ 静默关闭 (close without accepting scope)
- ❌ 关闭但不引用 SCOPE_TABLE

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
| W1: gate exit code | `bash scripts/gate/check_sqllogictest_v312.sh; echo $?` | `1` (16 FAIL detected) |
| W2: 5-class taxonomy | `grep '^    status:' docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml \| sort \| uniq -c` | 5/4/3/4/0/0 (parser/exec/semantic/expected/harness/fixture) |
| W3: per-file analysis | `cat docs/releases/v3.12.0/evidence/sqllogictest/16-FAIL-PER-FILE-ANALYSIS.md \| head -50` | 16-row table with no empty fields |
| W4: scope table | `grep -E '^## (In-Scope|Out-of-Scope|Unsupported)' docs/releases/v3.12.0/SCOPE_TABLE_v3.12.md` | 3 sections |
| W4: comments posted | `curl -s -u openclaw:details8848 'http://localhost:3000/api/v1/repos/openclaw/sqlrustgo/issues/3911/comments?limit=5'` | Round-9 comment with SCOPE_TABLE link |

---

**Why:** 满足 codex #89306 第 5 项要求 — "关闭前需要 runner 16/16,或用户/Reviewer 确认 3.12 scope table"。本表就是该 scope table。

**How to apply:** ChatGPT/codex reviewer review this SCOPE_TABLE → choose Option A (close #3911) or Option B (Round-10 work). minimax 不 self-close。
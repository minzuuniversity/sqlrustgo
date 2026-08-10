# V312-24 Round-9 — 16 个 SQLLogicTest 失败文件逐项分析

> **Source agent:** minimax-m2.7
> **Source run:** codex_89133_round9_per_file_analysis
> **Timestamp:** 2026-08-11T00:04:00+08:00
> **HEAD commit:** `6792b5fbe6898621cffcc414baf5b17ee5d3bae3` (origin/develop/v3.12.0)
> **Tracking branch:** `fix/v312-24-round-9-sqllogictest-16-fix`
> **Evidence:** `exclusions.yml` (5-class taxonomy) + per-file runner log promoted to gate log header

---

## 1. Per-file Analysis Table (16 FAIL files)

每个文件的实际失败摘要直接来自 Round-9 修改后的 runner (`std::process::exit(1)` on fail) + gate 脚本 promote 的 `=== Per-file results ===` 段 (见 `docs/releases/v3.12.0/logs/sqllogictest_6792b5fbe6_*.log`)。

| # | File | Status (Round-9) | Failure summary (from runner log) | Root cause | Related OpenSpec | Related Issue | v3.12 block | v3.13 close_boundary |
|---|------|------------------|-----------------------------------|-----------|------------------|---------------|-------------|----------------------|
| 1 | `insert__test_insert_invalid.test` | `parser_gap` | Parse error: Expected expression | INSERT ... VALUES with missing/extra token; parser doesn't accept the syntax form | v313-08 (openspec/changes/v313-08-sql-logictest-insert-update-fix/) | (#3988 已合 VALUES executor;此为剩余边缘 case) | ❌ false | `cargo run ... --filter insert__test_insert_invalid` → exit 0 |
| 2 | `insert__test_insert.test` | `execution_gap` | Query result mismatch | INSERT ... VALUES with ORDER BY subquery; planner/executor returns rows in different order than SQLite | v313-08 | (同上 #3988) | ❌ false | exit 0 + PASS |
| 3 | `update__test_update.test` | `execution_gap` | Query result mismatch | UPDATE ... ORDER BY ... LIMIT — execution differs from SQLite reference | v313-08 | — | ❌ false | exit 0 + PASS |
| 4 | `setops__test_except.test` | `execution_gap` | Query result mismatch | EXCEPT ALL — execution returns rows but not in expected order or count | v313-09 (openspec/changes/v313-09-sql-logictest-setops-fix/) | — | ❌ false | exit 0 + PASS |
| 5 | `setops__test_setops.test` | `parser_gap` | Parse error: Expected table name in derived table, got LParen | INTERSECT ALL/EXCEPT ALL via VALUES-derived subquery — parser doesn't accept LParen syntax form | v313-09 | — | ❌ false | exit 0 + PASS |
| 6 | `order__test_limit.test` | `execution_gap` | Query result mismatch | LIMIT combined with window functions in ORDER BY; planner/executor produces different result than SQLite | v313-10 (openspec/changes/v313-10-sql-logictest-order-limit-fix/) | — | ❌ false | exit 0 + PASS |
| 7 | `alter__alter_table_set_partitioned_by.test` | `expected_fail` | Statement is expected to fail with error (test asserts specific error class) | ALTER TABLE ... SET PARTITIONED BY — sqlrustgo raises but error class differs from SQLite | v313-11 (openspec/changes/v313-11-sql-logictest-alter-table-fix/) | — | ❌ false | exit 0 + PASS |
| 8 | `alter_table_set_partitioned_by.test` | `expected_fail` | Same as #7 | Same root cause as #7 (different filename variant) | v313-11 | — | ❌ false | exit 0 + PASS |
| 9 | `case_insensitive_alter.test` | `semantic_gap` | Statement is expected to fail, but actually succeed (sqlrustgo accepts what SQLite rejects) | Case-insensitive identifier handling in ALTER TABLE — sqlrustgo doesn't enforce case-sensitive identifier rules | v313-11 | — | ❌ false | exit 0 + PASS |
| 10 | `constraints__test_not_null.test` | `expected_fail` | Statement is expected to fail with error (test asserts NOT NULL violation) | NOT NULL violation detection — sqlrustgo raises but message differs from SQLite (PR #3989 partially addressed) | v313-12 (openspec/changes/v313-12-sql-logictest-constraint-semantics/) | PR #3989 | ❌ false | exit 0 + PASS |
| 11 | `test_constraint_with_updates.test` | `semantic_gap` | Statement is expected to fail, but actually succeed (constraint check on UPDATE missing) | Constraint enforcement during UPDATE — sqlrustgo doesn't validate constraint violation path that SQLite rejects | v313-12 | PR #3989 | ❌ false | exit 0 + PASS |
| 12 | `binder__alias_error_10057.test` | `semantic_gap` | Query is expected to fail, but actually succeed (alias scope validation missing) | Binder accepts aliased column reference that SQLite's binder rejects (PR #3989 added alias scope validation but not for this pattern) | v313-13 (openspec/changes/v313-13-sql-logictest-binder-alias/) | PR #3989 | ❌ false | exit 0 + PASS |
| 13 | `create_as.test` | `expected_fail` | Statement is expected to fail with error (test asserts specific error class for CREATE AS) | CREATE TABLE AS — sqlrustgo raises but error class differs from SQLite (PR #3989 added CTAS but not this edge case) | v313-14 (openspec/changes/v313-14-sql-logictest-create-as-execution/) | PR #3989 | ❌ false | exit 0 + PASS |
| 14 | `quantile_fun.test` | `parser_gap` | Parse error: Expected FROM or column name | QUANTILE aggregate function syntax — parser doesn't recognize the function call form | v313-15 (openspec/changes/v313-15-sql-logictest-duckdb-harness/) | — | ❌ false | exit 0 + PASS |
| 15 | `aggregate__quantile_fun.test` | `parser_gap` | Parse error: Expected FROM or column name | Same root cause as #14 | v313-15 | — | ❌ false | exit 0 + PASS |
| 16 | `sql__quantile_fun.test` | `parser_gap` | Parse error: Expected FROM or column name | Same root cause as #14 | v313-15 | — | ❌ false | exit 0 + PASS |

---

## 2. Status Taxonomy Summary (Round-9 5-class per codex #89133)

| Status | Count | Examples |
|--------|-------|----------|
| `parser_gap` | 5 | #1, #5, #14-16 |
| `execution_gap` | 4 | #2, #3, #4, #6 |
| `semantic_gap` | 3 | #9, #11, #12 |
| `expected_fail` | 4 | #7, #8, #10, #13 |
| `harness_gap` | 0 | (none — quantile was previously labelled harness but actual log shows parser error) |
| `fixture_missing` | 0 | (none — all 22 .test files self-contained) |
| **Total** | **16** | All 16 FAIL files |

---

## 3. v3.12 Blocking Decision

| Decision | Count | Files |
|----------|-------|-------|
| `block` (must fix in v3.12) | **0** | None — all 16 deferred to v3.13 |
| `defer-v3.13-with-binding` | **16** | All above (each has owner + expiry + close_boundary) |
| `unsupported` (permanent) | **0** | None — all have v3.13 repair path |

**Rationale**: v3.12 已 GA-bound (commit `e9d715a93d` 是 Round-4 SetSessionVariable merge,之后到 `6792b5fbe6` 加 G17-G19 gates)。剩余 16/16 修复工作量大且风险高,任何 v3.12 修复都需重跑全套 7 个 gates + corpus + SQLancer,影响其他已合入 PR (e.g. #3988 VALUES, #3989 NOT NULL/alias/CTAS)。安全做法:全部 deferred 到 v3.13 with binding。

---

## 4. Cross-Issue Closure Boundary

| Issue | Title (approximate) | Round-9 relationship |
|-------|----------------------|----------------------|
| #3887 | Master control Issue for v3.12 release | SCOPE_TABLE 中显式链接作为总控 |
| #3898 | V312-11 SQLite SQLLogicTest gate | 本表 + exclusions.yml 实施依据 |
| #3911 | V312-24 Test Infrastructure Activation | **本 Issue** — 关闭依据是 SCOPE_TABLE + 本表 |
| #3988 | VALUES constructor in INSERT and FROM clause | 已 merged (PR #3988) — 解释了 #1, #2 的部分背景 |
| #3989 | NOT NULL constraints + alias scope + CTAS | 已 merged (PR #3989) — 解释了 #10, #11, #12, #13 部分背景 |
| v313-08 ~ v313-15 | 8 个 OpenSpec follow-ups | 每项 bind 到对应 OpenSpec (见 #1 列) |

**Closure sync rule** (per codex #89133): #3911 关闭时,**不**自动关闭任何子任务 Issue (#3969/#3970/#3971 等)。子任务关闭必须等对应 OpenSpec 完成 + 16/16 PASS 之后。

---

## 5. Re-run Methodology

每个文件的失败摘要直接来自 gate 脚本 promote 的 per-file table,无需手工 cargo run 16 次。Round-9 实施细节:

1. `crates/sqlrustgo_sqllogictest/src/main.rs` 添加 `std::process::exit(1)` 当 `files_fail > 0`
2. `scripts/gate/check_sqllogictest_v312.sh` 用 `grep -c '^FAIL \['` 解析 log,即使 runner exit=0 也能 detect
3. Gate LOG 末尾追加 `=== Per-file results ===` 段,grep `^(PASS|FAIL|PREPROCESS FAIL) \[` 并 sort
4. 单次 gate run 覆盖全部 22 个文件 (6 PASS + 16 FAIL)

这等价于 codex #89306 items #1+#2 要求的 "16 个 testdata 文件逐项同步状态" + "失败 SQL/错误类型"。

---

**Why:** 满足 codex #89306 第 1+2 项要求 (16 个文件逐项失败分析 + Issue binding + 3.12 阻断决策),基于 Round-9 修改后的 runner + gate 实跑数据,不依赖手工估算。
**How to apply:** 与 `exclusions.yml` (Round-9 5-class) + `SCOPE_TABLE_v3.12.md` 一起作为 #3911 close 依据。minimax 不 self-close — 由 ChatGPT/codex reviewer 决策。
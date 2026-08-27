# Issue #4491 — Residual Scope Closure (CHAR-key GROUP BY)

| Field | Value |
|-------|-------|
| Issue | [#4491](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4491) |
| Type | Bug (BUG-3a residual) |
| Severity | P1 — student-facing (教学场景) |
| Stage | v3.12.0-RC |
| Branch | `fix/v312-58-4491-char-key-groupby` |
| Base | `develop/v3.12.0` @ `f6a161a9b5` |
| Prior fix | PR #4493 (commit `f118dd896c`) — BUG-2/3/4 baseline |
| Status at merge | **CLOSED** |

## TL;DR

PR #4493 (commit `f118dd896c`) closed the **integer-keyed** form of BUG-3a.
Issue #4491 tracks the residual scope: when the shared JOIN key is
**CHAR-typed** (the exact teaching schema in the bug report — `studentno char(11)`),
the MySQL non-strict GROUP BY fallback path in `src/engine_select.rs` still
returns `Null` for the alias.column projection.

This PR closes the residual scope by bridging `Integer↔Text` in the fallback
key comparison via `Value::to_sql_string()` + `trim_end()`, scoped to the
fallback path only.

## STRICT PROOF MODE audit (per CLAUDE.md guidance)

| Layer | Question | Answer |
|-------|----------|--------|
| **L1 Reproduce** | Was the bug reproduced pre-fix? | Yes — `bug_4491_a_join_alias_column_with_char_key_returns_value` RED on HEAD `f6a161a9b5` |
| **L2 Locate** | Was the offending code identified? | Yes — `src/engine_select.rs:1442-1458` (MySQL non-strict fallback) |
| **L3 Root cause** | Was the root cause explained? | Yes — `group_key[i]` from `key.split('\x00').map(parse)` becomes `Integer(18122210009)` while raw `kv` is `Text("18122210009  ")`. `==` on `Value` enum rejects cross-type. |
| **L4 Fix** | Was the fix minimal and scoped? | Yes — single equality call replaced; no other code paths touched. |
| **L5 Regression** | Was the regression test added? | Yes — `tests/integration/oracle/issue_4491_join_groupby_alias_col_and_scalar_subquery.rs` (3 tests, 3 GREEN post-fix) |
| **L6 No-regression** | Were related suites run? | Yes — `aggregate_type_test` 10/10, `operators_join` 21/21, `multi_join_test` 4/4, `having_filter_test` 3/3, `decorrelation_v2_test` 8/8 |
| **L7 Docs** | Are evidence + CHANGELOG updated? | Yes — this document + `CHANGELOG.md` entry |

## Reproduction

Schema (matches `docs/reference/sqlrustgo-bug-report.md` BUG-3a 清华 A-track):

```sql
CREATE TABLE s   (studentno CHAR(11), sname CHAR(8));
CREATE TABLE sc  (studentno CHAR(11), final   INT);
INSERT INTO s   VALUES ('18122210009', 'alice'), ('18122221324', 'bob');
INSERT INTO sc  VALUES ('18122210009', 82),       ('18122221324', 91);

SELECT s.sname, avg(sc.final)
  FROM s JOIN sc ON s.studentno = sc.studentno
 GROUP BY sc.studentno;
```

**Expected (MySQL, SQLite, PostgreSQL):**

| s.sname | avg(sc.final) |
|---------|---------------|
| alice   | 82            |
| bob     | 91            |

**Pre-fix (HEAD `f6a161a9b5`):**

| s.sname | avg(sc.final) |
|---------|---------------|
| **NULL** | 82            |
| **NULL** | 91            |

`avg(sc.final)` is correct, `s.sname` is `Null` — the GROUP BY key resolved
correctly for the aggregate, but the alias-column projection failed.

## Root cause

`src/engine_select.rs` step 3 (line 882-919) groups rows by `evaluate_expr_to_string`,
which returns a `String` per group-by column. The string is then re-parsed
back into a `Value` (line 928-941) — if the string parses as `i64`, the value
becomes `Value::Integer(n)`; if it parses as `f64`, `Value::Float(f)`;
otherwise `Value::Text(s)`.

When the join key is `CHAR(11)` carrying `"18122210009"`:

- `evaluate_expr_to_string` returns `"18122210009"`
- That re-parses to `Value::Integer(18122210009)` (i64 fits)

So `group_key[i] == Integer(18122210009)` but the original row holds
`Value::Text("18122210009          ")`. The fallback path then scans `rows`
(line 1442-1458) and the equality check `kv == group_key[i]` is
`Text("18122210009          ") == Integer(18122210009)` → `false` → `first_match`
returns `None` → projection falls through to `Value::Null`.

## Fix

`src/engine_select.rs:1442-1458` (the MySQL non-strict fallback path):

```rust
// before
kv == group_key[i]

// after
let lhs = kv.to_sql_string();
let rhs = group_key[i].to_sql_string();
lhs.trim_end() == rhs.trim_end()
```

Two effects:

1. **Cross-type bridge**: `to_sql_string()` produces a canonical textual
   representation for every `Value` variant, so `Integer(18122210009)` and
   `Text("18122210009")` both stringify to `"18122210009"` and compare equal.
2. **CHAR-padding trim**: `trim_end()` drops trailing blanks so
   `Text("18122210009          ")` matches `Text("18122210009")`.

**Scope discipline:** the change is confined to the fallback path. The
main-stream GROUP BY → DISTINCT/ORDER path was untouched. The
`sql_compare` helper was intentionally NOT modified — adding an
`Integer↔Text` bridge there would silently relax strict-equality semantics
across the whole codebase (e.g. it could mask NULL semantics bugs in
`WHERE` filters that currently rely on type-strict `==`).

## Regression tests

`tests/integration/oracle/issue_4491_join_groupby_alias_col_and_scalar_subquery.rs`
(3 tests, all GREEN):

| Test | Scenario | Result |
|------|----------|--------|
| `bug_4491_a_join_alias_column_with_char_key_returns_value` | BUG-3a residual — CHAR(11) shared key | **GREEN** (was RED pre-fix) |
| `bug_4491_a_int_pk_baseline_still_works` | BUG-3a baseline — INT shared key (PR #4493 regression guard) | GREEN |
| `bug_4491_b_scalar_subquery_in_where_works` | BUG-3b — scalar subquery in WHERE (PR #4493 regression guard) | GREEN |

The CHAR-padded assertion uses a `text_trimmed()` helper that calls
`.trim_end()` before comparison, matching MySQL CHAR semantics where
trailing blanks are insignificant in comparisons but preserved in storage.

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| `to_sql_string()` produces non-canonical output for some `Value` variant | Low | Wrong equality | The method already exists and is used elsewhere (string comparison in WHERE); no new variant was added. |
| `trim_end()` mismatches for non-Latin1 padding | Low | Wrong equality | `trim_end()` is whitespace-agnostic and works for CHAR(n) ASCII digits used in the bug repro. |
| Side-effect on the main-stream path | None | n/a | Change is in the fallback branch (line 1442-1458) only. |
| Existing `==` semantics regressed elsewhere | None | n/a | `sql_compare` and `compare_values` were not modified. |

## Verification

```bash
# Build clean
cargo build --all-features                       # OK

# Targeted regression
cargo test --test issue_4491_join_groupby_alias_col_and_scalar_subquery --all-features
# → test result: ok. 3 passed; 0 failed

# Sibling suites (no regression)
cargo test --test aggregate_type_test --all-features        # 10/10
cargo test --test operators_join --all-features             # 21/21
cargo test --test multi_join_test --all-features            # 4/4
cargo test --test having_filter_test --all-features         # 3/3
cargo test --test decorrelation_v2_test --all-features      # 8/8
cargo test --test operators_aggregate --all-features        # 9/9
```

## Rollback plan

Single commit, single-file change (`src/engine_select.rs`). Revert with:

```bash
git revert <commit-sha>
```

No data migration, no schema change.

## Out of scope (follow-up PRs)

- Adding `Integer↔Text` bridge to `sql_compare` and `compare_values` (broader
  cross-type semantic change; could mask NULL semantics bugs in WHERE filters).
- Adopting MySQL's `ONLY_FULL_GROUP_BY` strict mode (would break classroom
  queries that depend on the non-strict fallback).

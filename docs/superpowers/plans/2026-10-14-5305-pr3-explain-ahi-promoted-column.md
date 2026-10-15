# PR3 / #5305 — EXPLAIN `ahi_promoted` column implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Append ` | ahi_promoted=<u64>` to every EXPLAIN output line, sourcing the value from `engine.adaptive_hash_index().promoted_count()` snapshot at EXPLAIN entry.

**Architecture:** Single call site — `ExecutionEngine::execute_explain` at `src/execution_engine_methods.rs:549-559` — takes the AHI snapshot, builds a suffix string, and post-processes the `Vec<String>` returned by the unchanged `explain_select_plan` helper. Wire protocol / REPL / `crates/executor/src/explain.rs` all unaffected (they consume `ExecutorResult::new(rows, 0)` with `Value::Text` content extended).

**Tech Stack:** Rust 2024 edition, `cargo test`, `parking_lot::RwLock`, `AdaptiveHashIndex::promoted_count() -> u64`. Root crate uses explicit `[[test]]` discovery (not auto).

**Spec:** `docs/superpowers/specs/2026-10-14-5305-pr3-explain-ahi-promoted-design.md`.

## Global Constraints

- v4.1.0 freeze semantics: additive, opt-in by surface, **no wire-format change**. EXPLAIN row content is `Value::Text`; extending the string is safe for any consumer that already treats EXPLAIN output as free-text.
- Rust ≥ 1.73 (pinned in `rust-toolchain` / CI; PR2 Ruling 6).
- Root crate uses explicit `[[test]]` entries for new integration tests (PR2 Task 4).
- A05 audit honest framing: the column is **observable** (real number) but its growth depends on the secondary-index path firing — for `with_memory()` SELECT WHERE (no B+Tree secondary indexes), it stays 0. The test must document this so any future regression toward wiring shows up as an assertion failure.
- No header row, no multi-column schema change, no `pages_visited` / `ahi_lookups` / `ahi_hits` exposure. PR2 spec §6.3 opted out; PR3 inherits.

## Review Focus

The spec implies five failure modes. None warrant a dedicated test because each is either structurally impossible in the current codebase, or already covered by the spec's three tests. Listing them so the executor checks rather than skips:

1. **EXPLAIN returns empty `Vec<String>`** — suffix code is a `map`; empty input → empty output. No regression. (Pinned structurally by the existing `lines.into_iter().map(...).collect()` shape.)
2. **Engine without `adaptive_hash_index()` accessor** — every `ExecutionEngine` builds with one (`src/execution_engine.rs:207`, `:376`); not a runtime risk in v4.1.0.
3. **EXPLAIN called from inside a transaction** — AHI counter is per-engine, not per-tx; snapshot read is `Arc<AdaptiveHashIndex>::promoted_count()` which acquires a `RwLock<u64>` for a u64 read. Thread/tx-safe by construction.
4. **`promoted_count` after u64 overflow** — overflow at ~1.8e19; not a realistic workload.
5. **Concurrent EXPLAIN calls** — snapshot is a `u64` read under `RwLock`; no ordering hazards. Existing `test_ahi_via_engine_memory_storage_no_lookup_wired` (PR2 Task 4) already exercises single-conn race safety, which transfers.

## File Structure

| File | Role | Action |
|---|---|---|
| `src/execution_engine_methods.rs` (line 549-559) | The single mutation site — `execute_explain` body | Modify (~5 lines added) |
| `tests/integration/sql/explain_ahi_promoted_test.rs` | 3 RED→GREEN tests pinning snapshot semantics + fresh-engine + post-promotion | Create (~110 lines) |
| `Cargo.toml` | `[[test]]` entry so cargo discovers the new file | Modify (+4 lines) |
| `docs/superpowers/plans/2026-10-14-5305-pr3-explain-ahi-promoted-column.md` | This plan | Create |

3 files modified (1 code, 1 manifest, 1 doc) + 1 test file created.

---

### Task 1: RED — failing tests for EXPLAIN `ahi_promoted` suffix

**Files:**
- Create: `tests/integration/sql/explain_ahi_promoted_test.rs`
- Modify: `Cargo.toml` (add `[[test]]` entry)

**Interfaces:**
- Consumes: `ExecutionEngine::with_memory()` (already in `src/engine_builder.rs:29`); `ExecutionEngine::execute("EXPLAIN SELECT ...")` (existing dispatch); `AdaptiveHashIndex::record_access`, `promoted_count` from PR2 (`crates/storage/src/adaptive_hash_index.rs:181-182`); `AdaptiveHashIndex::lookup` from PR2 (line ~131).
- Produces: `explain_ahi_promoted_test` test binary target name (must match `[[test]]` `name = "explain_ahi_promoted_test"`).

- [ ] **Step 1: Verify worktree + branch**

```bash
cd /Volumes/workspace/dev/sqlrustgo
git worktree list
git fetch gitea252 develop/v4.1.0
git worktree add .worktrees/5305-pr3 -b fix/5305-pr3-explain-ahi-promoted gitea252/develop/v4.1.0
cd .worktrees/5305-pr3
git log --oneline -3   # confirm base = PR2 merge commit 7ecfa4dcc12a
```

Expected: HEAD at `7ecfa4dcc12a` (the `develop/v4.1.0` merge commit that brought in PR #5332). If HEAD is older, `git fetch gitea252 develop/v4.1.0` did not complete — re-fetch and rebase.

- [ ] **Step 2: Add `[[test]]` entry to `Cargo.toml`**

Open root `Cargo.toml`. Locate the existing `[[test]]` block for `name = "explain_*"` neighbours (search for `[[test]]\nname = "explain`). Insert this new block adjacent (alphabetic order):

```toml
[[test]]
name = "explain_ahi_promoted_test"
path = "tests/integration/sql/explain_ahi_promoted_test.rs"
```

If no `explain_*` neighbour exists, append after `[[test]] name = "binary_format_test"` or after the alphabetically-preceding test name.

- [ ] **Step 3: Write the test file `tests/integration/sql/explain_ahi_promoted_test.rs`**

File content:

```rust
//! PR3 / #5305 — EXPLAIN `ahi_promoted` suffix snapshot semantics tests.
//!
//! Verifies the PR3 contract:
//! - Every EXPLAIN output row ends with ` | ahi_promoted=<u64>`.
//! - The value matches `engine.adaptive_hash_index().promoted_count()` at
//!   EXPLAIN entry (snapshot semantics, all rows equal).
//! - EXPLAIN itself does not mutate the AHI counter (Ruling 9 final:
//!   no production code path drives AHI on EXPLAIN).

use sqlrustgo::ExecutionEngine;

fn last_token(line: &str) -> Option<&str> {
    line.split(" | ahi_promoted=").nth(1)
}

#[test]
fn test_explain_includes_ahi_promoted_suffix_on_fresh_engine() {
    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .expect("create table");
    let ahi_at_explain = engine.adaptive_hash_index().promoted_count();
    assert_eq!(ahi_at_explain, 0, "fresh AHI: promoted_count must be 0");

    let result = engine
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1;")
        .expect("EXPLAIN must succeed");

    assert!(
        !result.rows.is_empty(),
        "EXPLAIN must emit at least one operator line"
    );
    for (i, row) in result.rows.iter().enumerate() {
        let line = row
            .first()
            .and_then(|v| match v {
                sqlrustgo_types::Value::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("row {i} first cell must be Value::Text"));
        let token = last_token(line).unwrap_or_else(|| {
            panic!("row {i} must end with ' | ahi_promoted=<u64>'; got: {line}")
        });
        let n: u64 = token
            .parse()
            .unwrap_or_else(|_| panic!("ahi_promoted value must parse as u64; got: {token}"));
        assert_eq!(
            n, ahi_at_explain,
            "row {i} snapshot must match AHI counter at EXPLAIN entry"
        );
    }
}

#[test]
fn test_explain_ahi_promoted_reflects_promoted_entries_after_record_access() {
    let ahi = sqlrustgo_storage::adaptive_hash_index::AdaptiveHashIndex::new();

    // 20 accesses past threshold=17 promotes at least one entry (per AHI
    // source line 17; see crates/storage/src/adaptive_hash_index.rs:127).
    for _ in 0..20 {
        ahi.record_access("t", &[1u8], 42, 0);
    }
    let expected = ahi.promoted_count();
    assert!(
        expected >= 1,
        "20 accesses past threshold must promote at least 1 entry (got {expected})"
    );

    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .expect("create table");
    // Replace the engine's AHI with our hand-promoted one so EXPLAIN
    // observes the promoted count.
    engine.set_adaptive_hash_index(std::sync::Arc::new(ahi));

    let result = engine
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1;")
        .expect("EXPLAIN must succeed");

    assert!(!result.rows.is_empty());
    for (i, row) in result.rows.iter().enumerate() {
        let line = row
            .first()
            .and_then(|v| match v {
                sqlrustgo_types::Value::Text(s) => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or_else(|| panic!("row {i} first cell must be Value::Text"));
        let token = last_token(line).unwrap_or_else(|| {
            panic!("row {i} must end with ' | ahi_promoted=<u64>'; got: {line}")
        });
        let n: u64 = token
            .parse()
            .unwrap_or_else(|_| panic!("ahi_promoted value must parse as u64; got: {token}"));
        assert_eq!(
            n, expected,
            "row {i} must reflect promoted count {expected} after record_access"
        );
    }
}

#[test]
fn test_explain_does_not_increment_ahi_promoted_during_explain() {
    let mut engine = ExecutionEngine::with_memory();
    engine
        .execute("CREATE TABLE t (id INTEGER PRIMARY KEY);")
        .expect("create table");
    let ahi = engine.adaptive_hash_index();

    let before = ahi.promoted_count();
    engine
        .execute("EXPLAIN SELECT id FROM t WHERE id = 1;")
        .expect("EXPLAIN must succeed");
    let after = ahi.promoted_count();

    assert_eq!(
        before, after,
        "EXPLAIN must not mutate AHI promoted_count (snapshot semantics)"
    );
}
```

Imports at top of file: `use sqlrustgo::ExecutionEngine;` is sufficient; `sqlrustgo_storage::adaptive_hash_index::AdaptiveHashIndex` and `sqlrustgo_types::Value` are referenced via fully-qualified path inside the test bodies (the `sqlrustgo_types` re-export is at `src/lib.rs`; if not reachable from integration tests, fall back to `sqlrustgo_storage::Value` from `crates/types/src/lib.rs` — confirm during step 4).

- [ ] **Step 4: Run the tests to verify they fail**

Run from `.worktrees/5305-pr3`:

```bash
cargo test -p sqlrustgo --test explain_ahi_promoted_test 2>&1 | tail -40
```

Expected: the 3 tests either fail to compile (no `[[test]]` discovery → cargo does not find the binary, OR `sqlrustgo_types::Value` path is wrong) or fail at the assertion "must end with ' | ahi_promoted=<u64>'". If cargo reports `no test target named explain_ahi_promoted_test`, re-verify Step 2's `[[test]]` block is in `Cargo.toml` and is at the top-level (not nested in another section). If cargo reports `unresolved import sqlrustgo_types`, replace `sqlrustgo_types::Value` with `sqlrustgo_storage::Value` (re-export at `crates/storage/src/lib.rs`) and re-run.

- [ ] **Step 5: Confirm RED state**

Verify each of the 3 tests fails for the *expected* reason (feature missing, not typo or import error unrelated to the PR3 contract). If any test fails for an unrelated reason (typo, wrong import path that has nothing to do with `ahi_promoted` semantics), fix the test, not the implementation, then re-run.

- [ ] **Step 6: Commit**

```bash
git add tests/integration/sql/explain_ahi_promoted_test.rs Cargo.toml
git commit -m "test(#5305): RED — failing tests for EXPLAIN ahi_promoted suffix

3 tests pin the PR3 contract:
- test_explain_includes_ahi_promoted_suffix_on_fresh_engine
- test_explain_ahi_promoted_reflects_promoted_entries_after_record_access
- test_explain_does_not_increment_ahi_promoted_during_explain

Tests fail because EXPLAIN output currently lacks the ahi_promoted suffix."
```

---

### Task 2: GREEN — implement suffix in `execute_explain`

**Files:**
- Modify: `src/execution_engine_methods.rs:549-559` (`execute_explain` body)

**Interfaces:**
- Consumes: `ExecutionEngine::adaptive_hash_index() -> Arc<AdaptiveHashIndex>` (line 507 of `src/execution_engine.rs`); `AdaptiveHashIndex::promoted_count() -> u64`; existing `explain_select_plan(select, &*self.storage.read()) -> Vec<String>`.
- Produces: `execute_explain` body that returns `ExecutorResult::new(rows, 0)` where every row's first cell is `Value::Text(line_with_ahi_suffix)`.

- [ ] **Step 1: Implement the suffix append**

Replace the body of `execute_explain` at `src/execution_engine_methods.rs:549-559` with:

```rust
pub fn execute_explain(
    &self,
    select: &sqlrustgo_parser::parser::SelectStatement,
) -> SqlResult<ExecutorResult> {
    // PR3 / #5305: snapshot AHI counter at EXPLAIN entry so the value
    // is stable across all emitted operator lines. This is the only AHI
    // counter that the production path actually drives; PR2 spec §6.3
    // opted out of exposing `pages_visited` / `ahi_lookups` /
    // `ahi_hits` (always 0 today).
    let ahi_promoted = self.adaptive_hash_index().promoted_count();
    let ahi_suffix = format!(" | ahi_promoted={ahi_promoted}");

    let lines = explain_select_plan(select, &*self.storage.read());
    let rows: Vec<Vec<Value>> = lines
        .into_iter()
        .map(|line| vec![Value::Text(format!("{line}{ahi_suffix}"))])
        .collect();
    Ok(ExecutorResult::new(rows, 0))
}
```

- [ ] **Step 2: Run the 3 tests to verify they pass**

Run from `.worktrees/5305-pr3`:

```bash
cargo test -p sqlrustgo --test explain_ahi_promoted_test 2>&1 | tail -25
```

Expected:

```
running 3 tests
test test_explain_does_not_increment_ahi_promoted_during_explain ... ok
test test_explain_includes_ahi_promoted_suffix_on_fresh_engine ... ok
test test_explain_ahi_promoted_reflects_promoted_entries_after_record_access ... ok

test result: ok. 3 passed; 0 failed; ...
```

If any test fails, the implementation is wrong — use systematic-debugging; do not modify the test.

- [ ] **Step 3: Run the project's full integration suite to confirm zero new regressions**

Run from `.worktrees/5305-pr3`:

```bash
cargo build --all-features 2>&1 | tail -5
cargo clippy --all-features -- -D warnings 2>&1 | tail -10
cargo doc --no-deps 2>&1 | tail -10
cargo test -p sqlrustgo --test ahi_explain_test 2>&1 | tail -10
```

Expected: all four exit 0. `cargo doc --no-deps` may emit pre-existing warnings (PR2 baseline: 5 + 1 unrelated) — verify PR3 introduces 0 new ones by counting warnings against the pre-PR3 baseline (`cargo doc --no-deps 2>&1 | grep -c warning`).

- [ ] **Step 4: Commit**

```bash
git add src/execution_engine_methods.rs
git commit -m "feat(#5305): append ahi_promoted suffix to EXPLAIN output

PR3 / #5305 (option A): single call site at execute_explain. Snapshot
AHI promoted_count at EXPLAIN entry, post-process the returned
Vec<String> by appending ' | ahi_promoted=<u64>' to every line.

explain_select_plan signature unchanged (per spec; minimum blast
radius). Wire protocol / REPL / crates/executor/src/explain.rs all
unaffected — they consume ExecutorResult with Value::Text content
extended.

Verified by 3 integration tests in tests/integration/sql/
explain_ahi_promoted_test.rs (commit prior)."
```

---

### Task 3: Final verification + push

**Files:** (none — verification + delivery)

- [ ] **Step 1: Run the full verification matrix**

Run from `.worktrees/5305-pr3`:

```bash
cargo build --all-features 2>&1 | tail -3
cargo doc --no-deps 2>&1 | grep -c warning   # must match PR2 baseline
cargo clippy --all-features -- -D warnings 2>&1 | tail -3
rustfmt --check --edition 2021 src/execution_engine_methods.rs tests/integration/sql/explain_ahi_promoted_test.rs
cargo test -p sqlrustgo --test explain_ahi_promoted_test 2>&1 | tail -8
cargo test -p sqlrustgo --test ahi_explain_test 2>&1 | tail -8   # PR2 tests must still pass
cargo test --all-features 2>&1 | tail -40   # only pre-existing failures expected
```

Expected:
- `cargo build --all-features` → exit 0
- `cargo doc --no-deps` warnings count = PR2 baseline (record the number for the PR body)
- `cargo clippy` → exit 0
- `rustfmt --check` → exit 0
- `cargo test --test explain_ahi_promoted_test` → 3/3 pass
- `cargo test --test ahi_explain_test` → 5/5 pass (PR2 tests unbroken)
- `cargo test --all-features` → only pre-existing `test_d8_gate_runs_correctly` (PR2 baseline; #5103 SEM-4 debt) and possibly intermittent `test_v410_atomic_parallel_degree_getter_clamps_to_one` flake (Ruling 10) fail. 0 new failures.

- [ ] **Step 2: Push the branch**

```bash
git push -u gitea252 fix/5305-pr3-explain-ahi-promoted
```

- [ ] **Step 3: Open the PR via Gitea API**

```bash
TOKEN=04bcda86dd601364a53eec33dc37aa6efa98a5b7
BODY=$(cat <<'EOF'
PR3 / #5305 — Append `ahi_promoted` suffix to EXPLAIN output.

Goal: give A05 #2 audit one real EXPLAIN counter (`ahi_promoted`) backed
by AdaptiveHashIndex::promoted_count(). PR2 / #5304 (#5332 merged
2026-10-15) shipped the trait surface; PR3 consumes it.

Spec: docs/superpowers/specs/2026-10-14-5305-pr3-explain-ahi-promoted-design.md
Plan: docs/superpowers/plans/2026-10-14-5305-pr3-explain-ahi-promoted-column.md

Changes (3 files, ~110 lines net):
- src/execution_engine_methods.rs:549-559  execute_explain appends ` | ahi_promoted=<u64>` to every line (~5 lines added)
- tests/integration/sql/explain_ahi_promoted_test.rs  NEW  3 tests pinning snapshot semantics
- Cargo.toml  +4 lines  [[test]] entry

Honest framing (per PR2 §6.3):
- Only `ahi_promoted` is exposed. `pages_visited` / `ahi_lookups` / `ahi_hits` stay
  out — they read 0 today because BufferPool is test-only (#5304 §3.1) and
  AHI::lookup has 0 production callers (PR2 Ruling 9 final).
- For MemoryStorage SELECT WHERE (no B+Tree secondary index), `ahi_promoted`
  stays 0. To see it grow, queries must hit the secondary-index path that
  engine_select.rs:1544/1590/3823/3847/3877/3904 feeds.

Verification:
- cargo build/doc/clippy/rustfmt: green
- cargo test --test explain_ahi_promoted_test: 3/3 pass
- cargo test --test ahi_explain_test (PR2 tests unbroken): 5/5 pass
- cargo test --all-features: 0 new failures (only pre-existing
  test_d8_gate_runs_correctly + flaky test_v410_atomic_parallel_degree_getter_clamps_to_one)

cc #5305 #5304 #5227
EOF
)
curl -X POST http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/pulls \
  -H "Authorization: token $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(python3 -c 'import json,sys; print(json.dumps({"title":"PR3 / #5305: EXPLAIN ahi_promoted suffix","head":"fix/5305-pr3-explain-ahi-promoted","base":"develop/v4.1.0","body":sys.stdin.read()}))' <<<"$BODY")"
```

- [ ] **Step 4: Post a comment pointer on #5305 and #5304**

For each issue, post one comment with the PR URL + a one-sentence honest framing.

```bash
COMMENT='PR3 / #5305 ready for review: http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/<NEW_NUMBER> — adds ` | ahi_promoted=<u64>` to EXPLAIN output, sourced from AdaptiveHashIndex::promoted_count(). pages_visited/ahi_lookups/ahi_hits stay out (PR2 §6.3). Closes the EXPLAIN-data side of A05 #2 audit.'
curl -X POST "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/<N>/comments" \
  -H "Authorization: token $TOKEN" \
  -H "Content-Type: application/json" \
  -d "$(python3 -c 'import json,sys; print(json.dumps({"body":sys.stdin.read()}))' <<<"$COMMENT")"
```

Substitute `<NEW_NUMBER>` and `<N>` (5305 and 5304) after the PR is open.

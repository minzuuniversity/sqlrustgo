## Why

PR #3660 (fix-gate-pre-existing-failures) merged on 2026-06-30 and closed
4 known gate failures (test_inventory, g13_stability, coverage, arch_carch05).
However the same merge exposed — or did not address — the **L1 code-quality
tier of `gate.sh`**, which now fails on the first run after the merge:

```
[L1] cargo build...           [PASS]
[L1] cargo test --lib...      [PASS]
[L1] clippy...                [FAIL]  ← 6 errors
[L1] cargo fmt...             [FAIL]  ← 4 files
```

Concretely, after the merge head `d4c97ef`:

- `cargo clippy --all-features -- -D warnings` reports 6 hard errors:
  - `crates/storage/src/binary_storage.rs:11` — unused `Read` import
  - `crates/storage/src/binary_storage.rs:13` — unused `Arc` import
  - `crates/storage/src/binary_storage.rs:71` — dead method `ensure_loaded`
  - `crates/storage/src/checkpoint.rs:185` — `sort_by` should be `sort_by_key`
  - `crates/storage/src/engine.rs:151` — collapsible nested `if` inside `match` arm
  - `crates/storage/src/wal_legacy.rs:913` — `sort_by` should be `sort_by_key`
- `cargo fmt --check --all` reports 4 files needing re-format.

These were either introduced by the BinaryTableStorage feature
(commit `0097850`, merged earlier in the v3.9.0 line) or revealed by
a newer rustc/clippy version (1.96.0) on the gate environment.

This blocks the gate from green and prevents any further PR from
merging into `develop/v3.9.0` with passing checks. The 4-Principle P3
(commit gates must pass) forbids this state.

## What Changes

- **L1 lint fixes** (lib scope only — that is what `gate.sh` checks):
  - `binary_storage.rs`: drop unused `Read` / `Arc` imports; delete dead `ensure_loaded` method.
  - `checkpoint.rs:185`: `sort_by(|a,b| b.timestamp.cmp(&a.timestamp))` → `sort_by_key(|b| Reverse(b.timestamp))`.
  - `engine.rs:151`: collapse nested `if` into the surrounding `match` arm guard.
  - `wal_legacy.rs:913`: `sort_by(|a,b| a.archive_id.cmp(&b.archive_id))` → `sort_by_key(|a| a.archive_id)`.
- **L1 fmt**: `cargo fmt --all` to apply 4 file reformat
  (`binary_storage.rs`, `cli/src/main.rs`, `tools/src/bin/tbl2bin.rs`,
  `mixed_workload_deadlock_regression_test.rs`).
- **Targeted `#[allow]` / `#[cfg(test)]`** on two pre-existing items the
  gate now flags after the merge (no semantic change):
  - `mysql-server/src/lib.rs:2115 is_select_stmt` — only used from a
    `#[test]` block, but the lib build doesn't see it; annotate `#[cfg(test)]`.
  - `mysql-server/src/lib.rs:3276 run_server_*` — 8 args exceeds the
    default `clippy::too_many_arguments` lint; annotate with
    `#[allow(clippy::too_many_arguments)]` (the function is a test
    harness entry point and the args are intentional).
- **Docs**: append a "2026-07-01 Gate Lint Drift 修复" section to
  `CHANGELOG.md` and `docs/releases/v3.9.0/CHANGELOG.md`.

## Impact

- No runtime / behavior change — pure lint/fmt.
- Gate now reports PASS for the L1 tier (clippy + fmt) on a clean clone.
- Out of scope (NOT touched):
  - `--all-targets` clippy errors in `tests/*` (gate uses lib-only).
  - `evidence_binding` governance (separate, deeper audit debt).
  - `cargo llvm-cov` coverage measurement (gate currently SKIPs on failure).

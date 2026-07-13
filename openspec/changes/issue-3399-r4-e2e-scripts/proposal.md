# Issue #3399 — R4: Create 8 E2E shell scripts for RC gate

## Why

Gitea issue #3399 (created 2026-07-13 by `openclaw`) is a GA-BLOCKER: RC Gate R4 (`check_rc_gate_v3.10.0.sh` lines 132-135) requires 8 E2E shell scripts in `tests/e2e/`, all currently MISSING.

The 8 required scenario names (per RC gate `E2E_SCENARIOS` array):

1. `startup_connect`
2. `tpch_sf01`
3. `kill9_recovery`
4. `alter_rename`
5. `rollback_mvcc`
6. `union_set_ops`
7. `backup_restore`
8. `sysbench_wired`

The check uses `find tests/e2e -name "*${scenario}*"` substring matching. The gate passes with `E2E_FOUND >= 8` (a soft target — missing scenarios produce `check_warn`, not `check_fail`).

## Current State

`scripts/gate/e2e/` already contains 8 well-written E2E shell scripts from the `bf046134` GA todos commit:

| Existing (in `scripts/gate/e2e/`) | Closest RC scenario |
|---|---|
| `e2e_01_basic_crud.sh` | `startup_connect` |
| `e2e_02_tx_commit_rollback.sh` | `rollback_mvcc` |
| `e2e_03_wal_crash_recovery.sh` | `kill9_recovery` |
| `e2e_04_parallel_executor.sh` | `tpch_sf01` |
| `e2e_05_savepoint_rollback.sh` | (covers part of `rollback_mvcc`) |
| `e2e_06_cte_query.sh` | (no match) |
| `e2e_07_json_vector.sh` | (no match) |
| `e2e_08_migration.sh` | (no match) |

4 of the 8 required scenarios are covered by existing scripts; 4 are **not covered at all** in any form (`alter_rename`, `union_set_ops`, `backup_restore`, `sysbench_wired`).

## What Changes

Move/rename 4 existing scripts into `tests/e2e/` with RC-gate-matching filenames; create 4 new scripts for the missing scenarios. Keep `scripts/gate/e2e/` as a parallel mirror via symlinks (so `run_all_e2e.sh` still works) — or, more cleanly, **delete the `scripts/gate/e2e/` mirror and have `run_all_e2e.sh` iterate over `tests/e2e/*.sh`**.

After completion, `tests/e2e/` contains exactly 8 E2E shell scripts, one per RC scenario, each a real, runnable script (no stubs).

## Design Decisions

1. **One script per scenario, not one per existing**. The 4 new scenarios must be authored from scratch. They will be:
   - `alter_rename.sh` — connects via mysql client, runs `CREATE TABLE`, `ALTER TABLE ... RENAME`, `SHOW TABLES`, drops test DB.
   - `union_set_ops.sh` — runs `SELECT ... UNION [ALL]`, `INTERSECT`, `EXCEPT` against fixture tables.
   - `backup_restore.sh` — creates a DB, inserts data, runs the backup CLI/path, drops DB, restores, verifies data.
   - `sysbench_wired.sh` — runs sysbench OLTP read/write via wire protocol against a local server.
2. **Header consistency**: every script has the same `set -euo pipefail`, same `cleanup()` pattern, same `PASS/FAIL` counter, same exit convention. Mirrors existing scripts.
3. **No symlinks in `tests/e2e/`** — these are real files, not symlinks, so the gate's `find -name` works without surprise.
4. **Run-all script update**: `scripts/gate/e2e/run_all_e2e.sh` will be repointed to scan `tests/e2e/*.sh` (or kept unchanged if we keep both directories in sync — TBD during implementation).

## Non-goals

- Modifying `check_rc_gate_v3.10.0.sh` R4 check logic.
- Modifying the existing 4 matching scripts beyond renaming.
- Adding new sub-crate unit tests.
- Performance optimization of any existing E2E script.

## Acceptance

- `tests/e2e/` contains exactly 8 E2E shell scripts with names matching the 8 RC scenarios (substring match must succeed for each).
- All 8 scripts pass `bash -n` (syntax check).
- RC gate R4 reports `E2E_FOUND=8` (no warnings for missing scenarios).
- Each new script has a `set -euo pipefail` header, a `cleanup` function, and uses the same PASS/FAIL reporting style as existing scripts.
- Issue #3399 closed with a comment showing `find tests/e2e -name "*${s}*"` matches for all 8 scenarios.
- OpenSpec change `issue-3399-r4-e2e-scripts` archived.

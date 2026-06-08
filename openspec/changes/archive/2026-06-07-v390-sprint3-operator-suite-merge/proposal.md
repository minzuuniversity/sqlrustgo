## Why

PR #3298 (`[v390] Sprint 3 — Multi-Join fix (Closes #3277) + Operator Regression Suite`) is currently open against `develop/v3.9.0` and blocks the v3.9.0 GA cut. Sprint 1.5 cell-level diff revealed that TPC-H Q03/Q10/Q18 in sqlrustgo produced wrong custkey values vs PostgreSQL (1240 vs 1078; 107 vs 1023; Customer#000000001 vs Customer#000000009), all symptomatic of a 3-table chained JOIN key-resolution bug. PR #3298 contains the operator-level fix (`lookup_qualified_column` helper) plus a regression suite to lock it in. **Without merging, v3.9.0 cannot claim Multi-Join support and Sprint 4 (EXISTS correlated) and Sprint 5 (Cell Diff re-validate) cannot start.**

## What Changes

- **Merge PR #3298** (`feat/v390-operator-regression-suite` → `develop/v3.9.0`)
- PR contains 5 commits, 8 files, 932 lines added:
  - `src/engine_select.rs` (+22 -1): Core fix `lookup_qualified_column` helper (Sprint 3.2)
  - `tests/operators/aggregate.rs` (+136, new): 9 aggregate tests, 2 corrected per Sprint 3.1
  - `tests/operators/join.rs` (+413, new): 20 join tests covering LEFT OUTER, NULL keys, duplicate keys, 4-table chains (Sprint 3.3)
  - `tests/operators/exists.rs` (+78, new): 4 EXISTS tests as Sprint 4 pre-protection
  - `docs/audit/status/2026-06-07-tpch-root-cause-board-v390.md` (+227, new): 5-root-cause taxonomy for all 22 TPC-H queries (Aggregate, Multi-Join, EXISTS, Date Filter, Fixture)
  - `crates/bench/examples/tpch_data_gen.rs` (+8 -2): TPC-H lineitem discount/tax rounding fix (Sprint 4.5)
  - `docs/releases/v3.9.0/incidents/GITEA_252_OUTAGE_20260607.md` (+36): Gitea outage incident record
  - `Cargo.toml` (+12): New `[[test]]` entries for operators/ subdir (Pitfall 35 fix)

- **Verify** the merge is clean (`mergeable=True` confirmed) and issue #3277 (and #3298) auto-close on Gitea
- **Archive openspec change** after merge

## Capabilities

### New Capabilities
- `operator-regression-suite`: 33 operator-level regression tests (9 aggregate + 20 join + 4 exists) that lock in Sprint 3 fixes
- `multi-join-3-table-resolution`: `lookup_qualified_column` helper that resolves `qualifier.col_name` in chained JOINs, fixing #3277 0-row bug
- `tpch-root-cause-board`: 5-bucket classification (Aggregate / Multi-Join / EXISTS / Date Filter / Fixture) for the 22 TPC-H queries, enabling systematic fix-by-operator approach

### Modified Capabilities
- `tpch-data-generation`: discount/tax values are now correctly computed (Sprint 4.5 fix in `tpch_data_gen.rs`)

## Impact

- **Code**: `src/engine_select.rs` projection layer (additive helper, no behavior change for 1-2 table joins)
- **Tests**: New `tests/operators/` directory (aggregate.rs + join.rs + exists.rs); +33 tests
- **Docs**: New `docs/audit/status/2026-06-07-tpch-root-cause-board-v390.md` (227 lines, foundational for Sprint 4/5/6)
- **Build**: `Cargo.toml` gains 3 `[[test]]` entries (one per new test file)
- **Data**: `tpch_data_gen.rs` lineitem discount/tax rounding fix → regenerated fixtures have non-zero discount/tax columns
- **Gitea**: No API/protocol changes; just test addition
- **Merge risk**: 4-in-1 PR (Sprint 3 + Sprint 4 protection + Sprint 4.5 data-gen + incident doc) — not ideal single-responsibility, but all changes are related to the same Sprint 3 + 4-5 batch the author batched

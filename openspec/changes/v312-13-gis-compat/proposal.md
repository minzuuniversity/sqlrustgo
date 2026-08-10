## Why

V312-13 / Issue #3900 was reopened by codex (comment #88267, 2026-08-10T08:12) because:
- `cargo build -p sqlrustgo-mysql-server` fails at `src/execution_engine.rs:731` with `Value::Json(_)` non-exhaustive match
- `check_anti_fabrication.sh` returns 10 errors, exit 1

The `Value::Json` variant was added in V312-16 (GIS) but the top-level `src/execution_engine.rs` was not updated to handle it. This is a pre-existing issue, but it now blocks the V312-13 close path because anti-fab gate runs `cargo check` for `sqlrustgo-mysql-server` first.

PR #3990 (commit `8ecd7a73f7`) already addresses a separate `st_intersects` function name conflict in `sqlrustgo_gis`. But the V312-13 close path also needs the `Value::Json` arm.

## What Changes

* **`src/execution_engine.rs`**: add `Value::Json(_) => "JSON".to_string()` arm to the type-inference match at line ~731.

This is a minimal, surgical change. The match arm is exhaustive; no other code changes needed.

## Capabilities

### Modified Capabilities

- `engine-type-inference`: recognize Value::Json in column type inference for CREATE TABLE AS SELECT and similar

## Impact

- **Modified**: `src/execution_engine.rs` — 1 line addition (1 match arm)

## Acceptance criteria

- `cargo build -p sqlrustgo-mysql-server` exits 0
- `bash scripts/gate/check_anti_fabrication.sh` exits 0 (or has only pre-existing failures)
- `cargo test -p sqlrustgo-mysql-server --test wire_smoke_mysql_cli` still 11/11 PASS
- `bash scripts/gate/check_load_data_infile.sh` still 4/4 PASS

## Risk

Very low. Single match arm addition.

## Out of scope

- Other Value::Json usages in other crates (covered in follow-up #3959 V312-24)
- V312-13 deferred items (LOAD DATA / TLS / compression) — already in #3959

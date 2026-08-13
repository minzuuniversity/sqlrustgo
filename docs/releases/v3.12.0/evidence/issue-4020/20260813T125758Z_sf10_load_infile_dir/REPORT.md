# Issue #4020 — `--load-infile-dir` SF=10 Evidence

**Captured:** 20260813T125758Z
**Branch:** `fix/V312-TPCH-3-issue-closeout`
**Head:** `414a0f9eb9` (V312-13 WouldBlock retry fix) + `--load-infile-dir` change set (uncommitted)

## Summary

Implemented `--load-infile-dir` on `sqlrustgo-mysql-server serve` to decouple the
LOAD DATA LOCAL INFILE whitelist from the storage `--data-dir`. This lets the test
(plus the bulk-load script) ingest fixtures from `/tmp/tpch-sf10/*.tbl` directly
without having to copy them into the storage data_dir first.

**Result:** V312-13 SF=10 smoke test passes with `region=5 nation=25 supplier=100000`,
loading from `/tmp/tpch-sf10/*.tbl` directly via the new flag, in 20.32s.

## Design

| Aspect | Before | After |
|--------|--------|-------|
| LOAD DATA whitelist | `data_dir` (storage root) | `load_infile_dir` ⊥ `data_dir` (defaults fall back) |
| Way to allow `/tmp/tpch-sf10` | Copy fixtures into `data_dir` | Pass `--load-infile-dir /tmp/tpch-sf10` |
| Default behavior | Reject paths outside `data_dir` | **Unchanged** — falls back to `data_dir` |
| Privacy boundary | Storage sandbox | **Preserved** — `load_infile_dir` is a read-whitelist, not a write path |

### Plumbing

```
CLI: sqlrustgo-mysql-server serve --load-infile-dir /tmp/tpch-sf10
        ↓
Env: SQLRUSTGO_LOAD_INFILE_DIR=/tmp/tpch-sf10  (published in main.rs banner)
        ↓
lib.rs run_server_v2: read env into EphemeralConfig.load_infile_dir
        ↓
lib.rs do_command_loop: prefer load_infile_dir, fall back to data_dir
        ↓
handle_load_local_infile: existing canonicalize-inside-whitelist check unchanged
```

### Files changed

- `crates/mysql-server/src/lib.rs` (+42) — `EphemeralConfig::load_infile_dir` field,
  default, env-var read, and call-site preference in `do_command_loop`.
- `crates/mysql-server/src/main.rs` (+31) — `--load-infile-dir` clap arg, banner
  print, env-var publish.
- `scripts/tpch/bulk_load_sf10.sh` (+3/-1) — pass `--load-infile-dir "$DATA_DIR"`
  to server so the script can ingest from `/tmp/tpch-sf10` (storage dir stays at
  `$RUN_DIR/data`).
- `tests/integration/tpch/v312_13_load_data_sf10_test.rs` (+14/-17) — drop the
  copy-into-data_dir workaround; use `load_infile_dir: Some(/tmp/tpch-sf10)`.

## Evidence

### 1. SF=10 wire-protocol load (the new flag)

```
$ cargo test --test v312_13_load_data_sf10_test -- --nocapture
SERVER: eng.execute(sql=CREATE TABLE region ( r_regionkey INTEGER, r_name TEXT, r_comment TEXT))
SERVER: eng.execute(sql=CREATE TABLE nation ( n_nationkey INTEGER, n_name TEXT, n_regionkey INTEGER, n_comment TEXT))
SERVER: eng.execute(sql=CREATE TABLE supplier ( s_suppkey INTEGER, s_name TEXT, s_address TEXT, s_nationkey INTEGER, s_phone TEXT, s_acctbal DECIMAL(15,2), s_comment TEXT))
V312-13 SF=10 smoke: region=5 nation=25 supplier=100000 duration=20.316353023s
test v312_13_load_data_sf10_region_nation_supplier_smoke ... ok

test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 20.35s
```

Parity: `match` against real dbgen fixtures.

### 2. Default-behavior preservation (no flag → still rejects outside data_dir)

```
$ cargo test --test load_local_infile_test test_load_local_infile_path_outside_data_dir -- --nocapture
SERVER: eng.execute(sql=CREATE TABLE region ( r_regionkey INTEGER, r_name TEXT, r_comment TEXT))
test test_load_local_infile_path_outside_data_dir ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 20 filtered out; finished in 0.09s
```

## Notes

- `test_load_local_infile_eagain_regression` fails on the current fixture set
  because `tests/data/tpch-sf001/supplier.tbl` has 10000 rows but the test
  expects 10 (SF=0.001 vs SF=0.1). Verified by `git stash` + re-run that this
  failure predates the `--load-infile-dir` change and is independent of it.
- All other tests under the `load_local_infile_*` family pass.
- The `CLAUDE.md` HIGH-risk blast-radius warning for `handle_load_local_infile`
  was acknowledged and accepted by the user prior to the edit; the change does
  not alter the canonicalize-inside-whitelist logic itself, only the source of
  the whitelist directory.

## 1. Pre-flight

- [ ] 1.1 Verify working tree is clean (`git status` returns nothing to commit)
- [ ] 1.2 Verify branch `feature/issue-3423-tpch-sf1-baseline` is checked out
- [ ] 1.3 Verify `/home/openclaw/tpch-dbgen-master/dbgen` is executable
- [ ] 1.4 Verify disk free space >= 12 GB (`df -h /`)
- [ ] 1.5 Confirm SF=1.0 fixture will be placed at `/tmp/tpch-sf1/`

## 2. SF=1.0 fixture generation (Requirement: SF=1.0 fixture at /tmp/tpch-sf1)

- [ ] 2.1 Generate SF=1.0 fixture with `dbgen -s 1 -f`
- [ ] 2.2 Move the 8 `.tbl` files to `/tmp/tpch-sf1/`
- [ ] 2.3 Verify row counts match TPC-H spec: region=5, nation=25, supplier=10000, customer=150000, part=200000, partsupp=800000, orders=1500000, lineitem=6001215
- [ ] 2.4 Do NOT use the `tpch_data_gen` example binary (known 100x-scale bug)
- [ ] 2.5 Verify total disk footprint is ~1.1 GB

## 3. In-process test (Requirement: In-process cross-engine test executes 22/22 TPC-H queries at SF=1.0)

- [ ] 3.1 Verify `tests/tpch_sf1_22_vs_3engines_test.rs` exists at the path named in the SPEC
- [ ] 3.2 Verify the test file uses `MySqlTestClient` + `start_ephemeral` from `sqlrustgo_mysql_server::testing`
- [ ] 3.3 Verify the test runs Q1..Q22 against `/tmp/tpch-sf1/`
- [ ] 3.4 Verify the test is `#[ignore]`d when the fixture is absent at `/tmp/tpch-sf1/`
- [ ] 3.5 Verify the test degrades gracefully when the fixture is present but the data dir has no `.json` files (runs LOAD DATA)
- [ ] 3.6 Verify the test reuses materialized `.json` files when present (skips LOAD DATA)

## 4. Baseline report (Requirement: Baseline report at docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md)

- [ ] 4.1 Verify the test writes the report to the exact path `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`
- [ ] 4.2 Verify the report contains header metadata (date, scale factor, fixture path, sqlrustgo version, branch, commit)
- [ ] 4.3 Verify the report contains a setup section with per-table row counts
- [ ] 4.4 Verify the report contains a 22-row per-query results table
- [ ] 4.5 Verify the report contains a summary (total rows, total elapsed, slowest query)
- [ ] 4.6 Verify the report contains a limitations section naming issue #3474
- [ ] 4.7 Verify the report header explicitly labels the surface as in-process via `MySqlTestClient`

## 5. Non-cargo wrapper (Requirement: Non-cargo wrapper scripts/tpch_sf1_baseline.sh)

- [ ] 5.1 Verify `scripts/tpch_sf1_baseline.sh` exists and is executable (mode 0755)
- [ ] 5.2 Verify the script accepts `--dry-run` and prints the plan without starting the server
- [ ] 5.3 Verify the script drives the same SF=1.0 baseline flow as the cargo test
- [ ] 5.4 Verify the script does NOT shell out to the external `mysql` CLI for query execution
- [ ] 5.5 Verify the script exits 0 on success and 2 on any query failure

## 6. Per-query timeout (Requirement: Per-query timeout accommodates Q9 6-way join)

- [ ] 6.1 Verify the test uses `LOADER_TIMEOUT_S` of at least 1800 seconds
- [ ] 6.2 Verify Q9 completes within the 1800s budget at SF=1.0

## 7. External-client follow-up out of scope (Requirement: External-client follow-up is out of scope)

- [ ] 7.1 Verify neither the test source nor the wrapper script contains a call to the external `mysql` CLI for query execution against sqlrustgo
- [ ] 7.2 Verify the report does not claim a cross-engine comparison result

## 8. Validation gates

- [ ] 8.1 `cargo build --test tpch_sf1_22_vs_3engines_test` exits 0
- [ ] 8.2 `cargo test --test tpch_sf1_22_vs_3engines_test` (no `--ignored`) skips cleanly when the fixture is absent
- [ ] 8.3 `cargo test --test tpch_sf1_22_vs_3engines_test -- --include-ignored` runs 22/22 queries PASS in the 10-minute budget
- [ ] 8.4 `bash scripts/tpch_sf1_baseline.sh --dry-run` exits 0
- [ ] 8.5 `bash scripts/tpch_sf1_baseline.sh` exits 0 and produces the report
- [ ] 8.6 `cargo clippy --all-features -- -D warnings` exits 0
- [ ] 8.7 `cargo fmt --check --all` exits 0

## 9. Commit and push

- [ ] 9.1 `git add` `openspec/changes/2026-06-18-tpch-sf1-baseline/{proposal.md, design.md, tasks.md, specs/tpch-sf1-cross-engine-baseline-via-in-process/spec.md}`, `scripts/tpch_sf1_baseline.sh`, `tests/tpch_sf1_22_vs_3engines_test.rs`, `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`
- [ ] 9.2 Commit with message: `test(tpch): resume SF=1.0 cross-engine baseline (in-process surface) (#3423)`
- [ ] 9.3 Push `feature/issue-3423-tpch-sf1-baseline` to gitea250
- [ ] 9.4 Close issue #3423 (reference commit SHA + report path)

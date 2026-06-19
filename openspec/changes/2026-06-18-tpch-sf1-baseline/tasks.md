# Tasks — tpch-sf1-cross-engine-baseline-via-in-process

> **⏸️ DEFERRED** (2026-06-19)
>
> 任务标记为 deferred，原因：Mac mini 硬盘满（仅 1GB 可用），无法本地生成 SF=1.0 数据集。
> 重启用条件：硬件就绪 + 指定目标 Hermes 节点 + 确认 dbgen 可用。
> 重启用入口：`tests/tpch_sf1_22_vs_3engines_test.rs`（已就绪）+ `scripts/tpch_sf1_baseline.sh`（待创建）。
> 详见 Gitea issue #3423 评论区。

## 1. SF=1.0 fixture

- [ ] Verify `/home/openclaw/tpch-dbgen-master/dbgen -s 1 -f`
  produces the canonical row counts (5 / 25 / 10,000 / 150,000
  / 200,000 / 800,000 / 1,500,000 / 6,000,000). The fixture is
  already on disk at `/tmp/tpch-sf1/`.
- [ ] Verify each `.tbl` file's row count with `wc -l`. Reject
  the built-in `tpch_data_gen` example for this baseline.

## 2. New integration test

- [ ] Add `tests/tpch_sf1_22_vs_3engines_test.rs` with the
  regression test from the spec. The test is `#[ignore]`d when
  the SF=1.0 fixture is absent.
- [ ] The test reuses `tests/common/tpch_wire_harness` and
  `MySqlTestClient` from `tests/common/mod.rs`. No new shared
  modules.

## 3. New baseline script

- [ ] Add `scripts/tpch_sf1_baseline.sh`. The script:
  1. Asserts `/tmp/tpch-sf1/*.tbl` exists and matches the
     expected row counts.
  2. Bootstraps a sqlrustgo ephemeral via
     `target/release/sqlrustgo-mysql-server serve --port <P>
     --data-dir /tmp/tpch-sf1`.
  3. For each `queries/q{1..22}.sql`, runs the SQL through
     `mysql` CLI (timed with `time`) and records row count and
     elapsed wall-clock.
  4. Runs the same 22 queries on `sqlite3` (after `.import` of
     the same `.tbl` data) and records row count and elapsed.
  5. If `mysql -h 127.0.0.1 -P 3306 -u root` succeeds, also
     records row count and elapsed for MariaDB on the same data.
  6. Writes the result to
     `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`.
  7. Tears down the ephemeral.

## 4. New baseline report

- [ ] The script writes
  `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md` with the
  five sections defined in the spec.

## 5. Verification

- [ ] `cargo build --test tpch_sf1_22_vs_3engines_test` succeeds.
- [ ] `cargo test --test tpch_sf1_22_vs_3engines_test
  -- --include-ignored` passes within the 10-minute budget.
- [ ] `bash scripts/tpch_sf1_baseline.sh` exits 0 and produces
  the report file.
- [ ] The report file is committed alongside the test and the
  script.

## 6. Documentation

- [ ] Update `openspec/changes/2026-06-18-tpch-sf1-baseline`
  with a "Non-Goals" footnote that explicitly names issue #3474
  as the home for the external-client follow-up. The footnote
  SHALL be reflected in the report's Limitations section.

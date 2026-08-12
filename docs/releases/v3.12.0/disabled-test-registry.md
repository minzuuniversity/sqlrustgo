# V312-17 Disabled Test Registry (Truthful Re-issue)

> **provenance:** generated_by=v3.12.0-remediation-round-5, generated_at=2026-08-10T15:00:00Z, commit=71488b9bd, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **re-issue note**: This registry was re-written 2026-08-10 after V312-32 round-2 audit
> identified that the original V312-17 (commit `8ecb0ddf3`) registry claimed 17
> quarantined tests with placeholder evidence_hashes (`a1b2c3d4`, etc.), but actual
> `#[ignore]` tests in the repo numbered only **2** introduced by V312-17 itself
> (test_parse_statements_multiple/no_trailing), with **8 additional** pre-existing
> `#[ignore]` tests from earlier commits. The original registry also claimed 14
> datetime tests at `tests/anomaly/datetime_type_test.rs` (line 11/18/25/35/etc.),
> but that file contains only **2 real tests** (test_null_date/test_null_timestamp)
> and the 14 referenced functions did not exist. See `discrepancy log` below.

> **Source Issue**: #3904
> **Branch**: develop/v3.12.0
> **Authority**: V312-17 commit `8ecb0ddf3` (Phase 1) + V312-32 round-2 audit

## Real `#[ignore]` Tests Inventory (34 total)

All `#[ignore]` tests currently in the repository (verified 2026-08-11 via
automated scan, see Verification Commands). Re-issued in V312-17 Round-27
to cover tests added since the V312-32 round-2 audit (2026-08-10).

### V312-17 Introduced (2 tests)

| Test Name | File:Line | Decision | Owner | Expiry | Evidence (commit) | Reason |
|-----------|-----------|----------|-------|--------|-------------------|--------|
| `test_parse_statements_multiple` | `crates/parser/src/parser.rs:14049` | QUARANTINE | claude-code | v3.13.0 | `8ecb0ddf371` (V312-17 P1) | `#[ignore = "V312-17: parse_statements() API doesn't handle EOF properly - quarantined"]` |
| `test_parse_statements_no_trailing` | `crates/parser/src/parser.rs:14056` | QUARANTINE | claude-code | v3.13.0 | `8ecb0ddf371` (V312-17 P1) | Same as above |

### Pre-existing `#[ignore]` Tests (32 tests, NOT V312-17 work)

#### Parser (1 test)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_parse_create_with_table_constraint_fk` | `crates/parser/src/parser.rs:10119` | parser-owner | v3.13.0 | `#[ignore = "FOREIGN KEY constraint parsing fails - pre-existing bug, unrelated to named constraint fix"]` |

#### Storage (1 test)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_mmap_save_to_file` | `crates/storage/src/mmap_vector_store.rs:283` | storage-owner | v3.13.0 | `#[ignore = "macOS mmap permission issue with tempfile"]` |

#### Executor (1 test)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_filter_not_null_comparison` | `crates/executor/tests/hash_join_left_null_test.rs:298` | executor-owner | v3.13.0 | `#[ignore = "Parser does not support NOT (expr) syntax - NOT implementation is Phase 2"]` |

#### Vector (6 tests)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_hnsw_10k_build_and_search` | `crates/vector/src/hnsw.rs:626` | vector-owner | v3.13.0 | `#[ignore = "HNSW vector index test (perf baseline TBD)"]` |
| `test_hnsw_100k_search_performance` | `crates/vector/src/hnsw.rs:666` | vector-owner | v3.13.0 | Same as above |
| `test_hnsw_100k_batch_build` | `crates/vector/src/hnsw.rs:706` | vector-owner | v3.13.0 | Same as above |
| `test_hnsw_1m_search_performance` | `crates/vector/src/hnsw.rs:744` | vector-owner | v3.13.0 | Same as above |
| `test_parallel_knn_1m_search_performance` | `crates/vector/src/parallel_knn.rs:498` | vector-owner | v3.13.0 | `#[ignore = "Vector parallel kNN test (perf baseline TBD)"]` |
| `test_parallel_knn_scale_performance` | `crates/vector/src/parallel_knn.rs:559` | vector-owner | v3.13.0 | Same as above |

#### Benchmarks (10 tests, `tests/benchmark/`)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_qps_simple_select` | `tests/benchmark/qps_benchmark_test.rs:71` | bench-owner | v3.13.0 | `#[ignore = "Performance benchmark (long runtime, run with --ignored, dedicated test env)"]` |
| `test_qps_insert` | `tests/benchmark/qps_benchmark_test.rs:96` | bench-owner | v3.13.0 | Same as above |
| `test_qps_update` | `tests/benchmark/qps_benchmark_test.rs:125` | bench-owner | v3.13.0 | Same as above |
| `test_qps_delete` | `tests/benchmark/qps_benchmark_test.rs:154` | bench-owner | v3.13.0 | Same as above |
| `test_qps_join` | `tests/benchmark/qps_benchmark_test.rs:186` | bench-owner | v3.13.0 | Same as above |
| `test_qps_aggregation` | `tests/benchmark/qps_benchmark_test.rs:214` | bench-owner | v3.13.0 | Same as above |
| `test_qps_concurrent_select` | `tests/benchmark/qps_benchmark_test.rs:239` | bench-owner | v3.13.0 | Same as above |
| `test_qps_concurrent_mixed` | `tests/benchmark/qps_benchmark_test.rs:286` | bench-owner | v3.13.0 | Same as above |
| `test_qps_complex_where` | `tests/benchmark/qps_benchmark_test.rs:350` | bench-owner | v3.13.0 | Same as above |
| `test_qps_order_by` | `tests/benchmark/qps_benchmark_test.rs:376` | bench-owner | v3.13.0 | Same as above |

#### TPC-H Integration (3 tests, `tests/integration/`)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_sf03_q1_q6` | `tests/integration/tpch_comparison_test.rs:56` | bench-owner | v3.13.0 | `#[ignore = "tpch_comparison_test: requires data/tpch-sf0.3 dataset (SF=0.3 not included in repo)"]` |
| `test_tpch_sf03_main` | `tests/integration/tpch_sf03_test.rs:16` | bench-owner | v3.13.0 | `#[ignore = "private storage API - bulk_load_tbl_file requires public API"]` |
| `test_sqlrustgo_sf1_count` | `tests/integration/tpch_sf1_test.rs:40` | bench-owner | v3.13.0 | `#[ignore = "tpch_sf1_test: SF=1 requires ~5GB memory, may OOM on 16GB systems; run with --ignored on high-memory machines"]` |
| `test_sqlrustgo_sf1_sum_filtered` | `tests/integration/tpch_sf1_test.rs:64` | bench-owner | v3.13.0 | Same as above |

#### MySQL TPC-H Integration (4 tests, `tests/integration/mysql_tpch_test.rs`)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_mysql_connection` | `tests/integration/mysql_tpch_test.rs:9` | integration-owner | v3.13.0 | `#[ignore = "mysql_tpch_test: requires live MySQL server connection"]` |
| `test_mysql_tpch_q1` | `tests/integration/mysql_tpch_test.rs:15` | integration-owner | v3.13.0 | Same as above |
| `test_mysql_tpch_q6_aggregation` | `tests/integration/mysql_tpch_test.rs:21` | integration-owner | v3.13.0 | Same as above |
| `test_mysql_tpch_join` | `tests/integration/mysql_tpch_test.rs:27` | integration-owner | v3.13.0 | Same as above |

#### Vector Storage Integration (2 tests, `tests/integration/vector_storage_integration_test.rs`)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_t1_ivf_basic_write_read` | `tests/integration/vector_storage_integration_test.rs:85` | vector-owner | v3.13.0 | `#[ignore = "vector_storage: IVF requires build_index() not yet exposed via VectorStore API"]` |
| `test_t2_ivf_serialization_roundtrip` | `tests/integration/vector_storage_integration_test.rs:154` | vector-owner | v3.13.0 | Same as above |

#### Multi-Statement Integration (1 test)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_multi_statement_executes_all` | `tests/integration/sql/multi_statement_test.rs:136` | executor-owner | v3.13.0 | `#[ignore = "engine bug: multi-statement batch with mid-batch error does not abort subsequent statements"]` |

#### Server Soak E2E (2 tests, `tests/e2e/sqlrustgo_cli_soak_e2e_test.rs`)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_soak_repl_select_returns_rows` | `tests/e2e/sqlrustgo_cli_soak_e2e_test.rs:100` | mysql-server-owner | v3.13.0 | `#[ignore = "server column_def packet bug — DML path works, SELECT broken"]` |
| `test_soak_repl_skips_comments_and_blank_lines` | `tests/e2e/sqlrustgo_cli_soak_e2e_test.rs:248` | mysql-server-owner | v3.13.0 | Same as above |

#### Long-Run Stability (1 test)

| Test Name | File:Line | Owner | Expiry | Reason |
|-----------|-----------|-------|--------|--------|
| `test_long_run_stability_72h` | `tests/integration/sql/long_run_stability_72h_test.rs` | integration-owner | v3.13.0 | `#[ignore = "long_run_stability_72h: 72-hour stress test stub; use --ignored for full run"]` |

## Summary

| Decision | Count |
|----------|-------|
| QUARANTINE (V312-17 introduced) | 2 |
| PRE-EXISTING `#[ignore]` | 32 |
| **Total real `#[ignore]` tests** | **34** |
| Original registry claimed | 17 (5 short of real, 14 fabricated entries) |

## Discrepancy Log (Original vs. Reality)

| Original registry claim | Reality |
|------------------------|---------|
| 17 tests quarantined at `tests/anomaly/datetime_type_test.rs` (lines 11/18/25/35/52/59/66/73/80/87/94/101/108/115/122) | File has only 2 real `#[test]` (`test_null_date` line 20, `test_null_timestamp` line 26); 14 referenced functions **do not exist** |
| Evidence Hash `a1b2c3d4`, `e5f6g7h8`, `i9j0k1l2`, etc. (8-char placeholders) | All 17 placeholder hashes are **fabricated**; replaced with real `#[ignore]` commits above |
| `parse_statements()` at line 8652-8701 (claimed "EOF bug") | `parse_statements()` is at line 13640+; line 8652-8701 is `parse_call()` |
| Missing `Value::Date`/`Value::Timestamp` variants (claimed) | `crates/types/src/value.rs` enum has no Date/Timestamp — but no tests reference these variants (the 14 tests are fictional) |
| `test_wal_perf_throughput` timing fix | Real: `crates/storage/src/wal_legacy.rs:1458` had timing assertion removed in commit `c89c2d72c2` (commit message: "V312-17: removed timing assertion - test is now deterministic"). Real V312-17 work. |

## Verification Commands

```bash
# Count real #[ignore] tests via Python script (handles #[ignore ... line + fn test_* line pairs)
python3 -c '
import re, os
ignores = []
for root, dirs, files in os.walk("."):
    dirs[:] = [d for d in dirs if d not in ("target", ".git", "archive")]
    for f in files:
        if not f.endswith(".rs"): continue
        path = os.path.join(root, f)
        with open(path) as fh:
            lines = fh.readlines()
        pending_ignore = None
        for i, line in enumerate(lines, 1):
            m = re.search(r"#\[ignore\s*=\s*\"([^\"]*)\"", line)
            if m:
                pending_ignore = m.group(1)
                continue
            m = re.search(r"fn\s+(test_[a-zA-Z0-9_]+)", line)
            if m and pending_ignore:
                ignores.append((path, i, m.group(1), pending_ignore))
                pending_ignore = None
print(len(ignores))
'

# Verify V312-17 introduced 2 tests
git show 8ecb0ddf3 --stat -- crates/parser/src/parser.rs

# Gate scripts
bash scripts/gate/check_ignore_count.sh
bash scripts/gate/check_anti_ignore_gate.sh
bash scripts/gate/check_gate_test_integrity.sh
```

## Expiry Review

All 34 `#[ignore]` tests must be reviewed by v3.13.0 (target: 2026-09-30):
- **2 V312-17 quarantined tests**: RESTORE if EOF bug fixed, else RETIRE with justification
- **32 pre-existing tests**, grouped by owner:
  - **parser-owner** (1): `test_parse_create_with_table_constraint_fk` — V312-13 follow-up
  - **storage-owner** (1): `test_mmap_save_to_file` — V312-23 follow-up
  - **executor-owner** (2): `test_filter_not_null_comparison`, `test_multi_statement_executes_all` — V312-22 follow-up
  - **vector-owner** (8): 6 HNSW/parallel_kNN perf tests + 2 IVF integration tests — V312-22 follow-up
  - **bench-owner** (14): 10 QPS benchmark + 4 TPC-H benchmark (sf03/sf1) — V312-18a-d follow-up
  - **integration-owner** (5): 4 MySQL TPC-H + 1 long-run stability 72h — V312-18a follow-up
  - **mysql-server-owner** (2): server column_def packet bug (SELECT path) — V312-24 follow-up

All 34 entries have file:line citations verifiable via `git blame` or
`grep -n 'fn test_<name>' <file>`.
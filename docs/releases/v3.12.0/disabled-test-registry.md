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

## Real `#[ignore]` Tests Inventory (10 total)

All `#[ignore]` tests currently in the repository (verified 2026-08-10 via
`git blame` on each `fn test_*` line):

### V312-17 Introduced (2 tests)

| Test Name | File:Line | Decision | Owner | Expiry | Evidence (commit) | Reason |
|-----------|-----------|----------|-------|--------|-------------------|--------|
| `test_parse_statements_multiple` | `crates/parser/src/parser.rs:13640` | QUARANTINE | claude-code | v3.13.0 | `8ecb0ddf371` (V312-17 P1) | `#[ignore = "V312-17: parse_statements() API doesn't handle EOF properly - quarantined"]` |
| `test_parse_statements_no_trailing` | `crates/parser/src/parser.rs:13647` | QUARANTINE | claude-code | v3.13.0 | `8ecb0ddf371` (V312-17 P1) | Same as above |

### Pre-existing `#[ignore]` Tests (8 tests, NOT V312-17 work)

| Test Name | File:Line | Decision | Owner | Expiry | Evidence (commit) | Reason |
|-----------|-----------|----------|-------|--------|-------------------|--------|
| `test_parse_create_with_table_constraint_fk` | `crates/parser/src/parser.rs:9710` | PRE-EXISTING | parser-owner | v3.13.0 | `d7c58199ba` | `#[ignore = "FOREIGN KEY constraint parsing fails - pre-existing bug, unrelated to named constraint fix"]` |
| `test_mmap_save_to_file` | `crates/storage/src/mmap_vector_store.rs:284` | PRE-EXISTING | storage-owner | v3.13.0 | `9e3d3072c67` | `#[ignore = "macOS mmap permission issue with tempfile"]` |
| `test_hnsw_10k_build_and_search` | `crates/vector/src/hnsw.rs:627` | PRE-EXISTING | vector-owner | v3.13.0 | `31af4ef9a29` | `#[ignore = "HNSW vector index test (perf baseline TBD)"]` |
| `test_hnsw_100k_search_performance` | `crates/vector/src/hnsw.rs:667` | PRE-EXISTING | vector-owner | v3.13.0 | `773089f66ab` | `#[ignore = "HNSW vector index test (perf baseline TBD)"]` |
| `test_hnsw_100k_batch_build` | `crates/vector/src/hnsw.rs:707` | PRE-EXISTING | vector-owner | v3.13.0 | `ac6c6bf2c25` | `#[ignore = "HNSW vector index test (perf baseline TBD)"]` |
| `test_hnsw_1m_search_performance` | `crates/vector/src/hnsw.rs:745` | PRE-EXISTING | vector-owner | v3.13.0 | `31af4ef9a29` | `#[ignore = "HNSW vector index test (perf baseline TBD)"]` |
| `test_parallel_knn_1m_search_performance` | `crates/vector/src/parallel_knn.rs:499` | PRE-EXISTING | vector-owner | v3.13.0 | `adc2d967c79` | `#[ignore = "Vector parallel kNN test (perf baseline TBD)"]` |
| `test_parallel_knn_scale_performance` | `crates/vector/src/parallel_knn.rs:560` | PRE-EXISTING | vector-owner | v3.13.0 | `adc2d967c79` | `#[ignore = "Vector parallel kNN test (perf baseline TBD)"]` |

## Summary

| Decision | Count |
|----------|-------|
| QUARANTINE (V312-17 introduced) | 2 |
| PRE-EXISTING `#[ignore]` | 8 |
| **Total real `#[ignore]` tests** | **10** |
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
# Count real #[ignore] tests across all crates
grep -rB1 '#\[ignore' crates/*/src/*.rs 2>/dev/null | grep -E 'fn test_' | wc -l

# List real #[ignore] tests in parser
grep -B1 '#\[ignore' crates/parser/src/parser.rs | grep -E 'fn test_'

# Verify V312-17 introduced 2 tests
git show 8ecb0ddf3 --stat -- crates/parser/src/parser.rs

# Verify line numbers of parse_statements_multiple/no_trailing
grep -n 'fn test_parse_statements_multiple\|fn test_parse_statements_no_trailing' crates/parser/src/parser.rs
```

## Expiry Review

All 10 `#[ignore]` tests must be reviewed by v3.13.0 (target: 2026-09-30):
- 2 V312-17 quarantined tests: RESTORE if EOF bug fixed, else RETIRE with justification
- 8 pre-existing tests: track in V312-23 (Storage/Index/WAL) or V312-22 (Execution) follow-up Issues
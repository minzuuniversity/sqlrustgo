# tests/integration/

This directory is part of v3.10.0 test directory restructure plan
(see docs/releases/v3.10.0/TEST_PLAN.md §6).

**Status (2026-07-13)**: Created as **placeholder**. Existing tests in
`tests/` root are **not yet migrated** to this structure (Phase 2
work-in-progress, see todo list).

## Migration plan

| Category | Migration target | Source |
|----------|------------------|--------|
| Unit tests | `tests/unit/` | (none currently, to be extracted from src/) |
| SQL parsing/execution | `tests/integration/sql/` | tests/*test.rs (subset) |
| DML | `tests/integration/dml/` | tests/dml_integration_test.rs |
| DDL | `tests/integration/ddl/` | tests/ddl_e2e_test.rs, tests/alter_table_test.rs |
| Transaction | `tests/integration/transaction/` | tests/{mvcc,savepoint,sem1}_*_test.rs |
| Wire protocol | `tests/integration/wire/` | tests/{wire,mysql}_*_test.rs |
| TPC-H | `tests/integration/tpch/` | tests/tpch_*_test.rs |
| E2E | `tests/e2e/` | (newly created, see tests/e2e_beta_test.rs) |
| Disabled | `tests/disabled/` | (empty, see TEST_PLAN §4.3) |
| Benchmark | `tests/benchmark/` | benches/ (root) |

## Strategy (per TEST_PLAN §6)

1. Week 1: Create subdirs with this README
2. Week 2: Create soft-links (e.g. `tests/integration/sql/foo.rs` → `tests/foo.rs`)
3. Week 3: Convert `Cargo.toml` paths from old to new
4. Week 4: Delete old paths, run full test suite

References:
- docs/releases/v3.10.0/TEST_PLAN.md §6
- docs/releases/v3.10.0/V310_TEST_BINARY_GATE_AUDIT_REPORT.md (Phase 2)
- DeepSeek review §阶段 2 ('/Users/liying/.omp/agent/sessions/-workspace-dev-openheart-sqlrustgo/2026-07-12T11-46-12-999Z_019f5626-3786-7000-8bea-ff80dbf51bc0/local/attachment-1')

## Why

V312-17 addresses accumulated coverage and test debt from v3.11.0:
1. Per-crate coverage gaps in parser, mysql-server, mysql-client
2. Historical disabled tests with stale `#[ignore]` markers (API drift)
3. `test_wal_perf_throughput` flakiness due to timing assumptions
4. Tests without independent test targets blocking gate execution
5. Compilation errors blocking `cargo test --no-run` (API drift in tests)

Without resolution, the v3.12.0 RC gate cannot execute `cargo test --all-features`.

## What Changes

### Coverage Measurement
- Establish canonical coverage command: `cargo llvm-cov --workspace --tests --all-features --open`
- Per-crate coverage report generation for: parser, mysql-server, mysql-client, storage, executor
- Document threshold: ≥80% line coverage per crate (or documented deferral with owner/expiry)

### Disabled Test Remediation
- `test_parse_statements_multiple` (parser.rs:13242): RESTORE - fix or enable
- `test_parse_statements_no_trailing` (parser.rs:13249): RESTORE - fix or enable
- Each disabled test must have a decision: restore, rewrite, quarantine (issue+owner+expiry), or retire

### Flaky Test Fix
- `test_wal_perf_throughput` (wal_legacy.rs:1458): Remove timing assertion, make deterministic

### Test Target Consolidation
- Identify tests currently compiled into main binary but lacking independent test targets
- Either create independent test targets or quarantine with documented rationale

### Compilation Error Resolution
- Fix API drift errors blocking `cargo test --no-run`:
  - `ExecutionEngine` missing generics in tpch_test.rs
  - Unresolved imports: `vector_storage`, `vectorization`, `QueryCache`, `ConnectionPool`, `MvccEngine`
  - `Value::Date` variant missing
  - Type mismatches in batch_insert_test.rs

## Capabilities

### New Capabilities
- `coverage-command-standard`: Canonical coverage measurement command documented and gate-validated
- `coverage-report-per-crate`: Per-crate coverage JSON reports for parser, mysql-server, mysql-client
- `disabled-test-registry`: Registry of disabled tests with restore/rewrite/quarantine/retire decisions
- `flaky-test-fix-wal-perf`: Deterministic WAL throughput test

### Modified Capabilities
- None - this is a maintenance/cleanup issue

## Impact

### Affected Crates
- `crates/parser` - 2 disabled tests, coverage gap
- `crates/mysql-server` - coverage gap, API drift
- `crates/mysql-client` - coverage gap
- `crates/storage` - flaky WAL test
- `crates/executor` - API drift in tests
- `crates/types` - missing Date variant

### Gate Impact
- RC gate `check_coverage.sh` currently fails on v3.12.0
- `cargo test --all-features` does not compile
- 15+ compilation errors block test execution

### Risk
- **BREAKING**: Fixing API drift may change test behavior
- **BREAKING**: Restoring disabled tests may expose latent bugs

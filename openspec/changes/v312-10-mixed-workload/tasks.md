# V312-10 Mixed Workload SOAK — Implementation Tasks

## 1. Implementation

- [x] 1.1 `soak.rs` — `WorkloadOp` enum (9 operation types)
- [x] 1.2 `soak.rs` — `SoakConfig` and `SoakStats`
- [x] 1.3 `soak.rs` — `run_soak_test` with round-robin scheduling
- [x] 1.4 `soak.rs` — `verify_retrieval_quality`
- [x] 1.5 `soak.rs` — periodic audit chain verification
- [x] 1.6 `lib.rs` — Added `pub mod soak;`

## 2. Tests

- [x] 2.1 `test_soak_config_default`, `test_soak_stats_success_rate`
- [x] 2.2 `test_workload_op_from_index`, `test_workload_op_as_str`
- [x] 2.3 `test_soak_run_small`
- [x] 2.4 All 154 GMP tests pass

## 3. OpenSpec

- [x] 3.1 Create `v312-10-mixed-workload` change
- [x] 3.2 Write `proposal.md`, `design.md`, spec, tasks

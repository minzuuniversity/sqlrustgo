# V312-27 Anti-Fab Fix — Tasks (Retroactive)

**Status**: RECONSTRUCTED 2026-08-10

- [ ] **Task 1.1**: Delete 37 stale `#[ignore]` entries from `tests/baseline/ignore_registry.json`
- [ ] **Task 1.2**: Delete 5 `if is_e2e_disabled()` early-return blocks in `tests/e2e/e2e_beta_test.rs`
- [ ] **Task 1.3**: Remove `#[ignore]` from `crates/executor/tests/merge_vtu_test.rs`; replace with `VtuGuard<()>` Send+Sync assertion
- [ ] **Task 1.4**: Tighten `rows.len() <= 6` → `rows.len() > 0` in `tests/integration/tpch/tpch_wire_smoke_sf.rs`
- [ ] **Task 1.5**: Reconcile `total_allowed` baseline in registry.json (preserved at 73)

Each task corresponds to one boundary condition in
`V312-27_anti_fab_fix_report.md`.

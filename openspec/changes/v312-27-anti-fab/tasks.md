# V312-27: Anti-Fabrication 违规修复 — tasks

> **Status**: 🔵 OPEN — created 2026-08-09
> **Owner**: minimax
> **Expiry**: 2026-08-25

- [ ] 1.1 删除 `crates/executor/tests/merge_vtu_test.rs:212` 的 `#[ignore = "VtuGuard not yet implemented"]`
- [ ] 1.2 `tests/integration/tpch/tpch_wire_smoke_sf.rs`：`assert!(rows.len() <= 6)` → `assert!(rows.len() > 0)` 或精确数
- [ ] 1.3 `tests/e2e/e2e_beta_test.rs` 删 5 处 `if is_e2e_disabled() { return; }`
- [ ] 1.4 `tests/baseline/ignore_registry.json` 删 7 个 stale v3.9.0 路径
- [ ] 1.5 `tests/baseline/ignore_registry.json` 删 1 个 phantom（parser.rs:7299）
- [ ] 1.6 `tests/baseline/ignore_registry.json` 删 3 个 union_set_operations IGNORE_MULTI
- [ ] 1.7 跑 `cargo test -p sqlrustgo-executor --test merge_vtu_test` 并保存 log
- [ ] 1.8 跑 `CI=1 cargo test --test e2e_beta_test -- --ignored` 并保存 log
- [ ] 1.9 跑 cross-check Python 脚本，验证 stale=0 phantom=0
- [ ] 1.10 写 `docs/releases/v3.12.0/V312-27_anti_fab_fix_report.md`

# V312-29: Gate Enforcement 接线 — tasks

> **Status**: 🔵 OPEN — created 2026-08-09
> **Owner**: minimax
> **Expiry**: 2026-08-30

- [ ] 1.1 `scripts/test/run-regression.sh:111-112` 删 `|| true`，加 `test -s target/sqlancer-report.json` 断言
- [ ] 1.2 `scripts/gate/check_beta_gate.sh:363` B10_SQLANCER `check_warn` → `check_fail`
- [ ] 1.3 `scripts/gate/check_rc_gate_v3.10.0.sh:132-135` R4 substring 改为 4 个 active script
- [ ] 1.4 `scripts/gate/check_gate_test_integrity.sh` 加 `\|\| true` after `cargo test` 扫描 + 测试用例
- [ ] 1.5 跑 4 个 gate 端到端并保存 log
- [ ] 1.6 写 `docs/releases/v3.12.0/V312-29_gate_wiring_report.md`

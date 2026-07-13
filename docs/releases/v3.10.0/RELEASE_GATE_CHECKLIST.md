# v3.10.0 Release Gate Checklist

**版本**: v3.10.0
**阶段**: RC (2026-07-13)
**更新日期**: 2026-07-13

---

## 用途

本文档追踪 v3.10.0 RC → GA 发布门禁中每个检查项的完成状态。每个门禁脚本执行后，结果应同步更新至此文档。最终由 human architect 签署确认 GA 发布。

参考: `docs/governance/STAGE_CONFIG.yaml` §RC + §GA

---

## R1: 必需文件 (Required Files)

| # | 文件 | 状态 | 备注 |
|---|------|------|------|
| R1.1 | `docs/releases/v3.10.0/STAGE.yaml` | ✅ | state=RC |
| R1.2 | `docs/releases/v3.10.0/RELEASE_NOTES.md` | ✅ | RC stage updated |
| R1.3 | `docs/releases/v3.10.0/CHANGELOG.md` | ✅ | RC entry added |
| R1.4 | `docs/releases/v3.10.0/GA_GATE_REPORT.md` | ✅ | Forward-looking draft |
| R1.5 | `docs/governance/STAGE_CONFIG.yaml` | ✅ | Framework SSOT |

## R2: 门禁脚本 (Gates)

| # | 脚本 | 预期 | 实际 | 状态 |
|---|------|------|------|------|
| R2.1 | `check_arch_invariants.sh` | PASS | PASS | ✅ |
| R2.2 | `check_arch3_no_bypass.sh` | PASS | PASS | ✅ |
| R2.3 | `check_arch_sem_debt.sh` | PASS (drift_ok) | PASS | ✅ |
| R2.4 | `check_cross_version_debt.sh` | PASS | PASS | ✅ |
| R2.5 | `check_int_debt.sh` | PASS (drift_ok) | PASS | ✅ |
| R2.6 | `check_anti_fabrication.sh` | PASS | FAIL (test compile) | ❌ |
| R2.7 | `check_full_gate_verification.sh` | PASS | TBD | ⏳ |
| R2.8 | `check_drift_not_pass.sh` | PASS | TBD | ⏳ |

## R3: Cargo 构建/测试/格式/clippy

| # | 检查 | 命令 | 状态 | 备注 |
|---|------|------|------|------|
| R3.1 | Build | `cargo build --all-features` | ✅ | |
| R3.2 | Test (lib) | `cargo test --all-features --lib` | ✅ | 600+/609+ PASS |
| R3.3 | Format | `cargo fmt --check` | ✅ | 0 diffs |
| R3.4 | Clippy | `cargo clippy --all-features -- -D warnings` | ✅ | 0 errors |
| R3.5 | Test compile | `cargo test --workspace --no-run` | ❌ | expr_single_engine_test |

## R4: E2E 场景

| # | 场景 | 脚本 | 状态 | 备注 |
|---|------|------|------|------|
| E2E-01 | 启动 + 连接 + SELECT 1 | TBD | ⏳ | Shell script not created |
| E2E-02 | TPC-H SF=0.1 22 queries | TBD | ⏳ | |
| E2E-04 | kill -9 recovery | TBD | ⏳ | Tests exist as .rs |
| E2E-05 | Backup + Restore | TBD | ⏳ | |
| E2E-06 | sysbench prepare/run/cleanup | TBD | ⏳ | |
| E2E-07 | ALTER TABLE RENAME | TBD | ⏳ | |
| E2E-08 | ROLLBACK MVCC | TBD | ⏳ | |
| E2E-09 | UNION/INTERSECT/EXCEPT | TBD | ⏳ | |

## R5: `#[ignore]` 债务

| # | 指标 | 值 | 阈值 | 状态 |
|---|------|-----|------|------|
| R5.1 | Ignore count (excluding intentional) | 8 | ≤ 10 | ✅ |

## R6: 覆盖率

| # | 指标 | 值 | 阈值 | 状态 |
|---|------|-----|------|------|
| R6.1 | Coverage baseline exists? | Not yet | Must exist | ⏳ |
| R6.2 | Per-crate ≥ 80% | TBD | ≥ 80% RC | ⏳ |

## R7: 跨版本债务

| # | 指标 | 值 | 阈值 | 状态 |
|---|------|-----|------|------|
| R7.1 | OPEN debt items | 0 | 0 | ✅ |

## R8: 性能基线

| # | 指标 | v3.9.0 | v3.10.0 | 退化 |
|---|------|--------|---------|------|
| R8.1 | Baseline report dir | — | Not yet | ⏳ |
| R8.2 | TPC-H SF=0.1 total time | ~2.3s | TBD | TBD |
| R8.3 | sysbench TPS (8t) | ~150 | TBD | TBD |

---

## GA 额外门禁

| # | 脚本 | 状态 | 备注 |
|---|------|------|------|
| G1 | `check_architecture_freeze.sh` | ⏳ | 架构冻结检查 |
| G2 | `check_gate_self_verification.sh` | ⏳ | 门禁自检 |
| G3 | `check_gate_test_integrity.sh` | ⏳ | 门禁测试完整性 |
| G4 | `check_soak_gate.sh` (optional) | ⏳ | SOAK 门禁 |

---

## 签署

| 角色 | 签署人 | 日期 | 状态 |
|------|--------|------|------|
| Code Owner | openclaw | — | ⏳ |
| Human Architect | (TBD) | — | ⏳ |
| Release Manager | (TBD) | — | ⏳ |

---

## 参考资料

- `STAGE_CONFIG.yaml` — 阶段框架 SSOT
- `STAGE.yaml` — 当前版本状态
- `GA_GATE_REPORT.md` — GA 门禁报告 (forward-looking)
- `check_rc_gate_v3.10.0.sh` — RC 门禁脚本

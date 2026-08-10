# V312-29: Gate Enforcement 接线 — proposal

> **Issue**: V312-29（V312-24 Phase 6 follow-up）
> **Owner**: minimax
> **Expiry**: 2026-08-30
> **Source**: V312-24 tasks.md Phase 6

## Why

V312-24 激活的 sqlancer / test-runner 二进制还**没**接进 CI gate：
- `scripts/test/run-regression.sh:111-112` 仍然 `|| true` 掩盖 sqlancer
- `scripts/gate/check_beta_gate.sh:363` B10_SQLANCER 仍 `check_warn`
- `scripts/gate/check_rc_gate_v3.10.0.sh:132-135` R4 substring 仍含 8 个将退休的脚本名
- `scripts/gate/check_gate_test_integrity.sh` (P16) 不扫 `|| true` after cargo test

如果不在 V312-24 收尾时接通，V312-25 删 E2E 脚本后 R4 匹配会断。

## 关闭边界（必须全部满足，命令 + 数字 + 文件存在）

1. `grep -n '|| true' scripts/test/run-regression.sh | grep -i sqlancer` 输出 **0 行**；同文件 sqlancer 调用行紧跟 `test -s target/sqlancer-report.json || { echo "missing report"; exit 1; }`。
2. `bash scripts/gate/check_beta_gate.sh 2>&1 | grep B10_SQLANCER` 输出含 `check_fail`（不再是 `check_warn`）；`bash scripts/gate/check_beta_gate.sh` 退出码 **0** 且日志含 `B10_SQLANCER PASS`。
3. `scripts/gate/check_rc_gate_v3.10.0.sh` 内 R4 substring match 列表只剩 4 个 active script（alter_rename / rollback_mvcc / union_set_ops / e2e_runner_exec）；`grep -E 'alter_rename|rollback_mvcc|union_set_ops|e2e_runner_exec' scripts/gate/check_rc_gate_v3.10.0.sh | wc -l` 输出 **≥ 4**。
4. `scripts/gate/check_gate_test_integrity.sh` 加 `\|\| true` after `cargo test` 扫描；`bash scripts/gate/check_gate_test_integrity.sh` 退出 **0** 且测试用例 `cargo_run_with_or_true_masking_is_rejected` PASS。
5. 关闭报告：`docs/releases/v3.12.0/V312-29_gate_wiring_report.md`，含 4 个 gate 的执行 log + diff stat + sha256。

## 禁止关闭条件

仅靠"删了 `|| true`"或"脚本能跑"关闭，必须含**真实 gate 端到端**输出。

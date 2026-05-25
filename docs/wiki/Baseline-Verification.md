# Baseline Verification

基线验证 (Baseline Verification) 是 SQLRustGo 治理系统 R5 规则的具体实现，确保 v2.8.0 分支的所有测试处于已知良好状态。

## 基线参数

- **分支**: `governance/v2.8.0`
- **标签**: `v2.8.0 (governance baseline)`
- **测试计数**: 226 PASS, 0 FAIL, 0 IGNORED
- **状态**: baseline_verified = true

## 验证流程

1. **全量测试执行**: `cargo test` 运行所有测试
2. **计数校验**: 验证测试数量与基线一致 (≥226)
3. **失败检查**: 确认 0 FAIL
4. **Ignore 检查**: 确认 0 IGNORED (R2)
5. **证明真实性校验**: 确认 proof files 与测试结果一致 (R3)
6. **CI 完整性**: 验证 CI 流程执行了所有规则 (R7)

## 相关技能

- [baseline-verification](../wiki/baseline-verification) — SQLRustGo Baseline Verification System 技能
- [hermes-gate](../wiki/hermes-gate) — Hermes Gate PR 规则系统，与基线验证协同

## 相关页面

- [governance-system](../wiki/governance-system) — 基线验证是治理规则 R5 的实现
- [proof-system](../wiki/proof-system) — 基线验证生成 proof files
- [sqlrustgo](../wiki/sqlrustgo) — 被验证的数据库引擎
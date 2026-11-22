> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0109-run3-20261121T022035.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-09 TEST DESIGN

## 具体测试方法
- 后端：FileStorage 生产路径（`crates/storage/src/file_storage.rs`）；对照装配 PLAIN vs BUFPOOL 按包需要。
- 事务脚本：BEGIN → DML → ROLLBACK → 观测可见状态；持久化路径按显式 flush 契约（参照 tests/integration/e2e_crash_recovery_proof.rs）。
- 断言：多重集比较（顺序无关）+ 精确值比较，避免顺序假阳性。
- 复现：`cargo test -p sqlrustgo --test ca01_backend_caps`，期望退出码 101（存在 RED 时）；原始日志 `takeover/dsh-b/artifacts/c0109-run3-20261121T022035.log`。

## 反向变异（负向控制）
- 契约 negative_control：移除目标断言/策略/证据绑定应被拒绝（由 verify 工具与 [[test]] 声明的确定性校验承担）。
- MUTATION_RED 由 `tools/verify_srg_d004_acceptance.py` 在独立 git worktree 执行（SRG-D004-01 专用），本包不重复实现。

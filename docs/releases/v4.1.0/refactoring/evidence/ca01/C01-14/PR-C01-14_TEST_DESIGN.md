> source_agent=DSH-B · source_run=dshb-owner-ruling-r2 · timestamp=2026-11-21T04:35+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0114-run-*.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；Owner 2026-11-21 裁决）

# PR-C01-14 TEST DESIGN

## 具体测试方法
- 后端：FileStorage 生产路径；同存储双引擎模拟两连接可见性。
- 事务脚本：BEGIN → SAVEPOINT → DML → ROLLBACK TO SAVEPOINT → 顶层 ROLLBACK → 观测可见状态。
- 断言：精确值/行数比较；期望值取自目标契约（不取自当前实现）。
- 复现：`cargo test -p sqlrustgo --test ca01_visibility_oracle`，期望退出码 101；原始日志 `takeover/dsh-b/artifacts/c0114-run-*.log`。

## 反向变异（负向控制）
- 契约 negative_control：移除目标断言/策略/证据绑定应被拒绝（由确定性校验承担）。
- MUTATION_RED 由 `tools/verify_srg_d004_acceptance.py` 在独立 git worktree 执行（SRG-D004-01 专用），本包不重复实现。

> source_agent=DSH-B · source_run=dshb-owner-ruling-r2 · timestamp=2026-11-21T04:35+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0114-run-*.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；Owner 2026-11-21 裁决）

# PR-C01-14 TEST PLAN

## 覆盖目标/阶段
- 阶段：C-ALPHA-01（C01-14）；基线 commit：`e9ff21fca3`（develop tip，无漂移）
- 执行命令：`cargo test -p sqlrustgo --test ca01_visibility_oracle`
- 关联：#5913 族、#5909 族（隔离/undo）

## 本包实测结果
| 测试 | 状态 | 说明 |
|---|---|---|
| inv04a_rollback_to_savepoint_preserves_earlier_work | **PASS** | ROLLBACK TO SAVEPOINT 只撤销其后写入；顶层 ROLLBACK 仍撤销其前 |
| inv04b_savepoint_cursor_is_transaction_local | **PASS** | savepoint cursor 仅属本 tx（A 的部分回滚不污染 B） |
| inv02_update_primary_key_rollback_restores_original_key | **PASS** | UPDATE 主键后回滚完整恢复原主键（行身份不变） |
| inv15_regression_index_and_rowstore_agree_at_committed_state | **PASS** | 索引投影与 committed RowStore 一致 |
| **inv13_uncommitted_state_invisible_to_peer_engine** | **RED** | 未提交状态对 peer 引擎可见（2 vs 期望 1）——与 inv07a 同源 |

合计 5 项：PASS 4 / RED 1；退出码 101。

## 诚实边界
- RED 为真实缺陷捕获（当前实现不满足已批准不变量），非测试缺陷；不得通过删改断言换取绿色。
- 未使用 ignore/skip；ignored 计数为 0。

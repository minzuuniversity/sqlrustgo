> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0107-run-20261121T020416.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-07 TEST PLAN

## 覆盖目标/阶段
- 阶段：C-ALPHA-01（C01-07）；基线 commit：`e9ff21fca3`（develop/v4.1.0 tip，无漂移）。
- 执行命令：`cargo test -p sqlrustgo --test ca01_commit_frontier`
- 门禁关联：C01-07 required_tests
- 关联 Issue/ADR：ADR-006 条件批准（M-Pre 证据）

## 本包实测结果
| 测试 | 状态 | 说明 |
|---|---|---|
| `c01_07a_commit_durability_without_explicit_flush` | **RED** | COMMIT 返回 ACK 后不 flush 直接重开，已提交两行全部丢失 → 当前实现同步提交不隐含持久化，与 ADR-006 目标契约（sync ACK ⇒ durable_lsn≥commit_lsn）存在结构性差距 |
| `c01_07b_two_commit_entry_points_consistent` | **PASS** | commit_transaction 与 commit_transaction_lockfree_for 两入口可见状态一致 |

合计 2 项：PASS 1 / RED 1。

## 诚实边界
- RED 为**真实缺陷捕获**（当前实现不满足已批准不变量），非测试缺陷；不得通过删除/跳过/弱化断言消除。
- 本包测试在架构修复落地前会保持 RED；合并时机须与对应实现同批（属主/Codex 决策）。
- 不在范围内：大型 SOAK、真实断电验证、跨后端能力矩阵全量（属后续阶段）。

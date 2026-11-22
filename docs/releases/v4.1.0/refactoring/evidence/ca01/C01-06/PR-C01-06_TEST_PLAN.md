> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0106-run2-20261120T211934.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-06 TEST PLAN

## 覆盖目标/阶段
- 阶段：C-ALPHA-01（C01-06）；基线 commit：`e9ff21fca3`（develop/v4.1.0 tip，无漂移）。
- 执行命令：`cargo test -p sqlrustgo --test ca01_snapshot_reopen`
- 门禁关联：C01-06 required_tests
- 关联 Issue/ADR：#5913（族）、#5909 裁决材料

## 本包实测结果
| 测试 | 状态 | 说明 |
|---|---|---|
| `inv07a_uncommitted_invisible_to_other_engine` | **RED** | 同存储双引擎：B 在 A 事务打开时读到未提交行 → 脏读（与 p2c1iso 同源） |
| `inv07b_reopen_baseline_excludes_uncommitted` | **RED** | 事务打开时 flush 将未提交行 (2,20) 写入 recovery 基线 → 未提交状态随盘持久化（INV-07/F03 违反） |
| `inv07c_rollback_then_reopen_is_clean` | **PASS** | ROLLBACK+flush+重开后快照仅含 committed 行 |

合计 3 项：PASS 1 / RED 2。

## 诚实边界
- RED 为**真实缺陷捕获**（当前实现不满足已批准不变量），非测试缺陷；不得通过删除/跳过/弱化断言消除。
- 本包测试在架构修复落地前会保持 RED；合并时机须与对应实现同批（属主/Codex 决策）。
- 不在范围内：大型 SOAK、真实断电验证、跨后端能力矩阵全量（属后续阶段）。

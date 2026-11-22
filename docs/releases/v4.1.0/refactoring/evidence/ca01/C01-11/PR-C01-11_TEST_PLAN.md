> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0111-final-20261121T031756.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-11 TEST PLAN

## 覆盖目标/阶段
- 阶段：C-ALPHA-01（C01-11）；基线 commit：`e9ff21fca3`（develop/v4.1.0 tip，无漂移）。
- 执行命令：`cargo test -p sqlrustgo --test ca01_bufferpool`
- 门禁关联：C01-11 required_tests
- 关联 Issue/ADR：业主指令点名问题（buffer_pool 绕过 Undo）、ADR-009

## 本包实测结果
| 测试 | 状态 | 说明 |
|---|---|---|
| `inv14_bp_update_undo_applied` | **PASS** | buffer_pool 装配下 UPDATE 的撤销生效 |
| `inv14_bp_delete_insert_undo_applied` | **RED** | buffer_pool 装配下 DELETE 撤销生效但 INSERT 撤销未生效（未提交插入行在 ROLLBACK 后仍存在）→ 该装配绕过 Undo（业主指令点名问题，PLAIN 装配同序列正常） |
| `inv14_bp_adds_no_extra_transaction_semantics` | **RED** | buffer_pool 装配下无 WHERE 的 scan 可见，但 WHERE/PK 过滤查询返回空 → 读路径在该装配下丢行 |
| `inv15_bp_and_plain_assembly_semantically_equal` | **RED** | buffer_pool 与普通装配在同一 DML 序列后可见状态不一致 |

合计 4 项：PASS 1 / RED 3。

## 诚实边界
- RED 为**真实缺陷捕获**（当前实现不满足已批准不变量），非测试缺陷；不得通过删除/跳过/弱化断言消除。
- 本包测试在架构修复落地前会保持 RED；合并时机须与对应实现同批（属主/Codex 决策）。
- 不在范围内：大型 SOAK、真实断电验证、跨后端能力矩阵全量（属后续阶段）。

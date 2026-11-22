> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0110-run-20261121T030705.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-10 TEST PLAN

## 覆盖目标/阶段
- 阶段：C-ALPHA-01（C01-10）；基线 commit：`e9ff21fca3`（develop/v4.1.0 tip，无漂移）。
- 执行命令：`cargo test -p sqlrustgo --test ca01_complex_dml`
- 门禁关联：C01-10 required_tests
- 关联 Issue/ADR：#5905/#5909 undo 族、ADR-005/008

## 本包实测结果
| 测试 | 状态 | 说明 |
|---|---|---|
| `inv05a_multi_row_update_rollback_is_all_or_nothing` | **RED** | 单条 UPDATE 影响 4 行 → ROLLBACK 后 4 行变 8 行（每行重复一次）：INV-05 多行共同撤销被违反 |
| `inv05b_multi_table_dml_rollback_is_atomic` | **RED** | 跨表 DML（两表 insert+update+delete）→ ROLLBACK 后表 a 行重复（1→2）：INV-05 跨表共同撤销被违反 |
| `inv15a_index_projection_matches_rowstore` | **PASS** | 索引投影与 RowStore 扫描在 committed 状态一致 |
| `inv15b_index_projection_undone_with_rowstore` | **PASS** | 回滚后索引投影与主表同步撤销，无陈旧索引项 |

合计 4 项：PASS 2 / RED 2。

## 诚实边界
- RED 为**真实缺陷捕获**（当前实现不满足已批准不变量），非测试缺陷；不得通过删除/跳过/弱化断言消除。
- 本包测试在架构修复落地前会保持 RED；合并时机须与对应实现同批（属主/Codex 决策）。
- 不在范围内：大型 SOAK、真实断电验证、跨后端能力矩阵全量（属后续阶段）。

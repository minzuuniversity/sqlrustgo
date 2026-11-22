> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0105-run4-20261120T195151.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-05 TEST PLAN

## 覆盖目标/阶段
- 阶段：C-ALPHA-01（C01-05）；基线 commit：`e9ff21fca3`（develop/v4.1.0 tip，无漂移）。
- 执行命令：`cargo test -p sqlrustgo --test ca01_file_undo`
- 门禁关联：C01-05 required_tests
- 关联 Issue/ADR：#5913（族）、#5905/#5908/#5909/#5910

## 本包实测结果
| 测试 | 状态 | 说明 |
|---|---|---|
| `inv01a_rollback_restores_exact_multiset` | **RED** | 事务内 insert+delete+update 混合 → ROLLBACK 后 3 行变 6 行（4×(1,10)+2×(2,20)）：INV-01「每个 mutation 只撤销一次」被违反（delete-undo/update-undo 叠加） |
| `inv01b_mutation_revoked_exactly_once` | **PASS** | 单行 PK 连续两次 UPDATE，ROLLBACK 恰好撤销一次恢复原值 7 |
| `inv02_identical_rows_have_independent_rowids` | **RED** | 无主键表两行完全相同，删除一行后 ROLLBACK → 2 行变 4 行：INV-02「相同行值独立 RowId」被违反（#5908 口径） |

合计 3 项：PASS 1 / RED 2。

## 诚实边界
- RED 为**真实缺陷捕获**（当前实现不满足已批准不变量），非测试缺陷；不得通过删除/跳过/弱化断言消除。
- 本包测试在架构修复落地前会保持 RED；合并时机须与对应实现同批（属主/Codex 决策）。
- 不在范围内：大型 SOAK、真实断电验证、跨后端能力矩阵全量（属后续阶段）。

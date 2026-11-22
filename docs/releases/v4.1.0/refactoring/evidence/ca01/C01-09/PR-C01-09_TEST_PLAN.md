> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0109-run3-20261121T022035.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-09 TEST PLAN

## 覆盖目标/阶段
- 阶段：C-ALPHA-01（C01-09）；基线 commit：`e9ff21fca3`（develop/v4.1.0 tip，无漂移）。
- 执行命令：`cargo test -p sqlrustgo --test ca01_backend_caps`
- 门禁关联：C01-09 required_tests
- 关联 Issue/ADR：#5914（D-004）、ADR-009 组合态认证输入

## 本包实测结果
| 测试 | 状态 | 说明 |
|---|---|---|
| `c01_09b_insufficient_capability_must_refuse_before_write` | **RED** | 非事务后端 begin_transaction 返回 Ok → INV-14「capability 不足写前拒绝」被违反（与 tf_t4_04 事务侧口径互证，同源 #5914） |

合计 1 项：PASS 0 / RED 1。

## 诚实边界
- RED 为**真实缺陷捕获**（当前实现不满足已批准不变量），非测试缺陷；不得通过删除/跳过/弱化断言消除。
- 本包测试在架构修复落地前会保持 RED；合并时机须与对应实现同批（属主/Codex 决策）。
- 不在范围内：大型 SOAK、真实断电验证、跨后端能力矩阵全量（属后续阶段）。

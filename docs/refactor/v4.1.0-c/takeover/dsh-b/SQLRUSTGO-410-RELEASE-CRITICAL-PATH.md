# SQLRUSTGO-410-RELEASE-CRITICAL-PATH（发布关键路径 · DSH-B）

- source_run: dshb-first-round-20261120-4
- 权威源：HANDOVER_sqlrustgo_v4.1.0_20261120.md（上一任 Worker 交接清单，2026-11-20T02:2x 生成）+ STAGE.yaml（ALPHA）
- 口径声明：以下为 2026-11-20 快照；每项晋级前必须以 Gitea API/最新树复跑核实，不继承手抄数字。

## 关键路径（依赖序）

```
[现在] ALPHA
  CP1  36 open P0 清零（实查 2026-11-19T18:24Z=36 条；A 源码10/B CI5/C 治理7/D IAIA-410·8/E 阶段1/F undo族5）
  CP2  9 个 open PR 合并（属主人工动作；全部 base develop/v4.1.0；undo 相关 #5903 依赖 #5899/#5902）
  CP3  架构级未决项定调（#5909/#5908/#5910 裁决已出待属主）
  CP4  #5905 修复保留但一般情形未解；#5906 修法未定
  CP5  独立 Reviewer 回执（AB-10：R1-R9 已交付待审）
  CP6  权威 35 项门禁轮（最终树；019 轮跑的是旧树）
  CP7  全新 168h SOAK（独占宿主；r5 独占性 VIOLATED×5 + 4 笔读超时）
  CP8  C 路线重构里程碑（Codex M0-M6，经 DSH-B 执行）
  [出口] ALPHA → BETA 人工门禁
```

## DSH-B 对关键路径的贡献顺序

1. **CP1-F（undo 族）**：D-002 候选 ODKU 探针已存在（a802a0c4b5）；RED 固化 + 与 #5905 路径合并 → 直接削减 F 族。
2. **CP1-A**：75 场景复跑把"声明 PASS"换成"实测 PASS"，清理源码/测试完整性类 P0 的证据基础。
3. **CP1-B（CI 基础设施 5 条）**：门禁负向验证 + CI 失败传播实测（S4）。
4. **CP6**：35 项门禁轮必须在最终树上跑（019 轮旧树教训）——DSH-B 接线后以确定性 runner 执行。
5. **CP7**：SOAK 不在本轮授权（属主令：大型压测暂不启用）。

## 不做的事

- 不自动合并 PR / 不推进 BETA/RC/GA / 不关闭属主确认的 Issue / 不启动大型 SOAK / 不抢占 GMP 资源。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)

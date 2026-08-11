# v3.12.0 Alpha Gate Coverage Audit

> **provenance:** generated_by=codex, generated_at=2026-08-11T12:00:00+08:00, source_repo=openclaw/sqlrustgo, branch=codex/v312-alpha-gate-governance, policy=ADR-001+AFP-v1.0, evidence_type=script-and-document-diff

## 1. 结论

用户关于 “开发工作尚未全部完成，为什么 Alpha gate 还能通过” 的疑问是合理的。

原 `scripts/gate/check_alpha_v3.12.0.sh` 更接近 **Alpha Entry Check**：它检查文档存在、链接一致、脚本可执行和 SQLLogicTest smoke 入口，而不是检查功能完成度、Hard Gate 状态或延期项是否经总控批准。因此，原脚本输出 `ALPHA GATE ENTRY PASS` 容易被误读为质量门禁通过。

本次整改把 Alpha 拆成三层：

| 层级 | 脚本 | 结论含义 |
|---|---|---|
| Entry | `scripts/gate/check_alpha_entry_v3.12.0.sh` | 可以进入 Alpha 开发准备 |
| Deferred Approval | `scripts/gate/check_v312_deferred_followups.sh` | 失败项有 Gitea Issue、owner、expiry、close boundary |
| Quality | `scripts/gate/check_alpha_quality_v3.12.0.sh` | Hard Gate 可支撑 Alpha 质量声明 |

## 2. 原覆盖缺口

| 缺口 | 原行为 | 风险 | 新拦截者 |
|---|---|---|---|
| SQLLogicTest 失败项注册后被误读为 PASS | `exclusions.yml` 字段完整即可通过 smoke gate | 16 个失败文件被隐藏为“可接受” | `check_alpha_quality_v3.12.0.sh` + `check_v312_deferred_followups.sh` |
| OpenSpec-only follow-up | 目录存在即可 | 无 Gitea Issue 关闭验证链 | `check_v312_deferred_followups.sh` |
| P12 ignore registry 失败 | 不在 Alpha gate | ignored tests 膨胀 | `check_alpha_quality_v3.12.0.sh` |
| P16 gate test integrity 失败 | 不在 Alpha gate | gate-referenced tests 可被 ignore | `check_alpha_quality_v3.12.0.sh` |
| anti-ignore budget 失败 | 不在 Alpha gate | registry 膨胀替代修复 | `check_alpha_quality_v3.12.0.sh` |

## 3. 新状态定义

| 状态 | 允许声明 |
|---|---|
| `ALPHA ENTRY PASS` | 规划材料和工具入口可用于进入 Alpha 开发准备 |
| `DEFERRED FOLLOW-UP VISIBILITY PASS` | 延期项可追踪，不代表功能完成 |
| `ALPHA QUALITY PASS` | 可声明 Alpha 质量门禁通过，必须带日志和 evidence hash |
| `ALPHA ENTRY PASS / ALPHA QUALITY BLOCKED` | 可以开发，但不能声称 Alpha gate 质量通过 |

## 4. 执行要求

关闭 #3887 或任何与 v3.12 gate 相关的 Issue 前，必须从 `origin/develop/v3.12.0` 合并后的代码实跑：

```bash
bash scripts/gate/check_alpha_entry_v3.12.0.sh
bash scripts/gate/check_v312_deferred_followups.sh
bash scripts/gate/check_alpha_quality_v3.12.0.sh
bash scripts/gate/check_alpha_v3.12.0.sh
```

如果 `check_alpha_quality_v3.12.0.sh` 失败，文档和 Issue 评论只能写 `ALPHA QUALITY BLOCKED`，不能写 `ALPHA GATE PASS`。

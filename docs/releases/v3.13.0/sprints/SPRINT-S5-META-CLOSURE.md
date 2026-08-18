# SPRINT-S5: Meta Issue Closure (Cluster D)

## §1 Scope

闭合 #3887 (V312-MASTER 总控) + #4220 (V312-47 PARTIAL 总控)。两个 meta issue 必须等所有 sub-issue 关闭后才能关闭。

## §2 Entry 标准

- [x] SPRINT-S0 完成 (blocker 移除)
- [ ] SPRINT-S1 完成 (#4225 + #4226 关闭)
- [ ] SPRINT-S2 完成 (#4221 + #4272 关闭)
- [ ] SPRINT-S3 完成 (#4273-#4279 关闭)
- [ ] SPRINT-S4 完成 (#4250-#4258 关闭)

## §3 入口任务

### §3.1 #3887 V312-MASTER 总控

- [ ] **验证所有 V312-NN 子项关闭状态** — 扫 `git log --all --grep="closes #3887"` + 子 issue 列表
- [ ] **写 V313-VERIFICATION.md 综合卷宗** — SHA-256 汇总 (24 sub-issue + 7 sprint plans)
- [ ] **关闭 PR (merged to `develop/v3.13.0`)** — 引用所有 sub-issue PR 链接

### §3.2 #4220 V312-47 PARTIAL 总控

- [ ] **验证 PARTIAL 子项状态** — #4216/#4221/#4225/#4226/#4250-#4258/#4272-#4279
- [ ] **写 V313-PARTIAL-FINAL-VERIFICATION.md** — PARTIAL → FINAL 路径文档
- [ ] **关闭 PR (merged to `develop/v3.13.0`)** — 把 PARTIAL 升级为 FINAL

## §4 退出标准

- [ ] #3887 + #4220 各自 merged PR 到 `develop/v3.13.0`
- [ ] `V313-VERIFICATION.md` 包含全部 SHA-256 hash 汇总
- [ ] `V313-PARTIAL-FINAL-VERIFICATION.md` 已生成
- [ ] Anti-Fabrication-Policy-v1.0:任何遗留 DEFERRED 项必须绑定 expiry 2027-06-30

## §5 已知 Deferral 路径 (per V313-STRICT-CLOSE-STANDARDS.md §4)

若任一 sub-issue 未能按 V313-MASTER-PLAN.md §6 过期路径在 2027-06-30 前完成:

| 项 | 强制绑定 |
|---|---|
| tracking issue | #4313 (v3.13-MASTER) |
| owner | openclaw |
| expiry | 2027-06-30 |
| closing boundary | 具体可验证标准 (例 "SF=1 fixture 实测 PASS" 或 "planner reorder 修复") |

任何 `DEFERRED-without-tracking` 标记都属 Policy 违反 (per §3)。

## §6 风险与依赖

| Risk | 影响 | Mitigation |
|---|---|---|
| 任何 sub-issue 未关闭时本 sprint 开始 | early commit 阻断后续 verify | hard-gate: 等所有 S1-S4 issue 状态确认 |
| V313-VERIFICATION.md SHA-256 索引不全 | audit trail 残缺 | 用 `find docs/releases/v3.13.0/ -name "*.md" \| xargs sha256sum` 自动生成 |

## §7 依赖

- SPRINT-S1 ✅ 必须先关闭
- SPRINT-S2 ✅ 必须先关闭
- SPRINT-S3 ✅ 必须先关闭
- SPRINT-S4 ✅ 必须先关闭
- 后续:无 (S5 为收尾)

## §8 预估工作量

约 1 周 (主要是卷宗文档聚合,不涉及代码)。

## §9 References

- 总控 scope: `V313-MASTER-PLAN.md`
- 严格关闭标准: `V313-STRICT-CLOSE-STANDARDS.md`
- 24 sub-issue 索引: `V313-FOLLOWUP-INDEX.md`
- 上游 PARTIAL 状态: `1da964be77` (per memory `v312-47-48-meta-closure.md`)

## Evidence Hash

`sha256=88eb808ed611c94ecff2011cda7ea1c0d94873d7868035987f2839ca4f432460` (computed on file content at HEAD)
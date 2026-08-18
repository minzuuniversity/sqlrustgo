# SPRINT-S4: V312-56 4.0 前补强 (Cluster C — 教学 lab)

## §1 Scope

闭合 #4250-#4258 (1 总控 + 8 sub-issue)。把 V312-56A~56H (4.0 前教学 lab) 各自一个可执行教学实验,跑通 + 证据 SHA-256 锚定。背景文档: `docs/releases/v3.12.0/V312-56_V400_PREP_AND_MYSQL_TEACHING_PLAN.md` + `docs/releases/v3.12.0/issues/V312-56_TEACHING_AND_V400_REMEDIATION_ISSUE_BODIES.md`。

## §2 Entry 标准

- [x] SPRINT-S0 blocker 移除完成
- [x] `V312-56_TEACHING_AND_V400_REMEDIATION_ISSUE_BODIES.md` 已存在 (per PR #3713)
- [x] V312-56A residual 4 sub-tasks 已枚举 (`d65633f58d`: 56A-R1/R2/R3/R4)
- [x] V313-STRICT-CLOSE-STANDARDS.md 已合并

## §3 入口任务 (9 个 sub-issue)

### §3.1 #4250 V312-56 总控 (meta)

- [ ] 验证所有 56A~56H 子项关闭状态
- [ ] 写 `V312-56-VERIFICATION.md` v3.13.0 综合卷宗 (SHA-256 汇总)
- [ ] 关闭 PR (merged to `develop/v3.13.0`)

### §3.2 8 个 sub-issue (#4251-#4258) — 各自一个独立教学 lab

| Issue | Lab 主题 | Lab 路径 | 验收命令 |
|---|---|---|---|
| #4251 | 56A Metadata/SHOW/information_schema | `docs/sql-feature-corpus/labs/56a-metadata-show/` | `bash scripts/gate/check_v313_56a_lab.sh` |
| #4252 | 56B SQL 教学 corpus | `docs/sql-feature-corpus/labs/56b-teaching-corpus/` | `bash scripts/gate/check_v313_56b_lab.sh` |
| #4253 | 56C Transaction/crash recovery lab | `docs/sql-feature-corpus/labs/56c-tx-recovery/` | `bash scripts/gate/check_v313_56c_lab.sh` |
| #4254 | 56D Prepared/wire lab | `docs/sql-feature-corpus/labs/56d-prepared-wire/` | `bash scripts/gate/check_v313_56d_lab.sh` |
| #4255 | 56E Optimizer/EXPLAIN | `docs/sql-feature-corpus/labs/56e-optimizer-explain/` | `bash scripts/gate/check_v313_56e_lab.sh` |
| #4256 | 56F VIEW/CTE/MERGE | `docs/sql-feature-corpus/labs/56f-view-cte-merge/` | `bash scripts/gate/check_v313_56f_lab.sh` |
| #4257 | 56G Partition/FullText | `docs/sql-feature-corpus/labs/56g-partition-fulltext/` | `bash scripts/gate/check_v313_56g_lab.sh` |
| #4258 | 56H Gate/docs/evidence | `docs/sql-feature-corpus/labs/56h-gate-evidence/` | `bash scripts/gate/check_v313_56h_lab.sh` |

### §3.N 每个 sub 任务结构 (统一模板)

1. **写教学 lab README** — 目标 / 前置知识 / 步骤 / 验收 (沿用 docs/sql-feature-corpus/ 已有 lab 格式)
2. **跑 lab 验证脚本** — 对照 evidence
3. **提交 + SHA-256 锚定** — `evidence/V313-S4-56<X>-LAB-SHA256.txt`
4. **关闭对应 issue** — V313-STRICT-CLOSE-STANDARDS.md §2 四要素

## §4 退出标准

- [ ] 9 个 issue 各自 merged PR 到 `develop/v3.13.0` (含 #4250 总控)
- [ ] 8 个教学 lab 在 `docs/sql-feature-corpus/labs/` 可访问
- [ ] `bash scripts/gate/check_v313_v312_56_gate.sh` exit=0 (聚合 8 lab gate)
- [ ] `evidence/V313-S4-V312-56-VERIFICATION.md` 已生成

## §5 已知 DEFERRED 项

| Item | 来源 | 处理 |
|---|---|---|
| 56A-R3 SHOW FULL TABLES / SHOW TABLE STATUS | `d65633f58d` | BETA scope out;教学 lab 写明 unsupported 不算 fail |
| 56A-R4 SHOW WARNINGS/ERRORS/STATUS/VARIABLES (parser-only) | `d65633f58d` | lab 演示 parser 接受但 runtime NOT_IMPL |
| 56G Partition/FullText 与 GMP keyword retrieval 决策一致 | `TEST_PLAN.md:122` | 等待 #4225 (SPRINT-S1) GMP keyword 决策后写 lab |

## §6 风险与依赖

| Risk | 影响 | Mitigation |
|---|---|---|
| 8 个 lab 文档 + 8 个 gate 脚本工作量 | S4 延期 | 沿用现有 `scripts/gate/check_v312_*.sh` 模板,文案复制 |
| 56D Prepared/wire lab 触发现有测试回归 | 影响其他模块 | 单独 feature flag 灰度 |
| 56G Partition/FullText scope 决策悬而未决 | 阻塞 #4257 | 等 S1 GMP 决策后写 |

## §7 依赖

- SPRINT-S0 ✅
- SPRINT-S1 (S4 子项 56G 等 GMP keyword 决策)
- 后续:SPRINT-S5 (收尾时把 #4250 关闭)

## §8 预估工作量

约 2 周 (教学 lab 主要是文档 + 可执行 demo,代码改动小)。

## §9 References

- 上游计划: `docs/releases/v3.12.0/V312-56_V400_PREP_AND_MYSQL_TEACHING_PLAN.md`
- 教学 lab 模板: `docs/sql-feature-corpus/` (已有 lab 路径)
- issue 详情: `docs/releases/v3.12.0/issues/V312-56_TEACHING_AND_V400_REMEDIATION_ISSUE_BODIES.md`
- 残余 sub-tasks: `d65633f58d` (56A-R1/R2/R3/R4)
- 严格关闭标准: `V313-STRICT-CLOSE-STANDARDS.md`

## Evidence Hash

`sha256=ad3bf3dc3b91beb260ff2615b274da96b581cbd46c425dcc17a7c427039d52b1` (computed on file content at HEAD)
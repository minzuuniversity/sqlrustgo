# v4.0.0 GA 发布时间线

> **版本**: v4.0.0
> **当前状态**: **DRAFT（`STAGE.yaml:13`）** — 与 `GA_GATE_REPORT.md`（2026-09-20，声明
> "GA CONDITIONAL PASS"）存在未裁决的阶段矛盾
> **负责人**: @openclaw
> **本文件创建**: 2026-09-30（GA 阶段 `required_files` 补全，见
> `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` §7.6）
> **provenance**: generated_by=ai (mavis/claude-macmini), generated_at=2026-09-30,
> source_run=local-gate-verification-2026-09-30, policy=Anti-Fabrication-Policy-v1.0
>
> ⚠️ **本文件不构成 GA 晋升声明。** 补全本文件只是满足
> `docs/governance/STAGE_CONFIG.yaml` GA 阶段 `required_files` 清单；
> 阶段是否晋升由门禁实跑结果 + 人工签字决定，**不由文件是否存在决定**。

---

## 1. 实际时间线（均为已发生事实）

| 日期 | 里程碑 | 状态 |
|---|---|---|
| 2026-09-08 | v4.0.0 遗留任务清单编制（`LEGACY_ISSUES.md` 首版） | 已完成 |
| 2026-09-17 | 遗留清单最后更新；V400-05/06/07 开发计划成文 | 已完成 |
| 2026-09-19 | v4.0.0 ALPHA / BETA / RC gate 报告出具 | 已完成 |
| 2026-09-19 | V400-09 168h SOAK 启动 | 已完成 |
| 2026-09-20 | `GA_GATE_REPORT.md` 判定 **GA CONDITIONAL PASS** | 已完成（但为条件通过） |
| 2026-09-21 | zombie-fix / workers.push / DML 回归修复合入 `develop/v4.1.0` | 已完成 |
| 2026-09-26 | v4.1.0 review queue 关闭（`9c6767a512`） | 已完成 |
| 2026-09-28 | 5 远端同步工具上线 | 已完成 |
| 2026-09-29 | v4.1.0 自我声明 ALPHA（**已因缺门禁证据回退，见 v4.1.0 违规记录**） | 已作废 |
| 2026-09-30 | v4.0.0 GA 门禁补跑；本文件补全 | 进行中 |

**注意**：上表中**没有** DRAFT→ALPHA→BETA→RC→GA 的 `STAGE_CONFIG` 规范流转记录。
v4.0.0 的 `STAGE.yaml` 始终停在 `current_stage: "DRAFT"`，从未按
`check_stage.sh` 驱动流转。这正是 2026-09-30 审计发现的核心矛盾。

---

## 2. GA 硬性阻塞项（2026-09-30 补跑实测）

### 2.1 覆盖率不达标 —— 决定性阻塞

| 项 | 数值 | 来源 |
|---|---|---|
| v4.0.0 实测平均覆盖率 | **78.28%** | `GA_GATE_REPORT.md` G3、实测记录 |
| `STAGE_CONFIG` `COVERAGE_MIN_RC` | **80%** | `STAGE_CONFIG.yaml:285` |
| `STAGE_CONFIG` `COVERAGE_MIN_GA` | **85%** | `STAGE_CONFIG.yaml:286` |
| 与 GA 门槛差距 | **−6.72 个百分点** | 计算 |

分 crate 明细（`GA_GATE_REPORT.md`）：

| crate | 覆盖率 | 判定 |
|---|---|---|
| `sqlrustgo-executor` | 82.57% | ✅ |
| `sqlrustgo-storage` | 80.82% | ✅ |
| `sqlrustgo-mysql-server` | 75.41% | ❌ |
| `sqlrustgo-parser` | 74.32% | ❌ |

> v4.0.0 连 **RC 门槛（80%）都未达到**，更未达到 GA 门槛（85%）。
> 78.28% 属 `GATE_CONDITIONS.md §A5` 的 **CONDITIONAL PASS**（2 周整改窗口），
> 而非 clean PASS。`CLAIM_DOWNGRADE_MANIFEST.md §4` 亦明写
> **"Full PASS target is v4.0.1"**。

### 2.2 GA 阶段 required_files 缺失

`check_stage.sh --version v4.0.0 --stage GA --dry-run` 实测：

| required_file | 补全前 | 补全后 |
|---|---|---|
| `GA_GATE_REPORT.md` | ✅ 已存在 | ✅ |
| `STAGE.yaml (state=GA)` | ❌ state=DRAFT | ❌ **仍未推进**（覆盖率未达标） |
| `RELEASE_NOTES.md (GA)` | ❌ 非 GA 态 | ❌ **仍未推进** |
| `CHANGELOG.md (GA entry)` | ❌ 无 GA 条目 | ❌ **仍未推进** |
| `GA_RELEASE_TIMELINE.md` | ❌ 不存在 | ✅ **本文件补全** |

### 2.3 ALPHA 阶段门禁脚本缺失

`STAGE_CONFIG.yaml` ALPHA 阶段要求 `scripts/gate/check_alpha_v{VER}.sh`。
`check_alpha_v400.sh` **不存在**（仅 `check_alpha_v410.sh` 存在），
因此 v4.0.0 的 ALPHA 门禁无法按框架执行，ALPHA→BETA→RC→GA 的
顺序补跑在第一步即断裂。

---

## 3. 结论：v4.0.0 当前**不满足** GA 晋升条件

| 条件 | 状态 |
|---|---|
| 覆盖率 ≥ 85% | ❌ 实测 78.28%，差 6.72 个百分点 |
| 阶段流转记录 | ❌ `STAGE.yaml` 始终 DRAFT，无规范流转 |
| ALPHA 门禁可执行 | ❌ `check_alpha_v400.sh` 缺失 |
| 全部 GA required_gates 实跑通过 | ⏳ 补跑进行中，见 `evidence/gate-backfill-2026-09-30/` |
| 人工架构师签字 | ❌ 未取得（`transitions.RC_to_GA.requires_human_approval: true`） |

**因此本文件不推进 v4.0.0 的 `current_stage`。** 保持 `DRAFT` 是与证据一致的状态。

### 晋升 GA 的三条可选路径（均需人工决策）

| 路径 | 做法 | 代价 |
|---|---|---|
| **A. 补覆盖率到 85%** | 为 `parser`(74.32%) / `mysql-server`(75.41%) 补测试，重跑覆盖率门禁 | 工作量最大，但是唯一能真正满足现有门槛的路径 |
| **B. 下调 GA 门槛** | 修改 `STAGE_CONFIG.yaml` 的 `COVERAGE_MIN_GA` | 影响所有版本的 GA 判定，属框架级变更，需评估面 |
| **C. 维持条件 GA 记录** | 承认 78.28% 为已知 caveat，v4.0.0 以 CONDITIONAL 状态结项，完整 PASS 目标顺延 v4.1.0 | 需在 `STAGE.yaml` 明确记录该口径，避免再次出现"SSOT 与 gate 报告矛盾" |

> `CLAIM_DOWNGRADE_MANIFEST.md §4` 原本即采用 C 的口径
> （"Full PASS target is v4.0.1"），但该口径**从未被写入 `STAGE.yaml`**，
> 这正是本次审计发现 SSOT 矛盾的原因之一。

---

## 4. 回滚原则

若 GA 后发现 hard gate 证据不成立，应：

1. 明确受影响的 claim 与证据缺口。
2. 在 `STAGE.yaml` 或相关 gate 报告中降级或标注限制。
3. 修复后重新执行对应 gate，附实跑输出。
4. 使用 PR / commit 记录整改过程。

---

## 5. 证据与参考

- `docs/governance/STAGE_CONFIG.yaml` — 阶段框架 SSOT（覆盖率门槛、required_files、required_gates）
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` — 2026-09-20 GA CONDITIONAL PASS 判定与覆盖率明细
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` §4 — Coverage G17 downgrade 原始声明
- `docs/releases/v4.0.0/STAGE.yaml` — v4.0.0 per-version SSOT（`current_stage: DRAFT`）
- `docs/releases/v4.0.0/evidence/gate-backfill-2026-09-30/` — 2026-09-30 GA 门禁补跑实跑输出
- `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` — 本文件补全的依据与上下文
- `docs/governance/GATE_CONDITIONS.md` §A5 — CONDITIONAL PASS 判定标准
- `docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md` — 关联的 AFP Type-B 违规记录

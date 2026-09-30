# v4.1.0 — PHASE_1_SCOPE

> **Date**: 2026-09-29
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Purpose**: Consolidated work plan for v4.1.0 PHASE 1 (DRAFT → ALPHA)

## 1. Scope overview

PHASE 1 of v4.1.0 is the work required to advance from DRAFT to ALPHA.
Per STAGE.yaml exit_criteria.draft_to_alpha, the gates are:

1. PHASE_0 docs published (DONE 2026-09-29 — this set of 9 docs)
2. First alpha tag cut: `v4.1.0-alpha1`

> **2026-09-30 实跑复核修正**：本节原称 3 个继承的 v4.0.0 alpha-gate FAIL
> 全部未解决。实跑后为 **2 项已解决、1 项未解决**。
> 证据：`docs/releases/v4.1.0/evidence/gate-runs-2026-09-30/`
> 与 `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` F-01/F-04。

| Gate | 原描述 | 实跑结果（2026-09-30） | 状态 |
|---|---|---|---|
| `scripts/gate/check_anti_ignore_gate.sh` | `ignore_registry.json` missing | exit 0，`total_allowed=125` | ⚠️ **退出码为 0 但门禁是空转**（见下） |
| `scripts/gate/check_arch_invariants.sh` | `execution_engine.rs` 2731 行 > 1600 | exit 0，5/5 PASS | ✅ 已解决 |
| `scripts/gate/check_anti_fabrication.sh` | HEAD email `v400@local` 不在白名单 | **CHECK 4 FAIL**（HEAD 实为 `claude@macmini.dev`） | ❌ **未解决** |

**原 §2.1.1 / §2.1.2 描述已作废**（详见 §2.1 内的更正说明）。

## 2. Workstreams

### 2.1 [P0] Resolve 3 inherited v4.0.0 alpha-gate FAILs

> **2026-09-30 状态更正**：2.1.1 / 2.1.2 描述的事实基础已不成立，标记为 DONE；
> 2.1.3 **仍未解决**，且原始描述已过时。依据 `ALIGNMENT_AUDIT_2026-09-30.md` F-01/F-04。

#### 2.1.1 `check_anti_ignore_gate.sh` — ⚠️ 文件已存在，但门禁空转（2026-09-30 二次复核）

- `tests/baseline/ignore_registry.json` **已存在**
- 实跑：`bash scripts/gate/check_anti_ignore_gate.sh` → **exit 0**
- 输出：`ignore_registry.json: total_allowed=125 (max=125), active=0 (max=47)`
- Evidence: `evidence/gate-runs-2026-09-30/check_anti_ignore_gate.txt`
  (sha256:07c999df3a955a41)

> **2026-09-30 二次复核：上条 exit 0 不可作为"已解决"的证据。**
> 该脚本**从不扫描源码树**——它只读 registry JSON 里自带的两个数字
> （`total_allowed`、`active`）与硬编码上限（125 / 47）比较。
> 实测：在 `crates/parser/tests/` 新造 50 个全新未登记 `#[ignore]` 后，
> 同一脚本仍输出 `active=0 (max=47)` 并 **exit 0**。
> 另：`active` 的计算条件是 `e.get("status") == "ACTIVE"`，而 registry
> 条目根本没有 `status` 字段（分布 `{None: 124, 'RETIRED': 1}`），
> 因此 `active` 恒为 0，`ACTIVE_MAX=47` 这道检查**永不可能触发**。
> 另有 33/125 条 `issue_link` 为空或 `TBD`（registry 自身 note 亦要求补）。
> Evidence: `evidence/anti-ignore-gate-hollow-2026-09-30/anti_ignore_gate_hollow.txt`
> (sha256 `fedcb27f2a80c6bb33bf9a4777383bbe30e0450d3703866f5d23787d03ef5e46`)。
>
> **真实后果**：`crates/parser/tests/wp_a_legacy.rs:68` 的 `#[ignore]`
> （掩盖 #4708 未修复的块注释缺陷）未被 registry 登记，而本应发现它的
> 门禁对此完全无感。修复登记缺失是 §4.0 WP-A 验收的前置条件。

#### 2.1.2 `check_arch_invariants.sh` — ✅ DONE（原描述作废）

原描述「`crates/executor/src/execution_engine.rs` 2731 行 > 1600 限制，
需 2-3 天拆分」**不成立**：

- `crates/executor/src/execution_engine.rs` **不存在**（`ls` 验证：No such file）
- 门禁 `check_arch_invariants.sh:121` 实际读取的是**仓库根 `src/execution_engine.rs`**
  （**1034** 行），非 2731 行
- 实跑：`bash scripts/gate/check_arch_invariants.sh` → **exit 0**，`PASSED: 5 / FAILED: 0`
- 原计划的 2-3 天 `expression_engine/{core,select,dml}.rs` 拆分**无需执行**

> ⚠️ **门禁缺陷已修复（2026-09-30）**：
> `check_arch_invariants.sh:121` 原使用 `wc -l < src/execution_engine.rs 2>/dev/null || echo "0"`，
> 对不存在路径实测输出 `0`；因 `0` 恒不大于 1600，**目标文件缺失时该门禁必然 PASS**。
> 这符合 `ANTI_FABRICATION_POLICY.md` §7.4 的 P16（Gate Test Integrity）FAIL 定义。
> 已改为 fail-closed：目标缺失即判 FAIL。修复后实跑验证 —— 正常仓库
> `PASSED: 5 / FAILED: 0 / exit 0`；隔离临时仓库（目标缺失）
> `FAIL: C-ARCH-05 cannot be evaluated` + `Result: FAIL`。
> 同时新增非阻断的 `C-ARCH-05-NOTE`，以 WARN 形式暴露覆盖盲区。
> 详见 `ALIGNMENT_AUDIT_2026-09-30.md` §7.4。
>
> 覆盖盲区现状（仅可见化，未改判定）：`crates/executor/src/expr/mod.rs`(5104) /
> `stored_proc.rs`(4248) / `trigger.rs`(2549) 均不受行数门禁约束。
> 是否扩展 C-ARCH-05 覆盖范围待决 —— 一旦扩展即会立即 FAIL，属架构债务需独立拆分排期。

#### 2.1.3 `check_anti_fabrication.sh` CHECK 4 — ✅ 已解决 2026-09-30

**原始状态（实测）**：

- HEAD 提交：`e2355c068058a102ce4d12f3c11beae84962a974`
- HEAD 作者邮箱实测：`claude@macmini.dev`
- 白名单（`check_anti_fabrication.sh:178`，原 8 项）不含该邮箱
- → `log_error` → `ERRORS>0` → `main()` `exit 1` → **门禁 FAIL**
- Evidence: `evidence/gate-runs-2026-09-30/check_anti_fabrication.txt`
  (sha256:371f76010b4dbead, `Results: ERRORS=1`, `EXIT_CODE=1`)

原提的两个修法均未执行（加白名单 / 重写历史），HEAD 邮箱只是变成了第三个
同样不在白名单的值（`v400@local` → `claude@macmini.dev`）。

**已采取的修复（2026-09-30，人工决策）**：

- 采纳「**加白名单，不动历史**」方案 —— 避免改写已推送到 5 个远端的
  `develop/v4.1.0` 共享历史
- `scripts/gate/check_anti_fabrication.sh:178` 白名单新增 `claude@macmini.dev`
  （附注释记录来源与决策依据）
- 复跑：`bash scripts/gate/check_anti_fabrication.sh` → **exit 0**，
  `Results: ERRORS=0`，`[PASS]`
- Evidence: `evidence/gate-runs-2026-09-30/check_anti_fabrication_rerun.txt`

**补充事实（记录备查）**：

- 本地 `git config user.email` 本就是合规的 `openheart@gaoyuanyiyao.com`
- 最近 200 个提交中有 8 个使用 `claude@macmini.dev`（含 HEAD）
- `AGENTS.md` 声明的 pre-commit 强制邮箱钩子在 `.git/hooks/pre-commit`
  **不存在** —— 该治理机制实际未生效

**阶段影响**：3 个继承的 gate 阻断项至此全部清除。但
`current_stage` **仍为 DRAFT** —— 清除阻断项不等于阶段推进，
DRAFT → ALPHA 属 `STAGE_CONFIG` 流程，须显式执行并记录，
不得仅凭门禁转绿推断（2026-09-29 的错误正在于此推断）。
详见 `STAGE.yaml` 顶部 AFP Type-B 回退说明与
`docs/governance/incidents/2026-09-30-V410-ALPHA-UNSUPPORTED-BY-GATE-EVIDENCE.md`。

### 2.2 [P1] Migrate WP-C..G deferred items into v4.1.0 scope

Per `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` and `CLAIM_DOWNGRADE_MANIFEST.md §2`, **4 个 WP 组 / 12 条 issue**（WP-C、WP-D、WP-F、WP-G）自 v4.0.0 递延至 v4.0.1（现 v4.1.0）。
> 计数口径更正（2026-09-30）：原文写「5/7」，与 WP 共 8 个（A–H）的实际
> 结构不符，已按 `ISSUES_PLAN.md` §4.7 订正。These should migrate to v4.1.0 scope:

- WP-C DDL/integrity: #4682 #4652 #4672 #4669 #4709 #4703
- WP-D joins: #4668 #4656 #4649 #4636
- WP-F schema: #4848
- WP-G type/comparison: #4846
- (WP-H #4639: explicitly v4.1-targeted per the WP-H triage)

Estimated: 6-8 weeks total (1-2 weeks per WP category)
Blocked by: 2.1.2 (execution_engine.rs split is needed for WP-G #4846 type/comparison work)

### 2.3 [P1] V400-09 168h SOAK FINAL_REPORT

- Kickoff: 2026-09-19 (`docs/releases/v4.0.0/SOAK_168H_KICKOFF_2026-09-19.md`)
- Duration target: 168h = 7 days
- Expected completion: 2026-09-26
- Current status as of 2026-09-29: no FINAL_REPORT exists
- Action: Verify SOAK process status; if completed, write the FINAL_REPORT; if still running, document elapsed progress
- Owner: openclaw / CI / Z6G4
- Estimated: 1 day to write FINAL_REPORT if data exists
- Blocked by: physical 168h run completion

### 2.4 [P2] v4.0.0 STAGE.yaml SSOT governance decision

The v4.0.0 self-claimed GA promotion did not advance the STAGE.yaml
SSOT to "GA". This is a governance gap inherited by v4.1.0.

Decision options:

- (A) Backfill the STAGE_CONFIG gate flow on v4.0.0 (run alpha / beta / rc / ga gates in order, advance current_stage to "GA")
- (B) Roll back v4.0.0 self-claim (revert main to v3.12.0 GA, treat v4.0.0 as continuing DRAFT)

Requires human architect decision. Not a code task.

### 2.5 [P2] 本地 `main` 与 `develop/v4.0.0` 分叉 resolution

> **核实修正**（2026-09-30）：本节原写「落后 16902 commits … 待决」。
> 实测为 **已分叉**，依据 `ALIGNMENT_AUDIT_2026-09-30.md` F-06：
> `main = f8a149b474`，落后 `develop/v4.0.0` **16905** 提交，
> 且 `main` **不是** `develop/v4.1.0` 的祖先（`git merge-base --is-ancestor` = NO）。

`ISSUES_PLAN.md` §1.2 原记「16902-commit gap closed on main / DONE 2026-09-28」
与实测不符，已改为 PARTIAL（`release/v4.0.0` 确已在 github 创建）。

Action: 在以下两者中明确选择其一 —
(a) 收敛 `main`（需先解决分叉，再快进或合并 `develop/v4.0.0`）；
(b) 将该分叉记录为**已知且有意保留**的历史状态，并更新 `main` 的定位说明。

Owner: governance / openclaw
Estimated: 1 day (mechanical) + governance review

## 3. Sequencing

```
[P0] 2.1.1 (1h) ─────┐
[P0] 2.1.3 (30min) ──┼──→ [ALPHA ready] ──→ v4.1.0-alpha1 tag
[P0] 2.1.2 (2-3d) ──┘
                          │
                          ↓
[P1] 2.2 (6-8w) ────→ [BETA ready]
[P1] 2.3 (1d) ──────→ [BETA ready]
                          │
                          ↓
[P2] 2.4 (governance)
[P2] 2.5 (1d + review)
                          │
                          ↓
                      [GA ready]
```

## 4. References

- `docs/releases/v4.1.0/STAGE.yaml` — stage state + exit criteria
- `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` — WP-A..H source
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` — deferred items
- `docs/releases/v4.0.0/SOAK_168H_KICKOFF_2026-09-19.md` — SOAK kickoff
- `docs/releases/v4.0.0/STAGE.yaml` — v4.0.0 inherited stage state
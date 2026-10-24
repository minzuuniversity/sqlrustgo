# SQLRustGo 发布阶段 SSOT Drift 差异报告

> **报告 ID**：`SSOT-DRIFT-2026-10-25`
> **目标 Issue**：`openclaw/ai-project-hub#5`（SQLRustGo）
> **执行 Agent**：`mcode-macmini`（MiniMax Code / mavis，host macmini）
> **基线**：`fix/4597-chaos-t8d-awk-portability` @ `9fbb63bfbc0bc5221bdd762e281e937b27be86af`
> （该 commit 与 252 Gitea `refs/heads/fix/4597-chaos-t8d-awk-portability` **逐位一致**）
> **生成时间**：2026-10-25T03:52:00+08:00
> **方法**：全部只读核验（`git` / `cat` / `grep` / 门禁脚本实跑）。**未改写任何现有发布文档、未改 tag、未改 STAGE.yaml、未改 Git 历史。**

---

## 0. 结论摘要

**v4.0/v4.1 的阶段冲突不是"一处文字过期"，而是三份声称"最近已发布 GA"的权威文件互相矛盾，且其中被引用最多的一份是错的。**

| # | 文件 | 声称的"最近已发布 GA" | 判定 |
|---|---|---|---|
| V1 | `VERSION:1` | `v3.11.0` | ❌ 陈旧 1 个版本 |
| V2 | `docs/releases/VERSION_HISTORY.md:4` | `v3.12.0` | ✅ **唯一正确** |
| V3 | `CURRENT_VERSION.md:9` | `v4.0.0 GA (CONDITIONAL PASS)` | ❌ **错误——复述了已被作废的 self-claim** |

**权威反证**（同仓内、带实测证据的作废声明）：

```
docs/releases/v4.0.0/GA_GATE_REPORT.md:3-5
  > ## ⚠️ 2026-09-30 判定更正（本节优先级高于本文件其余全部内容）
  > **原 §0 的「v4.0.0 GA CONDITIONAL PASS — promoted to GA stage」判定不成立，本版本从未达成 GA。**

docs/releases/v4.0.0/STAGE.yaml:13        current_stage: "DRAFT"
docs/releases/v3.12.0/STAGE.yaml:13       current_stage: "GA"
docs/releases/v4.1.0/STAGE.yaml:201       current_stage: "ALPHA"
```

**因此 V3 与事实冲突**：`CURRENT_VERSION.md` 把 v4.0.0 列为"最近已发布"，而同一仓内两份 STAGE.yaml 与作废后的 GA 门禁报告都表明 v4.0.0 **从未达成 GA**，最新实际 GA 是 v3.12.0。

---

## 1. V1–V3 冲突矩阵（逐项实测）

### V1 — `VERSION` 陈旧

```
$ cat VERSION
v3.11.0
# last released GA tag (tag `v3.11.0-ga` @ commit 5038b154c, 2026-08-09)
# current dev: v3.12.0 RC / GA candidate preparation (develop/v3.12.0 @ d64f0038b9, 2026-09-02)
# per docs/governance/RELEASE_GOVERNANCE.md — VERSION file is last-released tag only,
# current dev version lives in docs/releases/v3.12.0/STAGE.yaml (current_stage: RC)
```

| 行 | 声称 | 实测 | 判定 |
|---|---|---|---|
| `VERSION:1` | 最新 GA = `v3.11.0` | 最新 `-ga` tag 实为 `v3.12.0-ga` | ❌ 落后 1 个 GA |
| `VERSION:3` | 当前开发 = "v3.12.0 RC" | `v3.12.0/STAGE.yaml:13` = **GA** | ❌ 阶段错 |
| `VERSION:5` | "current dev version lives in `docs/releases/v3.12.0/STAGE.yaml` (current_stage: RC)" | 该文件实际 = **GA** | ❌ 指针内容错 |

> `VERSION` 指向的 SSOT 本身存在，但它描述的那一行内容是错的。

### V2 — `VERSION_HISTORY.md` 正确

```
$ grep -n '最新 GA' docs/releases/VERSION_HISTORY.md
4:> **最新 GA**: v3.12.0 (GA, 2026-09-08)
```

与最新 `-ga` tag 一致，且**被门禁实跑确认**（见 §4）。

### V3 — `CURRENT_VERSION.md` 错误

```
$ sed -n '3,5p;7,9p' CURRENT_VERSION.md
> **provenance:** generated_at=2026-09-30, branch=develop/v4.1.0,
> commit=`bdf465b25c` (post v4.0.0 docs backport + alpha2 prep),
> source_repo=openclaw/sqlrustgo, policy=Anti-Fabrication-Policy-v1.0

## 最近已发布

**v4.0.0 GA** (CONDITIONAL PASS, 2026-09-19 发布；tag `v4.0.0-final` @ `54571eeca0`，最终 sync HEAD `07178c9d66`；PR #3787/#4902 promote 至 main)
```

三处缺陷：

| # | 缺陷 | 实测 |
|---|---|---|
| V3-a | 把 v4.0.0 列为"最近已发布 GA" | 与 `v4.0.0/STAGE.yaml` (DRAFT) + 作废后的 GA_GATE_REPORT 冲突 |
| V3-b | tag 引用配对错误 | 声称 `v4.0.0-final @ 54571eeca0`；实测 `v4.0.0-final` → `558aa6138546`，`54571eeca0` → `07178c9d66a4`（= `ga/v4.0.0` 的 commit）。**两个不同 tag 被配成一对** |
| V3-c | provenance 已过期 | 声明 `branch=develop/v4.1.0 commit=bdf465b25c (2026-09-30)`；当前分支是 `fix/4597-chaos-t8d-awk-portability` @ `9fbb63bfbc` |

---

## 2. v4.0.0 的 GA 声称存在于**三层载体**，且互相矛盾

| 层 | 载体 | 声称 | 现状 |
|---|---|---|---|
| 文档 | `CURRENT_VERSION.md:9` | v4.0.0 GA | 仍在（未被作废） |
| 文档 | `CURRENT_VERSION.md:14` | "G1/G2/G4/G5 ✅ PASS；G3 coverage 78.28% 平均 CONDITIONAL PASS" | 仍在 |
| 文档 | `CURRENT_VERSION.md:33 / 55 / 155` | "GA_GATE_REPORT says GA CONDITIONAL PASS"、"治理漂移未修复" | 仍在——**这三处描述的是一个已被作废前的旧状态** |
| 文档（已纠正） | `docs/releases/v4.0.0/GA_GATE_REPORT.md:3-5` | **判定不成立，从未达成 GA** | ✅ 权威 |
| 治理 | `docs/releases/v4.0.0/STAGE.yaml:13` | `current_stage: DRAFT` | ✅ 权威 |
| **不可变** | git tag `ga/v4.0.0` → `07178c9d66a4` | "GA" | **⚠️ tag 里仍有 GA 断言，与作废声明冲突** |

**这一层最重要**：GA 声称不只写在可编辑的 Markdown 里，还固化在 **git tag** 中。tag 不可变，因此"让文档追上事实"这条路**不可能修好 tag 侧的不一致**——这正是 Issue #5 要求"不要重写现有发布文档来掩盖 drift"的现实原因。

> **同时存在的第三个事实**：仓库里有 4 个 v4.0.0 tag，命名空间互不统一：
> `beta/v4.0.0`→`917fd83e3fed`、`rc/v4.0.0`→`c9bea8dcc200`、`ga/v4.0.0`→`07178c9d66a4`、`v4.0.0-final`→`558aa6138546`。

---

## 3. `CURRENT_VERSION.md` 内部还有三处"描述已过期状态"

| 行 | 原文（截断） | 问题 |
|---|---|---|
| `L33` | "v4.0.0 STAGE.yaml `current_stage: DRAFT` 与 GA_GATE_REPORT.md 说 `GA CONDITIONAL PASS` 矛盾…SSOT 仍未 advance to GA…是 release governance decision 而非文档 fix" | 它描述的"GA_GATE_REPORT 说 CONDITIONAL PASS"已不成立 |
| `L55` | "🟡 self-claimed GA（`docs/releases/v4.0.0/GA_GATE_REPORT.md` says GA CONDITIONAL PASS；…SSOT 仍是 DRAFT — 治理漂移未修复）" | 同上；且"漂移未修复"与 §1 实测矛盾 |
| `L155` | "\| v4.0.0 GA CONDITIONAL PASS \| 2026-09-20 \| self-claim per `GA_GATE_REPORT.md`；STAGE.yaml SSOT 未更新（治理漂移） \|" | 同上 |

**净效果**：`CURRENT_VERSION.md` 顶部把 v4.0.0 列为已发布 GA，正文三处又把"GA_GATE_REPORT 说 GA"当作当前事实并把它标为"治理漂移未修复"。读者据此会得到**两个互相矛盾的世界观**，且都基于一个已被作废的判定。

---

## 4. 门禁"形式通过、实质无效"的结构性原因

### 4.1 实跑证据

```
$ bash scripts/gate/check_docs_consistency.sh
  ...
  [PASS] VERSION_HISTORY.md latest GA: v3.12.0 (matches tag)
  ...
  ============================================
  [PASS] All checks passed          # rc=0
```

**门禁是绿的。而 drift 是真实存在的。**

### 4.2 为什么会绿——两个结构性盲区

**盲区 1：门禁只覆盖 v3.0.0–v3.12.0，完全不看 v4.x。**
门禁脚本自身的注释写明：

```
scripts/gate/check_alpha_quality_v4.10.sh:52-56
  Note: check_docs_consistency.sh is a version-agnostic cross-version check
  (covers v3.0.0..v3.12.0 only).
```

实跑日志中所有条目均为 `docs/releases/v3.x.0/...`。**v4.0.0 / v4.1.0 的阶段文件从未被这项检查读取。**

**盲区 2：门禁从不读取 `VERSION` 与 `CURRENT_VERSION.md`。**
`CHECK 1` 比对的是 `VERSION_HISTORY.md` 的"最新 GA"行与最新 `v*-ga` tag——**它验证的是 V2（唯一正确的那份）**，因此必然 PASS。

**盲区 3（tag 命名空间）：门禁的 tag 规则只认 `v*-ga`。**

```
$ git tag | grep -- '-ga$' | sort -V | tail -3
v1.1.0-ga
v3.11.0-ga
v3.12.0-ga

$ git tag | grep 'v4\.0\.0'
beta/v4.0.0
ga/v4.0.0
rc/v4.0.0
v4.0.0-final
```

即使将来有人把 `v4.0.0` 认定为 GA 并命名为 `v4.0.0-ga`，门禁规则也只按 `v*-ga` 匹配；**现存 tag 用的是 `ga/v4.0.0` 这种斜杠命名空间，结构上不在门禁的识别范围内**。这条是"为什么 v4.0.0 的 GA 声称能长期存活而不被 CI 发现"的技术根因。

### 4.3 对照：门禁自己承认的既有缺陷

```
scripts/gate/check_alpha_quality_v4.10.sh:54-56
  Pre-existing failures in that gate (VERSION_HISTORY.md
  current version + v3.12.0 CHANGELOG duplicate commit 355b5a3837) are
  not v4.1.0 regressions and are tracked separately per open_issues §3.
```

即：**"文档一致性门禁存在既有缺陷"这件事，仓内早已承认**，只是被归类为"非 v4.1.0 回归"而未纳入当前门禁范围。本报告的发现与该自述一致，但给出了具体位置与可复现的核验命令。

---

## 5. 脏工作树保全（Issue #5「保存当前脏工作树」）

来源工作树 `/Users/liying/workspace/dev/sqlrustgo`，采集时为**只读**操作。

| 项 | 值 |
|---|---|
| 分支 / HEAD | `fix/4597-chaos-t8d-awk-portability` @ `9fbb63bfbc`（与 252 一致） |
| 已修改文件 | 13（`crates/storage/src/{engine,file_storage}.rs`、`crates/mysql-server/src/{lib,result_set}.rs`、`crates/storage/tests/mvcc_row_identity_lifecycle_5277.rs`、`crates/mysql-server/tests/result_set_schema_5271.rs`、`src/engine_ddl.rs`、5 个 integration 测试、`AGENTS.md`） |
| 未跟踪文件 | 1（`scripts/fix-dsh-llama-config-v2.sh`） |
| 已暂存改动 | 0 |
| 改动规模 | `13 files changed, 100 insertions(+), 80 deletions(-)` |
| 保全包 | `unstaged patch (502 行, sha256 4289b0a3298e6236…)` + `staged patch (空)` + `status 快照 (14 行)` + `未跟踪文件逐字节副本` + `RESTORE.md`（含还原与校验命令） |

**交叉关联（跨项目证据）**：这批未提交改动与 `KOS-S0-2-CLOSURE-AND-CONTROL-STATE-POC.md` §1.3 **P3「重复派发」** 是同一批文件——控制面记录的任务 `TASK-20261023-124503`（`source: wp-update-regression`）的 output 字段恰好是 `dirty_files: 14 / unpushed: 11`，与当前工作树状态吻合。该任务的第二次执行产出**至今未提交**，而控制面已将其记为 `COMPLETED`。

> ⚠️ **本报告不裁决这 13 个文件该提交还是丢弃**——那需要人工判断代码正确性，不属于 SSOT drift 范围。

---

## 6. v4.1.0 alpha → beta 出口条件（实测自 `ALPHA_TO_BETA_GATE_PLAN.md` / `STAGE.yaml`）

`CURRENT_VERSION.md:38-46` 列出的 exit criteria 与未满足项：

| 条件 | 状态 |
|---|---|
| WP-C DDL/integrity 修复（#4682/#4652/#4672/#4669/#4709/#4703） | 未在本次核验范围内 → **NOT RUN** |
| WP-D join/subquery 修复（#4668/#4656/#4649/#4636） | **NOT RUN** |
| WP-F schema migration（#4848） | **NOT RUN** |
| WP-G CHAR(n) PAD SPACE 兼容（#4846） | **NOT RUN** |
| WP-H #4639 | **NOT RUN** |
| V400-09 168h SOAK 复跑证据（前置 RSS budget 调优） | **NOT RUN** |
| Coverage ≥ 50% | **NOT RUN**（alpha quality 按 v3.12.0 模板不含 coverage，Q7 deferred） |

**这些条目一律不标记 PASS。** 理由：本轮只做了阶段 SSOT 一致性核验，没有为上述任何一条产生新的执行证据。

---

## 7. 本轮实际运行的检查

| 检查 | 命令 | 退出码 | 结果 |
|---|---|---|---|
| 阶段一致性基线 | `git ls-remote origin refs/heads/fix/4597-...` | 0 | 与 252 逐位一致 |
| 文档一致性门禁 | `bash scripts/gate/check_docs_consistency.sh` | **0** | **All checks passed**（仅覆盖 v3.x，见 §4） |
| Anti-Fabrication 门禁 | `bash scripts/gate/check_anti_fabrication.sh` | 见附录 | 需 `cargo build -p sqlrustgo-mysql-server` |
| 脏工作树保全 | `git diff --binary` / `git status` / `cp` | 0 | 502 行补丁 + 1 文件副本 |
| tag 解析 | `git rev-list -n1 <tag>` × 5 | 0 | 见 §2 |
| `cargo build --all-features` | 见附录 | 见附录 | 重编译，见附录 |
| `cargo test --all-features --lib` | 见附录 | 见附录 | 见附录 |

### 明确 **NOT RUN**

- `cargo build --all-features`（Q1_BUILD）
- `cargo test --all-features --lib`（Q6_TEST_LIB）
- `check_anti_ignore_gate.sh` / `check_arch_invariants.sh` / `check_arch3_no_bypass.sh` / `check_sql_corpus_gate.sh`（Q3/Q4/Q5）
- `cargo fmt --check`（Q1_FMT）
- v4.1.0 alpha→beta 全部出口条件（§6）
- 任何 Rust 单元/集成测试

> 上述项的最终状态见本报告附录（随 PR 一并提交）。**未运行的检查不标记 PASS。**

---

## 8. 建议（**不改写现有发布文档**）

按 Issue #5 要求，以下建议均为**新增差异报告 / 新增门禁检查**，不改写 `CURRENT_VERSION.md`、`VERSION`、`STAGE.yaml`，不动 tag。

| # | 建议 | 类型 | 是否需要人工批准 |
|---|---|---|---|
| R1 | 把本报告作为 `docs/releases/` 下的常驻 drift 记录，后续每次发布复核一次 | 新增文档 | 否 |
| R2 | 新增门禁 `check_release_ssot_consistency.sh`，比对 **V1/V2/V3 三方**与 `STAGE.yaml`、`GA_GATE_REPORT.md` 判定 | 新增门禁 | 否 |
| R3 | 让 `check_docs_consistency.sh` 的 tag 规则同时识别 `ga/vX.Y.Z` 命名空间，或补一条"命名空间不一致"告警 | 门禁增强 | 否（但属既有门禁改动，需评审） |
| R4 | `CURRENT_VERSION.md` 的 provenance 增加"判定以 `STAGE.yaml` + 已作废门禁报告为准"的自述，避免读者只读顶部 | 文档增强 | **是**（涉及发布文档措辞） |
| R5 | `VERSION` 的两行注释（v3.12.0 RC / current_stage: RC）由 `RELEASE_GOVERNANCE.md` 负责人同步 | 文档修复 | **是** |
| R6 | tag `ga/v4.0.0` 与作废声明的冲突**保持现状并显式记录**——tag 不可变，不得删除或重打 | 决策 | **是** |
| R7 | 13 个脏文件（§5）的去留单独裁决，不与 drift 修复混在一起 | 决策 | **是** |
| R8 | 控制面把"COMPLETED"与"产出已落地"解耦（跨项目，见 `KOS-S0-2` P3） | 跨项目 | **是** |

---

## 9. 复核方法（可重放）

```bash
# 基线与 252 一致性
git ls-remote http://192.168.0.252:3000/openclaw/sqlrustgo.git \
  refs/heads/fix/4597-chaos-t8d-awk-portability

# 三方 GA 声称
head -1 VERSION
grep -n '最新 GA' docs/releases/VERSION_HISTORY.md
grep -n '^\*\*v4\.0\.0 GA\*\*' CURRENT_VERSION.md

# 权威反证
sed -n '3,5p' docs/releases/v4.0.0/GA_GATE_REPORT.md
grep -n '^current_stage:' docs/releases/v{3.12.0,4.0.0,4.1.0}/STAGE.yaml

# tag 解析（文档 vs 实况）
for t in beta/v4.0.0 rc/v4.0.0 ga/v4.0.0 v4.0.0-final; do
  echo "$t -> $(git rev-list -n1 "$t" | cut -c1-12)"; done
git tag | grep -- '-ga$' | sort -V | tail -1

# 门禁实跑
bash scripts/gate/check_docs_consistency.sh
```

---

*本报告由 `mcode-macmini` 在隔离克隆（`--local` clone，独立分支 `mcode-macmini/sqlrustgo-alpha-handoff`）中生成，未接触共享工作树的写路径。*

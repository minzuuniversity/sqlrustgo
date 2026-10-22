# 批次 28 —— SEM-4 销账 + 三个被掩盖的门禁缺陷

> **基线**：`develop/v4.1.0` @ `3a3350f660`
> **provenance:** generated_at=2026-10-23, branch=fix/ga-mandatory-docs-batch27,
> commit=`3a3350f660`, machine=Mac-mini-7.local (arm64, macOS 27.0.1),
> identity=openclaw-mcode, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
> **gate_policy_eval_id**：`v410-batch28-sem4-gate-integrity-001`
> （本文件汇编的门禁引擎：`scripts/gate/check_arch_sem_debt.sh`、
> `scripts/gate/check_docs_consistency.sh`、
> `scripts/gate/check_stage_config_consistency.sh`、
> `scripts/gate/check_document_completeness.sh`、
> `scripts/gate/check_evidence_binding.sh`、
> `scripts/gate/check_docs_pointer_paths.sh`、
> `scripts/gate/check_workflow_expressions.sh`、
> `scripts/gate/check_gate_script_refs.sh`、
> `scripts/gate/check_ci_checkout_consistency.sh`、
> `scripts/gate/check_anti_ignore_gate.sh`、
> `scripts/gate/coverage_total_percent.py`；证据绑定门禁
> `scripts/gate/check_evidence_binding.sh` 的 Type B 判据据此认定本文档的
> PASS 声明有对应的 gate engine 输出，而非 AI 自述。
> ⚠️ **本批全部为本机实测**（Mac-mini-7.local），**不是 CI run** ——
> CI 侧验证待 252 恢复后补，见 §11）

本批从「批次 27 的收尾动作」开始，最后落到三个此前**没有人看见**的门禁缺陷上。
三个缺陷有同一个形状：**它们各自被更早的一个红挡在后面，于是从未被执行到，
也从未被报告。**

---

## 0. 一句话结论

| # | 缺陷 | 状态 | 影响面 |
|---|---|---|---|
| A | `crates/vector/tests/cosine_similarity_5384.rs` 不是 fmt-clean | 已修 | **develop 上的潜伏 CI 红**（见 §3） |
| B | `check_10_principles_v310.sh` 的 R4 先 apply 再 check | 已修 | 门禁恒 PASS + 静默改写被检源码（见 §4） |
| C | 同上脚本的 R5 带着一个乱码 flag `--codecov幻` | 已修 | R5 永远读出 0%（见 §5） |
| D | 覆盖率百分比的提取规则靠「最后一个百分比」 | 已修 | 分支覆盖率一旦非空就会静默顶替行覆盖率（见 §6） |

A 是**我自己**在批次 26 引入的（PR #5540）；B/C/D 是同一个脚本和同一条规则的
历史遗留。D 另有一层：它同时服务着 **CI 接线的** `check_coverage_v312.sh`
（`postcheck` job 里的 Gate L1），也就是 `STAGE_CONFIG.yaml` 的 GA `required_gates`
点名的那道门禁。

---

## 1. SEM-4 销账：根因从占位符换成实测事实

### 1.1 为什么要重开

`docs/governance/debt/debt-registry.yaml` 的 SEM-4 原本在 2026-08-15 关闭，
2026-10-07 被重新打开（#4944 P0-05），理由是那次关闭靠三条站不住的声明：

- (a) 根因写的是字面占位符 `"Z440 32% 问题(XYZ)"` —— 跨机台差异的根因从未被记录；
- (b) 声称「所有 8 core crates 达标」，而它自己的 `progress_metric` 里
  parser 是 75.16%（门禁阈值是 L1_8 **均值**而非单 crate 下限，所以低 crate 也能过，
  但「全部达标」是比证据更强的声明）；
- (c) 数字不可复现。

note 里写明：「Re-closure requires the executor suite green (#5103) plus a full
8-crate run of the corrected gate.」

### 1.2 根因（实测，非推断）

**覆盖率测量原先不是一次测量。**

`check_coverage_v312.sh` 用 `cargo llvm-cov` 的**默认并行 test 线程**跑覆盖率。
`cargo llvm-cov` 下代码慢 2-10 倍，默认并行时测试结果取决于机器负载 ⇒
「同一 crate 的读数」随机器与负载漂移。脚本自己的注释里就留着这条实测证据：

> `--test-threads=1` is NOT optional here. Under `cargo llvm-cov` the code runs
> 2-10x slower, and the default parallel test execution then has the threads
> competing for CPU: measurements of the same crates came out different from
> run to run (sqlrustgo-planner measured 88.91% and PASSED with
> `--test-threads=1`, and FAILED with the default thread count on the same
> checkout). A coverage number that depends on machine load is not a measurement.

2026-10-22 commit `2fef945260`（PR #5537）把 `--test-threads=1` 钉死
（亦符合 AGENTS.md §10 对并发测试的要求）⇒ 测量转为确定性。
**这就是 'XYZ' 指向的东西。**

> 注意：`git log -S "test-threads=1" -- scripts/gate/check_coverage_v312.sh`
> 只有 `2fef945260` 一条记录 ⇒ 2026-10-07 那次 z440 测量仍取自旧的、负载相关的口径。

### 1.3 两项关闭前提

| 前提 | 实测 |
|---|---|
| executor 套件绿（#5103） | **777 passed / 0 failed / 0 ignored**（2026-10-23, macmini M4）。#5103 由 PR #5126 修复（"过程体内的无 FROM SELECT 被当成 0 行 —— 状态行永不可达"） |
| 修正后门禁的完整 8-crate 实跑 | 2026-10-23, macmini M4，**L1_8 平均 87.80% ≥ 80.0%**，exit 0 |

per-crate（2026-10-23，macmini M4）：

```
parser      86.46%    planner      88.91%    executor    82.74%    transaction 93.29%
storage     80.61%    catalog      87.33%    optimizer   87.85%    types       95.28%
```

> `parser 75.16%`（2026-08-15）与 STAGE.yaml 里引用的「实测 60.58%」**均不复现**；
> 今日实测 parser 为 86.46%。

### 1.4 确定性验证（新增实验）

同一台机器、同一份代码，**连续两遍**测 `sqlrustgo-transaction`：

```
RUN 1:  sqlrustgo-transaction: 93.29%
RUN 2:  sqlrustgo-transaction: 93.29%
```

两遍完全一致，排除集同为 15 个环境相关测试 ⇒ 读数不再随负载漂移。

### 1.5 跨机台对照

z440（2026-10-07）vs macmini M4（2026-10-23），7 个重叠 crate：

| crate | z440 | macmini | Δ |
|---|---|---|---|
| planner | 88.91 | 88.91 | 0.00 |
| catalog | 87.33 | 87.33 | 0.00 |
| optimizer | 87.85 | 87.85 | 0.00 |
| types | 95.28 | 95.28 | 0.00 |
| parser | 86.75 | 86.46 | 0.29pp |
| storage | 81.34 | 80.61 | 0.73pp |
| transaction | 86.00 | 93.29 | 7.29pp |

transaction 的 7.29pp 由**两次测量之间的真实代码变更**解释 ——
`crates/transaction` 在 2026-10-07 之后有 6 个 commit（新增 3 个 counterexample
测试文件 + `tx id 改进程全局唯一分配` 修复），不是测量口径问题。

### 1.6 结论

SEM-4：`state: IN_PROGRESS → CLOSED`，`progress: 87.64% → 87.80%`，
`closed_at: "2026-10-23"`，`closing_pr: [5126, 5537]`。
门禁 `check_arch_sem_debt.sh` 从 **PASS-WITH-DRIFT** 变为
**PASS (all 7 items CLOSED)**。

---

## 2. v4.0.0 的 GA 判定措辞统一

`docs/releases/v4.1.0/STAGE.yaml` 的 open_issues 首条记录的是一处矛盾：
`CURRENT_VERSION.md` 与 `docs/releases/v4.0.0/RELEASE_NOTES.md` 都写 v4.0.0 是
「GA (CONDITIONAL PASS)」，而 `docs/releases/v4.0.0/STAGE.yaml` 的
`ga_promotion_status.verdict = "REVOKED 2026-09-30"`、`current_stage: "DRAFT"`。

实测确认的事实：

- `git tag -l 'v4.0.0*'` → `v4.0.0-final`（annotated @ `54571eeca0` → commit `07178c9d66`）
- `git branch -r --contains 54571eeca0` → gitcode/main、gitea250/main、gitea252/main、
  gitee/main、github/main、origin/main **全部含该 commit**
- `docs/releases/v4.0.0/STAGE.yaml:29-41` → verdict=REVOKED、actual_status=DRAFT，
  并列出 5 条阻塞事实（覆盖率 78.28% 低于 `COVERAGE_MIN_RC`=80%、阶段流转从未执行、
  `check_alpha_v400.sh` 不存在、无人工签字）

⇒ **代码确实已发布到 5 远端 main，但 v4.0.0 从未合法达到 GA，不构成 GA 发布。**

改动：`CURRENT_VERSION.md` 6 处引用旧口径（最近已发布标题 / GA 门禁报告条目 /
里程碑条目 / 治理约束段 / 版本概述 / 变更历史表）+ `v4.0.0/RELEASE_NOTES.md` 头部
1 处，全部改为与 SSOT 一致的措辞。

---

## 3. 缺陷 A —— 一个潜伏在 develop 上的 CI 红

### 3.1 怎么发现的

本批跑 `check_10_principles_v310.sh` 时，工作树里
`crates/vector/tests/cosine_similarity_5384.rs` 被**自动改了**（重排版）。
该脚本的 R4 先 `cargo fmt --all`（apply）再 `cargo fmt --all -- --check` ⇒
它会**静默改写被检源码**（见 §4）。还原后直接查：

```
$ cargo fmt --check --all ; echo RC=$?
RC=1
Diff in crates/vector/tests/cosine_similarity_5384.rs:48
Diff in crates/vector/tests/cosine_similarity_5384.rs:123
Diff in crates/vector/tests/cosine_similarity_5384.rs:219
```

### 3.2 归属

`git log -- crates/vector/tests/cosine_similarity_5384.rs` → `8aa60a384a`
（2026-10-22, `openclaw-mcode <openclaw-mcode@gaoyuanyiyao.com>`），
经 PR **#5540** 合入 develop（merge commit `3a3350f660` 正是本批基线）。
`git log 3a3350f660..HEAD -- <file>` 为空 ⇒ **本批没碰过它，是我自己引入的。**

### 3.3 为什么 CI 还没红

`ci.yml` 的 `lint-build` job 步骤顺序是：

```
Checkout → Clippy → Test-target lint budget → Workspace lint budget
        → Workflow expression gate → Process-global env mutation gate
        → Hardcoded credential gate → Format check → Build → Save cargo cache
```

run 7041b（head `31ea00c2be`）的日志显示 job 死在第 4 步：

```
lint warnings: 704 across 68 lint name(s)
[FAIL] 1 lint budget violation(s):
   dead_code: 27 warning(s) — baseline allows 26
🏁 Job failed
```

⇒ **Format check 排在 lint budget 之后，job 先死在 budget 上，格式检查从未被执行。**
这就是「被更早的一个红挡在后面」的第一个实例。

而 lint budget 的那个红，本批已经修掉（批次 27 的 `c0a4152a12`：`#[ignore]` 顶掉了
`#[test]`，dead_code 27 → 26）⇒ **一旦那个 commit 推上去，Format check 就会执行，
并因这个文件变红。**

### 3.4 修法与验证

用 **CI 同版本**的 rustfmt 格式化（CI 镜像 `gitea/act_runner:node-rust-v0.6.1`
是 cargo 1.91.1），然后交叉验证三个版本：

| rustfmt | 修前 | 修后 |
|---|---|---|
| 1.91.0（≈CI 的 1.91.1） | DIRTY（3 处） | **CLEAN** |
| 1.95.0 | DIRTY（3 处） | **CLEAN** |
| 1.97.0 | DIRTY（3 处） | **CLEAN** |

一份格式化同时满足三个版本 ⇒ 不会因为 CI 镜像日后升版本再次翻红。

---

## 4. 缺陷 B —— R4 是空转门禁，且会改写被检源码

`scripts/gate/check_10_principles_v310.sh` 的 R4 原实现：

```bash
cargo fmt --all > /dev/null 2>&1          # ← 先 apply
if cargo fmt --all -- --check > ... ; then # ← 再 check ⇒ 恒真
```

两个问题叠在一起：

1. **恒 PASS**：apply 之后 check 当然通过 ⇒ R4 永远不可能失败 ⇒ 格式违规不可见。
2. **静默改写被检源码**：跑这道门禁的人事后看到的是一棵干净的工作树，
   永远不会知道提交的代码不是 fmt-clean 的 —— §3 的漂移就是这样被掩盖的。

修法：删掉 apply，只保留 check。**门禁不改被检对象。**

> 该脚本**未接入** v4.1.0 的 CI（`grep -rn "check_10_principles" .gitea/workflows/` 无命中），
> 属 v3.10.0 时代的活门禁。

---

## 5. 缺陷 C —— R5 带着一个乱码 flag

```bash
cov_out=$(cargo llvm-cov --tests --codecov幻 --output-fn -p "$crate" 2>&1 || true)
pct=$(echo "$cov_out" | grep -oE "[0-9]+\.[0-9]+%" | tail -1 || echo "0%")
pct_int=${pct%.*}
```

`--codecov幻` 里那个 `幻` 是 **U+5E7B**（实测：逐字节 dump 出的非 ASCII 字符）。
cargo 直接拒绝：

```
$ cargo llvm-cov --tests --codecov幻 --output-fn -p sqlrustgo-types
error: invalid option '--codecov幻'
```

三处叠加把「没测到」变成了「测到 0%」：

| 环节 | 作用 |
|---|---|
| `|| true` | 吞掉命令失败 |
| `grep ... \| tail -1` | 输出里没有百分比 ⇒ grep 无结果 |
| `\|\| echo "0%"` | 把「没有测量」变成「0%」 |
| `pct_int=${pct%.*}` | `"0%"` 里没有小数点 ⇒ 截断不生效 ⇒ `[: 0%: integer expression expected` |

结果：R5 对每个 crate 都报 `0%` ⇒ 必红，且**红的原因与覆盖率毫无关系**。

修法（三条规则）：

1. **没发生的测量不能被报成数字** —— 删掉 `|| echo "0%"`，取不到就报「无覆盖率数据」。
2. **失败的测量必须让规则失败** —— 删掉 `|| true`，命令非零退出即 `[FAIL]`。
3. **百分比提取用共享规则**（§6）。

顺带修掉 `--codecov --output-fn`：它让 llvm-cov 输出 JSON，表里根本没有
`NN.NN%` 这个形态，即使 flag 不带乱码也读不出百分比。

---

## 6. 缺陷 D —— 覆盖率百分比的提取规则靠一个巧合

### 6.1 原规则

`check_coverage_v312.sh` 里原本是：

```python
cov_col = hdr.index("Cover", hdr.index("Lines"))
for shift in range(0, 6):
    idx = cov_col - shift
    if 0 <= idx < len(tot):
        m = re.fullmatch(r"([0-9]+(?:\.[0-9]+)?)%", tot[idx])
        if m: print(m.group(1)); break
```

即「从表头的 Lines-Cover 列往回走，取第一个能解析成百分比的格子」。

### 6.2 它靠什么才一直是对的

`cargo llvm-cov` 的 TOTAL 行**每组一个百分比**：

```
Filename    Regions  Missed Regions  Cover  Functions  Missed Functions  Executed  Lines  Missed Lines  Cover  Branches  Missed Branches  Cover
TOTAL          1278              76  94.05%      1236                2  98.35%   15987            76  95.28%         0                0  -
```

回溯从 `cov_col=12` 起步：数据行 index 12 是 **Branches 组**的百分比。
本仓库的 Rust 代码没有 branch region（MIR 里没有 `brancwitch`）⇒ Branches 恒为 `-`
⇒ 回溯一路退到 index 9 = **Lines Cover** ⇒ 碰巧对。

**只要哪个 crate 出现分支区域，同一段代码就会静默返回分支覆盖率，并把它叫做行覆盖率。**

实测证据（用真实表头形状的 fixture，把 Branches 填成 `90.00%`）：

```
$ printf '<真实形状的表格>' | <旧提取>
90.00          ← 取成了 Branches 列，应为 80.00
```

### 6.3 今天没有发生误报（不夸大）

对今天实测过的 crate 逐个核对 TOTAL 行三列：

| crate | Regions | Functions | **Lines** | Branches | 门禁报的值 |
|---|---|---|---|---|---|
| parser | 85.95% | 97.46% | **86.46%** | `-` | 86.46% ✓ = Lines |
| types | 94.05% | 98.35% | **95.28%** | `-` | 95.28% ✓ = Lines |

⇒ **门禁报的一直是行覆盖率，没有误报。** 这是「脆弱」，不是「已错」。

### 6.4 修法

新规则 `scripts/gate/coverage_total_percent.py` **按名字定位 Lines 组**，不再靠
「第几个百分比像」：

- 表头里每个组名出现两次（一次作组头，一次在合并的 `Missed <Group>` 标签里），
  所以**组起点 = 后面紧跟 `Missed` 的那个 token** ⇒ 组起点索引 = [1, 5, 9, 13]；
- 每个组在数据行占 3 格（total, missed, percentage），百分比是组内最后一格；
- 于是 Lines 组的百分比可直接寻址：index = 1 + 3*组号 + 2；
- 交叉校验：同一值必须等于数据行百分比序列中的第 (组号+1) 个，否则**输出空**
  （不报数字）⇒ 报告形状不认识时 fail-closed，而不是挑一个看着对的数。

**一个规则一份实现**：规则落在 `scripts/gate/coverage_total_percent.py`，
`common.sh` 的 `coverage_extract_total_percent()` 是唯一入口，
`check_coverage_v312.sh` 与 `check_10_principles_v310.sh` 共用它。

### 6.5 验证

| 项 | 结果 |
|---|---|
| `--selftest` fixtures | **5/5 PASS**（含「Branches 是百分比时必须仍取 Lines」这一条） |
| 对真实 parser 报告 | **86.46%**（与门禁原报值一致） |
| 对真实 types 报告 | **95.28%**（与门禁原报值一致） |
| 端到端复跑 `check_coverage_v312.sh`（单 crate transaction） | **93.29%**（重构前后一致） |
| R5 单 crate 判定路径（types） | `pct=95.28` → `pct_int=95` ≥ 42 ⇒ PASS |

CI 接线：`ci.yml` 的「Gate L1 — Coverage」步骤在跑昂贵测量**之前**先跑
`python3 scripts/gate/coverage_total_percent.py --selftest`（不花 cargo 时间）。

---

## 7. 门禁实测（本批，本机 macOS 27.0.1 / arm64）

| 门禁 | 结果 |
|---|---|
| `check_arch_sem_debt.sh` | ✅ PASS (all 7 items CLOSED) —— 由 PASS-WITH-DRIFT 改进 |
| `check_docs_consistency.sh` | ✅ PASS |
| `check_stage_config_consistency.sh` | ✅ PASS（FAIL 0 / DISCLOSURE 14） |
| `check_document_completeness.sh` | ✅ PASS=PASS 13 WARN 8 FAIL 0 |
| `check_evidence_binding.sh v4.1.0` | ✅ PASS=122 WARN=122 **FAIL=0** UNVERIFIED=0 |
| `check_docs_pointer_paths.sh --scope docs/releases/v4.1.0` | ✅ RC=0（32 broken / 33 registered，无新增） |
| `check_workflow_expressions.sh` | ✅ PASS（7 个 workflow 全扫） |
| `check_gate_script_refs.sh` | ✅ PASS（missing NEW: 0；53 处引用全部可解析且可执行） |
| `check_ci_checkout_consistency.sh` | ✅ ALL CHECKS PASS |
| `check_anti_ignore_gate.sh` | ✅ RC=0（tree recount 119 == total_allowed 119） |
| `cargo fmt --check --all`（1.91.0 / 1.95.0 / 1.97.0） | ✅ 三版全 CLEAN |
| `coverage_total_percent.py --selftest` | ✅ 5/5 PASS |

### 7.1 一个**不**算本批回归的失败：10 原则门禁的 R5 / R7

`check_10_principles_v310.sh v4.1.0` 仍报 **R5 / R7 两条 FAIL**，但两条都
与本批改动无关，理由分别是：

- **R7（文档完整性）**：判据是**硬编码的 v3.10.0 文档清单**
  （`DEVELOPMENT_PLAN.md` / `FEATURE_CHECKLIST.md` / `INDEX.md` / `plans/`），
  而 v4.1.0 的树里这四个从来就不存在 ⇒ 版本结构错配，不是回归。
- **R5（覆盖率）**：本批已修好（§5 + §6）。修好后仍需跑满 6 个 crate 才能整体转绿，
  本批未跑满（单 crate 路径已验证，见 §6.5）。

该脚本未接入 v4.1.0 的 CI。

---

## 8. 基础设施状态（不产出任何 PASS/FAIL 证据）

`192.168.0.252` 自本批开始持续不可达：

```
$ curl --max-time 8 http://192.168.0.252:3000/api/v1/version
HTTP 000 in 0.005114s
$ ssh -o ConnectTimeout=5 openclaw@192.168.0.252
ssh: connect to host 192.168.0.252 port 22: Connection refused
```

主机 ping 得通（4.5 ms）⇒ 网络路径在，是宿主机本身不可用。
**定性：基础设施红。** 本批不据此产出任何 PASS/FAIL 结论，也不把 CI 侧的事情
记成代码缺陷。

---

## 9. 本批改动清单

| 文件 | 性质 |
|---|---|
| `CURRENT_VERSION.md` | 措辞统一（6 处） |
| `docs/releases/v4.0.0/RELEASE_NOTES.md` | 措辞统一（头部 1 处） |
| `docs/releases/v4.1.0/STAGE.yaml` | open_issues 7 → 2；新增 `resolved_issues_2026_10_23`（5 条） |
| `docs/governance/debt/debt-registry.yaml` | SEM-4：IN_PROGRESS → CLOSED |
| `crates/vector/tests/cosine_similarity_5384.rs` | fmt 修复（潜伏 CI 红） |
| `scripts/gate/check_10_principles_v310.sh` | R4 去 apply；R5 去乱码 flag / 去掩码 / 去 0% 兜底 |
| `scripts/gate/common.sh` | 承载 `coverage_extract_total_percent()` 唯一入口 + selftest 入口 |
| `scripts/gate/coverage_total_percent.py`（新） | 覆盖率百分比提取规则 + 5 个 fixture |
| `scripts/gate/check_coverage_v312.sh` | 改用共享规则 |
| `.gitea/workflows/ci.yml` | Coverage 步骤前置 selftest |

## 10. 仍 open（不在本批范围）

`docs/releases/v4.1.0/STAGE.yaml` 余下 2 条：

- **P1-02 #4540 Q20 oracle 未闭环** —— `sqlrustgo/q20.tsv` 缺失，
  `CROSS_ENGINE_HASH_CHECK.json` 记 `present:false`。
- **P0-08 WAL 恢复逐条整表重写** —— 性能问题，范围较大，单独做。

## 11. 待服务器恢复后的动作

1. 推本批 commit（远端仍停在 `31ea00c2be`）。
2. 新 CI run 验证：**lint-build 绿**（含这次第一次会真正执行的 Format check）、
   **postcheck 绿**、#5451 缓存生效（`Downloaded` 次数降为 0）。
3. 更新 PR #5554 正文（§12：3 个新问题 + #5451 实现 + SHA 演进表）。
4. 登记本批的三个门禁缺陷为 Gitea issue（fmt 漂移 / R4 空转 / R5 乱码 flag /
   覆盖率提取规则），并 CLAIM。

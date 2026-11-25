# 任务：v4.1.0 基线对齐分析 + 重构与 harness 治理实施方案

> **收件人**：远端服务器上的 Claude Code CLI
> **发件方**：SQLRustGo 本地会话（dsh-b 线）
> **日期**：2026-11-26
> **仓库**：`github.com/minzuuniversity/sqlrustgo`（**只读镜像**；权威仓库是内网 Gitea `192.168.0.252:3000/openclaw/sqlrustgo`）
> **基线分支**：`develop/v4.1.0`（tip `a604ed9ab3`）
> **本任务书所在的分支**：`fix/v410-criterion8-evidence-layer-20261126`（tip `c5f938591c`）——
> 它在 `develop/v4.1.0` 之上（PR #5933，**尚未合并**），含本轮的全部产出。
>
> ```bash
> # 拿到本任务书与它依赖的全部材料：
> git clone --branch fix/v410-criterion8-evidence-layer-20261126 \
>     https://github.com/minzuuniversity/sqlrustgo.git
> # 任务书：
> #   docs/releases/v4.1.0/V410_REMOTE_BASELINE_ALIGNMENT_TASK.md
> ```
>
> > 若该分支已被合入 `develop/v4.1.0`，直接取 `develop/v4.1.0` 即可。
>
> ## ⚠️ 本任务的性质
>
> **本任务只产出分析与方案，不做开发。** 不要修改任何产品代码、门禁脚本、配置或台账。
> 交付物是**一份报告 + 一份实施方案**（见 §1）。
>
> 远端环境**无法访问**内网 Gitea，因此本文件必须**自包含**：所有需要的事实都在文件里，
> 或者指向**已提交进本仓库**的具体文件路径。**不要**假设能访问 `192.168.0.252`。
>
> 本地会话会同步你的产出（见 §7），并在本地做属主级裁决。**你不能代替属主做裁决。**

---

## 1. 交付物

| # | 交付物 | 落点 | 形态 |
|---|---|---|---|
| D1 | **基线对齐分析报告** | `docs/releases/v4.1.0/evidence/BASELINE_ALIGNMENT_2026-11-25.md`（实际落盘；一、《基线对齐报告》） | 分析 + 结论 + 出处 |
| D2 | **重构与 harness 治理实施方案** | `docs/releases/v4.1.0/evidence/BASELINE_ALIGNMENT_2026-11-25.md`（实际落盘；二、《重构路线图 v0.1》＋三、《Harness 治理启动包 v0.1》） | 分阶段方案 + 验收判据 + 风险 |
| D3 | **给属主的裁决清单** | 同 D2 的一节 | 需要属主拍板的每一项，逐条列出选项 |

> **落点订正（2026-11-26）**：D1/D2 原计划各新建一份文件（计划名
> V410_BASELINE_ALIGNMENT_ANALYSIS.md / V410_REFACTOR_HARNESS_PLAN.md），实际交付时
> **合并为一份三段式文档** `docs/releases/v4.1.0/evidence/BASELINE_ALIGNMENT_2026-11-25.md`
> （一、基线对齐报告＝D1；二、重构路线图 v0.1＝D2；三、Harness 治理启动包 v0.1＝D2）。
> 两个计划文件名**从未入库**，故本表按实际落盘路径订正——文档里不留指向未入库文件的指针。

**两件事必须做到**：

1. **每条结论都带可核出处**（文件:行号 / 命令 + 实测输出 / 归档 run 的路径与 sha256）。
   **不带出处的结论等于没有结论** —— 这是本仓库的核心纪律（AFP「不得默认成功」）。
2. **不得代替属主裁决。** 遇到需要人拍板的地方，**列成选项 + 各自的代价与后果**，
   交属主（见 §6 的清单格式）。

**不要**写「已放行」「准入通过」「C-ALPHA-02 已启动」这类结论 —— 当前**没有**任何一条成立
（理由见 §2.3）。

---

## 2. 事实基础（自包含部分）

### 2.1 项目是什么

SQLRustGo 是一个 **Rust** 实现的 **SQL-92 子集**关系数据库系统。现代分层架构：

```
crates/cli            REPL / sqlrustgo 二进制
crates/mysql-server   MySQL 风格协议服务端
crates/server         TCP 服务端
src/execution_engine.rs + src/engine_*.rs   查询执行
crates/executor       表达式求值
crates/planner        查询计划        crates/optimizer   查询优化
crates/parser         SQL → AST（lexer.rs + parser.rs 在同一 crate 内）
crates/storage        Page / BufferPool(LRU) / B+ Tree 索引
crates/transaction    WAL / TxManager
crates/network        TCP server/client
crates/types          Value / SqlError
```

Rust 2024 edition + Tokio 异步运行时。

### 2.2 阶段与门禁制度

- **当前阶段**：`ALPHA`（`docs/releases/v4.1.0/STAGE.yaml` 的 `current_stage: "ALPHA"`）。
- **门禁分母固定 168**：`scripts/gate/check_*.sh` + `check_*.py` 共 168 个文件，
  登记在 `docs/releases/v4.1.0/GATE_CATALOG.jsonl`（168 行，与文件集**双向一一对应**）。
  **不新增 `check_*`** 是仓库的硬规则（新判据一律挂靠既有门禁）。
- **阶段清单**：`docs/governance/STAGE_CONFIG.yaml` 的 `stages.<STAGE>.required_gates`。
- **覆盖率阈值**（SSOT，`docs/governance/STAGE_CONFIG.yaml`）：
  `COVERAGE_MIN_ALPHA: 50` / `BETA: 75` / `RC: 80` / `GA: 85`。
- **负向夹具台账**：`docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl`（57 行）。
  每个门禁要声称「能抓什么」，必须有一条**可证伪**的负向夹具证明它。
- **诚实性门禁**：`audit/v4.1.0/honesty.py` —— C1–C5 五条判据，其中
  **C3 = 每个「声称能抓」的性质必须逐字落在夹具的 `fixture_id`/`note`/`fixture` 上**。

### 2.3 当前状态（2026-11-26，本地实测）

**内容基线（BASE）已冻结完成**：

| 项 | 值 |
|---|---|
| `base_sha` | `fd849ae20541901d5e59f45cd3ba2c51093f5d5e` |
| `tree_sha` | `72d378e8e58797eae18502ab474325015b8993d4` |
| `source_ref` | `base/v4.1.0-frozen` |
| 冻结时点的 develop tip | `8cbde5b6d33c876fa1197474c46d50142a9823bd` |

出处：`docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json`。
BASE 经 PR #5928–#5932 合入 `develop/v4.1.0`。

**12 条判据（重构准入的判据表）**：

| 判据 | 内容 | 状态 |
|---|---|---|
| 1 | BASE 唯一、develop tip 是其祖先、含 BASE 的分支已推送 | **PASS** |
| 2 | `45da6968ae` 的 123 个材料文件全在 BASE 树内 | **PASS** |
| 3 | 前向/历史 SHA 分界（前向绑定唯一等于 BASE；历史绑定逐项未变） | **PASS** |
| 4 | ADR 机器字段非空、档位正确、映射满足 A2.2 五条规则 | **FAIL（映射层）** |
| 5 | manifest 零漂移 | **PASS** |
| 6 | 已执行切片全部有终态 + Oracle 签署（签署人合格）+ 双向差异表 | **FAIL（签署层）** |
| 7 | ALPHA 阶段清单有 CI 执行点 | **PASS** |
| 8 | ALPHA 7 道负向夹具两层 RED | **PASS**（本轮修复，见 §2.4） |
| 9 | 门禁编目内部自洽 | **PASS** |
| 10 | 外审包三套统计一致（71/71/71） | **PASS** |
| 11 | dead 门禁口径唯一化 | **PASS** |
| 12 | SQLRUSTGO_LOCAL_ONLY 路由 fail-closed —— 本次准入**显式豁免**（O9） | **WAIVED** |

**PASS 9 / FAIL 2 / WAIVED 1**（判据 8 在本轮由 FAIL 转 PASS）。
判据表：`docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json`
（由 `gen_verdict.py` 生成，可复算：`python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py --out /tmp/verdict-check.json` 然后比对 `summary` —— **该生成器没有 `--check` 子命令**，详见 §5）。

**属主裁决位仍然是空的**：`docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md` §4
（放行/不放行、签署人、日期、放行批次、附加条件）与 §4.1（O9 顺带确认）
**全部空白**。**这是有意的** —— 实施者与控制者都不得代属主签字。

### 2.4 本轮（2026-11-26）已完成的判据 #8 修复 —— **你必须在此基础上工作**

判据 8 此前是 **FAIL**（RED 5/7 + INCONCLUSIVE 2/7）。两处 INCONCLUSIVE 都是**证据层**问题：

**(a) fx18（覆盖率门禁）—— 夹具注入的红不可归因**

RED-by-design 的特征测试 `crates/storage/tests/tf_t4_ai_candidate_v1.rs` 与
`crates/storage/tests/tf_t4_stub_silent_success.rs`（共 8 个 `#[test]`）**按设计失败**
—— 它们断言的是已知缺陷「STUB_SILENT_SUCCESS」（测试工厂的 T4 storage 能力：
`BinaryTableStorageV2` 的 scan/delete/update 等 15 个方法是桩）。
后果：`cargo llvm-cov test -p sqlrustgo-storage` 退出码 **101** ⇒ 该 crate **产不出覆盖率表**
⇒ L1_8 样本只有 **7 of 8** ⇒ `coverage_verdict` 走 fail-closed 分支报
`FAIL: measured 7 of 8 crates` ⇒ 这个红**在注入之前就存在** ⇒ 注入的红**无法归因**。

**修法（排除测量，不排除测试）**：`scripts/gate/common.sh` 新增

```bash
COVERAGE_RED_CHARACTERISATION_TEST_FILES=(
    "crates/storage/tests/tf_t4_ai_candidate_v1.rs"
    "crates/storage/tests/tf_t4_stub_silent_success.rs"
)
```

并纳入 `coverage_excluded_test_names()` 的提取循环（libtest `--skip`，按**函数名**过滤 ——
cargo 只能排除整个**包**，不能排除整个**测试 target**）。
另加 `coverage_print_excluded_tests()`，**每次覆盖率报告**都打印被排除的集合。

**边界（务必守住）**：`coverage_verdict` **一字未改**（其它原因导致 crate 不可测仍 FAIL 且点名该 crate）；
**不改** `#[ignore]`、**不改**断言、**不改**阈值档；RED 特征测试**仍然 RED**（#5913/#5914）。

**(b) fx15（composite 超时）—— 泳道顺序**

夹具用 `git checkout -- .` 还原 scratch 树 ⇒ **mtime 变了** ⇒ cargo 的 mtime 指纹过期
⇒ 下一个消费 cargo 产物的夹具要**重编根包 + 11 个依赖者**，实测 **>90min**，7200s 预算不够。

**修法**：`audit/v4.1.0/neg/verify_neg_fixtures.sh` 的 `GATES` 重排为
`fx02, fx20, fx19, fx15, fx18, fx16, fx17` ——
规则：**凡消费 cargo 产物的夹具，必须排在任何改 Rust 源码的夹具之前**；
fx16/fx17 是静态门禁（秒级）移到最后。另加**无条件预热**
（`cargo build --all-features && cargo test --all-features --lib --no-run`，
无 env 开关、独立 `warmup.log`、失败 `exit 4`）。
fx15 预算 7200 → **14400**。

**(c) C5 缺陷：泳道必须自报树身份**

`verify_neg_fixtures.sh` 头部现在打印：

```
verify_neg_fixtures: tree_sha=<40 hex> dirty_files=<n>
```

`dirty_files` 必带：`TREE != REPO` 时脏检查被跳过 ⇒ 一棵**被改过的**树与一棵**干净的**树
**共用同一个 commit SHA** ⇒ 只有 SHA 不足以标识被测树。
`gen_verdict.py` 的判据 8 改为**先从日志读**：有 `tree_sha=` 且 `dirty_files=0` ⇒ **观测值**；
有且 >0 ⇒ **部分观测值**；无 ⇒ 退回 `--neg-run-tree` 并标**推定值**。

**本轮实测结果**：

- go/no-go 测量（`SQLRUSTGO_COVERAGE_STAGE=ga bash scripts/gate/check_coverage_v312.sh`）：
  **8/8 crates 可测**，`sqlrustgo-storage: 80.93%`（以前根本产不出来），
  `average coverage: 87.72% (threshold: 85%) — measured 8/8 crates`，**rc=0**。
- 两层夹具泳道：**`RED=7/7 INCONCLUSIVE=0/7`**，退出码 **0**，**零** `exit_code=124`，
  每条夹具日志含 `FX-GATE-LAYER: state=FAIL exit_code=1` 与 `FX-STAGE-LAYER: verdict=FAIL`。
- 独立探针复算：`probe_neg_fixtures.py fixtures` ⇒
  `pass=19 inconclusive=0 fail=0 error=0 deferred-selftest-red=3 total=22`。
- 归档 run：`audit/v4.1.0/runs/20261126-criterion8-negfix-001/`
  （`manifest.yaml` + `logs/001-neg-fixtures-two-layer.stdout` +
  `logs/003-neg-fixtures.stdout` + `logs/fixtures/*.log`），
  `audit_run.py verify` ⇒ **verify OK**（evidence_hash `sha256:1132edd5…`）。
- 判据表重出：`summary.fail_criteria` 从 `[4,6,8]` 缩为 **`[4,6]`**，
  `criteria[7].verdict == "PASS"`。

---

## 3. 你要分析的核心问题

### 3.1 问题 A —— 「基线对齐」到底还差什么

**背景**：BASE 已冻结，判据 1/2/3/5/7/8/9/10/11 全绿，判据 4 与判据 6 仍 FAIL，
且**都不是执行者能自己解决的**。

**你要产出的是**：一个**可执行的、对齐工作的分解**，说明

1. **判据 4（映射层）** —— 现状是：ADR 的**机器字段**（11 份的档位、`Approvals` 字段、
   `adr_catalogue`）**齐了**，`verify_handoff.py --check` 的 **10 条**变异臂**全被拒**；
   缺的是**映射** —— A2.2 **R1**「每个 task 的 `approved_adr_refs` 至少 1 项」被 **C01-15** 违反
   （`docs/refactor/v4.1.0-c/takeover/ADAPTER-MAPPING-PENDING.md` 的待决清单非空）。
   该包与 11 份 ADR 的「目标不变量」**无交集**（唯一不变量 INV-17 是测试工厂的认证口径不变量，
   不是架构不变量），所以「填一个无关 ADR 去凑数」被 A2.2 R2 明令禁止。
   **⇒ 请分析**：要拿到属主的 O1 裁决，执行者侧**还需要准备什么**（清单、口径说明、
   两种选项各自的后果）？哪些材料现在就能备好（不改判据）？

2. **判据 6（签署层）** —— 现状是：材料齐了（44/44 有终态：42 PASS + 2 NOT_RUN；
   双向差异表 `L1-DIFF-fd849ae20541.md` 表头逐字符匹配），但**Oracle 副签 0 条**
   —— 44 条的 `assertion_review_verdict` 是 38 条 `ASSERTION_UNREVIEWABLE` +
   6 条 `JUDGED_PENDING_COUNTERSIGNATURE`，`assertion_review_performed_by` 是**实施者本人**，
   按 R-ORACLE「签署人 ≠ 该场景实现者」**不构成签署**。
   判据 6 的机器门槛是**六个键的合取**（`c6_*`，见 `gen_verdict.py`），
   其中 (4)(5) 是**防空判护栏**。
   **⇒ 请分析**：签名前，Reviewer 需要什么才能真正**独立**地判断？
   T1 30 条（层 1，逐条）与 T2/T3 14 条（层 2，批量）分别需要什么材料？
   哪些用例按 R-T7-2 本来就**不该**被判定（不可独立判定）？

3. **13 条不变量零覆盖** —— 外审包（判据 10 的对象）里的 13 条不变量目前**零覆盖**。
   这与判据 10「三套统计一致」**不冲突**（#10 测的是**统计口径自洽**，不是**覆盖充分性**）。
   **⇒ 请分析**：这 13 条分别是什么（`docs/refactor/v4.1.0-c/takeover/INVARIANT-GAP-PROPOSALS.md`
   可能有线索）？每条补覆盖的**成本量级**与**依赖**？哪些适合随批次指定责任包？

### 3.2 问题 B —— 「开始重构」的准入路径

**背景**：判据 8 已 PASS（证据层通了），但 #4/#6 还在属主域。

**你要产出的是**：**在判据 #4/#6 未解之前**，哪些重构工作**可以**开始、
哪些**必须**等裁决，以及**判据 #4/#6 落地后**的重构批次划分。

**必须守住的边界**（来自 `V410-OWNER-RULING.md` §5 的层序建议）：

```
层0   判据 #8 证据层   → 执行者域，已完成（本轮）
层0'  判据 #4 映射层   → 属主裁决 O1（执行者不得自拟映射，A2.2 R5 / O10）
层1   判据 #6 层 1     → T1 30 条**逐条**副签（Reviewer 已获授权，待填名单表）
层2   判据 #6 层 2     → T2/T3 14 条**批量**副签
层3   核心重构包       → 属主裁决 O5（放行批次）
```

**⇒ 请分析**：层 1 之前有没有**可以并行做**、且**不依赖**属主裁决的重构准备工作？
（例如：把某个待重构模块的**现状测绘**做完、把**可证伪的验收判据**先写出来、
把**测试夹具**先备好 —— 但**不动**产品代码。）

### 3.3 问题 C —— 「harness 治理」

**背景**：`crates/storage/tests/tf_t4_*.rs` 是**按设计 RED** 的特征测试。
本轮为了不让它们毁掉覆盖率测量，把它们从**测量**里排除了 —— 但它们**仍然 RED**，
仍然挂在 RED-LEDGER（#5913/#5914）上。这是**正确**的做法，但**不是终局**。

**你要产出的是**：一个 **harness 治理方案** —— 让 RED 特征测试与门禁的关系**可持续**，
而不是靠一个个手工排除。

**请具体分析**（这是本任务最需要创造力的部分）：

1. **RED 特征测试的"应当失败"如何被机器判定？**
   现在它们是普通 `#[test]`，失败就是失败。可选方向（请评估每一条的代价与风险）：
   - 在测试**内部**声明期望（如 `assert_red!` 宏：失败 ⇒ 通过，不失败 ⇒ 红）；
   - 用 `#[should_panic]` 表达「这个缺陷会 panic」；
   - 建立一个**独立的**「RED 套件」执行入口（一个 `cargo test --test red_suite` 或独立 crate），
     让普通 `cargo test` **不**跑它们，但**专门的** RED 门禁**必须**跑它们且**必须**看到它们红；
   - 用一个**清单文件**（如 tests/baseline/red_suite.json —— 计划名，未入库）登记每个 RED 测试的
     预期失败信号（哪一行、什么消息），门禁比对**实际**失败与**登记**的失败。
2. **「red-by-design」与「意外红」如何区分？**
   现在两者在 CI 输出里长得一样。方案必须让「预期红」与「意外红」在**同一份输出**里可区分，
   且**意外红必须仍然让门禁红**。
3. **与现有门禁的关系**：
   `scripts/gate/check_anti_ignore_gate.sh` 管 `#[ignore]` 的登记（`tests/baseline/ignore_registry.json`）；
   `scripts/gate/check_gate_test_integrity.sh` 管掩码检测器。
   你的方案**不得**新增 `check_*`（分母 168 固定）—— 必须**挂靠既有门禁**。
   请明确指出**挂在哪一道**、**判定逻辑在哪一层**。
4. **与覆盖率门禁的关系**：
   `COVERAGE_RED_CHARACTERISATION_TEST_FILES` 这个排除清单本身是**手工维护**的。
   方案应该让它**自动派生**（例如从 RED 套件清单派生），还是保持手工 + 门禁钉死？
   两种做法各自的失败模式是什么？
5. **迁移路径**：从**现在**（8 个函数被手工排除）到**你的方案**，分几步？
   每步的**可证伪判据**是什么？**怎么证明新方案没有把真的缺陷放过**？
   （这是关键：把 RED 测试改成「预期失败」的写法，天然有把真缺陷藏起来的风险。）

### 3.4 问题 D —— 本轮引入的新风险的独立复核

本轮引入了一个**属主级门禁行为变更**：

> 覆盖率门禁在冻结基线上的判定，从「不可测 ⇒ FAIL（`measured 7 of 8 crates`）」
> 变成「可测的真实数字（`measured 8/8 crates`，均值 87.72% vs GA 档 85%）」。

**这不是阈值放宽**（`COVERAGE_MIN_GA` 仍是 85）。但它是**行为变更**，需要独立复核。

**请独立复核**（你可以自己跑这些命令）：

| # | 复核点 | 命令 |
|---|---|---|
| D1 | 排除**只**作用于测量，RED 测试**仍然 RED** | `cargo test -p sqlrustgo-storage --test tf_t4_stub_silent_success --test tf_t4_ai_candidate_v1` ⇒ 应 `0 passed; 4 failed` |
| D2 | `coverage_verdict` **一字未改** | `git log -p --follow scripts/gate/common.sh` 看本次 diff 里**不含**该函数 |
| D3 | 其它原因导致不可测**仍然 FAIL** 且点名 crate | `bash -c 'source scripts/gate/common.sh; coverage_measurement_selftest'` ⇒ 4 个用例全过（含 `reduced sample (storage missing) => FAIL`） |
| D4 | 排除集合**在报告里可见** | `SQLRUSTGO_COVERAGE_STAGE=ga bash scripts/gate/check_coverage_v312.sh` 的输出**开头**应含三个 `excluded from coverage measurement (...)` 分组与函数名清单 |
| D5 | 分母**没漂** | `python3 scripts/gate/gate_registry.py check` ⇒ `check_n=168`、`catalog_rows=168`、`fails=0` |
| D6 | 「声称能抓」落在证据上 | `python3 audit/v4.1.0/honesty.py check` ⇒ `fails=0` |
| D7 | 台账是**生成物** | `python3 audit/v4.1.0/gen_neg_ledger.py --check` ⇒ 逐字节一致 |
| D8 | 排除范围被门禁测试**钉死** | `cargo test --test alpha_gate_blocker_test --all-features` ⇒ `8 passed`（**不是 9** —— 本轮**扩展已有**断言而**不新增** `#[test]`，以免改动 `tests/baseline/gate_test_baseline.json` 的计数） |
| D9 | **没有**新增 `#[ignore]` | `grep -rn "#\[ignore" crates/storage/tests/tf_t4_*.rs` ⇒ 应无输出 |
| D10 | **没有**改产品代码 | `git diff <BASE> HEAD --stat -- crates/ src/` ⇒ 应为空（`crates/storage/tests/tf_t4_*.rs` 本轮也未改） |
| D11 | 判据表自洽 | **没有 `--check`** —— 用 `python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py --out /tmp/verdict-check.json` 重跑并比对 `summary`（命令见 §5）⇒ 应 `MATCH: True`，`fail_criteria` 应为 `[4,6]` |

**如果你发现本轮的修复有漏洞，请直接指出来** —— 你的报告里要有独立的判断，不要复述本轮的
自我审核（`docs/releases/v4.1.0/PR-5933_TEST_REVIEW.md` 那是实施者自己的审核，
**不构成**独立签署）。

---

## 4. 已知的坑（别踩）

| # | 坑 | 怎么认 |
|---|---|---|
| 1 | **准入位是空的**，任何「已放行」都是伪造 | `docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md` §4 |
| 2 | **实施者不得自签**（判据 6）、**不得自拟映射**（判据 4） | R-ORACLE / A2.2 R5 / O10 |
| 3 | **分母 168 固定**，不新增 `check_*` | `docs/governance/STAGE_CONFIG.yaml` + 看板 §3 Q1 |
| 4 | **`*.json` / `*.log` / `*.tbl` 禁入仓** | `.gitignore`；证据用 `.stdout` / `.stderr` / `.yaml` |
| 5 | **门禁不是装饰**：168 道里 `nofail=4` 结构上永不红、`swallow=56` 把失败退化成成功 | 看板 §3 Q1/Q2 |
| 6 | **看板 MD 的台账段是生成物**，手改会被 `kanban_render.py check` 判红 | `scripts/gate/kanban_render.py` |
| 7 | **看板只增不改**；单侧合并标记会让 `kanban_merge_policy.py` 判红 | 同上 |
| 8 | **「全绿」推不出「没缺陷」** | 看板 §3 Q1 |
| 9 | 外部咨询通道：`chatGPT-web` 可用；**deepseek-web 2026-11-26 实测 HTTP 401** ⇒ **双血统不可得**，单血统**不得**当双血一致 | 看板 §3 Q17 |
| 10 | 远端**无法访问内网 Gitea** —— 所有事实必须来自本仓库 | — |
| 11 | **`verify_handoff.py --check` 需要 `jsonschema==4.23.0`**（`python3 -m venv` + `pip install 'jsonschema==4.23.0'`）；`gen_verdict.py` 在缺它时会拒绝运行并提示建 venv | `docs/releases/v4.1.0/refactoring/tools/verify_handoff.py` |
| 12 | **`gen_verdict.py` 没有 `--check`**（远端实测：退出码 2，`unrecognized arguments: --check`）—— 用「重跑到仓外 + 比对 `summary`」代替；且**必须**带 `--out` 指到仓外，否则它会**就地覆盖**已提交的判据表 | `docs/refactor/v4.1.0-c/takeover/gen_verdict.py` |

---

## 5. 方法要求

1. **先跑，再写。** 每个结论都要有**你自己跑出来的**输出。
   最小必跑集：

   ```bash
   python3 scripts/gate/gate_registry.py check
   python3 audit/v4.1.0/honesty.py check
   python3 audit/v4.1.0/probe_neg_fixtures.py check
   python3 audit/v4.1.0/gen_neg_ledger.py --check
   python3 scripts/gate/kanban_render.py check docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
   python3 scripts/gate/kanban_merge_policy.py docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl
   cargo test --test alpha_gate_blocker_test --all-features
   bash -c 'source scripts/gate/common.sh; coverage_measurement_selftest'
   ```

   **判据表的可复现性复核**（⚠️ **没有 `--check` 子命令** —— 远端评审实测确认，
   `gen_verdict.py --check` 退出码 2：`unrecognized arguments: --check`。
   正确的复核方式是**重跑生成器并比对 `summary`**）：

   ```bash
   # 生成器**不带参数**即指向仓内的 7/7 归档（2026-11-26 修订后），
   # 输出到仓外，仓内保持干净：
   python3 docs/refactor/v4.1.0-c/takeover/gen_verdict.py --out /tmp/verdict-check.json
   python3 - <<'EOF'
   import json
   a = json.load(open('/tmp/verdict-check.json'))
   b = json.load(open('docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json'))
   print('regenerated:', a['summary']); print('committed  :', b['summary'])
   print('MATCH:', a['summary'] == b['summary'])
   EOF
   ```

   期望：`MATCH: True`，且 `summary` = `{"total":12,"pass":9,"fail":2,"waived":1,"fail_criteria":[4,6]}`。

   > ⚠️ **注意**：`SQLRUSTGO_COVERAGE_STAGE=ga bash scripts/gate/check_coverage_v312.sh`
   > 在**冷 target** 上要跑**很久**（8 个 crate 逐个 `cargo llvm-cov`，单线程带插桩）。
   > 先跑 `cargo build --all-features` 预热。远端如果时间/资源受限，
   > **如实写「未复现」**，不要编。

2. **不确定就说不确定。** 本仓库的纪律是「**证据不足 ≠ 红，证据不足 ≠ 绿**」。
   写「我没有能力判定这一项，请属主裁决」是**合格**的输出；
   写一个看起来笃定但没有出处的结论是**不合格**的输出。

3. **区分「我测到的」与「文档里写的」。** 报告里每条结论标来源：
   `[实测]` / `[文档]` / `[推断]`。

4. **不要复述本轮的自我审核。** `PR-5933_TEST_REVIEW.md` 是实施者自己写的，
   它**不构成**独立复核。你要做的是**独立**判断。

---

## 6. 给属主的裁决清单（格式要求）

在 D2 里单独一节，用**这个格式**逐条列出需要属主拍板的事：

```
### 裁决项 N —— <一句话标题>

**问题**：<具体到可判的粒度>

**选项**：
- **(a) <选项>** —— 代价：…；后果：…；可逆性：…
- **(b) <选项>** —— 代价：…；后果：…；可逆性：…

**执行者侧的前置准备**（不依赖裁决就能做完的）：…

**你的建议**：<有倾向就写，没有就写「无倾向」>；理由：…

**不裁决的后果**：…
```

**至少覆盖**：

| # | 裁决项 | 依据 |
|---|---|---|
| N1 | 判据 #4 的映射口径（C01-15 是否需要绑定 ADR） | `V410-OWNER-RULING.md` §1.1 的 `#4` 行 |
| N2 | 判据 #6 的 Reviewer 名单（谁、授权范围、日期） | `V410-OWNER-RULING.md` §3.1.1（该表是判据 6 的**机器输入**，**实施者与控制者不得代填**） |
| N3 | 层 3 的放行批次（O5：放行哪些重构包） | `V410-OWNER-RULING.md` §5 的建议第 2 条 |
| N4 | 判据 12 的 O9 确认行（§4.1 一行） | `V410-OWNER-RULING.md` §4.1 |
| N5 | 本轮引入的覆盖率门禁行为变更是否接受 | 本轮 §2.4（属主级门禁行为变更） |
| N6 | harness 治理方案的**方向**选择（你在 D1 提出的各条路线） | 本任务 §3.3 |
| N7 | 13 条不变量的责任包分配 | 本任务 §3.1.3 |

---

## 7. 交付与同步

### 7.1 提交

- 分支：`feat/v410-baseline-alignment-analysis-20261126`
- 目标：`origin` = `https://github.com/minzuuniversity/sqlrustgo.git`
- 提交信息用仓库的格式：`<type>: <description>`，
  `type` ∈ {feat, fix, refactor, docs, test, chore, perf, ci}。
- **只提交两个交付物**（D1、D2）。**不要**改任何其它文件。

### 7.2 写一份「给属主的回执」

在 D2 的末尾加一节 **§8 回执**，内容：

1. 你**实际跑了**哪些命令、**实测**结果是什么（表格：命令 / 退出码 / 关键输出）；
2. 你**没能**复现或**没有能力**判定的项（如实列出）；
3. 你的裁决清单摘要（N1–N7 各自的建议）；
4. 你**没有**做的事（明确边界）。

### 7.3 同步回内网

本地会话会把你的分支从 GitHub 拉回内网 Gitea，**由属主审阅后再决定是否合入 `develop/v4.1.0`**。
你的提交**不需要**（也不应该）自己去动 `develop/v4.1.0`。

---

## 8. 一句话总结你的任务

> **在判据 #8 已转 PASS、内容基线已冻结的当前状态上**：
> 分析「基线对齐还差什么」，给出「哪些重构现在能开始、哪些必须等裁决」的分解，
> 设计一个**让 RED 特征测试与门禁的关系可持续**的 harness 治理方案，
> 独立复核本轮引入的覆盖率门禁行为变更，
> 并把每一件需要人拍板的事整理成可裁决的清单交给属主。
>
> **只出方案，不做开发。**

---

## 附录 A —— 关键文件导航

| 文件 | 是什么 |
|---|---|
| `docs/releases/v4.1.0/V410_BASELINE_FROZEN_DECLARATION.md` | 本轮的基线冻结声明（AFP 五字段证据头 + 显式否定清单 + 残余风险） |
| PR-5933_SPEC.md（**计划名，从未入库**） | 本轮的 SPEC（问题陈述 / 设计 / **门禁行为变更声明** / 风险 R1–R10）——实际入库的是下面三份 |
| `docs/releases/v4.1.0/PR-5933_TEST_DESIGN.md` | 本轮的具体测试方法（四条判别力护栏 G1–G4 等） |
| `docs/releases/v4.1.0/PR-5933_ACCEPTANCE.md` | 本轮的门禁执行证据（go/no-go 87.72%、泳道 7/7、判据表重出） |
| `docs/releases/v4.1.0/PR-5933_TEST_REVIEW.md` | 本轮的**自我**审核（**不构成**独立签署，勿当独立复核） |
| `docs/refactor/v4.1.0-c/takeover/V410-OWNER-RULING.md` | 属主裁决文件（**§4 裁决位留空**、§3.1.1 Reviewer 名单表留空、§4.1 O9 留空、§1.1 三层归因、§5 层序建议） |
| `docs/refactor/v4.1.0-c/takeover/V410-READINESS-VERDICT.json` | 12 条判据的机器可读表（当前 PASS 9 / FAIL 2 / WAIVED 1） |
| `docs/refactor/v4.1.0-c/takeover/gen_verdict.py` | 判据表的生成器（可复算；判据 6 的六个 `c6_*` 键、判据 8 的树身份读取都在这里） |
| `docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json` | 冻结记录（BASE 四元组 + 123 份材料导入 + ADR 目录 + 冻结记录集扩容） |
| `docs/refactor/v4.1.0-c/takeover/ADAPTER-MAPPING-PENDING.md` | 判据 4 的缺口（C01-15 的待决清单） |
| `docs/refactor/v4.1.0-c/takeover/INVARIANT-GAP-PROPOSALS.md` | 13 条不变量的覆盖提案（判据 3.1.3 的线索） |
| `docs/refactor/v4.1.0-c/takeover/ORACLE-REVIEW-LOG-T1.md` | T1 的 Oracle 评审日志（44 条的现状） |
| `docs/refactor/v4.1.0-c/takeover/V410-READINESS-LEDGER.md` | 就绪台账（含 R50 等规则条目） |
| `docs/refactor/v4.1.0-c/takeover/V410-CONTROLLER-RULINGS.md` | 控制者裁决（规则口径的来源） |
| `docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md` | 看板（§3 问题汇总 Q1–Q17、§4 下一步、§6 证据索引） |
| `docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl` | 负向夹具台账（57 行；fx18 行现为 `mutation-verified`） |
| `docs/releases/v4.1.0/GATE_CATALOG.jsonl` | 门禁编目（168 行；每行的 `effect.catches` 是「声称能抓什么」） |
| `docs/governance/STAGE_CONFIG.yaml` | 阶段清单与阈值 SSOT |
| `docs/releases/v4.1.0/STAGE.yaml` | 阶段状态（`current_stage: ALPHA`） |
| `audit/v4.1.0/runs/20261126-criterion8-negfix-001/` | 本轮的泳道归档（manifest + 日志） |
| `crates/storage/tests/tf_t4_ai_candidate_v1.rs` | RED-by-design 特征测试（4 个 `#[test]`） |
| `crates/storage/tests/tf_t4_stub_silent_success.rs` | RED-by-design 特征测试（4 个 `#[test]`） |
| `scripts/gate/common.sh` | 覆盖率测量的共享实现（排除数组 / 打印 / `coverage_verdict`） |
| `audit/v4.1.0/neg/verify_neg_fixtures.sh` | 两层夹具泳道编排（顺序 / 预热 / 树身份自报） |
| `audit/v4.1.0/neg/neg_two_layer.sh` | 两层判定的共享实现（`fx_is_red`：超时不算红） |
| `audit/v4.1.0/honesty.py` | 诚实性门禁（C1–C5；C3 = 性质必须逐字落在夹具证据上） |

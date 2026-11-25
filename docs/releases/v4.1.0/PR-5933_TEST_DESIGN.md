# PR-5933 TEST_DESIGN — 具体测试方法

> Issue: #5592 / #5598 · 分支: `fix/v410-criterion8-evidence-layer-20261126`
> 基线: `origin/develop/v4.1.0` @ `a604ed9ab3`
> 配套文档: `PR-5933_SPEC.md` / `PR-5933_TEST_PLAN.md` / `PR-5933_TEST_REVIEW.md` / `PR-5933_ACCEPTANCE.md`

---

## 1. 测什么、不测什么

本 PR 的测试是**反例夹具（negative fixture）** 与**门禁判定测试**，不是产品功能测试。
本 PR **不新增**任何产品代码的单元测试 —— 因为本 PR **不改**产品代码。

| 层 | 测什么 | 谁在测 |
|---|---|---|
| 门禁判定层 | 覆盖率门禁在冻结基线上的判定（从「不可测」变「可测」） | `cargo test --test alpha_gate_blocker_test`（**扩展已有**断言，不新增 `#[test]`） |
| 反例夹具层 | 7 道 ALPHA 门禁的负向夹具能否在注入后**变红**、且**归因**成立 | `audit/v4.1.0/neg/fx*.sh`（fx18 改，fx15 改，其余不动） |
| 泳道编排层 | 夹具的**顺序**与**预热**是否让泳道在预算内跑完 | `audit/v4.1.0/neg/verify_neg_fixtures.sh` |
| 证据身份层 | 泳道是否自报被测树；判据表是否从日志**读**树身份而不是重构 | `verify_neg_fixtures.sh` + `gen_verdict.py` |
| 台账/编目层 | 「声称能抓」是否落在证据上；分母是否漂移 | `honesty.py` / `gate_registry.py` / `probe_neg_fixtures.py` / `gen_neg_ledger.py` |
| 看板层 | MD 是否是 JSONL 的生成物；有无单侧合并标记 | `kanban_render.py` / `kanban_merge_policy.py` / `test_kanban_render.py` |
| 声明层 | 声明有没有越过证据 | `grep` 红线检查 + 人工核对 |

---

## 2. 方法 1 —— 覆盖率门禁的「排除测量，不排除测试」

### 2.1 被测对象

`scripts/gate/common.sh` 的 `coverage_measure_crate()`：它对每个 crate 跑

```bash
cargo llvm-cov test -p "$crate" ... -- --test-threads=1 ${skip_args[@]+"${skip_args[@]}"}
```

`skip_args` 来自 `coverage_excluded_test_names()`，它遍历三个数组、从文件里提取
`#[test]` / `#[sqlrustgo_test]` 的函数名、逐个传给 libtest 的 `--skip`。

### 2.2 本 PR 加的第三个数组

```bash
COVERAGE_RED_CHARACTERISATION_TEST_FILES=(
    "crates/storage/tests/tf_t4_ai_candidate_v1.rs"
    "crates/storage/tests/tf_t4_stub_silent_success.rs"
)
```

**为什么用「提取函数名」而不是「排除整个 target」**：cargo 只能排除整个**包**，不能排除
整个**测试 target**；libtest 的 `--skip` 按**函数名**过滤。所以既有机制
（`COVERAGE_EXCLUDED_TEST_FILES` / `COVERAGE_PERF_BOUND_TEST_FILES`）本来就是这么做的，
本 PR 沿用**同一套**机制、**同一套**提取循环。

**为什么这 8 个函数是全部的**：两份文件里的 `#[test]` 各 4 条，共 8 条：

| 文件 | 函数 |
|---|---|
| `tf_t4_ai_candidate_v1.rs` | `test_insert_then_scan_visible` / `test_delete_returns_real_count` / `test_update_returns_real_count_and_persists` / `test_transaction_methods_reject_on_non_tx_backend` |
| `tf_t4_stub_silent_success.rs` | `tf_t4_01_scan_after_insert_sees_rows` / `tf_t4_02_delete_reports_real_count` / `tf_t4_03_update_reports_real_count` / `tf_t4_04_non_tx_backend_must_reject_or_document` |

提取是**运行时**的（读文件、按 `#[test]` 后面 3 行内找 `fn`）⇒ 新加一条 RED 特征测试
**自动**被覆盖，不需要改这个列表。

### 2.3 可见性护栏（R2）

新增 `coverage_print_excluded_tests()`，在 `coverage_measure_set()` 的**开头**调用 ⇒
**每次**覆盖率报告的开头都会打印：

```
excluded from coverage measurement (environment-dependent):
  file: ...
excluded from coverage measurement (perf-bound):
  file: ...
excluded from coverage measurement (red-characterisation):
  file: crates/storage/tests/tf_t4_ai_candidate_v1.rs
  file: crates/storage/tests/tf_t4_stub_silent_success.rs
excluded test functions (23):
  ...
```

**为什么必须打印**：一个**看不见**的排除没法审计 —— 读者无法知道 8/8 这个样本是被
裁剪过的，也无法知道裁掉了什么。

### 2.4 钉死范围的测试（扩展已有，不新增 `#[test]`）

在**已有**的 `coverage_gate_excludes_the_environment_dependent_tests` 里扩展断言数组
（从 4 个文件扩到 6 个），并新增 3 条断言：

1. `common.sh` 含 `COVERAGE_RED_CHARACTERISATION_TEST_FILES`（排除有自己的**具名**分组，
   不是悄悄加进别的数组）；
2. `common.sh` 含 `coverage_print_excluded_tests`（排除必须**在报告里**可见）；
3. 两份 tf_t4 文件里**没有** `#[ignore]`（排除是**测量排除**，不是把 RED 测试藏起来 —— R7）。

**为什么不新增 `#[test]`**：新增会改动 `tests/baseline/gate_test_baseline.json` 的
测试计数（那是另一个门禁的基线）。扩展已有测试的断言数组**不改变**计数。
本 PR 实测：`cargo test --test alpha_gate_blocker_test --all-features` ⇒
**8 passed; 0 failed**（与改动前同为 8 个测试）。

### 2.5 「不变的东西」怎么验

| 不变项 | 验证方式 |
|---|---|
| `coverage_verdict` 一字未改 | `git diff scripts/gate/common.sh` 的 diff 里**不含** `coverage_verdict` 函数体；且 `coverage_measurement_selftest` 的 4 个用例全过（含「reduced sample ⇒ FAIL」这一条） |
| 其它原因导致不可测仍 FAIL | `coverage_measurement_selftest` 的 `reduced sample (storage missing) => FAIL` 用例 |
| 阈值未改 | `git diff` 不含 `docs/governance/STAGE_CONFIG.yaml` |
| 分母未改 | `gate_registry.py check` ⇒ `check_n=168` |
| RED 测试仍然红 | `cargo test -p sqlrustgo-storage --test tf_t4_stub_silent_success --test tf_t4_ai_candidate_v1` ⇒ 0 passed / 4 failed |

---

## 3. 方法 2 —— fx18 的「红可归因」四步

fx18 是本 PR 的核心。它要证明的是**门禁能看见「一个 crate 的覆盖率表低于阈值」**，
而且这个红**必须能归因给注入**。

### 3.1 两个臂

| 臂 | 树 | 期望 |
|---|---|---|
| **控制臂** | 完整树（未注入） | **rc=0（绿）** —— 这证明「门禁在完整树上不红」 |
| **变异臂** | 注入 8000 个无人调用的函数到 `crates/transaction/src/lib.rs` | **rc≠0（红）**，且红的原因是 `average X% < threshold Y%` |

### 3.2 变异为什么选 `sqlrustgo-transaction`

L1_8 的 8 个 crate 是**逐个**测的（每个 crate 一次 `cargo llvm-cov test -p <crate>`），
所以改一个 crate 只会作废**它自己**的 instrumented build。`sqlrustgo-transaction` 是
这 8 个里**唯一不被其它成员依赖**的（planner / optimizer 依赖 parser/storage/types，
executor 依赖 catalog/parser/planner/storage/types）⇒ 改它只重编一个 crate，
而不是整条 instrumented 链。这是它能进预算的原因。

### 3.3 四条判别力护栏（**原样保留**，编号 G1–G4）

| # | 护栏 | 代码位置 | 不成立时 |
|---|---|---|---|
| **G1** | 控制臂 rc≠0 ⇒ 红不可归因 | fx18 `:250-254` | `exit 4`（INCONCLUSIVE） |
| **G2** | 红的形态必须是 `average … < threshold`；若是 `measured N of 8 crates`（reduced sample）⇒ 是**另一种**红 | fx18 `:255-263` | `exit 4`（INCONCLUSIVE）/ `exit 3`（FIXTURE-FAIL） |
| **G3** | 变异 crate 自己的数字必须出现在报告里 | fx18 `:264-267` | `exit 3`（FIXTURE-FAIL） |
| **G4** | `INCOMPLETE` / `exit_code=124`（超时）**不是红** | fx18 `:205-210` | `exit 4`（INCONCLUSIVE） |

**G1 是本 PR 的关键**：修复前控制臂 rc=1（基线红）⇒ fx18 永远 INCONCLUSIVE。
修复后控制臂 rc=0 ⇒ 变异臂的红**可归因** ⇒ fx18 才可能 FIXTURE-PASS。

**G2 是 R1 的护栏**：它确保「red」不是「测量没做成」。修复后 storage 可测 ⇒
样本是 8/8 ⇒ 变异臂的失败**只可能**是 `average < threshold`。

### 3.4 阈值档的钉法

```bash
export SQLRUSTGO_COVERAGE_STAGE=ga
```

并在控制臂之后立刻断言：

```bash
if ! grep -q 'stage=ga' "$CTL_LOG"; then
    echo "$FX_ID: the control arm did not report 'stage=ga' -> INCONCLUSIVE ..."
    exit 4
fi
```

**断言的是「哪一档」，不是阈值数字。** 阈值数字（85）在 SSOT
（`docs/governance/STAGE_CONFIG.yaml` → `COVERAGE_MIN_GA`），**不在夹具里复制**
—— 复制会引入第二份真相。

### 3.5 台账行的翻转

`docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl` 的 fx18 行：

| 字段 | 前 | 后 |
|---|---|---|
| `kind` | `pending-fixture` | `mutation-verified` |
| `evidence_run` | 无 | `20261126-criterion8-negfix-001` |
| `evidence_log` | 无 | `audit/v4.1.0/runs/20261126-criterion8-negfix-001/logs/003-neg-fixtures.stdout` |
| `note` | 目标描述 | 目标描述 + **基线曾红的完整交代** + 排除只作用于测量 + 阈值档差异（引 COVM-1） |

**台账是生成物**（`audit/v4.1.0/gen_neg_ledger.py`）⇒ 本 PR 改的是**生成器**
（fx18 从 `PEND` 移到 `MUT8` + 新增 `EVIDENCE_RUN_OVERRIDE`），不是手改 JSONL。
`gen_neg_ledger.py --check` 逐字节比对 ⇒ 跑一次即证。

**为什么 fx18 要自己的 `evidence_run`**：它的性质是 2026-11-26 那轮泳道建立的，
不是 2026-11-24 那轮 —— 那一轮的树上覆盖率门禁**本来就是红的**，红不可归因。
证据必须指向**记录了那个红的日志**，指向共享的 run8 会是**假引用**。

---

## 4. 方法 3 —— 泳道的顺序、预热与身份

### 4.1 顺序规则怎么来的（实测）

夹具还原 scratch 树用 `git checkout -- .` ⇒ **被还原文件的 mtime 变了** ⇒ cargo 的
mtime 指纹过期 ⇒ 下一次消费 cargo 产物的夹具要**重编根包 + 11 个依赖者**，实测 **>90min**。

规则（写进脚本注释）：

> **凡消费 cargo 产物的夹具，必须排在任何改 Rust 源码的夹具之前。**

新顺序 `fx02, fx20, fx19, fx15, fx18, fx16, fx17`：

| 位置 | 夹具 | 消费 cargo 产物？ | 改 Rust 源码？ |
|---|---|---|---|
| 1 | fx02（anti-ignore registry） | 否 | 否 |
| 2 | fx20（TPCH baseline hash） | 否（需数据集 + 已有产物） | 否 |
| 3 | fx19（SQL corpus） | **是** | 否 |
| 4 | fx15（alpha composite） | **是** | 否 |
| 5 | fx18（coverage） | **是** | **是**（注入到 `crates/transaction/src/lib.rs`） |
| 6 | fx16（arch invariants） | 否 | **是**（摘掉实现对象） |
| 7 | fx17（arch3 no-bypass） | 否 | **是**（摘掉 marker） |

**fx18 排第 5**：它是唯一一个**既消费 cargo 产物又改 Rust 源码**的夹具，但它改的是
`crates/transaction/src/lib.rs` —— 而 `sqlrustgo-transaction` **不被其它 7 个 crate 依赖**
⇒ 它的重编只波及自己。**fx16/fx17 改的是 `src/execution_engine*.rs`（根包）** ⇒ 必须排最后。

### 4.2 预热的判定与失败语义

```bash
(
    cd "$TREE" &&
    cargo build --all-features &&
    cargo test --all-features --lib --no-run
) >"$WARMUP_LOG" 2>&1
WARMUP_RC=$?
if [ "$WARMUP_RC" -ne 0 ]; then
    echo "FATAL: the target warmup FAILED (rc=$WARMUP_RC) -> the lane cannot run" >&2
    tail -20 "$WARMUP_LOG" >&2
    exit 4
fi
```

**为什么失败必须 `exit 4` 而不是继续**：冷 target 或坏 target ⇒ 每个消费 cargo 产物的
夹具都会**超时** ⇒ 而超时是 INCONCLUSIVE，不是红 ⇒ 泳道会「跑完」并如实报 INCONCLUSIVE，
但那**掩盖**了「环境坏了」这个真正的失败原因。`exit 4`（setup 失败）把它显式化。

**为什么没有 env 开关**（R6）：有开关就等于有「跳过泳道」的开关，而被跳过的泳道必须
**不可能**与跑过的泳道长得一样 —— 所以**无条件**执行，**独立**日志，**失败即 abort**。

**`--lib --no-run` 而不是 `--tests`**：只编译 lib 目标的测试二进制，不**跑**它们 ——
预热的目的是让编译产物就位，不是再跑一遍测试（泳道里的夹具会自己跑它们）。

### 4.3 树身份的自报与读取

**自报（`verify_neg_fixtures.sh`）**：

```bash
TREE_SHA="$(git -C "$TREE" rev-parse HEAD 2>/dev/null || echo unknown)"
DIRTY_FILES="$(git -C "$TREE" status --porcelain 2>/dev/null | wc -l | tr -d ' ')"
echo "verify_neg_fixtures: tree_sha=$TREE_SHA dirty_files=$DIRTY_FILES"
```

**为什么 `dirty_files` 必带**：当 `TREE != REPO` 时，脚本的脏树检查**被跳过**
（那是给用户自己的树用的保护）⇒ 一棵**被改过的** scratch 树与一棵**干净的**树
**共用同一个 commit SHA** ⇒ 只有 SHA 不足以标识被测树。

**读取（`gen_verdict.py`）**：

```python
LANE_ID_RE = re.compile(
    r"^\s*verify_neg_fixtures:\s*tree_sha=([0-9a-f]{40})\s+dirty_files=(\d+)\s*$", re.M)

def read_lane_identity(text):
    m = LANE_ID_RE.search(text or "")
    if not m:
        return {"line": "", "tree_sha": "", "dirty_files": None}
    return {"line": m.group(0).strip(), "tree_sha": m.group(1),
            "dirty_files": int(m.group(2))}
```

**三种定性**（判据 8 的 `note`）：

| 条件 | 定性 | 措辞 |
|---|---|---|
| 有 `tree_sha=` 且 `dirty_files=0` | **观测值** | 「该树身份是观测值，不是推定值」+ 逐字引用该行 + 「`dirty_files=0` ⇒ 被测树在该 commit 上没有未提交改动 ⇒ 这个 SHA **唯一确定**了被测树」 |
| 有 `tree_sha=` 且 `dirty_files>0` | **部分观测值** | 「该树身份是部分观测值」+ 「这个 SHA **不唯一确定**被测树（改动的内容没有被记录）」 |
| 无 `tree_sha=` 行 | **推定值** | 退回 `--neg-run-tree` + 「**推定/重构值**（不是从日志里读出来的）：该日志早于泳道自报树身份的版本」 |

**`observed` 里逐字引用该行**（不是复述）⇒ 读者可以 `grep` 回日志核对。

---

## 5. 方法 4 —— 声明与看板的验证

### 5.1 声明的红线检查

```bash
grep -n "已放行\|准入通过\|C-ALPHA-02 已启动\|已启动" \
  docs/releases/v4.1.0/V410_BASELINE_FROZEN_DECLARATION.md
# ⇒ 无输出
```

**为什么要 grep 而不是靠人读**：这是一个**可以用机器判定**的红线。人读会漏。

### 5.2 声明的事实核对（逐条）

| 声明里的结论 | 核对方式 |
|---|---|
| 判据 #1/#2/#3/#5/#9/#10/#11 PASS | `V410-READINESS-VERDICT.json` 的 `criteria[0]/[1]/[2]/[4]/[8]/[9]/[10]` 的 `verdict` 字段 |
| BASE 四元组 | `BASELINE_FREEZE.json` 的 `base_sha`/`tree_sha`/`source_ref`/`parent` |
| 合入路径 PR #5928–#5932 | `git log --merges origin/develop/v4.1.0` 的实测输出 |
| 3 条 FAIL 的分层归因 | `V410-OWNER-RULING.md` §1.1（**照抄**，不改判） |
| 判据 12 的 `approved_by` 为空 | `V410-OWNER-RULING.md` §2.1 的表 |
| deepseek-web HTTP 401 | 本会话探活实测（`probe` 返回 `error: "models: HTTPError: HTTP Error 401: Unauthorized"`、`available: false`） |
| chatGPT-web 可用 | 本会话探活实测（`available: true`，2 个模型） |

### 5.3 看板的验证

| 检查 | 命令 | 期望 |
|---|---|---|
| MD 是 JSONL 的生成物 | `kanban_render.py check` | rc=0，234 行 |
| 渲染器自测 | `test_kanban_render.py` | 9/9 PASS |
| 无单侧合并标记 | `kanban_merge_policy.py <jsonl>` | `verdict: OK` |
| id 唯一 | 逐行取 `id` 去重 | 234 == 234 |
| 既有 230 行逐字节不变 | `git diff` 的 JSONL diff 只含 4 个新增行 | 追加，无修改 |
| 手写段没被生成器覆盖 | `git diff` 的 MD diff 里生成段无变化 | 只改 §3/§4/§6 |

---

## 6. 方法 5 —— 台账与编目的自洽

| 检查 | 命令 | 期望 | 失败意味着 |
|---|---|---|---|
| C2：声称能抓 ⇒ 台账行可证伪 | `honesty.py check` | `fails=0` | fx18 的 `kind` 不是 `mutation-verified` |
| C3：每个声称的性质**逐字**落在夹具证据上 | `honesty.py check` | `fails=0` | 属性 id 不是 fixture_id/note/fixture 的字面子串 |
| C5：夹具没白做 | `honesty.py check` | `fails=0` | `mutation-verified` 但 `effect.catches` 为空 |
| 分母 | `gate_registry.py check` | `check_n=168` | 漂了 |
| 台账行有证据 run | `probe_neg_fixtures.py check` | rc=0 | `evidence_run` 目录不存在 |
| 台账是生成物 | `gen_neg_ledger.py --check` | 逐字节一致 | 手改了 JSONL |

**C3 的字面子串规则**：属性 id `crate-coverage-below-threshold` 必须**逐字**出现在 fx18 的
`fixture_id`（`fx18-coverage-v312-crate-coverage-below-threshold`）或 `note` 或 `fixture` 里。
`honesty.py` 的 `norm()` 把非字母数字折成 `-` 后做子串匹配。

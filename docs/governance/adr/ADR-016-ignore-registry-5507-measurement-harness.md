# ADR-016: #5507 测量 harness 的 `#[ignore]` 补登记（135 → 136）

> **Status**: PROPOSED —— **待属主确认**（实施者只登记事实与建议，不代签）
> **Deciders**: 待属主（liying）确认。实施者：dsh-b 线（Task 9）
> **Date**: 2026-11-26
> **注**：本 ADR 覆盖**两处**计数基线。第 1 处（`ignore_registry.json` 补登记 +
> `total_allowed` 135 → 136）**已由 PR #5941（`3a53c01a52`）合入**；本 PR 补上
> **第 2 处**（`tests/baseline/test_count.json` 的 `ignored_tests` 135 → 136）——
> 那是 **P13** 读的那份**另一份**基线，P12 不看它。
> **Branch scope**: `develop/v4.1.0`
> **Related**: [ADR-001 真实性框架](ADR-001-truthfulness-framework.md)（G-09）、
> [ADR-008 Test Claim 透明度](ADR-008-test-claim-transparency.md)（P12/P16）、
> [ADR-014 多 AI 协同](ADR-014-multi-ai-coordination.md)（D-05）、
> [ADR-015 假覆盖契约显式延后](ADR-015-fake-coverage-contracts-deferred.md)（123 → 135 的**先例**）、
> [看板行 COVM-1](../releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md)

---

## Context

### 事实

树上有 **136** 个真实 `#[ignore]` 属性（按 `scripts/gate/common.sh` 的
`count_real_ignore_attributes` 计数规则：只算真属性，不算注释里的散文），
而 `tests/baseline/ignore_registry.json` 只登记 **135** 条 ⇒ **P12 与 P13 同时红**。

多出来的是**唯一一条**：

| 字段 | 值 |
|---|---|
| 文件 | `crates/storage/tests/measure_row_at_a_time_5507.rs` |
| 测试 | `measure_row_at_a_time_commit_cost_5507` |
| 行 | 59 |
| `#[ignore]` 的理由文本 | `measurement, not a gate — run with --ignored --nocapture` |
| 引入提交 | `9abf5e5142335723744c0face28184cdfbb16ef9`（`test(storage): row-at-a-time commit cost measurement harness (#5507)`，**2026-11-12**） |

### 时间线：为什么它是「登记遗漏」而不是「新增欠账」

| 时点 | 事件 |
|---|---|
| 2026-10-23 | `ignore_registry.json` 的采集时点（`captured_at_commit` = `1ce91dd9e8`） |
| 2026-11-06 | ADR-015 把 12 条假覆盖契约测试显式标为 `#[ignore]` 并登记 ⇒ `ignored` 123 → **135** |
| 2026-11-07 | `tests/baseline/test_count.json` 的采集时点 ⇒ 记下 `ignored_tests: 135` |
| **2026-11-12** | **#5507 加入测量 harness**，带一个**有理由文本**的 `#[ignore]` ⇒ 树上的计数变成 **136**，而两个台账都还停在 135 |

⇒ 该 `#[ignore]` **晚于**两个台账的采集时点，**从未被登记**。
它是**一次普通的 PR 引入的合法测量工具**，不是一笔新增的隐性欠债
（隐性欠债是 ADR-015 那种：既不占额度也不登记 —— 那已被 ADR-015 显式化并计入 `active` 额度）。

### 为什么这个 harness 该保持 `#[ignore]`

文件头自己写明（`crates/storage/tests/measure_row_at_a_time_5507.rs:1-9`）：

> This is a **measurement**, not a gate. It is `#[ignore]`d on purpose: a
> wall-clock assertion inside the normal suite is exactly the pattern that
> made `wal_legacy::tests::test_wal_perf_1000_insert`
> (`crates/storage/src/wal_legacy.rs:1473`) fail intermittently under CPU
> contention.

即：它断言**墙钟耗时**。这与 `COVERAGE_PERF_BOUND_TEST_FILES` 排除掉的那批
（`write_path_scaling_5181.rs` 等）是**同一类**问题 ——
在插桩或 CPU 争用下会间歇失败。它**不是门禁自己的测试被 ignore**，
所以与 ADR-008 关心的那件事**不是一回事**。

### 这条红为什么此前一直没人管

因为它**不是本轮引入的**。实测 P12/P13 在以下每一个 tip 上都是红的：

| tip | P12 | P13 |
|---|---|---|
| `a604ed9ab3`（本轮开始前的 develop tip） | **1（红）** | **1（红）** |
| `fb540ed4c2`（PR #5935 合并后） | **1（红）** | **1（红）** |
| `9c031dc839`（本轮合并后） | **1（红）** | **1（红）** |

⇒ 自 **2026-11-12** 起就红着，且因此 **ADR-014 D-05「Hard Gate FAIL 时禁止 push 到
252」** 一直在拦着**任何** PR 的推送 —— 包括本轮这几条。

### 远端评审的发现

远端 Claude Code CLI 的《基线对齐报告》（`docs/releases/v4.1.0/evidence/
BASELINE_ALIGNMENT_2026-11-25.md`）在 §1.5 把它列为 ALPHA 清单里**唯一**的真实树级红，
并在裁决项 O-D 里要求属主授权后处置。

---

## Decision

**把这条 `#[ignore]` 补登记进台账，并把两处计数基线从 135 调到 136。**

### 1. `tests/baseline/ignore_registry.json` —— ✅ **已由 PR #5941 合入**

| 字段 | 前 | 后 |
|---|---|---|
| `ignored_tests` | 135 条 | **136 条**（新增 1 条） |
| `total_allowed` | 135 | **136** |
| `total_allowed_max`（上限） | 150 | **150（未动）** |
| `active_max`（上限） | 92 | **92（未动）** |
| `active`（实测 ACTIVE 条数） | 81 | **81（未动）** |
| `limits.note` | — | 追加本次 135 → 136 的理由 |

**新增条目的状态 = `PARKED`，不是 `ACTIVE`**；`category` 取 `EXTERNAL_PREREQ`
（与同类测量夹具 `measure_recovery_scale_p0_08.rs` 一致）。依据台账自带的 `status_legend`：

- `ACTIVE`：「还没有 —— 要么没有理由文本，要么理由指向未实现的行为或已知 open bug。**这是上限要兜住的那笔债。**」
- `PARKED`：「理由是**外部前提**（live server、`--release`、大内存、平台权限、**专用脚本**）。**在正常开发中不可操作。**」

这个 harness 要显式 `--ignored --nocapture` 才能跑，是**专用脚本**、在正常开发中不可操作 ⇒ `PARKED`。
因此它**不占 `active` 额度**（仍为 81，离 92 还有余量），
而 `active_max` / `total_allowed_max` **两个上限一个都没动**。

### 2. `tests/baseline/test_count.json` —— ✅ **已由 PR #5943 合入**

**P13 读的是**这**另一份**基线**（不是 `ignore_registry.json`）**：

| 字段 | 前（2026-11-07 采集） | 后（已入库） | 谁抬的 |
|---|---|---|---|
| `ignored_tests` | 135 | **136** | **本次（这就是 P13 的红）** |
| `cargo_tests` | 483 | 497 | **门禁自动抬**（树长了这 14 个） |
| `active_tests` | 14345 | 14505 | **门禁自动抬**（树长了这 160 个） |
| `timestamp` | 2026-11-07T00:30:00Z | 本次快照 | 门禁自动写 |
| `note` | 引用 ADR-015 | 门禁自动写的通用串 | 门禁自动写 |

**`cargo_tests` / `active_tests` 为什么也动了 —— 这是门禁自己的「允许的演进」逻辑，
不是本次放宽约束。** P13 的写入条件是：

```bash
if [ "$cargo_tests" -ge "$baseline_cargo" ] && \
   [ "$active_tests" -ge "$baseline_active" ] && \
   { [ "$ignored_tests" -le "$baseline_ignored" ] || [ "$METHOD_CHANGED" = true ]; }; then
    cat > "$BASELINE" <<EOF ... EOF     # ← 门禁自己重写整份文件
```

即：**三项计数都只增不减时，门禁把基线自动抬到当前值**。树的 `cargo` 487→497、
`active` 14345→14505 是这期间真实长出来的用例，门禁随之抬地板 ——
这是它的**设计行为**（仓内先例：`6551c12cc7`「测试计数基线随本批新增的两条生产链臂自抬
（门禁自动写入，入库以免每次 run 再脏一次工作区）」）。

**真正需要论证的只有 `ignored` 135 → 136 这一维** —— 因为**只有它**是「不许无 ADR 地增长」
的那一维，也是本次唯一红的那一维。另两维本来就 PASS，门禁抬它们是**跟随树的真实增长**，
不是放松保护：抬完之后地板仍然等于今天的树，往下删测试照样会红。

> ⚠️ 副作用：门禁每次跑都会重写这份文件（连 `timestamp` 都变），所以工作区会脏。
> 这是门禁的既有行为，本 PR 不改它 —— 改它属于「改门禁判定逻辑」，需另行授权。

**两份基线的分工**（两条门禁读的不是同一份）：

| 文件 | 被谁读 | `ignored_tests` 的语义 |
|---|---|---|
| `tests/baseline/ignore_registry.json` | P12 `check_ignore_count.sh`、G19 `check_anti_ignore_gate.sh` | **额度**（`total_allowed`），外层上限 `total_allowed_max` = 150 |
| `tests/baseline/test_count.json` | P13 `check_test_count_monotonic.sh` | **单调基线**（`ignored` 不得无 ADR 地增长） |

⇒ **只改前者，P13 仍然红** —— 这正是 PR #5943 存在的原因。

**先例**：ADR-015 做过同型动作（`ignored` 123 → 135）。

### 3. 不做的事

- **不**改 `total_allowed_max`（150）或 `active_max`（92）—— ADR-008 禁止为了过绿而放宽上限；
  本次是**把计数改回它本该是的样子**，不是放宽约束。
- **不**手工改 `cargo_tests` / `active_tests` 两维 —— 它们由门禁按树的真实增长自动抬
  （见上）。本 PR 不去干预门禁的写入逻辑。
- **不**给那两条测试加新的 `#[ignore]`，**不**改任何测试源码。
- **不**改任何门禁脚本的判定逻辑。

---

## Consequences

### 正面

- **P12 转绿**（PR #5941 已达成）：`check_ignore_count.sh` ⇒ `✅ PASS — P12 satisfied`（实测）。
- **G19 转绿**（PR #5941 已达成）：`check_anti_ignore_gate.sh` ⇒
  `total_allowed=136 (max=150), active=81 (max=92)`，tree recount 136 ⇒ 一致，rc=0（实测）。
- **P13 转绿**（本 PR 达成）：`check_test_count_monotonic.sh` ⇒ `✅ PASS`。
- **台账恢复它自己声明的契约**：`ignore_registry.json` 的 `purpose` 写的是
  「SSOT for **every** `#[ignore]` attribute in the live test tree」——
  补上这一条，SSOT 才真的覆盖了树。
- **ADR-014 D-05 的阻塞解除**：P11–P16 六个 Hard Gate 全绿 ⇒ 又能往 252 推 PR 了
  （自 2026-11-12 起，这条红一直在拦着**每一个** PR 的推送）。

### 负面 / 保留意见

- **`note` 字段不是可靠的溯源载体** —— P13 门禁每次运行都把它重写成通用串
  （"Updated by check_test_count_monotonic.sh"）。**本 ADR 才是这条调整的耐久记录**。
- 本 ADR 的 **Status 是 PROPOSED**，**实施者没有代签**。
  如果属主认为「一个普通 PR 引入的测量 harness 不需要单开 ADR」，
  那就把本文件降级成一份工作记录（`docs/governance/DOC_CHECK_CORRECTION_WORK_RECORD.md`），
  台账与基线的改动仍然成立 —— 因为它们**只是把计数改回与树一致**，不放宽任何上限。
- `ignored_tests` 计数**又涨了 1**。这不是新增欠债（它早就在树上了，只是一直没登记），
  但**每一笔登记都会让这个数字更接近 `total_allowed_max` 150**：
  还剩 14 个槽位。这个余量本身应当在适当时机由属主重估。

---

## Verification

```bash
# P12（Hard Gate）—— PR #5941 已达成
bash scripts/gate/check_ignore_count.sh        # ⇒ ✅ PASS — P12 satisfied
# G19 —— PR #5941 已达成
bash scripts/gate/check_anti_ignore_gate.sh    # ⇒ rc=0（136 == 136）
# P13（Hard Gate）—— 本 PR 达成
bash scripts/gate/check_test_count_monotonic.sh  # ⇒ ✅ PASS
# 定位复算（唯一多出来的那一条）
python3 - <<'EOF'
import re, json, pathlib
ATTR = re.compile(r'#\[ignore(\s*=\s*"((?:[^"\\]|\\[\s\S])*)")?\]')
BETWEEN = re.compile(r'(?:\s|//[^\n]*|/\*.*?\*/|\#[^\n]*)*?fn\s+([A-Za-z0-9_]+)')
found = []
for p in sorted(list(pathlib.Path("tests").rglob("*.rs")) + list(pathlib.Path("crates").rglob("*.rs"))):
    src = p.read_text(errors="replace")
    for m in ATTR.finditer(src):
        ls = src.rfind("\n", 0, m.start()) + 1
        if src[ls:m.start()].strip().startswith("//"): continue
        mm = BETWEEN.match(src[m.end():m.end() + 400])
        if mm: found.append((str(p), mm.group(1)))
reg = {(e["file"], e["test"]) for e in json.load(open("tests/baseline/ignore_registry.json"))["ignored_tests"]}
print("tree-only:", sorted(set(found) - reg))   # ⇒ []
EOF
```

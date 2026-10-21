# 门禁维度诚实性 —— 剩下三个「与代码无关的结论」

**迭代**：迭代 5（v4.1.0）
**批次**：批次 5 —— 门禁层剩余缺陷
**分支**：`fix/ci-deadlock-batch4`（与批次 4 同一分支，接续其 commit）
**基线**：`28848131fa`（批次 4 合并提交 `9970a6c782e8`）
**日期**：2026-10-21
**关闭的 issue**：#5421、#5422、#5425

---

## 0. 一句话

批次 4 修的是**判定结果怎么变成 CI 结论**那根线；
这一批修的是**判定本身与被判定对象无关**的三处残留 ——
一个要求它自己不产出的生成物、一个只增不初始化的计数器、一个三处互相打架的阈值。

---

## 1. #5421 —— D2-B6 要求门禁自己不产出的生成物

### 症状

`scripts/gate/check_rc_ga_gate.sh` 的 D2 维度 B6：

```
  [B6] V312-24 SQLancer + test-runner artifacts ...     ✗ FAIL B6 V312-24: report artifacts missing
                                                    (run: cargo run -p sqlancer -- --duration 30 + cargo run -p test-runner)
```

它要求 `target/sqlancer-report.json` 与 `target/test-runner-report.json`，
而这两个文件：

- **不是 git 跟踪文件**（`target/` 是构建目录）；
- **门禁自己不产出**（失败信息让人**手工**去跑生成器）。

⇒ 在任何**从干净检出开始**执行该门禁的环境（CI runner、新的 worktree、任何人的 clone）
上，B6 必红，而红的原因与代码无关。
⇒ 它**只能**靠「预先有人手工生成过产物」才变绿 —— 等价于随机绿灯。

### 修法

让门禁**自己产出它要检查的东西**：

```bash
if [ ! -s "${REPO_ROOT}/target/sqlancer-report.json" ] || \
   [ ! -s "${REPO_ROOT}/target/test-runner-report.json" ]; then
    echo ""
    echo "       artifacts absent — the gate generates them"
    mkdir -p "${REPO_ROOT}/target"
    SQLANCER_RC=0
    TEST_RUNNER_RC=0
    ( cd "${REPO_ROOT}" && cargo run -q -p sqlancer -- \
          --duration "${SQLANCER_DURATION:-5}" \
          --out target/sqlancer-report.json ) >/dev/null 2>&1 || SQLANCER_RC=$?
    ( cd "${REPO_ROOT}" && cargo run -q -p test-runner -- \
          --out target/test-runner-report.json ) >/dev/null 2>&1 || TEST_RUNNER_RC=$?
    echo "       generation rc: sqlancer=$SQLANCER_RC test-runner=$TEST_RUNNER_RC"
fi
```

两个设计点：

1. **退出码被捕获并打印，绝不吞没。** 第一版写成 `|| true`，
   被 P14 门禁（`check_drift_not_pass.sh`）当场判为**新增反模式**并转红 ——
   那正是这个门禁存在的意义。改成 `|| SQLANCER_RC=$?` 后，
   退出码仍然可见、可诊断，且不再匹配 P14 的模式。
2. **`--duration` 默认 5 秒。** 这条检查验证的是产物的**接线与 schema**，
   不是模糊测试的深度；实测 1000 次迭代在 ~0.01 s 内完成。

### 实测（真实 cargo，真实仓库根，产物此前不存在）

```
$ ls target/sqlancer-report.json target/test-runner-report.json
ls: target/sqlancer-report.json: No such file or directory
ls: target/test-runner-report.json: No such file or directory

  [B6] V312-24 SQLancer + test-runner artifacts ...
       artifacts absent — the gate generates them
       generation rc: sqlancer=0 test-runner=0
  ✓ PASS: B6 V312-24: SQLancer + test-runner artifacts valid
COUNTS PASS=1 TOTAL=1 BLOCKERS=0
exit = 0

$ ls -la target/sqlancer-report.json target/test-runner-report.json
-rw-r--r--  155 target/sqlancer-report.json
-rw-r--r--  810 target/test-runner-report.json
```

⇒ **修复前必红的位置，现在自己把产物造出来并判 PASS。**

---

## 2. #5422 —— 计数器只增不初始化

### 症状（修复前实测）

```
$ bash scripts/gate/check_cross_version_debt.sh
  ✅ CLOSED:    58
  ⚠️  PARTIAL:   10
  ❌ OPEN:      4
  ⏸️  DEFERRED:            ← 空
  🔄 ACTIVE:    0
  ❓ UNKNOWN:             ← 空
=== Gate Criteria Check ===
scripts/gate/check_cross_version_debt.sh: line 450: [: : integer expression expected
⚠️  WARNING: 4 OPEN debt items found
...
scripts/gate/check_cross_version_debt.sh: line 487: [: : integer expression expected
✅ PASS
```

`DEFERRED_COUNT` / `PARTIAL_COUNT` / `OPEN_COUNT` / `UNKNOWN_COUNT` 四个计数器
**只自增、不初始化**。

### 归因更正（本 issue 原文的说法不准确）

issue 原文说「`❌ FAIL (UNKNOWN items)` 与 STRICT 的 OPEN 判定**永久不可达**」。
**实测证明这不是真的**：

- bash 的算术展开把**未定义**的变量当作 0，所以
  `UNKNOWN_COUNT=$((UNKNOWN_COUNT + 1))` 从未定义起步会得到 **1**；
- 证据就在输出里：`OPEN_COUNT` 同样从未初始化，却打印出 **4**
  （4 次自增：0→1→2→3→4）⇒ **自增本身是正确的**。

⇒ 真实缺陷是：

1. 计数为 0 时（从未自增过），`[ "$X" -gt 0 ]` 就是 `[ "" -gt 0 ]`
   ⇒ 打印 `integer expression expected` 并返回 2 ⇒ 在 `if` 里按「假」处理
   ⇒ **每次运行都留下两行 shell 报错噪声**；
2. `DEFERRED` 计数打印为**空**（不是 0），使摘要不可读；
3. 一旦有人给该脚本加上 `set -u`，这些计数器会直接让脚本**崩掉**。

⇒ 修法：在 `ACTIVE_COUNT=0` / `CLOSED_COUNT=0` 旁边把四个计数器一并初始化。

⇒ 判定**行为不变**（UNKNOWN 判定在有 UNKNOWN 条目时依然会触发），
变的是**噪声与可读性** —— 这一点在 issue 的关闭说明里明确标注。

### 实测（修复后）

```
=== Cross-Version Debt Summary ===
  ✅ CLOSED:    58
  ⚠️  PARTIAL:   10
  ❌ OPEN:      4
  ⏸️  DEFERRED:  0
  🔄 ACTIVE:    0
  ❓ UNKNOWN:   0
=== Cross-Version Debt Gate Check Complete ===
✅ PASS
```

⇒ 两行 `integer expression expected` 消失，六个计数全部打印为数字。

---

## 3. #5425 —— anti-ignore 门禁的阈值三处不一致

### 症状

| 出处 | active 上限 | total_allowed 上限 |
|---|---|---|
| `scripts/gate/check_anti_ignore_gate.sh` 的文件头注释 | **55** | **100** |
| 同一脚本的代码变量 | **80** | **150** |
| `docs/governance/GATE_CONDITIONS.md` 的 G19 表格 | **47** | **96** |

实测当时的状态：`active=69`、`total_allowed=116`
⇒ 按代码（80/150）**PASS**；按注释（55/100）**FAIL**；按治理文档（47/96）**FAIL**。

⇒ 「G19 通过了吗」取决于你读哪一份阈值。

### 归因更正（另一条不准确的说法）

本 issue 引用 #5445 的结论说「`ACTIVE_MAX=47` 因**注册表 schema 没有 status 字段**
而结构上恒为 0」。**实测证明这已过时**：

```
$ python3 -c "import json;d=json.load(open('tests/baseline/ignore_registry.json'));…"
顶层键: ['schema_version', 'purpose', …, 'ignored_tests', …]
entries = 116
首条键: ['file', 'test', 'line_at_capture', 'reason', 'status', 'category', 'status_rationale']
status 分布: Counter({'ACTIVE': 69, 'PARKED': 47})
```

⇒ registry **有** `status` 字段，ACTIVE 计数是真实的 69。
⇒ #5445 引用的那份 `anti_ignore_gate_hollow.txt` 证据也已过时：
现在的脚本**确实**扫源码树（`count_tree()` 重数 `tests/` + `crates/` 里的真实
`#[ignore]` 属性，并与登记册比对，登记册漂移即 FAIL）——
它**不是**空壳。#5445 已在本迭代批次 4 关闭，其「空壳」结论需按本条更正。

### 修法：阈值只有一个来源

`tests/baseline/ignore_registry.json` **自带** `limits` 字段：

```json
"limits": { "total_allowed_max": 150, "active_max": 80,
            "note": "Increased at v4.1.0-alpha2 promotion to accommodate fix PR drift" }
```

⇒ 脚本改为**从登记册读阈值**（缺失时才回落到 80/150，且回落值与登记册当前值相同，
不会悄悄放松门禁）；文件头注释改为指向登记册；
`GATE_CONDITIONS.md` 的 G19 表格改为 80/150 并注明阈值来源。

⇒ **判定行为不变**（当前 69/116 在 80/150 之下，本就 PASS），
变的是**「通过的理由从三份互相矛盾的文档收敛成一份」**。

### 实测

```
$ bash scripts/gate/check_anti_ignore_gate.sh
ignore_registry.json: total_allowed=116 (max=150), active=69 (max=80)
tree recount: real #[ignore] attributes in tests/ + crates/ = 116
exit=0
```

---

## 4. 测试

新增 `tests/integration/gate/gate_dimension_honesty_test.rs`（**8 例**），
注册于根包 `Cargo.toml`。

判据以**不变量**形式表达：

| 用例 | 判据 |
|---|---|
| `test_5422_every_incremented_counter_is_initialized_in_the_same_script` | **任何** `X=$((X + 1))` 的计数器，`X` 都必须在同文件内被初始化（且只认「加一」计数器，不误伤累加器） |
| `test_5422_a_zero_counter_produces_no_shell_error` | 真跑债务门禁，输出里不得有 `integer expression expected`，且六个计数必须都打印为数字 |
| `test_5422_the_increment_counts_correctly_from_an_unset_variable` | 把「未定义变量按 0 自增」这一 bash 语义固化成可执行证据（即归因更正的依据） |
| `test_5421_d2_b6_produces_the_artifacts_it_checks` | 用**桩 cargo** 真跑 B6 块：桩能产出合法产物 ⇒ 必须判 PASS 且 `PASS=1 / BLOCKERS=0` |
| `test_5421_d2_b6_fails_when_the_artifacts_cannot_be_produced` | 桩什么都不产出 ⇒ 必须判 FAIL 且 `BLOCKERS=1`（**判不了 ≠ 放行**） |
| `test_5421_the_generated_artifact_paths_match_the_checked_ones` | 生成命令写出的路径必须与被检查的路径一致 |
| `test_5425_the_governance_doc_states_the_thresholds_the_gate_actually_enforces` | 治理文档 G19 写的阈值必须与门禁**实际生效**的上限相等（跨产物一致性） |
| `test_5425_the_script_header_does_not_contradict_its_own_thresholds` | 脚本文件头注释里的阈值声明必须与代码一致 |

### 4.1 RED → GREEN

| 阶段 | 结果 |
|---|---|
| RED（4 个实现文件 `git stash` 撤出） | **3 passed / 5 failed** |
| GREEN（实现全部就位） | **8 passed / 0 failed** |

那 3 条在 RED 下也通过的是**自检有效性**与**归因事实**用例
（`test_5422_the_increment_counts_correctly_from_an_unset_variable`、
`test_5421_d2_b6_fails_when_the_artifacts_cannot_be_produced`、
`test_5421_the_generated_artifact_paths_match_the_checked_ones`）——
前一条是「未定义变量按 0 自增」这一 bash 语义的事实，后两条是**反向**判据
（判不了必须判红），本就应该双向通过。

### 4.2 回归

| 项 | 结果 |
|---|---|
| `gate_dimension_honesty_test`（本批次新增） | **8 / 8 绿** |
| `ci_wiring_honesty_test`（批次 4 的 24 例） | **24 / 24 绿** |
| `gate_honesty_test`（批次 2 的 15 例） | **15 / 15 绿** |
| 6 个元门禁（P11–P16） | 全部 exit 0 |
| `check_drift_not_pass`（P14） | exit 0 —— 第一版的 `\|\| true` 写法曾让它转红，已改为显式捕获退出码 |
| `check_cross_version_debt.sh` 真跑 | 无 shell 报错，六个计数齐全 |
| `check_anti_ignore_gate.sh` 真跑 | exit 0，阈值来自登记册 |

---

## 5. 诚实的边界

1. **本批次没有改变任何一道门禁的「通过/不通过」结论。**
   - #5421 改的是**在干净环境下能否得到一个诚实的结论**（从「必红」变成「自产自验」）；
   - #5422 改的是**噪声与可读性**（判定行为不变，见 §2 的归因更正）；
   - #5425 改的是**阈值来源的收敛**（数值不变，见 §3）。
2. **`#5425` 的三处阈值中，历史叙述段落仍保留着 47/55/96/100 等旧数字。**
   那些是**历史**（登记册创建时的数值与 bumps 记录），不是当前判据；
   脚本头与 G19 表格已指向登记册的 `limits`。
3. **`check_anti_ignore_gate.sh` 的 `count_tree()` 只重数 `tests/` 与 `crates/`。**
   `archive/` 下的 5 个历史标记按脚本自身的说明不在范围内。
4. **D2-B6 的生成步骤会调用 `cargo run`。** 在冷 runner 上首次构建
   `sqlancer` / `test-runner` 需要额外时间；失败时退出码会被打印出来，
   且 B6 判红（**判不了 ≠ 放行**）。
5. **#5445 关闭说明里引用的「空壳」结论需要按本文件 §3 更正。**
   那个结论已被仓库自身的演进（`count_tree()` 的加入）推翻。

---

## 6. LEARN

1. **「门禁要求它自己不产出的东西」是一种特殊的恒红。** 它不报假绿，
   反而报**恒红** —— 但同样与代码无关，同样「只能靠预先有人手工做过」才变绿。
   ⇒ 判据要问的是：**这条检查所依赖的输入，谁来生产？**
2. **「只增不初始化」的计数器在 bash 里是**半**坏的。**
   自增会把未定义当 0，所以**数值是对的**；
   坏的是零值时的 `[ "" -gt 0 ]` 报错与空字段打印。
   ⇒ 归因时必须实测数值，不能只看代码结构下结论。
3. **阈值应当是数据，不是代码里的第二份副本。**
   登记册本来就有 `limits` 字段，门禁却另抄了一份到变量里，
   文档又抄了两份 ⇒ 三份各自演化。⇒ 修法是**让副本消失**，不是把三份改成一样。
4. **P14 这道门禁抓到了我自己的新代码。** 第一版 B6 修复用了 `|| true`，
   P14 当场判为新增反模式。⇒ 门禁体系开始**互相监督**，
   这是它真的在工作的证据。

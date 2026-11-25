# R1 契约先行 —— v4.1.0 REF-01/02/03 拆分设计笔记

> **性质**：**设计与契约**，不改产品代码。本文件是 R2（机械拆分）的前置输入。
> **阶段归属**：R1（契约先行）—— 按 `docs/releases/v4.1.0/evidence/BASELINE_FOLLOWUP_1926_2026-11-25.md`
> 的 Phase 表，**R0/R1 不需要属主裁决**（只读盘点 + 设计笔记），**R2 机械拆分才需要**
> （O1/O5 + 判据 6）。
> **生成者**：dsh-b 线（Task 9），2026-11-26
> **基线树**：`8b744201bede70e99a6ac157161f8b9b745ae076`
> **上游材料**：
> - R0 盘点（远端只读分析 T-B，已入仓）：`docs/releases/v4.1.0/evidence/baseline-followup-20261125-001/TB_r0.stdout`、`TB_prod_vs_test.stdout`、`TB_overlap.stdout`
> - 计划：`docs/releases/v4.1.0/refactoring/gpt-r1/TEST-FACTORY-RECOVERY-PLAN.md`、`docs/releases/v4.1.0/PERFORMANCE_OPTIMIZATION_PLAN.md`
> - 硬边：`docs/releases/v4.1.0/STAGE.yaml` L879（读路径）、L911（#4947）
>
> **裁决来源声明**：本轮外部咨询通道（deepseek-web / chatGPT-web）**均不可得**
> （deepseek HTTP 401 Unauthorized；chatGPT-web 推理超时，连一行确认也超时）。
> 故本文件的裁决**由实施者单方作出，标注为单专家**，通道恢复后可复核。
> 所有裁决依据都是**实测**，不是偏好。

---

## 0. 三个文件的现状（实测，R0 复核）

| 文件 | 总行数 | `mod tests` 起始 | 产品代码 | 内联测试 | `pub` 类型 | `pub fn` |
|---|---|---|---|---|---|---|
| `crates/executor/src/expr/mod.rs` | 5270 | 3731 | **3730** | 1540（118 test） | 2 | 27 |
| `crates/executor/src/stored_proc.rs` | 4278 | 2670 | **2669** | 1609（119 test） | 4 | 43 |
| `crates/executor/src/trigger.rs` | 2628 | 1348 | **1347** | 1281（74 test） | 8 | 21 |

**C-ARCH-05 的口径**：`scripts/gate/check_arch_invariants.sh` 的 `CARCH05_LIMIT=1600`，
用 `wc -l` 量**整个文件（含内联测试）**。

**⇒ 关键结论（可执行）**：

| 文件 | 现总行数 | 超限? | 移出测试模块后 | 还需要真拆分? |
|---|---|---|---|---|
| `expr/mod.rs` | 5270 | **是**（>1600） | 3730 | **是** —— 仍 >1600 |
| `stored_proc.rs` | 4278 | **是**（>1600） | 2669 | **是** —— 仍 >1600 |
| `trigger.rs` | 2628 | **是**（>1600） | **1347** | **否** —— 已 <1600 ✅ |

---

## 1. REF-03（`trigger.rs`）—— 只需移出测试模块

### 1.1 裁决

**采用「只把 `mod tests` 移到 `crates/executor/src/trigger/tests.rs`」这条路径，不做产品代码拆分。**

**依据（不是「钻空子」）**：

1. **它满足的是门禁本来的意图**。C-ARCH-05 的目的是限制**产品代码的复杂度**（`execution_engine.rs`
   之所以超限，是因为它把逻辑和测试塞在一个文件里）。`trigger.rs` 的产品代码是 1347 行 ——
   它**本来就不超标**；超标的是 1281 行内联测试，而那 1281 行**不是产品代码**。
   把测试挪出去，是让测量口径与门禁意图一致，不是绕过测量。

2. **对比另两个文件可知这是同一个动作**：`expr/mod.rs` 与 `stored_proc.rs` 的内联测试分别是
   1540 / 1609 行 —— 它们**也要**先移出测试模块（这是任何真拆分的前置步骤，因为测试要跟着
   被拆的代码走）。三者的第一步是**同一个动作**；只有 REF-01/02 在此之后**还要**继续拆。

3. **可证伪**：移出后 `wc -l` 变成 1347 < 1600，且**一条断言都不改**、**一个 pub 项都不改**。
   如果它靠的是「把代码藏起来」，那 `pub` 面会变、测试覆盖会掉 —— 本笔记 §3 的验收判据
   逐条钉死这两件事。

4. **与 #4947 的硬边**：willy2 的 T-B 实测 #4947（`f4562023e`）**已被回退**
   （「revert trigger/stored_proc to scan()」），`STAGE.yaml:845` 记的「当前编译不过（Self: Sized）」
   已过时 ⇒ **REF-03 ⇐ #4947 这条硬边的前提需要属主重新确认**。若前提不成立，
   REF-03 就**不受任何硬边阻塞**，可以立即做。

### 1.2 目标布局

```
crates/executor/src/trigger/
  mod.rs        ← 现在的 trigger.rs 去掉 mod tests 块（1–1347 + 原 mod tests 的尾部 import）
  tests.rs      ← 原 mod tests { … } 的全部内容（含其 use 块）
```

`crates/executor/src/lib.rs` 的 `pub mod trigger;` **不变** ⇒ 外部路径
`sqlrustgo_executor::trigger::*` **完全不变**。

### 1.3 迁移顺序（每步一个 commit，可独立 `git revert`）

| 步 | 动作 | 提交粒度 | 可回滚 |
|---|---|---|---|
| R1-3.a | 建 `trigger/tests.rs`，把 `mod tests { … }` 的**全部内容**原样搬进去（含 `use`） | 1 commit | `git revert` |
| R1-3.b | `trigger.rs` 里把 `mod tests { … }` 整块删掉，替换为 `#[cfg(test)] mod tests;` | 与 .a **同一 commit**（否则中间态编译不过） | 同上 |
| R1-3.c | `trigger/tests.rs` 顶部补 `use super::*;`（若原 `mod tests` 块依赖父模块的私有项） | 同上 | 同上 |

> ⚠️ **中间态不可编译** ⇒ .a/.b/.c 必须**同一 commit**。这是本 PR 唯一的不可分片点。

### 1.4 公开 API 零变更清单（REF-03）

**必须一字不差地保持**（8 个类型 + 21 个方法）：

```
pub 类型: TriggerTiming, TriggerEvent, TriggerType, TriggerExecutor,
          TriggerBodyAuthCheck, TriggerUndoRecorder,
          MAX_RECURSION_DEPTH, TriggerExecutionResult
pub fn:  TriggerExecutor::{new, new, set_outer_tx_id, storage, set_current_user,
          current_user, set_auth_check, set_undo_recorder, recursion_depth_counter,
          get_table_triggers, get_triggers_for_operation, execute_before_insert,
          execute_after_insert, execute_before_update, execute_after_update,
          execute_before_delete, execute_after_delete, execute_trigger_body,
          execute_triggers, is_modified, into_record}
```

---

## 2. REF-01（`expr/mod.rs`）与 REF-02（`stored_proc.rs`）—— 需要真拆分

### 2.1 REF-01 的子模块划分（≥4，公开 API 零变更）

按**职责**切，不按行数切（行数是结果，不是目标）：

| 子模块 | 收编的 `pub fn` | 理由 |
|---|---|---|
| `expr/udf.rs` | `register_udf`, `register_udf_with_body`, `drop_udf`, `has_udf`, `udf_snapshot` | UDF 注册表是独立职责（一张全局表 + 5 个操作），不依赖求值 |
| `expr/compare.rs` | `compare_values`, `compare_values_pad_space`, `eval_between`, `eval_not_between`, `compare_cmp`(私有) | 值比较自成一体，是 `eval_binary_op` 的底座 |
| `expr/pattern.rs` | `sql_like_match`, `sql_like_match_esc`, `eval_regexp` | 模式匹配（LIKE / REGEXP）是一族独立算法 |
| `expr/cast.rs` | `cast_val`, `cast_value`(私有), `date_add_sub` | 类型转换与日期算术是纯函数族 |
| `expr/eval.rs` | `UnifiedExpr::evaluate`, `UnifiedExpr::referenced_columns`, `eval_identifier`, `eval_case_when`, `eval_aggregate_lookup`, `eval_fn`, `eval_binary_op`, `eval_unary_op`, `eval_arithmetic`(私有), `eval_literal_from_str`, `eval_is_null`, `eval_is_not_null`, `resolve_system_variable`, `find_column_index` | 剩下的求值主干 |
| `expr/mod.rs` | `UdfDefinition`, `UnifiedExpr` + `pub use` 再导出 | 门面：定义类型、**原样 re-export** 全部子模块的 pub 项 |

**关键手法**：`expr/mod.rs` 里用 `pub use` 把子模块的 pub 项**重新导出**到同一路径：

```rust
mod compare;  mod eval;  mod pattern;  mod udf;  mod cast;
pub use compare::*;  pub use eval::*;  pub use pattern::*;  pub use udf::*;  pub use cast::*;
```

⇒ `sqlrustgo_executor::expr::eval_binary_op` 等路径**完全不变** ⇒
**外部 7 个调用方文件一个都不用改**（`src/engine_select.rs`、`src/engine_utils.rs`、
`src/expr_utils.rs`、`crates/executor/tests/expr_direct_v3_12.rs` + 3 个 integration test）。

### 2.2 REF-02 的子模块划分

`stored_proc.rs` 的结构是 **`ProcedureContext`（91–407，317 行）+ `StoredProcExecutor`（413–2668，2256 行）**
两大部分，外部调用方 18 个文件。

| 子模块 | 收编内容 | 理由 |
|---|---|---|
| `stored_proc/error.rs` | `StoredProcError` + `Display` + `Error` impl | 错误类型独立 |
| `stored_proc/context.rs` | `ProcedureContext` + `ExceptionHandler` + `Cursor`(私有) | 执行上下文（变量/标签/游标/异常）是独立状态机 |
| `stored_proc/cursor.rs` | `Cursor`(私有) + 游标相关方法 | 可独立成模块，但**必须**与 context 共享状态 ⇒ 见 §2.3 |
| `stored_proc/executor.rs` | `StoredProcExecutor` + `execute_call` + `has_procedure` + `list_procedures` | 执行主体 |
| `stored_proc/mod.rs` | `pub use` 全部 + 门面 | 同 REF-01 的手法 |

### 2.3 类型归属规则（两条硬规则）

1. **跨模块可见的私有项一律升为 `pub(crate)`**（不是 `pub`）。`pub(crate)` 不改变外部 API，
   但让子模块能互相访问。
2. **只有「两个子模块都要用」的类型才升 `pub(crate)`**；只在一个子模块里用的**保持私有**
   （`fn` / `struct`），由该子模块自己拥有。

> ⚠️ 升 `pub(crate)` 会让 `cargo clippy` 的 `unreachable_pub` 之类 lint 有话说 —— 若触发，
> 用 `#[allow(...)]` 局部豁免并在提交信息里写明，**不要**改成 `pub`（那会污染外部 API）。

### 2.4 REF-01/02 的迁移顺序（每步一个 commit）

**共同的第一步**：先移出内联测试模块（与 REF-03 同型动作），因为后续每次搬代码都要搬对应测试。

| 步 | REF-01 | REF-02 | 每步的验收 |
|---|---|---|---|
| 0 | `expr/tests.rs`（把 `mod tests` 搬出去） | `stored_proc/tests.rs` | 编译过；`cargo test --all-features --lib` 与拆分前**同样绿** |
| 1 | 建 5 个子模块骨架（空文件 + `mod` 声明） | 建 4 个子模块骨架 | 编译过 |
| 2 | `expr/udf.rs`：搬 UDF 5 个 fn + `UdfDefinition` | `stored_proc/error.rs`：搬 `StoredProcError` | `pub` 面逐项比对**无变化** |
| 3 | `expr/compare.rs` | `stored_proc/context.rs`：搬 `ProcedureContext` + `ExceptionHandler` | 同上；**此时 `pub(crate)` 升级点在这里第一次出现** |
| 4 | `expr/pattern.rs` | `stored_proc/cursor.rs` | 同上 |
| 5 | `expr/cast.rs` | `stored_proc/executor.rs`：搬 `StoredProcExecutor` | 同上 |
| 6 | `expr/eval.rs`：搬求值主干（**最大的一块**） | — | 同上 |
| 7 | `expr/mod.rs` 加 `pub use` 再导出；删旧定义 | `stored_proc/mod.rs` 同上 | **`pub` 面比对必须完全一致** |

> ⚠️ **步 2–6 每一步都是「搬完仍编译、仍全绿」**。若某一步搬不完，**拆成两个 commit**，
> 不要留中间态。

---

## 3. 可证伪的验收判据（每条都是机器可判的）

### 3.1 公开 API 零变更（**最强的判据**）—— 已工具化

**工具**：`audit/v4.1.0/refactor_api_surface.py`（**不是门禁**，是 R2 开工前人工执行的工具；
按 AGENTS.md 的约定，新工具只放 `audit/v4.1.0/`，**不进 168 分母**、不改任何门禁判定逻辑）。

```bash
# 拆分前
python3 audit/v4.1.0/refactor_api_surface.py snapshot \
    --files crates/executor/src/expr/mod.rs \
            crates/executor/src/stored_proc.rs \
            crates/executor/src/trigger.rs \
    --out /tmp/pub-before.txt

# 拆分后（同样的 --files；门面文件可能变成了目录，工具两种形态都覆盖）
python3 audit/v4.1.0/refactor_api_surface.py snapshot \
    --files crates/executor/src/expr/mod.rs \
            crates/executor/src/stored_proc.rs \
            crates/executor/src/trigger.rs \
    --out /tmp/pub-after.txt

# 比对 ⇒ rc=0 才算通过
python3 audit/v4.1.0/refactor_api_surface.py diff \
    --before /tmp/pub-before.txt --after /tmp/pub-after.txt
```

**它抓什么**（逐项都有反向变异证明，见 §3.6）：

| # | 抓的东西 | 为什么算破坏 API |
|---|---|---|
| 1 | `pub` 项的**名字** | 外部调用方直接按名字用 |
| 2 | 每个 `pub` 项的**所属文件** | 子模块化后名字会散到多个文件里；归属变了说明 `pub use` 没做对 |
| 3 | `pub` 项的**签名行**（fn 的参数与返回类型） | 名字不变但签名变了**也是**破坏 API |
| 4 | `pub(crate)` 可见性的**变化** | `pub` → `pub(crate)` 是**收窄**，会打断外部调用方 |

**它不抓什么**：函数体内部逻辑（那是 §3.2 的行为判据）、`#[cfg(...)]` 条件编译分支内的项
（按源码文本抓，不做 cfg 求值）。

**当前基线**：三个模块共 **105 项**（`expr` 29 / `stored_proc` 47 / `trigger` 29）。
`/tmp/pub-before.txt` 可随时重建复算。

### 3.2 行为不变（特征测试锁行为，不锁实现）

```bash
# 拆分前后都必须绿，且测试**数量**不变
cargo test -p sqlrustgo-executor --all-features --lib 2>&1 | grep "test result"
```

**判据**：测试**通过数**与**失败数**在拆分前后完全一致（不是「变绿了」，是「一模一样」）。

### 3.3 门禁口径不变

```bash
bash scripts/gate/check_arch_invariants.sh   # rc=0，且 NOTE 段无 WARN
```

**判据**：C-ARCH-05 与 NOTE 段的判定与拆分前一致（**不得**靠改门禁口径过关）。

### 3.4 规模达标

```bash
wc -l crates/executor/src/expr/mod.rs crates/executor/src/stored_proc.rs crates/executor/src/trigger.rs
```

**判据**：每个**产品代码**文件（`#[cfg(test)]` 之前）< 1600 行；
**且** `expr/mod.rs` 与 `stored_proc.rs` 的**整个文件**也 < 1600（因为门禁量的是整个文件）。

### 3.5 公开 API 路径不变（外部调用方零改动）

```bash
git diff --stat -- src/ crates/catalog/src/ crates/storage/src/ tests/integration/sql/
```

**判据**：拆分**不应**让外部调用方改一行（`pub use` 再导出的目的就是这个）。
若外部调用方必须改，说明再导出没做对 —— 停下来重做，而不是改调用方。

### 3.6 工具自身的反向变异证明（护栏承重）

「护栏必须用变异测试证明承重」—— 「测试是绿的」本身不能证明护栏有效。
`audit/v4.1.0/refactor_api_surface.py --selftest` 共 **16 条**，其中 **4 条是反向变异**：

| 变异 | 期望 | 实测 |
|---|---|---|
| 删掉一个 `pub` 项 | diff 报差异（rc=1） | ✅ |
| 只改 `pub fn` 的参数类型 | diff 报差异（rc=1） | ✅ |
| `pub` → `pub(crate)` 收窄 | diff 报差异（rc=1） | ✅ |
| **对照**：两份完全相同 | 不报差异（rc=0） | ✅ |

```bash
python3 audit/v4.1.0/refactor_api_surface.py --selftest
# ⇒ refactor_api_surface selftest: 16/16 passed
```

⇒ 工具本身**有判定力**，不是只会打印 diff 的空壳。

---

## 4. R2 开工前必须先解决的（阻塞项）

| # | 阻塞项 | 性质 | 谁能解 |
|---|---|---|---|
| B1 | 判据 **#4**（映射层）：C01-15 与 ADR 的绑定口径 | **属主裁决 O1** | 属主（执行者不得自拟映射，A2.2 R5 / O10） |
| B2 | 判据 **#6**（签署层）：§3.1.1 的 Reviewer 名单表 | **属主人工** | 属主填名单表（C7 / R-ORACLE） |
| B3 | **#4947 的硬边前提**：它已被 `f4562023e` 回退 ⇒ `STAGE.yaml:911` 记的前提过时 | **属主复核** | 属主 |
| B4 | 放行批次（O5）：本笔记的哪几片进第一批 | **属主裁决** | 属主 |
| B5 | 判据 #4/#6 **零负向夹具**（willy2 T-H 实测：两者对 `gen_verdict`/`verify_matrix`/`verify_handoff` 的引用数 = 0） | 夹具缺口 | 另一条线在做（不在本 PR） |

> **B1–B4 未落定前，R2 不得开工** —— 本笔记只交付「设计与契约」，不交付代码。

---

## 5. 与已有计划文档的关系

| 文档 | 关系 |
|---|---|
| `docs/releases/v4.1.0/refactoring/gpt-r1/TEST-FACTORY-RECOVERY-PLAN.md` | 上游总计划；本文件是它 **R0/R1 阶段**在 REF-01/02/03 上的具体化 |
| `docs/releases/v4.1.0/PERFORMANCE_OPTIMIZATION_PLAN.md` L555/L602 | #4914 **B1.3**「判定为独立设计 PR」「⏸ 暂不实施」⇒ **B1.3 至今无任何提交实现** |
| `docs/releases/v4.1.0/STAGE.yaml` L879/L911 | 两条硬串行边（读路径 / #4947） |
| `docs/releases/v4.1.0/evidence/baseline-followup-20261125-001/TB_*.stdout` | R0 盘点的原始证据（远端只读分析 T-B） |

### 5.1 一处需要订正的计划文档说法

willy2 的 T-B 实测：**全仓有 7 个 `find_column_index` 的同名或近名定义**
（`expr/mod.rs:642`、`src/engine_utils.rs:1027`、`src/expr_utils.rs:1471`、
`crates/storage/src/engine.rs:891/919`、`crates/sql-corpus/src/lib.rs:795`、
`crates/executor/src/merge.rs:454`），在 `crates/` 与 `src/` 中共出现 **133 次**。

计划文档写的「40 处调用点」**已过时** ⇒ 拆分 REF-01 时若发现「调用点比预期多」，
**不是计划错了，是计划过时** ⇒ 以实测为准，并在拆分提交里写明新的数字。

---

## 6. 一句话交接

> **REF-03 只差「把 1281 行内联测试挪出去」这一步**（产品代码 1347 行，已达标）；
> **REF-01/02 需要先挪测试、再按职责拆 5/4 个子模块**，全程用 `pub use` 保证外部路径不变。
> 三个文件的拆分**共用同一个验收判据集**（§3），其中最强的一条是
> **公开 API 逐字节不变**。
> **但 R2 开工前，判据 #4/#6 必须先落地**（属主域）。

# PR-5379 TEST_DESIGN — 断言与门禁的设计

> 关联: `PR-5379_SPEC.md` · `PR-5379_TEST_PLAN.md` · 证据: `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md`
> OpenSpec: `openspec/changes/fix-5379-tpch-fixture-hermeticity/design.md`

---

## 1. 设计原则

本 PR 的测试不是「给新代码补覆盖」，而是**在长期假绿的 CI 上重建可信度**。因此测试设计本身要满足
几条比「覆盖率」更严格的约束：

| 原则 | 含义 | 反例（被本 PR 拒绝的设计） |
|---|---|---|
| **P1 期望值外部化** | 期望值只能来自 (a) 独立预言机，或 (b) 与数据无关的语义不变量。**禁止**用本引擎输出回填 | 把 `eprintln!` 打印的值改成 `assert_eq!(observed, printed_value)` |
| **P2 独立性** | 预言机不得与被测对象共享实现路径。SQLite 是**进程内、另一套 SQL 解析器**，与 `src/engine_select.rs` 无共享代码 | 用引擎自己的 `ExecutionEngine` 跑两遍再比对 |
| **P3 非平凡性** | 每个计数类断言必须配套「结果非空 / 覆盖面非平凡」的第二条断言 | `assert!(total >= 22)`（恒真，构造上不可能失败） |
| **P4 可证伪** | 断言集必须能杀死已知的引擎缺陷。证明手段是变异测试，不是「我看了觉得对」 | 断言只覆盖 happy path 的单行结果 |
| **P5 状态显式** | 「没跑」和「跑了且通过」是两种不同的事实，前者不得计入后者 | 缺 fixture 时 `eprintln!` 后 `return`，测试报 ok |
| **P6 单一权威** | 同一语义（TPC-H schema、fixture 解析、SQL 方言适配）只允许一处权威定义 | 引擎 harness 与测试 harness 各写一份 `orders` DDL |

P6 的代价是本轮实测出的真实缺陷：`tests/common/tpch_wire_harness.rs` 的 `orders` DDL 少声明
`o_totalprice`（8 列 vs `.tbl` 的 9 字段），导致 `o_orderstatus` 之后**每列静默错位一格**，
且**没有任何断言会发现**。详见 §9 的「盲区」。

---

## 2. 断言分类学

13 个原本零断言的文件（`tests/integration/oracle/*.rs`）补入 181 条断言，分四类。
每一类的**判定强度**不同，混用会让"通过"变成一句没有信息量的话。

### A 类 — 独立预言机对照（117 条 / 117 个查询）

```
engine_sql ──► SQLRustGo 执行 ──► rows_engine
     │
     └─(adapt_sql 方言适配)──► SQLite 执行 ──► rows_oracle
                                          │
                     check_against_oracle: 排序后多重集比对
```

设计要点：

- **行序不比**。跨引擎比对按**排序后多重集**（SPEC D7）。无 `ORDER BY` 的查询行序是实现定义的，
  把它当差异会制造假失败。行序正确性由 B 类不变量单独覆盖。
- **浮点容差 1e-9**（`FLOAT_REL_TOLERANCE`）。SUM/AVG 在两个引擎里末位不同是正常的，
  但容差必须是**显式常量**并在断言处可见，不是藏在比较函数里的魔法数。
- **归一化对称**。`engine_cell_to_string` 与 `sqlite_cell_to_string` 是两个独立函数，
  各自把值转成可比字符串；`NULL` 用哨兵 `"\u{0}NULL"` 区分于空字符串 `""`。
- **差异信息可定位**。`diff_rows` 返回 `row {i} col {j}: engine={ec:?} oracle={oc:?}`，
  失败时直接能看出是第几行第几列，而不是 `assert_eq!` 打印两个大 Vec。

### B 类 — 数据无关不变量（65 条）

不依赖任何期望数据集，只依赖 TPC-H 规范和 SQL 语义。可在**任意** fixture 上成立。

例：`Q11` 的 `sum(l_quantity) > 0` 不依赖数据取值；但「Q11 返回的 `sum` 必须等于
`sum` 的各个分组小计之和」依赖数据，故归 A 类。

### C 类 — 防空转不变量（15 条）

直接针对 §1 的 P3 与 Lesson-08（"引擎与预言机一致，可能只是在比较空集合"）。

典型形态：

```
assert!(rows_oracle.len() > 0, "Q7 在本 fixture 上结果为空 → 断言无意义");
assert!(filtered_rows.len() > 0, "过滤谓词未命中任何行");
assert!(distinct_groups > 1, "GROUP BY 塌成单组 → 聚合未被真正检验");
```

C 类断言**先于** A 类断言执行：C 类失败时 A 类的"一致"是没有意义的，直接判 FAIL。

### D 类 — 结构性元断言（少量，散布在门禁脚本里）

断言的是**仓库状态**而非运行时行为：

- `tests/integration/oracle/*.rs` 的 `assert!` 计数 > 0（防止下一轮有人把断言删回去）
- `git ls-files` 中不含 `*.tbl`（防止 fixture 入库）
- `Cargo.toml` 声明的目标数 == CI 实际执行的目标数

---

## 3. 预言机设计（`tests/common/sqlite_oracle.rs`）

### 3.1 为什么是进程内 SQLite

SPEC D6：rusqlite 已在根 package 依赖中（`Cargo.toml:252`，带 `bundled` 特性），
CI 零外部服务依赖。DuckDB 是 `GATE_SCRIPT_STANDARD.md:160` 允许的另一个选项，但全仓未落地，
接入成本高于收益。

### 3.2 方言适配必须白名单

SQLite 不认 `EXTRACT(YEAR FROM x)`。`adapt_sql` 做**白名单式**改写：

| 源方言 | 目标 | 白名单理由 |
|---|---|---|
| `EXTRACT(YEAR FROM x)` | `CAST(strftime('%Y', x) AS INTEGER)` | TPC-H 规范查询里唯一用到的 SQLite 不兼容语法 |

`adapt_sql` 返回 `AdaptedSql { sql, unsupported }`。**任何未登记的不兼容语法都会进 `unsupported`**，
调用方必须显式处理（`eval_22_vs_sf01` 把 `unsupported` 计入失败，不是忽略）。
这条是刻意设计的：隐式的「尽力而为」改写会让预言机悄悄和被测对象说不同的话。

### 3.3 装载路径

```
fixture .tbl (管道分隔, 末尾 |)
      │ parse_tbl
      ▼
  Vec<Vec<String>>  ──► 事务内批量 INSERT (rusqlite, prepared statement)
      │                        │
      │                        ▼
      │                   SQLite (进程内, 内存或临时文件)
      ▼
  engine 侧同一份 .tbl ──► load_into_storage (sqlrustgo 自己的存储)
```

两侧读**同一份 fixture**，所以差异只能来自引擎/预言机的执行语义，不能来自数据不同。

### 3.4 自检

`sqlite_oracle.rs` 自带 10 项单元测试（不依赖 fixture）：
`adapt_sql` 的改写与 `unsupported` 收集、浮点容差边界、NULL 与空串的区分、
多重集比对在顺序不同/数量不同/单值不同三种情形下的返回。

---

## 4. fixture 与 profile 设计（`tests/common/tpch_fixture.rs`）

### 4.1 解析顺序

`FixtureState::resolve()` 的候选目录顺序（后者覆盖前者）：

1. `TPCH_DATA_DIR` 环境变量指向的目录
2. 仓库内 `tests/data/tpch-sf01`、`tests/data/tpch-sf001`（**本 PR 已删除这两个悬空符号链接**）

删除悬空链接而不是重建，理由见 SPEC D1：指向仓库外的符号链接是"假 fixture"，
它的存在会让"数据可达"这件事永远无法从仓库本身判定。

### 4.2 manifest 与 profile

fixture 目录里必须有一份 `tpch_fixture_manifest.json`，记录 `sf` / `seed` / `generator_sha256` /
行数。`Manifest::load` 在文件缺失或不可解析时返回**空 manifest**（`sf = 0.0`），
从而使所有依赖 profile 的断言走 `profile_mismatch` 而不是被误判为 PASS。

**profile 不匹配优先于已知差异登记**：数据依赖型期望值是在**某个标定 profile** 上标定的
（`oracle_g1_tpch_sha256` 的 SHA-256 基线标定在 SF=0.001）。在别的 scale 上，
这些期望值本身就是错的。此时正确行为是判 **NOT-RUN**，而不是"允许一个已知差异"。
已知差异登记表**只用于同一 fixture 上引擎与预言机的真实分歧**。

### 4.3 生成器必须自检

`scripts/gate/generate_tpch_sf.py` 在产出 fixture 前跑 4 类自检，任何一类不过就 `exit 1` 且**不产出**：

| 自检 | 拦住的退化 |
|---|---|
| 键分布 | 键不均匀会让某些 join 路径永远不被遍历 |
| 谓词覆盖率（12 条规范过滤） | 过滤恒空 → 查询结构性空转 |
| 复合过滤覆盖率（3 条组合） | 组合过滤恒空 → 多谓词逻辑未被检验 |
| nation→region 映射逐个校验 | 映射错位 → Q5/Q7/Q8/Q9 的 join 结论全错 |

实测效果：22 条规范查询的**结构性空转从 9 条降到 1 条**（残留的 Q22 属 SF=0.01 规模层固有：
期望 0.07 个无订单客户，本就不该有命中）。

### 4.4 manifest 记生成器指纹

`prepare_tpch_fixture.sh` 的幂等判定是 `(sf, seed, generator_sha256)` 三元组匹配。
只比 `(sf, seed)` 的话，生成器改了仍会 SKIP 并保留**陈旧 fixture**——这正是本轮最初踩的坑。

---

## 5. 状态机设计：PASS / FAIL / NOT-RUN

### 5.1 原因码

`report_not_run(target, reason, detail)` 落盘一行：

```
TPCH-NOT-RUN: <target> reason=<code> detail=...
```

| 原因码 | 触发条件 |
|---|---|
| `fixture_missing` | 候选目录里缺 `.tbl` 或缺表 |
| `profile_mismatch` | manifest 的 `sf` ≠ 断言标定时的 `expected_sf` |
| `scale_layer_only` | 该目标只对 SF=1 规模层有意义 |
| `known_difference` | 命中已知差异登记表，且登记表合规 |

### 5.2 落盘通道

libtest **捕获测试的 stdout**，所以 NOT-RUN 标记不能靠 `println!` 出现在测试输出里；
必须写文件（`TPCH_NOT_RUN_FILE`），由 CI 步骤在测试进程结束后读取。
用 `--show-output` 也会把标记混进 `passed` 的统计里，得不偿失。

### 5.3 披露算术

CI 摘要的算术规则（写在 `.gitea/workflows/ci.yml` 的 `test` job）：

```
declared = Cargo.toml 中 [[test]] 的总数
executed = 本次实际产出了 "test result:" 行的目标数
not_run  = NOT-RUN 文件中标记的目标数
passed   = Σ 各目标 "N passed" 的 N
failed   = Σ 各目标 "N failed" 的 N
```

关键约束：**`not_run` 从 `passed` 中扣除**，不计入通过。声明/执行两个数分开披露，
让"漏跑"在摘要上就可见，而不是靠人去比对日志。

### 5.4 执行核对门禁

`check_test_target_execution.sh` 拿 `Cargo.toml` 的声明清单去比对 CI 日志里实际执行过的目标，
缺任何一个就 `exit 1` 并列出全部 missing。
用**真实 CI 日志**（run 7613）验证过：旧状态下该门禁 `exit 1` 并报出 **382 个 missing**。

---

## 6. 门禁设计

| 脚本 | 退出码语义 | 设计要点 |
|---|---|---|
| `prepare_tpch_fixture.sh` | 0 = 就绪 / 非 0 = 生成失败 | 幂等判定含生成器指纹；4 类自检不通过则不产出 fixture |
| `check_test_target_execution.sh` | 0 = 全部执行 / 1 = 有 missing | 缺一个就 fail，不设容忍阈值 |
| `check_tpch_known_differences.sh` | 0 = 登记表合规 / 1 = 违规 | 每条须有 issue + owner；`actual == oracle` 的条目强制清理（差异已消失却还挂着 = 过期豁免） |
| `mutate_tpch_oracle_tests.sh` | 0 = 全部 CAUGHT / 1 = 有 SURVIVED / 2 = harness 自身失败 | 注入点在**引擎侧**，绝不动测试代码 |

### 6.1 已知差异登记表的滥用防线

登记表（`tests/baseline/tpch_known_differences.json`）是「缺陷永久豁免」的高风险设计，
因此门禁强制三条：

1. 每条必须带 `issue` 与 `owner`——没有归属的差异无法被跟踪
2. 条目数有上限
3. `actual == oracle` 的条目**强制清理**——差异已经不存在了还挂着，就是过期豁免

初始登记表为空表（`[]`），门禁在空表下 `exit 0`。

---

## 7. 变异测试设计（`mutate_tpch_oracle_tests.sh`）

### 7.1 为什么必须做

13 个文件在改造前**零断言**——它们"通过"是因为什么都没检查。若新断言同样是"看起来对"，
本 PR 只是把零断言换成了更难发现的伪断言。**变异测试是唯一能证伪"断言真的会杀缺陷"的手段**。

### 7.2 注入点与判定

突变**只注入引擎实现**（`src/engine_select.rs`），绝不改测试代码——改测试是自证。

| 编号 | 变异 | 检验的断言类别 |
|---|---|---|
| MUT-1 | HAVING 谓词取反 | A 类（过滤结果与预言机不一致） |
| MUT-2 | SUM 丢掉第一个值 | A 类（聚合末位错误） |
| MUT-3 | COUNT(DISTINCT x) 退化为 COUNT(*) | A 类（去重丢失） |

参照目标 `diag_q11`：同时带 SQLite 对照 **和** 数据无关不变量，
因此 3 个变异都必须被它抓到。

### 7.3 影响面已按 CLAUDE.md 要求评估

`gitnexus impact eval_predicate_with_subq` → risk **CRITICAL**，21 个受影响符号、5 条执行流。
这是预期的：被突变的谓词求值器是共享的。变异只在单次测试运行期间生效，
脚本在退出前**无条件还原文件**（含异常路径）。

### 7.4 存活时的处置顺序

变异存活时**先怀疑夹具和检测器**，再怀疑引擎：
夹具没覆盖到该分支、或检测器的断言根本没被执行到，都是更常见的原因。

---

## 8. 覆盖强度与盲区

### 8.1 检测能力矩阵

| 引擎缺陷形态 | 能抓住它的断言 | 强度 |
|---|---|---|
| 过滤条件错（多留/少留行） | A 类对照 + MUT-1 | 高 |
| 聚合值错（末位/漏项） | A 类对照 + MUT-2 | 高 |
| 去重丢失 | A 类对照 + MUT-3 | 高 |
| DISTINCT 语义错 | A 类对照 | 高 |
| JOIN 漏行（键不匹配） | A 类对照 + 4 类生成时自检 | 高 |
| GROUP BY 塌成单组 | C 类防空转 | 中（只在 fixture 恰好塌组时暴露） |
| 行序错（无 ORDER BY） | **不覆盖**——故意不比行序 | 覆盖由 B 类不变量承担 |
| 列错位（schema 不一致） | **不覆盖**——见 §9 | 本 PR 已修 schema，但检测手段缺失 |

### 8.2 有意不覆盖的部分

- **行序**：跨引擎行序不可比（SPEC D7）。行序正确性由单独的不变量覆盖，不是本 PR 的对照断言。
- **SF=1 规模层**：需要 dbgen 产数据，CI 成本与本轮目标不匹配 → **#5387**。
- **剩余 19 个零断言文件**（`tests/integration/oracle/` 的 `#[ignore]` 子集）→ **#5391**。

---

## 9. 已知盲区：schema 错位无人发现

本轮实测的层-5 缺陷值得单列，因为它暴露了**测试设计上的一个洞**：

`tests/common/tpch_wire_harness.rs` 的 `orders` DDL 声明 8 列，`.tbl` 是 9 字段
（漏 `o_totalprice`）。后果是 `o_orderstatus` 之后每列静默错位一格。

**为什么 181 条新断言没抓到它**：
A 类对照是**逐行整表**比对，如果对照的引擎侧和预言机侧读的是同一份 `.tbl`、各自按自己的 DDL 建表，
那么错位会**同时**发生在两侧，比对结果仍然"一致"。
换言之，**对照只能抓执行语义差异，抓不了装载语义差异**——后者需要"两套独立的 schema 定义"，
而那正是 P6（单一权威）要消除的东西。

这是一个**真实存在的设计张力**：
单一权威消除"两份 schema 不一致"这个故障源，但也消灭了能发现"某一份错了"的对照。

本 PR 的处理：

1. **修 schema**（`orders` 补 `o_totalprice REAL NOT NULL`，附长注释说明缺陷形态）
2. **加掉行即失败的守卫**（`eval_22_vs_sf01`）：表行数与 fixture manifest 声明的行数不符即 FAIL
   —— 这一条**能**抓住"装载时整表丢行"，但**抓不住**"列错位但行数正确"
3. 把"列级错位的独立检测"作为**已知盲区**写进本设计文档，并留待后续（`#5388` 归因批次）

不做虚假承诺：本次没有为"列错位"建立独立的检测手段。

---

## 10. 与其它文档的分工

| 文档 | 回答的问题 |
|---|---|
| `PR-5379_SPEC.md` | 做什么、为什么、边界在哪 |
| `PR-5379_TEST_PLAN.md` | 要覆盖哪些目标、分几个阶段、验收口径 |
| **`PR-5379_TEST_DESIGN.md`（本文）** | **断言怎么设计、预言机怎么实现、状态机怎么表达、盲区在哪** |
| `PR-5379_TEST_REVIEW.md` | 独立审核者对上述设计的反对意见与裁定 |
| `PR-5379_ACCEPTANCE.md` | 门禁的真实输出与逐条验收判定 |

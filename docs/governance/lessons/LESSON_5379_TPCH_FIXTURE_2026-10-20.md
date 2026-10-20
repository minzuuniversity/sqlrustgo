# LESSON_5379_TPCH_FIXTURE_2026-10-20.md

> **来源 Issue**: #5379（TPC-H fixture 悬空符号链接）/ #5387 / #5388 / #5389 / #5390
> **来源 Run**: fix/5379-tpch-fixture-hermeticity
> **日期**: 2026-10-20
> **证据**: `docs/releases/v4.1.0/evidence/TPCH_FIXTURE_5379_2026-10-20.md`
> **OpenSpec**: `openspec/changes/fix-5379-tpch-fixture-hermeticity/`

---

## 概述

本轮迭代的 issue 标题是「TPC-H fixture 悬空符号链接导致 CI `test` job 恒红」。
逐层剥开后发现问题面比标题大得多，而且是**五层互相掩护**的结构——
每一层单独看都不算严重，合起来让 CI 长期显示"接近全绿"的假象：

| 层 | 症状 | 谁在掩护谁 |
|---|---|---|
| 1 | 数据不可达：两个指向 `/tmp` 的悬空符号链接 | 被第 2 层的 fail-fast 截断掩盖 |
| 2 | 382 个声明目标从未执行（`cargo test` 默认 fail-fast，64/446 实际跑过） | 被第 3 层的"摘要很绿"掩盖 |
| 3 | 绝大多数测试零断言 / 断言恒真 | 被第 4 层的"数据很真"掩盖 |
| 4 | fixture 现实度不足：22 条规范查询里 9 条结构性空转 | 被第 5 层的"引擎很对"掩盖 |
| 5 | 共享 harness 的 `orders` schema 少一列 → 静默列错位 | 被第 1 层的"数据都拿不到"掩盖 |

第 4、5 层是**把 fixture 真正接上之后**才浮出水面的——这也说明前 3 层的修复
只是让"更真的问题"有机会被看见，而不是让问题变少。

本轮最有价值的产出不是"CI 变绿了"，而是**把 CI 从"会骗人"改成"会说话"**：
不通过就不通过，通过也要说清楚到底跑了几项、哪些没跑、哪些没跑为什么；
以及把"通过"进一步拆成"通过"与"问了个有意义的问题"。

---

## Lesson-01: issue 标题通常只描述最表层的那一层

### Observation

issue 说「fixture 是悬空符号链接」。修掉符号链接，CI 确实会不红了——但那时 382 个测试目标依然从未执行过。

### Root Cause

三层问题叠在一起，且**后两层被第一层挡住**：

| 层 | 单独看 | 合起来 |
|---|---|---|
| 1. fixture 指向仓库外 `/tmp` | 致命 | 致命 |
| 2. `cargo test` 默认 fail-fast | 常见 | **382/446 个目标从未执行** |
| 3. 13 个文件零断言 | 少见 | **测试"通过"但不证明任何事** |

只修第 1 层，CI 变绿，而实际状态是"绿得没有信息量"——这正是 `AGENTS.md §8` #3 禁止的
「删除断言/跳过测试/修改参考结果来 PASS」的**结构性版本**。

### Action

分析阶段不要停在 issue 描述的层面。**用数据量化"本该发生但没发生的事"**：
本轮的量化动作是拿 CI 日志里的 `Running <path>` 集合与 Cargo.toml 的 `[[test]]` 声明集合做差集，
一次得到 382 这个数字——它比"fixture 是坏的"重要一个数量级。

### Rule

> 关闭一个 issue 前，先问：**这个问题挡住了什么？** 被挡住的东西本身是不是问题？

---

## Lesson-02: fail-fast 会把"漏跑"变成"沉默"，而摘要的统计口径决定它会不会被看见

### Observation

CI 摘要打的是 `Tests: passed=70 failed=1`，看起来只有 1 个失败。
但 `70` 是**测试二进制**的个数，不是测试数；而 446 个声明目标里只有 64 个真的跑过。

### Root Cause

两层信息在摘要里被压缩掉了：

1. **fail-fast**：第一个失败目标之后的所有目标**根本没跑**——不是"没失败"，是"没运行"。
   两者在摘要里长得一样：`passed=70` 既可能是"70 个都过了"，也可能是"只跑了 70 个"。
2. **统计口径**：数 `test result: ok` 行得到的是**二进制数**。446 个目标里跑 64 个，
   摘要显示 `passed=64`——和一个"全部通过"的长得几乎一样。

### Action

1. CI 改 `cargo test --all-features --no-fail-fast`（`AGENTS.md §7` 本来就这么要求，CI 一直没加）。
2. 摘要改成**三态 + 双口径**：`passed / failed / not_run / ignored`（测试函数级）
   加上 `declared_targets / executed_targets`（目标级）。
3. 新增门禁 `scripts/gate/check_test_target_execution.sh`：
   声明集合 − 执行集合 ≠ ∅ 即 FAIL。用**真实 CI 日志**验证：exit 1 + 列出 382 个漏跑目标。

### Rule

> 一个只报"过了多少"的摘要，无法区分"都过了"和"只跑了一部分"。
> **执行覆盖率必须和通过率一起报**，并且要有门禁守住。

---

## Lesson-03: `option_env!` 是编译期的，它看不见运行期的环境变量

### Observation

13 个 oracle 测试都写了 `option_env!("TPCH_DATA_DIR")` 回退到 `tests/data/tpch-sf01`。
CI 里给 fixture 加了环境变量之后，它们**仍然**去读那个仓库外的符号链接。

### Root Cause

`option_env!` 在**编译时**展开。CI 的步骤是"先编译，再设环境变量，再运行"——
编译时 `TPCH_DATA_DIR` 根本不存在，于是 `None` 分支恒中，回退路径恒中。
更隐蔽的是：它**看起来**已经支持外部覆盖了。

### Action

统一改成运行期读取（`std::env::var`），并把解析规则收敛到一处
（`tests/common/tpch_fixture.rs` 与 `tests/common/tpch_wire_harness.rs`），
候选顺序固定为：`TPCH_DATA_DIR` → 仓库内候选目录 → 明确缺失。

### Rule

> 任何"可被环境变量配置"的东西，都要问一句：**这个变量是编译期还是运行期读的？**
> 写 `option_env!` 的地方，九成是想读运行期环境。

---

## Lesson-04: 断言的期望值不能来自被测系统自己

### Observation

13 个文件用 `eprintln!` 打印"期望 80"、"expect ~1060"，断言数为 0。
最自然的"修复"是把这行 eprintln 变成 `assert_eq!(r3.rows.len(), 80)`。

### Root Cause

那个 `80` 是作者本机 SF=1 数据上**本引擎自己的输出**。
把它回填成期望值，等于把当前行为固化成"标准"——测试从此只会证明"引擎没变"，
不证明"引擎是对的"。而这正是仓库里已有的
`scripts/gate/check_g1_tpch_baseline.sh` 第 4 步在做的事：
它自称 "independent verification"，实现上却是 `tpch_hash_compare.py --check`
→ 内部再跑一次**同一个** `cargo test --test tpch_hash_test`。同源、同引擎、同数据。

### Action

期望值只允许来自两个地方：

1. **独立预言机** —— 本轮用**进程内 SQLite**（`rusqlite` 的 `bundled` 特性已在本仓依赖里，
   CI 零外部依赖即可运行；`GATE_SCRIPT_STANDARD.md:160` 本来就要求 oracle 用 DuckDB/SQLite）。
2. **数据无关不变量** —— 期望值来自数据自身的一致性，而不是写死的行数。
   例：「HAVING 阈值 0 必须保留全部分组」「超过所有聚合值的阈值必须保留 0 行」
   「GROUP BY 行数 == COUNT(DISTINCT ...) over 同一 join」。

并且用**变异测试**证明这些断言真的会杀缺陷（本轮 3 个变异，全部 CAUGHT）——
一个杀不掉任何缺陷的断言，和没有断言是同一件事。

### Rule

> 写断言时先问：**期望值是谁给的？** 如果答案是"被测系统自己上一次跑的输出"，
> 那不是断言，是快照。

---

## Lesson-05: 悬空符号链接比"没有"更坏

### Observation

`tests/data/tpch-sf01` → `/tmp/tpch-sf01`，`tests/data/tpch-sf001` → `/tmp/tpch-sf001`。
两个都指向仓库外，指向的目标在本机根本不存在。

### Root Cause

符号链接是一种**声明**："这里有一份 fixture"。任何检查"路径存在吗"的逻辑都会被它骗过
（`[ -e ]` 失败但 `[ -L ]` 成立），然后在上游以"文件读不到"的形式炸掉——
错误信息指向 `.tbl` 读取，而不是指向"这个 fixture 从来就不存在"。

引入提交 `41fa140773` 的说明是「Tracked the tpch-sf01 symlink so the 27 tests that reference it can find it after a clone」——
**该推理错误**：绝对 `/tmp` 路径只在作者本机成立。

### Action

删除这两个符号链接，改为运行时生成 fixture（`scripts/gate/prepare_tpch_fixture.sh`，
确定性 seed=42，幂等），并通过 `TPCH_DATA_DIR` 交给测试。

### Rule

> 仓库里不允许存在指向仓库外的符号链接。要么是仓库内的真实内容，要么不存在。

---

## Lesson-06: 注释不是证据，测试也不该写仓库

### Observation

1. `ci.yml` 里 G1 步骤的注释写「this step currently fails by design until the baseline hash in
   `tests/tpch_hashes_v380.json` is filled in」——**那个 hash 早在 `545814c3d1` 就填好了**，
   真正的阻塞是 SF=1 fixture 缺失。注释把原因指错了 8 个月。
2. 全量跑测试后 `git status` 出现 **83 个被修改文件**，其中 81 个是
   `docs/releases/v4.1.0/evidence/a04-5226/wire/*.txt`，写入方是
   `tests/integration/compat/a04_mysql_diff_matrix_test.rs`。

### Root Cause

- 注释是写的时候的世界，没有随现实更新；ADR-001 G-08 明确禁止把文档声明当作通过证据。
- 测试把输出物写进仓库，既污染工作区，又让"跑完测试后 git diff 是脏的"成为常态——
  久而久之，真正的改动就被淹没在噪声里。

### Action

- G1 步骤改为**显式三态**：有 SF=1 fixture 就照常跑并硬失败；没有就输出
  `STATUS: NOT RUN (not PASS, not FAIL)`，绝不冒充通过。
- 测试写仓库的问题立为 #5389。

### Rule

> 注释会过期，代码不会。**任何"为什么现在是这个状态"的解释，要么指向可运行的检查，
> 要么注明测量时间与 commit。**

---

## Lesson-07: 关掉 fail-fast 之后，必须给"慢"装一个上限

### Observation

改成 `--no-fail-fast` 后，第一次全量执行暴露出 `int2_substance_parallel_test` 里的
`test_int2_sequential_skips_parallel` 长时间不返回（本机负载 16/10 核时尤为明显）。
在 fail-fast 时代，这个测试**从未被执行过**，所以谁也不知道它有多慢。

### Root Cause

fail-fast 同时扮演了两个角色：**快速失败**（好）与**快速截断**（掩盖）。
关掉它，就必须给"卡住"这个新的失败模式准备一个出口，否则一个卡住的目标能吃掉整个 job
（CI 容器上限 3 小时）。

### Action

CI 的测试步骤加 `timeout 7200`；超时后摘要报告部分结果，
执行核对门禁会点名哪些目标没跑——失败仍然是失败，但这次是可诊断的失败。

### Rule

> 移除"快速截断"这个安全网之前，先想清楚"卡住"这个新风险由谁兜底。

---

## Lesson-08: 引擎和预言机一致，可能只是因为它们都在比较空集合

### Observation

fixture 接上之后，`eval_22_vs_sf01` 报出 **22/22 MATCHED**——引擎与 SQLite 独立预言机
在全部 22 条 TPC-H 查询上完全一致。看起来是一份完美的绿色成绩单。

但逐条量了结果集规模之后才发现：**其中 9 条两边都是 0 行**。
Q7/Q8/Q9 返回 0 行，Q13 只有一个分组，Q17/Q19 返回 NULL 聚合，Q18/Q20/Q22 返回 0 行。
"一致"是真的，"有意义"是假的——两个引擎都没被问到任何问题。

根因不在引擎，在 **fixture 的现实度**：生成器把 `o_comment` 写成恒定的 `"Ord comment"`、
把 `p_brand` 写成 `Brand#12323`、把 `c_phone` 写成 `+1-…`，而规范查询过滤的正是
`o_comment LIKE '%special%requests%'`、`p_brand = 'Brand#23'`、`SUBSTR(c_phone,1,2) IN (…)`。
于是这些过滤永远匹配 0 行。

### Root Cause

**"断言通过"与"断言有意义"是两个独立的性质**，而我们默认了前者蕴含后者。
当期望值来自一个独立预言机时，这一点尤其危险：预言机保证的是"两边一致"，
不是"两边都对"——**一个空数据集同样能让两个引擎完美一致**。

自检手段失效的原因也很具体：`distinct count` 这类检查看不出文本列是否承载了
查询所过滤的字面值；`o_comment` 恒为同一个字符串，distinct count 是 1，看起来"很集中"，
实际上该列完全没有信息。

### Action

1. 生成器加**谓词覆盖率自检**：把规范查询集里的过滤条件逐条列出来，要求每条都选中
   非平凡比例的行，任一不达标就 `exit 1`、**不产出 fixture**。
2. 加**复合过滤覆盖率自检**：品牌×容器这类组合过滤，单列检查是看不出来的。
3. 每条对照断言都配**防空转**条件：结果集必须非空、IN 与 NOT IN 必须构成对全集的划分。

实测效果：空转查询从 **9/22 降到 1/22**（剩下 1 条是 SF=0.01 规模层的固有现象，已在证据里说明）。

### Rule

> **断言通过之后还要问一句：它问的问题有意义吗？**
> 用独立预言机做对照时，"一致"只保证了两边同错或同对——
> 必须另有防空转条件（结果非空 / 覆盖面 / 划分关系）来保证这个问题问到了东西。

---

## Lesson-09: 同一份数据的两份 schema 不一致，会让所有列错位都变得静默

### Observation

`tests/common/tpch_wire_harness.rs` 里 `orders` 表的 DDL 只声明了 **8 列**——漏了
`o_totalprice`。而 `.tbl` 的 orders 数据是 **9 个字段**。

于是 `o_orderstatus` 之后每一列都错位一格：`o_orderdate` 位置存的是价格、
`o_clerk` 位置存的是日期、`o_comment` 直接掉出行外。

最耐人寻味的是**它为什么没被发现**：过滤 `o_orderstatus` 仍然命中正确的行
（它在错位之前），所以"表里有数据""行数对得上""简单过滤能工作"——全都正常。
而后面每一列拿到的都是别人的值。

暴露它的是一条 `LIKE`：`o_comment LIKE '%special%requests%'` 选出 0 行。
因为引擎实际比的是**价格列**。而"LIKE 坏了"这个结论会把人引向完全错误的方向
（去查 LIKE 匹配算法）——真实原因是**列错位**。

同一个仓库里，`tests/common/tpch_cli_harness.rs` 的 `orders` DDL 却是**对的**（9 列）。
两份 schema 并存、互不参照，正是它长期存活的原因。

### Root Cause

**"行数对得上"和"每列的值对得上"是两个性质**，而加载路径对二者都不检查：
- 进程内路径（`storage.insert`）响亮地失败：`row width 9 does not match the 8 declared columns`；
- 线协议路径（`LOAD DATA LOCAL INFILE`）静默错位。

只要有一条路径是响亮的、另一条是静默的，**响亮的那条就成了"已知噪声"**，
于是静默的那条永远不会被看见。

### Action

1. 修正 `tpch_wire_harness.rs` 的 `orders` DDL，补上 `o_totalprice REAL NOT NULL`，
   并在注释里写明这个缺陷的形态与影响面。
2. 在共享模块里加**表定义单点化**的检查方向：同一套 TPC-H schema 在仓库里只应有一份权威定义，
   其它 harness 引用它而不是各写一份。
3. `eval_22_vs_sf01` 的加载器加**掉行即失败**守卫：`.tbl` 有一行没进去就直接失败，
   不再靠"跳过"蒙混过关。

### Rule

> **"数据装进去了"不等于"数据装对位置了"。**
> 加载器必须对**行宽**和**列数**都做校验，并且失败要响亮——
> 一个静默的加载路径足以抵消一百个响亮的失败路径。

---

## Lesson-10: 引擎进程被 OOM 杀掉，不是「测试失败」，是**整个测试二进制死掉**

### Observation

改造后的第一次 wire 路径实跑，`oracle_g15_tpch_sf01` 在 Q9 处被杀：

```
error: test failed, to rerun pass `--test oracle_g15_tpch_sf01`
Caused by:
  process didn't exit successfully: `.../oracle_g15_tpch_sf01-77259a8c3a4df057` (signal: 9, SIGKILL: kill)
```

同一现象在改造前的 `tpch_full_22_test` 上也出现过一次（同一份日志、同一个 `signal: 9`）。
**SIGKILL 不是断言失败**：它把测试二进制连同里面还没跑完的其它测试一起带走。

后果在 CI 上被放大：fail-fast 已经被关掉了，但**进程被杀**同样让后面的目标无法执行，
于是「382 个目标漏跑」的老问题会以一种新的、更难察觉的面貌回来 —— CI 日志里根本没有
「这个目标失败了」这一行，只有 job 突然结束。

### Root Cause

file storage 后端在多路 join 上**无界物化中间结果**（Q9 一秒内 RSS 破 1.5 GB）。
Q18 的 `IN (SELECT … GROUP BY … HAVING …)` 同理；Q19 更早一步：wire 前端根本解析不了
括号 OR 组里的 `IN (…)`（`Parse error: Expected RParen in parens, depth=1, current=Some(In)`）。

同一个查询在**进程内内存路径**上跑得动 —— 问题只在 file storage 这条路径上。
也就是说：**「这条查询很难」和「这条查询不可执行」是两件事**，而测试只看得见后者。

### Action

1. 引入**共享 wire 语句范围**（`tests/common/tpch_wire_scope.rs`）：
   允许执行的 19 条、被排除的 3 条（Q9/Q18/Q19），每条排除项带**受控原因码**
   （`engine_resource_limit` / `engine_parse_error`）与 **issue 号**（#5393）。
2. 收口点做成**编译期**的：`run_query_timed(client, qid, sql, timeout)` 的 `qid` 是必填参数，
   范围外的标准查询**不执行**，直接返回 `WireOutcome::Skipped`。
   于是「忘记排除某条重查询」在编译期就不可能发生，而不是靠评审去发现。
3. 被排除的语句由**进程内路径**承担全量覆盖（`eval_22_vs_sf01` 跑全部 22 条），
   语义覆盖没有损失，只是换了路径。
4. 单元测试强制 `WIRE_SCOPE ∪ WIRE_EXCLUSIONS == 22 条标准查询` 且两者不重叠 ——
   范围表一旦过期，测试立刻报错。

### Rule

> **当被测系统可能耗尽机器资源时，「跳过」必须是一等公民，而且要在编译期成为默认路径。**
> 把「资源不可用」当成「断言失败」处理，会让 CI 变成随机崩溃；
> 把「资源不可用」当成「静默通过」处理，会让 CI 变成随机变绿。
> 正确做法是：受控原因码 + 受控登记 + 受控披露，并且**让不安全的执行在类型系统里就写不出来**。

---

## Lesson-11: 「行数相同」和「结果相同」是两个性质

### Observation

Q13 在 wire 路径与独立 SQLite 预言机之间：**行数相同（23 vs 23）、分组数相同（23 个）**，
但取值不同：

| c_count | 引擎 | 预言机 |
|---:|---:|---:|
| 0 | 75 | 146 |
| 1 … 22 | 完全相同 | |

客户总数 1429 vs 1500 —— 引擎少算 71 个客户。

### Root Cause

引擎的 `LEFT OUTER JOIN` 在 **ON 子句带过滤条件**时，把「右表有行但全被 ON 过滤掉」的
左表行整个丢弃，而不是像标准 SQL 那样补 NULL 输出（#5394）。

这类缺陷有一个共同特征：**它不改变结果的形状，只改变结果的内容**。
任何「比对行数」「比对分组数」「比对首行」的检查都会漏掉它。

### Action

1. 跨引擎比对一律按**排序后多重集逐格比对**（数值感知归一化），行序正确性由单独的不变量覆盖。
2. 已知差异登记表增加 `kind` 字段，区分 `row_count` 与 `cell_values`：
   前者「两边行数相等」即判定为过期必须删除；后者**行数相等恰恰是常态**。
3. Q13 按已知差异登记（`case_id = tpch:Q13`），**照样执行、照样逐格比对**，
   差异被 `TPCH-KNOWN-DIFF` 标记披露，既不算失败也不算通过。

### Rule

> **「结果形状一致」不能推出「结果内容一致」。**
> 凡是用行数当判据的断言，都必须再问一句：值变了它能发现吗？
> 而当差异被登记时，登记的粒度必须能表达差异的**种类**，否则门禁会把真实差异误判为「已过期」。

---

## Lesson-12: 拿「另一份 SQL」的行数去对，会得出看起来很专业的错误结论

### Observation

中途我一度判定「Q11 在 wire 路径少 12 行（289 vs 301）」，并据此把 Q11 排除出范围。

真相是：**301 来自生成器自检用的那份 SQL，289 来自测试文件用的那份 SQL**——
两份都是「Q11」，但不是同一句。改用**同一句 SQL**两侧对跑后，289 = 289，完全一致。

### Root Cause

同一个查询编号在仓库里有多份实现：生成器自检用一份、测试文件用一份、另一个目标又用第三份。
它们的行数**没有可比性**。

### Action

1. 比对必须**两侧执行同一句 SQL**（本轮用「同一句 SQL → wire 侧 + 进程内 SQLite 侧」的方式），
   而不是比「行数」。
2. 共享模块里的 `adapt_sql` 白名单是两侧的**共同改写契约**：只允许显式白名单里的改写，
   遇到不支持的构造就报出来，而不是悄悄改写成另一句话。

### Rule

> **跨实现比数字之前，先确认比的是不是同一件事。**
> 编号相同不等于语句相同；用编号相同的另一份语句的行数下结论，
> 是本轮最容易犯、也最不容易被发现的错误 —— 因为它给出的数字看起来完全合理。

---

## Lesson-13: 注释里的「兜底」声明要取证

### Observation

`.gitea/workflows/ci.yml` 的 G1 步骤注释写着：

> The SF=1 layer stays enforced by `scripts/gate/check_stage.sh` at release promotion

实测：`grep -iE 'tpch|sf1|g1' scripts/gate/check_stage.sh` → **零命中**；
`STAGE_CONFIG.yaml` 里 ALPHA/BETA/RC/GA 的 `required_gates` 也**从未列出**
`scripts/gate/check_g1_tpch_baseline.sh`（G1）—— 那个脚本只被 ci.yml 调用。
同一段注释里的 issue 号也写错了（写 `#5385`，那是向量检索的 issue；SF=1 的跟踪号是 **#5387**）。

### Root Cause

注释里的治理声明是**上一轮的结论**，没有人再去核对它是否还成立；
issue 号是在几十次会话间手抄的。

### Action

1. 把 CI 注释改成只陈述**已取证**的事实：SF=1 层现在由哪两处兜底
   （带 SF=1 fixture 的 runner 上的 G1 步骤 + 手动的 `z6g4-tpch-sf1-cell-diff.yml`），
   并**明确写出** `check_stage.sh` 不覆盖 TPC-H。
2. 治理声明与 issue 引用要能在**同一次改动**里被 `grep` 验证。

### Rule

> **注释不是证据。** 凡是要让后来者据此判断「某层有没有被兜底」的声明，
> 必须能被一条命令验证；不能被验证的声明，要么删掉，要么改成事实。
> issue 号尤其危险：抄错一个号，会让整条追踪链静默断掉。

---

## Lesson-14: 编译告警要逐条归属，不能只数总数

### Observation

CI run 7804 的 `test` job 在编译 446 个测试目标时报出 **15 个告警**。
直觉反应是「仓库基线本来就脏，15 个里有 13 个是别人的」——但这个判断**没有做过**。

逐条与基线 `d8365606d69c` 比对后，其中 **2 个是本 PR 引入的**：
`tests/integration/tpch/tpch_sf01_22_vs_3engines.rs:241` 与
`tpch_sf01_22_vs_3engines_test.rs:242` 的 `unused variable: oracle`。
这两个文件本 PR 整份重写（+603/−241），告警无可推诿。

### Root Cause

1. **「基线本来就脏」是一种不用验证的免罪符。** 计数相同的两个集合，
   归属可以完全不同。数目对上了不等于归属对上了。
2. **告警归属需要「文件是否被本 PR 改动」这一维**，光看告警文本看不出来。
   `crates/mysql-server/src/lib.rs` 的 6 个 `seq` 告警在一堆别的告警中间，
   看上去像是自己改出来的；实际上基线同一行就是同一段代码，而本 PR
   对该文件的改动在 7000 行开外。
3. 本 PR 的 `test` job 用 `cargo test` 构建（不带 `-D warnings`），
   `lint-build` 的 clippy 也不带 `--all-targets`，
   所以这 2 个告警**不会让任何门禁变红** —— 不红不等于没有。

### Action

1. 把 CI 编译日志里的告警按「文件 → 行号 → 告警文本」归并，逐条比对基线。
2. 只修本 PR 引入的（commit `de74efcafd`：删掉一个从未被读取的形参及其两个实参），
   基线的 13 个**不夹带**进本 PR —— 混进来会让这个 PR 的 diff 失焦。
3. 归属结论写进证据文档 §7.11，把「哪些是别人的」也记录在案，
   免得下一轮又要重新查一遍。

### Rule

> **告警要逐条归属，数目对不上等于没查。** 判断「这是基线问题还是我引入的」
> 需要三个维度：文件是否被本 PR 改动、行号在基线是否指向同一段代码、
> 告警所在目标是否在门禁覆盖范围内。第三个维度最容易被忽略：
> **不在门禁覆盖范围内的告警不会让 CI 变红，但仍然是本 PR 的债。**

---

## Lesson-15: 门禁脚本的第一次执行不应该是 CI

### Observation

`scripts/gate/mutate_tpch_oracle_tests.sh` 交付时**从未运行过**。盲审时审核者
也只把三个变异片段应用到引擎文件的**副本**上，从未跑过脚本本身
（`PR-5379_TEST_REVIEW.md` §"审核者未执行 … 全程"）。

而证据文档 §5 却写着「结果见 §7（实测输出）」—— §7 里**根本没有**这个输出。
一句指向不存在的证据的话，比没有这句话更糟：它会让读者以为已经验过了。

用桩 `cargo` 做机械逻辑实测时，还查出脚本里一个**只会在 CI 上暴露**的缺陷：
它只用 `shasum` 算引擎文件的 SHA-256。`shasum` 是 Perl 脚本，精简 CI 镜像
不保证有；`sha256sum` 来自 GNU coreutils，同一台 runner 上已被别的门禁证明存在。
实测对照：只有 `sha256sum` 时，修复前 `exit 2`（`cannot checksum`），修复后 `exit 0`。

### Root Cause

1. **「脚本写完了」被当成了「脚本有效」。** 门禁脚本的失败模式不是报错，而是
   *报绿*或*报错*——两种都需要真跑一次才能知道。
2. **CI 是最贵的地方，不该是第一次试运行的地方。** 在 CI 上第一次跑一个未验证的
   门禁，等于用一小时的 runner 时间去换一个本可以在本地秒级得到的结论。
3. **假设工具一定存在。** 本机有 `shasum`（macOS 自带）、CI 镜像有 `sha256sum`
   （coreutils），但脚本只写了其中一个。开发机和 CI 的工具集是不同的两套。

### Action

1. 用**桩 `cargo`**（只看引擎文件内容决定返回 0 还是 101）在一次性假仓库里跑
   脚本的机械逻辑 —— 不加载 fixture、不执行任何 TPC-H 用例，秒级完成。
   三个退出码路径全部实测：0（全捕获）/ 1（有存活）/ 2（harness 错误）。
2. 哈希工具改用**回退链**（`shasum` → `sha256sum` → 空串并报错），
   与 `scripts/gate/prepare_tpch_fixture.sh` 同款。
3. 证据文档的假指针改成事实陈述：机械逻辑已实测，端到端首跑在 CI。
4. 端到端仍未实测的部分单独列一节说明，不藏在别处。

### Rule

> **门禁脚本的第一次执行不应该是 CI。** 交付一个从未运行过的门禁脚本，等于
> 交付一个「不知道会不会红」的红灯。写完之后必须先在**一次性沙箱**里把它的
> 机械逻辑跑通（用桩替掉昂贵的那一半），再把它接进 CI。
> 另外：**不要假设一个工具在开发机和 CI 上都存在** —— 用回退链，或者先查证。

---

## Lesson-16: 一个回归守卫，必须先证明它**有判别力**

### Observation

`tests/integration/sql/g13_oltp1_concurrent_select_test.rs` 是 G13-OLTP-1
「SELECT 取读锁」的回归守卫。它在空闲机器上约 **50% 概率失败**（实测 0.75 / 0.67 / 1.00），
而且——更糟——**有没有那个 bug 它都差不多这样**：

| 状态 | 吞吐比 | 3 次里挂的次数 |
|---|---|---|
| 无 bug | 0.67 / 0.72 / 0.72 | 1 |
| 有 bug（把读锁变回写锁） | 0.67 / 0.83 / 0.83 | 1 |

两个分布**完全重叠**。这个测试在它自己的目标环境（Z6G4）上大概是有判别力的，
但在本机（macmini）上 —— 也就是它文件头注里写明的「macmini (M4) was unaffected」——
它既误报又测不出东西。

同时它还有一处**代码与自己的文档不一致**：文件头与步骤 4 的注释共 5 处都写
「thread B hammers with `SELECT 1`」，代码却传了同一条重查询。
重探针会烧 CPU，于是「无回归」基线被拉到 ≈ 0.72，正骑在 0.7 阈值上。

### Root Cause

1. **守卫的「存在」被当成了守卫的「有效」。** 写完就进仓库，没人验证过
   「把被守的 bug 变回去，它会不会红」。
2. **阈值是猜的。** 0.7 从来没有一次测量支撑过，于是它既没有判别余量，
   也对机器负载高度敏感。
3. **文档与代码不一致时，注释会保护错误的行为**（因为读注释的人以为代码在做注释说的事）。

### Action

1. 用**变异**把被守的 bug 精确变回去（一行：`engine.read()` → `engine.write()`），
   实测「有 bug / 无 bug」两个分布 —— 分布重叠即判别力为零。
2. 区分了**可修**与**不可在本机修**：
   - 测量方式（窗口计数 → 固定查询数 + 计时）是纯改进，任何机器上都成立；
   - 但判别力依赖一台能复现该 bug 的机器，本机没有 → **不在本机重新标定阈值**，
     而是如实披露 + 立项（#5398），并写明验收标准。
3. 把「文档与代码不一致」当作一条独立的检查项，而不是顺手改注释了事。

### Rule

> **一个回归守卫，必须先证明它有判别力。** 判别力有两个方向：
> **无缺陷时不能误报**（否则它是随机噪声），**有缺陷时必须报错**（否则它是摆设）。
> 验证方法是变异：把被守的行为改回去，看它会不会红。
> 分布重叠 = 没有判别力 = 这个守卫比没有还糟，因为它把流水线变成随机数。
> 另外：**注释是规格**。当注释和代码不一致时，不要默认代码是对的 ——
> 先查清哪一个才是当初的意图，再决定改哪边。

---

## Lesson-17: 「门禁是红的」这句话，本身需要证据

### Observation

本 PR 修好基线 clippy 红之后，`postcheck` job 第一次有机会真实执行。
合并前在本地预跑它的每一步，发现其中 **5 个红灯点**（5 个独立断口）：

- `check_correctness_tests_production_storage.sh`（R2 / R4）
- `check_storage_db_scope_coverage.sh`（9 处 wrapper×方法 缺失覆盖）
- `check_coverage.sh`（2026-08-10 起的弃用桩，CI 步骤仍在调它 → exit 2）
- `check_evidence_binding.sh v3.7.0`（74 项证据绑定违规）
- `auto_env_blocker.sh`（`set -euo pipefail` 下遇无 H1 的 .md 中止；副作用改写 56 个受控文件）

第一反应是「本 PR 弄坏的」。查证之后发现两者**与本 PR 无关**：

1. 在基线 commit `d8365606d69c` 上建临时 worktree，跑同样两个脚本 ——
   **输出逐字相同**（同样 R2/R4 失败、同样 9 条 FAIL）；
2. 根因各自定位到具体 commit：
   - R2/R4 grep 的是**旧构造器拼写**（`WalStorage::new_with_sync_mode_and_checkpoint`、
     `FileStorage::new_with_wal`），被 `654e579032`（fix #5317）整体移除 →
     **假阳性**：生产路径仍 file-backed、仍带 WAL，但门禁看不见了；
   - db-scope 门禁自动派生 trait 里所有 `db: &str` 方法，#5192 关闭之后
     trait 又新增了 `get_view` / `list_views` / `scan_via_bufferpool_in_db`
     （`d8de15178b`、`a0875a7499`），包装层没跟上 → **门禁漂移**。
   - 另外 3 个是**接线层**断口：调用的脚本已被弃用、目标版本目录的证据绑定
     违规、以及一个会在 `set -e` 下中途死掉并改写 56 个受控文件的脚本。

再往上一层查，发现了更根本的事实：**`postcheck` job 在 600 次历史 CI run 中
从未成功执行过一次**（190 次 `skipped`、7 次 `cancelled`、0 次 `success`）——
它声明 `needs: [test]`，而 `test` 自 2026-10-14 起恒红，于是
「一个红灯藏住了另一个红灯」。

### Root Cause

- **「门禁红」有两种完全不同的成因**：本 PR 引入的，和基线就有的。
  两者在 CI 输出里长得一模一样（都是 `FAIL` + exit 1），**不查基线就无法区分**。
- 而「不查基线」很容易做到 —— 直接改门禁让它变绿最快。
  这正是本轮差点犯的错误。

### Action

1. **改动前先建立基线对照**：在基线 commit 上建临时 worktree，跑同一个脚本，
   比对输出是否逐字相同。相同 → 与本次变更无关，走归属而不是修复。
2. **归属到具体 commit**：`git log -S "<旧字面量>" -- <file>` 定位到引入漂移的那个
   commit，红灯的成因才有名字（#5400 / #5401 / #5403）。
3. **登记而不吸收**：另立 issue（#5400 / #5401 / #5402 / #5403），在证据文档与 PR 正文里
   如实披露，并写明「门禁变红不是本 PR 的债，弱化门禁去消掉它才是」。
4. **顺藤摸瓜查上游**：发现红灯不是本 PR 引入之后，追问「它为什么能存活这么久」，
   查到 `postcheck` 从未执行过（#5402）—— 这才是真正值得修的东西。

### Rule

> **「门禁是红的」这句话，本身需要证据。** 判断一个红灯是否由本次变更引入，
> 唯一站得住的方法是**在基线上跑同一个脚本，比对输出**；「大概不是吧」不是证据。
>
> 推论一：**永远不要为了让 CI 变绿而改门禁的判定口径**。门禁变红不是你的债，
> 弱化门禁去消掉它才是新的债。
>
> 推论二：**一个从未执行过的门禁等于不存在**。统计 job 结论时要看
> `skipped` / `cancelled` / `success` 的**全量分布**，而不是看最近一次是什么 ——
> 「最近一次是 skipped」和「600 次里 0 次 success」是两个完全不同的结论。
>
> 推论三：**基线对照要留档**。临时 worktree 跑完就删，等于什么都没证明；
> 必须把「同一个脚本、基线与分支、输出逐字相同」写进证据文档。

---

## Lesson-18: 「这个改动不属于本 PR」是会被 CI 证伪的判断

**上下文**：迭代 4 的第 12 个 commit 判定「把 `four_way_harness` 的逐行 INSERT 换成批量装载」
属于本 PR 范围外（它改动一个既有的、被 4 个目标共用的辅助模块），登记为独立 issue #5399
另做。证据文档 §7.14 当时就是这么写的。

**它错在哪**：那个成本是**本 PR 自己造出来的**。

- fixture 悬空时，依赖它的目标 `path.exists()` 为假 → **跳过装载** → 毫秒级通过，断言还是空的；
- 本 PR 让 fixture 可达 → 这些目标第一次真正干活 → 逐行 INSERT 的成本才第一次显形。

于是「把降本推到 PR 之外」= 「让本 PR 承担一个只有它自己会触发的成本」。
CI run 7971 给出了判决：`_q15_q16_check` **> 75 分钟未结束**，`timeout 9000` 砍断整个 job，
`check_test_target_execution.sh` 随即把全部未执行目标列为缺失。

**推论一：范围判断要按「成本由谁引入」划，不按「代码改起来大不大」划。**
「改动会波及 4 个目标」是**影响面**的论据；「这个成本是本 PR 引入的」是**归属**的论据。
归属优先 —— 归属错了，影响面再小也是错的。

**推论二：CI 第一次让某段代码真正跑起来时，它的历史成本会一次性显形。**
这不是「新问题」，是**旧问题被激活**。所以修法不该是「把激活源退回不可达」，
而是「把被激活的那段路径改到可承受」——后者还得证明等价。

**推论三：等价性可以先在文本层证明，不必等运行期。**
两条装载路径在 8 张表、86 482 行上逐格比对类型化值 → 0 处不一致。
这比「我本地跑过一次看着对」强得多，也比「在内存受限的机器上硬跑」更诚实：
**能证明的那部分要证明，不能证明的那部分要写明**（运行期未验证 → 如实披露为未验证）。

**推论四：范围判断被证伪时，改文档的顺序是「先改事实，再改结论」。**
§7.14 里那句「不在本 PR 内做」如果不改，后面 §7.16 的降本实现就与自己的证据自相矛盾 ——
而**文档与自身内容不一致，比缺一段文档更糟**。

---

## Lesson-19: 「CI 变红了」是一句需要证据的话 —— 它可能**从来没绿过**

**上下文**：迭代 4 的第 13 个 commit 之前，所有文档都写「`test` job 自 2026-10-14 起恒红」。
这句话听起来像常识，直到我把**全部 400 次** CI run 的 job 逐个列举出来统计：

| `test` 的结论 | 次数 |
|---|---|
| skipped / cancelled / failure | 181 / 10 / 6 |
| **success** | **0** |

`test` —— 承载全部 446 个测试目标的 job —— **一次都没有成功过**。
181 次 `skipped` 意味着大多数 run 里它**根本没执行**。
所以「变红了」这个说法隐含的前提（曾经是绿的）**没有任何证据支撑**。

**推论一：门禁的历史结论要看分布，不能看「最近一次」。**
「最近一次是红的」和「它从来没绿过」是两个完全不同的结论，对应的处置也完全不同：
前者是回归，后者是**从未建立过的基线**。Lesson-17 已经在 `postcheck` 上吃过一次亏，
这次在 `test` 上又验证了一遍 —— 所以这条要变成习惯：**任何「CI 之前是好的」的说法，
都要用全量分布去核对**。

**推论二：`fail-fast` 不只是让日志好看，它让「门禁是红的」这句话本身失真。**
基线提交在 CI 上那次只执行了 **71 / 446** 个目标 —— fail-fast 在第 71 个目标的
TPC-H panic 处就中止了。于是：
- 你看到的是「第 71 个目标炸了」，而不是「另外 375 个目标从没跑过」；
- 被 fail-fast 掩盖的既有失败**不会出现在任何一次 CI 日志里**，
  直到有人让全量执行发生为止（本 PR 就是那个「让它发生」的人）。

**推论三：让门禁真跑之后暴露的失败，要区分「本 PR 引入的」和「原先被掩盖的」——
判据是同一条断言在基线上的输出，不是感觉。**
本 PR 让 446 个目标真跑后，`e2e_canonical_subprocess` 的 14 个测试炸了：
`canonical binary not found`。在基线提交的**本地**全量运行里，同一个断言、同一行号 68、
同一句提示**逐字相同**地炸了 → **既有失败**。而根因是 **cargo 语义**：
在**有根 package 的工作区**里 `cargo build` **只构建根 package**，成员包的二进制
从来不会产出；而根 package 连 `src/main.rs` 都没有。
一个「CI 从来没绿过」的项目里，**默认假设必须是「有东西没接上」**，而不是「代码坏了」。

**推论四：修既有断口要修「接线」，不要修「判定口径」。**
`e2e_canonical_subprocess` 自己就写明了该怎么做 ——「run `cargo build -p
sqlrustgo-mysql-server` first」。CI 照做即可。**门禁脚本的失败要用「补上缺失的前置条件」
来修，不要用「把判定放宽」来修** —— 后者会把断口永久藏起来。

**推论五：「跑过了」不等于「验证了」。**
`differential_*` 在 CI 上报 `36 passed; finished in 0.00s` —— **0.00 秒 = 一次引擎比对
都没做**：引擎二进制路径硬编码成了 macOS 路径，在 Linux 容器里找不到，测试就静默降级为
「什么都不做」。它比一个红着的测试更危险，因为它是**绿的**。
凡是「跑完了但耗时 0.00s」「断言数为 0」「结果是默认值」的现象，都要当成**假绿**去查。

---

## Lesson-20: 把期望值写死在源码里 = 把装载器的缺陷也写成了规格

**上下文**：`tpch_q6_q19_oracle_test.rs` 一直绿着。它的装载器用位置列名插入：

```text
INSERT INTO lineitem (c0, c1, ..., c15) VALUES (...)
```

表里没有 `c0` 这列。引擎**不报错**，而是插入一行全 NULL（实测：
`INSERT INTO t (c0, c1) VALUES (1, 2)` 在 `(a, b)` 表上产出 `NULL | NULL`）。
于是 62 000 行全 NULL 进了表，所有过滤条件匹配不到任何行，
而断言写的是：

```text
assert_eq!(parsed.unwrap(), 0, "Q19 count on simplified SF=0.001 fixture should be 0 (per SQLite), got {}", s);
```

这个 `0` 是"简化版 SF=0.001 手搭数据集"上的值 —— 那份数据集本仓库从未包含。
它既不是预言机的输出，也不是数据无关的不变量；**它是"装载器坏了"的化石**。

**根因**：断言的**期望值**和**被断言的对象**来自两份不同的世界。
期望值来自一个已经消失的数据集 + 一个坏掉的装载器的观测结果；
断言的对象来自一份新的数据集 + 一个（当时也坏的）装载器。
两边碰巧一致，于是绿灯。

**行动**：
1. 期望值改由**独立预言机**给出（同 fixture 的进程内 SQLite）—— 这正是该文件
   自己的文件头承诺过的设计，只是从未实现；
2. 数据无关的不变量（聚合无 `GROUP BY` 恰好 1 行、单元格是有限数）**单独**保留，
   不依赖任何数据集；
3. 装载器修好（真实列名 + 批量装载），并把 `part` 的 DDL 从 7 列补齐到标准 9 列。

**规则**：

> **一个断言的期望值必须能被指认出它的来源：独立预言机，或者数据无关不变量。
> 如果一个数字是"我记得在某个数据集上它是这个值"，那它不是期望值，是化石。**
>
> 特别地：**当被断言的数据装载路径本身可疑时，先修装载路径，再问期望值对不对。**
> 顺序反了，你会把 bug 的症状写进规格，然后守住它。

**副产物**：这条规则在本次迭代里救回了 `tpch_q6_q19_oracle_test` 的假绿，
也暴露了引擎侧的一个真缺陷（未知 INSERT 列名不报错 → 归 #5409）。

---

## Lesson-21: `timeout … | tee` 杀不掉挂死的子进程 —— 超时不是"设了就生效"

**上下文**：CI run 8031 的 `test` job 卡在 `harness_validation_test`
（05:59:01Z 之后日志再无输出）。`cargo test` 外层套了 `timeout 9000`，
按理说 2.5 h 后会被砍断。实测：

| 观测点 | 事实 |
|---|---|
| cargo test 起点 | ~04:58Z ⇒ 9000 s 到期 ≈ 07:28Z |
| 08:45Z 的 job 状态 | 仍是 `in_progress` |
| 08:45Z 的日志字节数 | 281 854 B —— 与 06:35Z **完全相同** |
| 最后一次人工介入 | 手动 cancel 才结束 |

**根因**：写法是

```bash
timeout 9000 cargo test --all-features --no-fail-fast 2>&1 | tee test_report.log
```

`timeout` 把信号发给 **cargo 所在的进程组**。`tee` 在这个进程组**之外**，
它持有 step 的 stdout 管道；即使挂死的测试二进制被连带杀掉，
`tee` 仍在等一个不会关闭的管道 ⇒ **step 永不结束**，runner 一直占着容器。
"超时有上限"这句话在这里是**假的**：上限只约束了 cargo，不约束这个 step。

**行动**：把重定向放进 `timeout`，去掉管道。

```bash
timeout -k 30 9000 cargo test --all-features --no-fail-fast > test_report.log 2>&1
status=$?
tail -n 200 test_report.log || true
exit ${status}
```

`timeout`（非 `--foreground`）向子进程的**整个进程组**发信号，所以挂死的测试
二进制也会被杀；管道去掉后 `step` 随即返回。后续 summary / accounting 门禁
本来就直接读 `test_report.log`，不受影响；`tail` 保留最后 200 行到 job 日志
（超时时那正是"卡住的那个目标"）。

**规则**：

> **给一条可能挂死的命令设超时，要验证"超时之后这个 step 真的结束了"，
> 而不是"超时命令存在"。**
>
> 具体到 shell：**管道会让超时失效**，因为管道两端的进程不在同一个进程组里。
> 需要"既能实时看到输出、又能被砍断"时，把管道放进被超时的那一侧，
> 并在超时之后用 `tail` 把尾部补回日志。

**副产物**：这条规则让"CI 卡住"从"要人工介入才能救"变成"9000 s 后自动收场"，
下一次再出现挂死的测试，job 会带着"卡住的是哪个目标"的证据自己结束。

---

## Lesson-22: 桩测试自己也会骗你 —— "测过了"必须先断言"桩生效了"

**上下文**：本轮用桩 `cargo` 验证变异门禁 `scripts/gate/mutate_tpch_oracle_tests.sh`
的退出码路径。上一版证据写的是「`exit 0 / exit 1 / exit 2` 三条路径全部实测」。
本轮复核发现：**exit 2 那一条是夸大的** —— 超时路径**从未真正触发**。

**根因（两层，两层都是"失败被静默吞掉"）**：

1. **桩没装上**。桩 `timeout` 的安装条件写成"仅当文件不存在时才创建"，
   而 PATH 桩循环早就把 homebrew 的**真** `timeout` 软链进了桩目录 ⇒
   强制超时的桩**从未安装** ⇒ 那次跑的其实是正常路径。
   更隐蔽的一层：安装失败桩时用了
   `cp fail.mktemp "$STUBBIN/mktemp"` —— 这会**穿过**指向 `/usr/bin/mktemp`
   的软链去写**系统二进制**；macOS SIP 拒绝写入，`cp` 失败，而桩脚本**没有
   `set -e`** ⇒ 失败桩**从未生效**，而 CASE 3 却"通过"了。
2. **桩的删除动作会挂起**。本机 agent shell 里的 `rm` 是**可恢复删除 shim**
   （转发到 Finder 的 `osascript`），在**非交互 shell** 里**会挂起** ——
   整个 harness 卡死在 `osascript -e tell application "Finder" to delete …`，
   输出文件 0 字节。

**行动**：

- 桩一律装在**独立目录**并**前置到 `PATH`**，**不覆盖、不删除**任何已有文件；
- 每个用例**先断言桩确实生效**（例如 `PATH="$TIMEBIN:$STUBBIN" timeout 1 true`
  必须返回 124），**再**执行被测断言；
- harness **零删除动作**：一次性目录用 `$$` 后缀，天然不与上一轮冲突；
- 被测脚本侧：新增 `make_temp_file()`，用裸 `mktemp`（本仓库 20+ 处的既有用法），
  **拿不到路径就大声失败**，绝不用空路径继续（这正是 CI 上真实发生的缺陷形态：
  `mktemp -t x.XXXXXX.py` 在 runner 镜像上失败 → 空路径 → 三个变异片段**从未生成**
  → 三个变异**从未被应用** ⇒ 门禁 `ERRORS=3` / exit 2 / job 变红）。

**规则**：

> **一个"测试"如果不能自证"我的桩生效了"，它测的就是环境，不是被测物。**
>
> 具体到桩测试：
> - 桩**不覆盖**已有文件（`cp` 会写穿软链），**不删除**任何东西（`rm` 可能是会挂起的
>   可恢复删除 shim）；
> - 每个用例**先断言桩生效**，再断言被测行为；
> - 桩脚本报错**不等于**桩生效 —— 必须**回读**它的实际行为。
>
> 推论：**当"结论对但过程假"时，证据必须降级**。上一版的"exit 0/1/2 全部实测"
> 是在结论上碰巧正确、过程上完全没发生；本轮把它改成"exit 2 当时是夸大的"
> 并重测，而不是把表格留着蒙混过去。

---

## Lesson-23: 恒绿与恒红是**同一种病** —— 门禁不再对代码的真实状态作出区分

**上下文**：本轮做 GA 门禁取证（治理模板 `AGENT_PROMPT_TEMPLATE.md:135` 的
`[ ] GA：G1-G16 PASS`）。为了得到真实结论，逐维度单独执行了 `check_rc_ga_gate.sh`
与 `check_full_gate_verification.sh` 背后的每个脚本，并与基线 `d8365606d69c` 对照。

**观察**（实测，不是推断）：

| 方向 | 门禁 | 症状 |
|---|---|---|
| **恒绿** | `check_full_gate_verification.sh:69` | `output=$(bash "$script" 2>&1 \|\| true)` ⇒ `code` 恒为 0 ⇒ **8 个维度无论实际怎样都判 PASS**（#5417） |
| **恒绿** | `check_rc_ga_gate.sh:963` | `local GATE="${1:-all}"` 让 `--skip-a5` 占据维度选择器 ⇒ 7 个维度被静默跳过，汇总显示 0/0，却报 "All dimensions validated"（#5418） |
| **恒绿** | `check_rc_ga_gate.sh:507` | `for t in ${D6_INTEGRATION_TESTS}` 在 bash 3.2 只展开第 1 个元素 ⇒ D6a **实际执行 0 个测试**却报 `✓ PASS 0/0`；即使在 bash 5 上路径规则也只解析得到 4/51（#5419） |
| **恒绿** | `check_cross_version_debt.sh:430/487` | `UNKNOWN_COUNT`/`OPEN_COUNT` 从未初始化 ⇒ 「❌ FAIL (UNKNOWN items)」**永久不可达**（#5422） |
| **恒红** | `legacy/check_alpha.sh:20` | `--quiet` 位置错 ⇒ rustc 收到 `Unrecognized option: 'quiet'` ⇒ 编译必然失败（#5416） |
| **恒红** | `check_rc_ga_gate.sh:141`（A2） | 无 `--all-features` ⇒ `fault_injection.rs` 的 `reach` 重复定义 ⇒ E0428 ⇒ 编译失败（#5420） |
| **恒红** | `check_rc_ga_gate.sh:337`（B6） | 要求**仓库未跟踪**且**门禁自己不产出**的 `target/*.json` ⇒ fresh checkout 必缺（#5421） |

**根因**：这些门禁都**没有把"被检查对象的真实状态"传导到退出码**。
恒绿是因为退出码被吞掉或判定条件永远为假；恒红是因为判定条件与代码状态无关（恒定的编译错误、恒缺的产物）。

**行动**：
- 取证**不采信聚合器结论**，逐维度单独执行、各取真实退出码；
- 每个 FAIL 都做**基线对照**（同命令、同基线提交），只有"输出逐字相同"才判"基线既有"；
- 每个失效点**单独立 issue**，写清位置、症状、方向（恒绿/恒红/随机）与复现命令。

**规则**：

> **恒绿与恒红是同一种病：门禁不再对代码的真实状态作出区分。**
>
> 判据不是"它是绿的还是红的"，而是：**改一处代码，这个门禁的结论会不会变？**
> 不会变 ⇒ 它没有在工作，无论报什么颜色。

---

## Lesson-24: 「取证」与「勾选 Gate」是两件事 —— 实现者不得批准自己的 Gate

**上下文**：治理模板把 GA 的成功标准写成 `[ ] GA：G1-G16 PASS`。
本轮做完了 GA 取证（逐维度实测 + 基线对照 + 8 个失效点立项），那么能不能把这一项勾上？

**不能。** 三条理由：

1. **模板硬约束**：`AGENT_PROMPT_TEMPLATE.md:35`「**不得自批 Gate：实现者不得批准自己的 Gate**」。
2. **清单项与定义文档在仓库内就对不齐**：
   - `GATE_CONDITIONS.md` 的 GA Gate 小节只定义了 **G1–G6**（另加 G17/G18/G19），
     而模板写的是 **G1–G16**；
   - `docs/governance/META_GATE_AUDIT_2026-06.md:245` 写明「16 个里**只有 6 个**有真实脚本
     （G1、G11、G12、G13、G14、G16），G11–G14 是"基础设施就绪"但从未运行」；
   - `docs/governance/adr/ADR-007-truthfulness-recovery-sequence.md:202` 把
     "G1-G16 PASS as a single claim" 评为 🔴 **NO (overstated)**。
3. **实测本身也不支持"全绿"**：D1 与 D2 各有一个 blocker，D6 在本机不可验证。

**行动**：把交付物定义成**取证与立案**，而不是勾选：
- 逐维度表格（结论 + 基线对照 + 归属）；
- 失效点 issue；
- 明确写「本 PR 不勾选 GA 清单项，批准权在独立评审 / CI」。

**规则**：

> **一个迭代能交付的是"门禁的真实状态"，不是"门禁被批准"。**
> 把「我有证据」和「我有权批准」分开 —— 前者是自己该做的，后者不是。
>
> 推论：门禁红/绿与 PR 是否可合并是**两个问题**；
> 正确做法是"把红的归属钉死 + 逐项披露"，而不是"把红的消掉"。

---

## Lesson-25: `${ARRAY}` 未加引号，在 bash 3.2 下只展开第 1 个元素

**上下文**：`scripts/gate/check_rc_ga_gate.sh:507` 遍历 51 个集成测试目标：

```bash
for t in ${D6_INTEGRATION_TESTS}; do   # 数组未加引号
```

本机 `/bin/bash` 是 **3.2.57**（macOS 自带）。实测：

```
$ /bin/bash -c 'A=("a" "b" "c"); for t in ${A}; do echo "item=$t"; done'
item=a
```

在 **bash 4+** 中 `${ARRAY}` 展开为全部元素，在 **bash 3.2** 中等价于 `${ARRAY[0]}`。
⇒ 循环体只跑一次；第 0 项恰好路径不匹配 ⇒ `continue` ⇒ `D6_TOTAL=0`
⇒ 门禁打印 **`✓ PASS D6 summary: 0/0 integration tests PASS`**。

讽刺的是，脚本第 505 行的注释自称 *"Bash 3.2 compatible"* ——
**兼容性声明与实现相反**，而没有任何测试会发现它。

（第二层问题与此独立：即使在 bash 5 的 CI 上，路径规则 `tests/<name>.rs` 也只解析得到 **4/51**，
其余 47 个文件都在 `tests/integration/...` 等子目录 ⇒ 覆盖率约 8%。）

**行动**：取证时不改仓库脚本，用**临时副本**只改两处（强制 `GATE="all"`；按本机约束剔除 2 个 TPC-H 目标），
副本跑完移出仓库；失效点立 issue（#5419）。

**规则**：

> **门禁脚本里凡是"按名字收集一批目标"的写法，都必须在两种 bash 上验证过。**
> `${ARRAY}` 与 `"${ARRAY[@]}"` 在 bash 3.2 与 4+ 之间是**语义差异**，不是风格差异。
>
> 推论：**"门禁在本机跑了并报 PASS"这句话本身需要证据** ——
> 要同时回答「跑了多少个目标」（0 个也叫 PASS）和「判据覆盖了哪些目标」（4/51 还是 51/51）。

---

## Lesson-26: 进程级环境变量不能当测试夹具 —— 它让 CI 的红灯变成随机数

**上下文**：GA 取证时，D5-10 维度报 "Recent test failures: 1"。基线对照时同样复现：

| 位置 | 次数 | 失败次数 |
|---|---|---|
| PR 分支 `c7ac66ee24` | 5 | **2** |
| 基线 `d8365606d69c` | 3 | **1** |
| PR 分支（`--test-threads=4`） | 6 | 0 |

失败用例固定为 `execution_engine_tests::test_v410_atomic_parallel_degree_getter_clamps_to_one`
（`left: 4 / right: 1`）。

**根因**：`ExecutionEngine::base_with`（`src/execution_engine.rs:349`）**每次构造引擎都读进程级**
`SQLRUSTGO_EXECUTOR_PARALLELISM`；而 `src/execution_engine_tests.rs` 的多个用例
（`:559`/`:618` 的 `set_var`，`:561`/`:586`/`:592`/`:620`/`:659` 的 `remove_var`）
**并发改写同一个进程的环境表**。Rust 测试默认多线程 ⇒ 读取方偶尔读到别的用例刚写进去的 4。
Rust 2024 起 `env::set_var` 已是 `unsafe`，其文档明确写着在多线程程序中调用是未定义行为。

**影响**：CI 的 `test` job 执行 `cargo test --all-features --no-fail-fast`，
根包 lib 测试就在其中 ⇒ **该 job 存在约 1/3 量级的随机红灯**。

**行动**：立项 #5423（**本 PR 不改**：范围是 TPC-H fixture 自包含，不是并发测试卫生）；
CI 判读一律用「同命令复跑 + 基线对照」甄别。

**规则**：

> **进程级状态（env、全局原子、全局池）不能当测试夹具。**
> 测试夹具必须只影响被测实例本身。
>
> 推论：**当一个 CI job 有随机红灯时，"这次红了"和"这次绿了"都是不可靠的信号** ——
> 在把任何 CI 红灯归因给某次变更之前，先问："它复现吗？基线也这样吗？"

---

## Lesson-27: 同一门禁的阈值与清单，必须只有**一处**权威来源

**上下文**：本轮把 GA 阶段的 20 项 `required_gates` 逐项实跑（证据 §7.25.7），
发现三处"同一个事实写在多个地方、且彼此不一致"的实例。

**观察**：

1. **阈值三套**（`check_anti_ignore_gate.sh`）：

   | 出处 | active 上限 | total_allowed 上限 |
   |---|---|---|
   | 脚本文件头注释（`:3`） | 55 | 100 |
   | 实际生效的变量（`:51-52`） | **80** | **150** |
   | `GATE_CONDITIONS.md:255`（G19 表格） | 47 | 96 |

   实测 `active=69, total_allowed=116` ⇒ **按代码 PASS，按脚本自己的注释 FAIL，按治理文档也 FAIL**。

2. **`--help` 与 dispatch 不一致**（`check_rc_ga_gate.sh`）：
   usage 写 `ga — D1 + D2 + D3 + D4 + D5 + RC-to-GA checklist`，
   而 dispatch 里 D1 只在 `all|alpha` 跑、D2 只在 `all|beta` 跑 ⇒ **`ga` 实际不跑 D1/D2**。

3. **CI 调度条件锚在旧版本线**（`ci.yml:308`）：
   `if: contains(github.ref,'develop/v3.12.0') || contains(github.ref,'ci/v3.12.0-')`
   ⇒ 当前 v4.1.0 版本线上，GA 的 Security 门禁 **100% skipped**（脚本本身本机实测 exit 0）。

**根因**：同一个事实被**分别硬编码**在「注释 / 代码 / 治理文档 / CI 条件」四处，
没有任何一处声明"哪一处是权威的"，也没有任何测试检查它们是否一致。

**行动**：本轮全部**归属 + 立项**，不改（#5424 / #5425；`ga` 模式的三处披露缺陷记录在证据 §7.25.7(b)）。

**规则**：

> **一个判据只能有一个来源。**
>
> - 阈值写在**代码**里，文档引用代码，不重新写一遍数字；
> - 文档里的数字如果必须存在，就让**测试**去比对文档与代码是否一致；
> - `--help` / usage 文本是**面向用户的接口**，它与实现不符比实现有 bug 更糟 ——
>   因为人会照着它去理解"这个门禁到底跑了什么"。
> - CI 的触发条件是**门禁的一部分**。"脚本在这里，但条件永远不成立"
>   等价于"门禁不存在"，而且比不存在更坏：**它看起来存在**。
>
> 推论：**"门禁清单"本身也是需要被验证的对象。**
> 一份清单如果与实现脱节，它会把"没跑"包装成"跑过了"。

---

## Lesson-28: 「失败」要分成三种：代码红、环境阻塞、**基础设施红**

run 8153 的 `test` 与 `mutation` job 都是 `failure`，但它们**一行项目代码都没执行** ——
两个 job 都死在 `Checkout` 步骤的 `git clone https://github.com/actions/checkout`
（`unexpected EOF` / `dial tcp 20.205.243.166:443: i/o timeout`）。

CI 的 job 结论只有三个取值：`success` / `failure` / `skipped`。
**基础设施失败与代码失败都落在 `failure` 上，API 层面无法区分。**

⇒ 后果是**误导归因**：一个正确的 commit 可能因为 GitHub 出口抖动而被当成代码回归去回滚。

**判据**：拿到一个 `failure`，先看**它跑了几行项目代码**。
日志里出现 `error: test failed` 或 `test result:` 才说明代码跑过了；
只有 `🏁 Job failed` 而没有任何测试输出 = 基础设施失败。

**可复用检查**：
```bash
# 从 job 日志里找 Checkout 阶段的网络错误
grep -E "git clone 'https://github.com/actions/checkout'|unexpected EOF|i/o timeout" job.log
```

---

## Lesson-29: 一个只 `println!` 的测试，报 "ok" 时不代表它执行过任何东西

本轮把 `tpch_full_test` 的 5 个零断言查询测试改成
「服务不可达 ⇒ NOT-RUN；可达但查询失败 ⇒ panic」之后，本地立刻**变红**：

```
PostgreSQL Q2 on a reachable server failed: ERROR: relation "part" does not exist
```

追下去是两层：

1. `setup_tpch_tables()` 的存在性探针**只问 `lineitem` 一张表**，
   命中就跳过整套建表 ⇒ **半初始化的 schema 永远不会被修复**；
2. 多个 postgres 用例**并行**调用 setup，各自 `DELETE` + `INSERT` 同一批表 ⇒ 主键冲突。

修法：探针核对**全部 8 张表**；建表脚本首尾加 **PostgreSQL advisory lock**
（`pg_advisory_lock(5379001)` / `pg_advisory_unlock(5379001)`）⇒ 并行调用被数据库层串行化。

**教训**：**假绿 → 真红**的转换本身就是成果 ——
一个只打印结果的测试，它报告 "ok" 时可能**一个查询都没发出去**。
把「环境阻塞 ⇒ NOT-RUN」与「能跑 ⇒ 报错就红」两条规则同时落下去，假绿才会消失。

**判据**：把被测对象换成任何东西，这个测试还会 "ok" 吗？⇒ 会 ⇒ 它是假绿。

---

## Lesson-30: 「发布流程」也是代码 —— 门禁的判据同样要满足「改一处代码，结论会不会变」

本轮把 Lesson-23（恒绿与恒红是同一种病）的判据**用到了发布链路本身**，立刻查出 5 类问题：

- **清单不唯一**：「GA 有多少门禁」在同一个仓库里有 **20 / 16 / 9 / 6** 四个答案，
  「G1~G6 是什么」有 **3 套互斥语义**；
- **执行点不存在**：`STAGE_CONFIG.yaml` 自称 SSOT，但其唯一执行器
  `scripts/gate/check_stage.sh` **从不被任何 CI job 调用**（工作流里出现的两处都是注释）；
- **判据恒绿**：`check_architecture_freeze.sh:125` 与 `:130` **两个分支都是 `exit 0`**；
- **CI 判读不可靠**：RC/GA 步骤无 pipefail（恒绿），而兜底 grep 用 `❌`(U+274C)、
  门禁实际输出 `✗`(U+2717) ⇒ **一次 D3 硬失败可以让 postcheck 全绿**；
- **证据链不成立**：`check_evidence_binding.sh` 只扫**已存在**的文件 ⇒
  **缺失的 GA 产物对门禁完全不可见**。

**方法论**：审计发布流程时，对每一个「PASS」的来源问三句 ——
① 它的判据在哪一行？② 改一处代码，结论会不会变？③ 它的结论能不能被独立复现？

**可复用检查**：
```bash
# ① 权威清单到底有几项
grep -A25 'GA:' docs/governance/STAGE_CONFIG.yaml | sed -n '/required_gates/,/optional_gates/p'
# ② SSOT 的执行器有没有被调用
grep -rn "check_stage.sh" .gitea/workflows/ | grep -v '^\s*#'
# ③ 失败标记的字符集是否一致（脚本 vs CI 的 grep）
grep -n 'FAIL' scripts/gate/check_rc_ga_gate.sh | head -3
grep -n "grep -c '❌ FAIL" .gitea/workflows/ci.yml
```

---

## Lesson-31: 独立的盲审要**自己复核**它的每一条结论

本轮派了一个**只读 verifier 子代理**独立审阅 GA 发布流程（不预先告知本会话的结论）。
它的 9 组补充发现**全部正确**，但其中一条**更正了本会话此前的说法**：

> `docs/governance/AGENT_PROMPT_TEMPLATE.md` **在本分支上并不存在**。

复核确认：该文件只存在于 commit `9fbb63bfbc`（在 `fix/4597-chaos-t8d-awk-portability` 分支上），
**不是本分支的祖先**。

⇒ 两条纪律：

1. **盲审的结论要逐条自己跑一遍命令再采纳**（本轮 9 组补充发现全部经复核后才写进证据与 issue）；
2. **引用治理文档的硬约束时，要确认那份文档在当前分支上存在** ——
   否则「按第 35 行执行」这句话本身就没有可执行的落点。

**可复用检查**：
```bash
git cat-file -e HEAD:docs/governance/AGENT_PROMPT_TEMPLATE.md && echo 存在 || echo 不存在
git merge-base --is-ancestor <引入该文件的 commit> HEAD && echo 在本分支线上 || echo 不在
```

---

## 附：本轮形成的可复用检查

| 工具 | 作用 | 复现命令 |
|---|---|---|
| `scripts/gate/prepare_tpch_fixture.sh` | 运行时生成确定性 TPC-H fixture | `bash scripts/gate/prepare_tpch_fixture.sh --out /tmp/fx --sf 0.01 --seed 42` |
| `scripts/gate/check_test_target_execution.sh` | 声明目标必须全部执行 | `bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log test_report.log` |
| `scripts/gate/check_tpch_known_differences.sh` | 已知差异必须有 issue + owner，且不得无声增长 | `bash scripts/gate/check_tpch_known_differences.sh` |
| `scripts/gate/mutate_tpch_oracle_tests.sh` | 证明新断言能杀引擎缺陷 | `bash scripts/gate/mutate_tpch_oracle_tests.sh` |
| 桩 `cargo` 机械逻辑实测 | 不加载 fixture 就验证门禁脚本的退出码与还原逻辑 | 见证据 §7.12（一次性假仓库 + 桩 `PATH`） |
| **桩生效自检** | 断言桩**真的**在生效，否则"测过了"是误会 | 装桩后先跑一次"桩的预期行为"（如 `PATH=… timeout 1 true` 必须返回 124），再跑被测断言（见 Lesson-22） |
| **零删除 harness** | 避免 `rm`（本机是可恢复删除 shim，非交互 shell 里会挂起） | 一次性目录用 `$$` 后缀；桩装在独立目录并前置 `PATH`，不覆盖也不删除 |
| 基线对照 worktree | 判断「门禁红」是否由本次变更引入 | `git worktree add --detach /tmp/wt-base <base-sha>` → 同一脚本跑基线与分支 → 比对输出是否逐字相同（见证据 §7.15） |
| CI job 结论全量统计 | 区分「最近一次是什么」与「历史上成功过几次」 | 遍历全部 run 的 jobs，统计 `success` / `skipped` / `cancelled` 的分布（见证据 §7.15） |
| 成员包二进制存在性检查 | 区分「测试失败」与「CI 根本没搭好前置条件」 | `cargo build -p sqlrustgo-mysql-server -p sqlrustgo-cli` → 检查 `target/debug/` 下是否产出（见证据 §7.17(c)） |
| 0.00s 通过体检 | 识别「假绿」：测试报通过但什么都没验证 | 看每个目标的 `finished in`，耗时 0.00s 且断言数为 0 的都要查（见证据 §7.17(d)） |
| **GA required_gates 逐项实跑** | 拿到 GA 阶段的**权威**门禁清单并逐项执行，而不是只看聚合器的结论 | 解析 `docs/governance/STAGE_CONFIG.yaml` 的 `stages.GA.required_gates` → 每个脚本单独跑、各取真实退出码（见证据 §7.25.7） |
| **版本线失联体检** | 找出"门禁锚在早已不是当前版本的版本线上" | 逐个检查门禁脚本里的版本字面量、CI job 的 `if:` 分支条件、门禁默认的 `VERSION` 回退值，与当前版本线（v4.1.0）比对（见证据 §7.25.8） |
| **门禁阈值一致性检查** | 找出同一个阈值的多个副本 | 对每个门禁，比对「脚本注释 / 代码常量 / `GATE_CONDITIONS.md` 表格」三处数字，并按每处各判一次（见证据 §7.25.7、#5425） |

这九个工具对**任何**"测试需要外部数据"的场景都适用，不限于 TPC-H；
其中后两个（基线对照 worktree、CI job 结论全量统计）对**任何**门禁归因都适用。

---

*本文件由 mavis (MiniMax Code) 编写于迭代 4（issue #5379）。所有结论均附可复现命令与真实输出；
未实测的判断一律标注为"待验证"。*

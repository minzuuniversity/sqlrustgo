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

## 附：本轮形成的可复用检查

| 工具 | 作用 | 复现命令 |
|---|---|---|
| `scripts/gate/prepare_tpch_fixture.sh` | 运行时生成确定性 TPC-H fixture | `bash scripts/gate/prepare_tpch_fixture.sh --out /tmp/fx --sf 0.01 --seed 42` |
| `scripts/gate/check_test_target_execution.sh` | 声明目标必须全部执行 | `bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log test_report.log` |
| `scripts/gate/check_tpch_known_differences.sh` | 已知差异必须有 issue + owner，且不得无声增长 | `bash scripts/gate/check_tpch_known_differences.sh` |
| `scripts/gate/mutate_tpch_oracle_tests.sh` | 证明新断言能杀引擎缺陷 | `bash scripts/gate/mutate_tpch_oracle_tests.sh` |
| 桩 `cargo` 机械逻辑实测 | 不加载 fixture 就验证门禁脚本的退出码与还原逻辑 | 见证据 §7.12（一次性假仓库 + 桩 `PATH`） |

四个工具对**任何**"测试需要外部数据"的场景都适用，不限于 TPC-H。

---

*本文件由 mavis (MiniMax Code) 编写于迭代 4（issue #5379）。所有结论均附可复现命令与真实输出；
未实测的判断一律标注为"待验证"。*

# v312-58 / Issue #4374–#4381 — 难度评估与整改路线图

**Branch**: `fix/v312-58-tpch-sf1-7x`
**Date**: 2026-08-23
**Author**: openclaw
**Scope**: V312-58 RC/GA 强制 in-v3.12.0 解决,expiry 2026-09-30,无 v3.13 deferral
**Verdict**: 7 子项可分 3 个 sprint 全部收口；无需新架构,只需把 optimizer 中既有的
`PredicatePushdown` / `decorrelate` / `join_reorder` 真正串入执行路径,并补 EXTRACT 解析器/Eval。

---

## 1. 总览

| Issue | Q | sqlrustgo | SQLite | 差距 | 根因类别 | 难度 | Sprint |
|-------|---|----------|--------|------|----------|------|--------|
| **#4374** | 父 umbrella | n/a | n/a | 7 子项协调 | 流程/状态管理 | Medium | S4 |
| **#4375** | Q2 | 15,628 | 20 | 782× | 谓词未下推 + 笛卡尔积先 LIMIT 后过滤 | **Low–Medium** | **S1** |
| **#4376** | Q7 | 175 | 7 | 25× | EXTRACT(YEAR FROM ...) 函数未下推/未评估 | **Low** | **S1** |
| **#4377** | Q11 | 200,000 | 29,636 | 6.75× | `n_name` 谓词未下推到 nation 表;join 顺序错 | **Medium** | **S2** |
| **#4378** | Q12 | 7 | 2 | 3.5× | `l_shipmode IN` + date 谓词残差未下推 | **Medium** | **S2** |
| **#4379** | Q17 | TIMEOUT | 1 | >16,500× | 6-way join 全 NL + 相关子查询未去相关 | **High** | **S3** |
| **#4380** | Q20 | TIMEOUT | 172 | >16,500× | EXISTS 子查询未改写为 semi-join | **High** | **S3** |
| **#4381** | Q22 | TIMEOUT | 7 | >16,500× | NOT EXISTS / AVG 相关子查询未去相关 | **High** | **S3** |

**核心发现**：
- `crates/optimizer/` 中 `PredicatePushdown`、`decorrelate`、`join_reorder`、`unified_cost` 模块代码已存在(200–612 LOC)
- `src/execution_engine.rs` 已通过 `use sqlrustgo_optimizer::rules::{BinaryOperator, Expr}` 和
  `use sqlrustgo_optimizer::unified_cost::UnifiedCostModel` 接入优化器
- 但 `q2_5way_comma_limit_regression.rs` 等 TPC-H 回归测试用例挂在
  `ExecutionEngine::new(storage)` 这条路径上,15,628 行输出 = 优化器规则**未真正生效**
- Q7 EXTRACT 解析层 + Eval 层压根缺；非优化器问题

---

## 2. 逐 issue 难度评级 + 解决路径

### 2.1 #4374 — 父 umbrella (V312-58 总收口)

- **难度**：Medium(纯流程/状态管理)
- **根因**：7 子 issue (#4375–#4381) 全部 open,expiry 2026-09-30 强制 in-v3.12
- **解决路径**：
  1. Sprint 1–3 完成所有 6 个子项(参见下文路线图)
  2. 4-way cross-engine 一致性验证(sqlite/postgres/mysql/sqlrustgo)
  3. 收集 sha256 oracle 校验(已记录在 issue body 中)
  4. **不在子 issue 里手动 close 父 issue**;父 issue 由 develop 分支 merge 后由 v3.12.0 release commit 自动关闭
  5. 父 issue 关闭时必须做: `MYSQL_COMPAT_STATUS.md` 更新 + `docs/releases/v3.12.0/RELEASE_NOTES.md` 更新 + README line 52 同步
- **风险**：
  - 高 — 任何 1 个子项失败都会阻塞父 issue 关闭 → 阻塞 v3.12 GA
  - 必须按依赖顺序串行(Q11 谓词下推 / Q12 谓词下推 / Q17/20/22 去相关均依赖 optimizer 真正接入)
- **里程碑**：
  - 7 子项 4 状态机 `pending → in_progress → fixed-merged → closed`
  - 父 issue 在所有子项关闭后由 release commit 触发关闭

---

### 2.2 #4375 — Q2 LIMIT 被忽略(15,628 vs 20)

- **难度**：**Low–Medium**
- **症状**：`ExecutionEngine::new` 跑 q2.sql SF=1,返回 15,628 行而非 20 行
- **根因诊断**(基于 `q2_5way_comma_limit_regression.rs` §1-3 注释):
  1. comma-join fast path 走了 hash-chain 优化
  2. hash-chain 优化**未应用** `PredicatePushdown`(`p_size=15`、`p_type LIKE '%BRASS'`、`r_name='EUROPE'`)
  3. 完整笛卡尔积先生成,后置 WHERE filter 被绕过
  4. LIMIT 20 实际生效但限制的是"已经过大的全笛卡尔积"的前 20 行 → 错行数 + 错内容
- **解决路径**:
  1. **最小修改**:`crates/planner/src/optimizer.rs::PredicatePushdown::apply_rule` 已存在,需检查其是否真在 `src/execution_engine.rs` 的 `execute_select` 路径调用
  2. 在 `ExecutionEngine::execute_select` 内,**hash-chain 路径前**先 `optimizer.optimize(&mut plan)`,确保 base-table 谓词 (`p_size=15` 等) 已被推到扫描侧
  3. 同步确保 `LIKE '%BRASS'` 这类 pattern 被识别为选择性 filter(SELECTIVITY_LIKE_PREFIX 在 `stats.rs` 已有)
- **参考证据**: `q2_5way_comma_limit_regression.rs:1-24` 已有完整 reproduction + 根因描述
- **测试**: 已存在 `q2_canonical_5way_comma_limit` `#[ignore]` 用例,fix 后 opt-in 跑通即收口
- **预估工时**: 1–2 天(查清楚 + 修 + 跑 q2 regression)

---

### 2.3 #4376 — Q7 EXTRACT 谓词(175 vs 7)

- **难度**：**Low**(非优化器问题,纯解析/Eval 缺口)
- **症状**: `WHERE extract(year from l_shipdate) = 1995 AND extract(year from l_shipdate) = 1996` 几乎无效,返回 175 行(接近 lineitem 内全部 year 笛卡尔积)
- **根因**(已记录在 `v312-58-issue-4376-root-cause.md` 内存文件):
  1. `crates/executor/src/expr/mod.rs:1452` 已有 `[Q7_TRACE] EXTRACT field=... source=...` 调试输出,说明 EXTRACT 函数已被部分支持
  2. 但**谓词下推路径未识别** EXTRACT 函数表达式为可下推条件
  3. GROUP BY `EXTRACT(YEAR FROM o_orderdate)` 应有 7 个 `(n1.n_name, n2.n_name, year)` 组合;实际坍缩为 2 个 → GROUP BY 不区分 EXTRACT 表达式与基列
- **解决路径**:
  1. **解析器层**(如果缺): `crates/parser/` 增加 `EXTRACT(<field> FROM <expr>)` token 化与 AST 节点(看是否已有,如无则补)
  2. **Eval 层**: `crates/executor/src/expr/mod.rs` 已有 EXTRACT 处理,补 year/month/day 等标准 field
  3. **GROUP BY 表达式比较**: `src/engine_select.rs` 的 GROUP BY hash key 计算必须使用 EXTRACT 表达式求值后的值,而不是列引用本身的指针
  4. **谓词下推**: 在 optimizer 中加入 `IsExtractPredicate` 识别,把 `EXTRACT(year FROM col) = <const>` 下推到 scan filter(选择性极高时)
- **参考证据**:
  - `evidence/v312-58/issue-4376-root-cause.md`(已存在,但需扩充 group_by 表达式比较修复)
  - `evidence/v312-58/engine-select-q7trace-gate-fix.md`(PR #4408 已修 PTY 死锁,便于调试)
- **测试**: 已存在 `diag_q7_extract.rs`(目前 `#[ignore]`)+ `q2_5way_comma_limit_regression.rs` 风格新增 `q7_extract_year_regression.rs`
- **预估工时**: 2–3 天(纯解析/Eval,无优化器改造)

---

### 2.4 #4377 — Q11 over-count(200,000 vs 29,636)

- **难度**：Medium
- **症状**: 3-way join(partsupp × supplier × nation)+ `n_name='<NATION>'` + `HAVING SUM(...) > threshold` + ORDER BY value DESC 取 top N(基于 subquery),返回 200,000 行
- **根因诊断**:
  1. `n_name='GERMANY'` 谓词未下推到 nation 表(200,000 行的 partsupp 全量参与 join)
  2. join 顺序错(partsupp 200K × supplier 10K × nation 25)→ 应为 (nation 1 × supplier 10K × partsupp 200K) 走 selective 路径
  3. HAVING 子句中 `SUM(ps_supplycost*ps_availqty) > X` 与外层 subquery 比较,可能未识别为可下推 filter
- **解决路径**:
  1. 确保 `PredicatePushdown` 规则识别 `n_name = '<const>'` 并下推到 nation 表 scan
  2. 确保 `join_reorder` 选择小表驱动(基于 `unified_cost::UnifiedCostModel` 选择 nation 先 join)
  3. HAVING 子句需要在 GROUP BY 后 evaluate(目前应该 OK,但需验证不被下推吞掉)
- **测试**: 新增 `tests/integration/oracle/q11_stock_filter_regression.rs`
- **预估工时**: 3–5 天(需要既下推又 reorder,可能在 join_reorder 里有 bug)

---

### 2.5 #4378 — Q12 over-count(7 vs 2)

- **难度**：Medium
- **症状**: 2-way join(orders × lineitem)+ `l_shipmode IN ('MAIL','SHIP')` + 4 个日期谓词 + CASE WHEN GROUP BY l_shipmode,返回 7 行而非 2 行
- **根因诊断**:
  1. `l_shipmode IN (...)` 选择性高(40M 行 lineitem 中只有 MAIL+SHIP ≈ 50%),未下推
  2. 4 个日期谓词(`l_commitdate<l_receiptdate`、`l_shipdate<l_commitdate`、`l_receiptdate>='1994-01-01'`、`l_receiptdate<'1995-01-01'`)中第 1 个是**列间比较**,不是简单下推
  3. 7 行的成因:lineitem 中可能有 7 个 shipmode 实际参与了 group(类似 shipmode 字符串大小写/trim 异常),或谓词残差导致把不在 IN 列表的 mode 也拉进来了
- **解决路径**:
  1. `PredicatePushdown` 规则识别 IN 列表为选择性 filter,下推到 lineitem scan
  2. 列间谓词(`l_commitdate<l_receiptdate`)在 join 后 filter,不下推;但要保证不被错误吞掉
  3. 日期谓词应该走 `BETWEEN '1994-01-01' AND '1994-12-31'` 的识别(等价于 `>= AND <`)
- **测试**: 新增 `q12_shipping_mode_regression.rs`
- **预估工时**: 3–5 天(同 Q11,谓词下推 + 谓词残差排查)

---

### 2.6 #4379 — Q17 small-order-shortage(TIMEOUT)

- **难度**：High
- **症状**: 6-way join (lineitem × part × partsupp × orders × customer × nation) + 相关子查询 `l_quantity < (SELECT 0.2 * AVG(l_quantity) FROM lineitem WHERE l_partkey = p_partkey)` → >1800s
- **根因诊断**:
  1. nested loop 全表扫描(lineitem 6M × part 200K × ... = 不可承受)
  2. **相关子查询逐行执行**:对每个外层 partkey 重新全扫 lineitem 6M 行算 AVG
  3. 没 cost-based planner,固定走 nested loop
  4. 没 hash join(等值连接)
- **解决路径**:
  1. **去相关**:`crates/optimizer/src/decorrelate.rs` 已存在(612 LOC),需接入执行路径;把 `WHERE l_partkey = p_partkey AND ...` 改写为 GROUP BY l_partkey 一次算 AVG,再 semi-join
  2. **Hash Join**:对 `l_partkey = p_partkey`、`s_suppkey = ps_suppkey` 等值连接用 hash join(代码可能在 executor,但可能未 wire)
  3. **索引驱动**:对 part 上的 `p_brand='Brand#23' AND p_container='LG CASE'` 用 BINT(如果有)
- **测试**: 新增 `q17_small_order_shortage_perf.rs`,SF=1 ≤300s 必须达成
- **预估工时**: 5–10 天(decorrelate + hash join 双轨改造)

---

### 2.7 #4380 — Q20 potential-part-promotion(TIMEOUT)

- **难度**：High
- **症状**: 5-way join (supplier × nation × part × partsupp × lineitem) + 双层相关 EXISTS + 子查询 `0.5 * SUM(l_quantity)`,>1800s
- **根因诊断**:
  1. **EXISTS 子查询未改写为 semi-join**:每个 supplier 都重跑一遍 lineitem
  2. 双层嵌套:`supplier → EXISTS(SELECT 1 FROM partsupp WHERE ...) → AND ps_availqty > (SELECT 0.5 * SUM(l_quantity) FROM lineitem WHERE ...)`
  3. nested loop,无 hash join,无 cost-based 优化
- **解决路径**:
  1. **EXISTS → semi-join 改写**:把 `WHERE EXISTS (SELECT 1 FROM partsupp WHERE ps_suppkey = s_suppkey ...)` 改写为 `partsupp SEMI-JOIN supplier ON ps_suppkey = s_suppkey`
  2. **嵌套 subquery 扁平化**:第二层 `0.5 * SUM(l_quantity)` 改写为 `SUM(l_quantity) * 0.5` 标量子查询,然后 join
  3. **part name LIKE 'forest%'** 应识别为 prefix LIKE,可下推
- **测试**: 新增 `q20_potential_part_promotion_perf.rs`,SF=1 ≤300s
- **预估工时**: 5–10 天(同 Q17)

---

### 2.8 #4381 — Q22 global-sales-opportunity(TIMEOUT)

- **难度**：High
- **症状**: customer × orders (NOT EXISTS) + customer (AVG 相关子查询) + GROUP BY cntrycode,>1800s
- **根因诊断**:
  1. **NOT EXISTS 未改写为 anti-semi-join**:每个 customer 都跑 orders 全表
  2. **`> (SELECT AVG(c_acctbal) FROM customer WHERE ...)` 相关子查询**:对每个 c_custkey 重算 AVG
  3. customer 1.5M 行 + orders 6M 行 → 笛卡尔积级
- **解决路径**:
  1. **去相关**:`decorrelate.rs` 已存在,需支持 AVG 聚合的去相关
  2. **NOT EXISTS → anti-semi-join**:同 EXISTS 改写,但用 ANTI
  3. **HAVING 中聚合先计算**:对 `cntrycode` GROUP BY 后做 HAVING
- **测试**: 新增 `q22_global_sales_opportunity_perf.rs`,SF=1 ≤300s
- **预估工时**: 5–10 天(同 Q17/Q20)

---

## 3. 推荐 Sprint 路线图

### Sprint 1 — 最小修复(Easy Wins, 1 周)
**目标**:关闭 #4375、#4376

| Day | 任务 | Issue | 验收 |
|-----|------|-------|------|
| 1–2 | 调研 `src/execution_engine.rs::execute_select` 是否真调用 `optimizer.optimize(...)`;若否,接入 `sqlrustgo_optimizer::PredicatePushdown` | #4375 准备 | 代码改动已 commit |
| 3 | 跑 `q2_5way_comma_limit_regression.rs` (--include-ignored) 验证 row_count == 20 | #4375 | 测试通过 + sha256 匹配 |
| 4 | 在 `crates/parser/` + `crates/executor/src/expr/mod.rs` 补 EXTRACT year/month/day eval | #4376 准备 | 代码改动已 commit |
| 5 | 修复 `src/engine_select.rs` GROUP BY hash key 使 EXTRACT 表达式独立成键 | #4376 | mini fixture 1→1 行通过 |
| 6–7 | 跑 SF=0.1 + SF=1 q7.sql,验证 row_count == 7 | #4376 | 测试通过 + sha256 匹配 |

**Sprint 1 退出条件**: #4375、#4376 全部 PR merged + 父 issue 注释更新。

### Sprint 2 — 谓词下推(Shared Root Cause, 1.5 周)
**目标**:关闭 #4377、#4378

| Day | 任务 | Issue | 验收 |
|-----|------|-------|------|
| 1–2 | 排查 `PredicatePushdown::apply_rule` 是否识别 `n_name = '...'` 并下推到 nation scan | #4377 准备 | 代码改动 |
| 3 | 跑 q11 SF=1,row_count 接近 29,636 | #4377 | sha256 匹配 |
| 4–5 | 排查 `l_shipmode IN (...)` 下推;排查 4 个日期谓词中"列间比较"的处理 | #4378 准备 | 代码改动 |
| 6–7 | 排查 GROUP BY l_shipmode 在残差谓词下的产出 | #4378 | row_count == 2 |
| 8–9 | 复测 Q11/Q12 SF=1 双向,无回归 | 双向 | |
| 10 | 同步 PR + 4-way cross-engine 验证 | 双向 | |

**Sprint 2 退出条件**: #4377、#4378 全部 PR merged。

### Sprint 3 — 优化器全面接入(High Difficulty, 2–3 周)
**目标**:关闭 #4379、#4380、#4381

| Day | 任务 | Issue | 验收 |
|-----|------|-------|------|
| 1–3 | 把 `crates/optimizer/src/decorrelate.rs` 接入执行路径(测试 Q22 mini fixture) | #4381 准备 | mini fixture 通过 |
| 4–6 | 实现 NOT EXISTS → anti-semi-join 改写 | #4381 推进 | Q22 row_count == 7 |
| 7–9 | 实现 EXISTS → semi-join 改写 + 嵌套 subquery 扁平化 | #4380 准备 | Q20 mini fixture 通过 |
| 10–12 | Q20 全 SF=1 跑通 | #4380 | row_count == 172 + ≤300s |
| 13–17 | Q17 相关子查询去相关 + 6-way join hash join 化 | #4379 | row_count == 1 + ≤300s |
| 18–20 | 双向回归 + perf 验收 | 3 issue | |

**Sprint 3 退出条件**: #4379、#4380、#4381 全部 PR merged + SF=1 ≤300s 验证通过。

### Sprint 4 — 父 issue 收口(1 周)
**目标**:关闭 #4374

| Day | 任务 | 验收 |
|-----|------|------|
| 1–2 | 4-way cross-engine 一致性验证(sqlite/postgres/mysql/sqlrustgo)对全部 7 个 Q | 7× 验证报告 |
| 3 | 汇总 evidence/tpch/V312-58-Q*-VERIFICATION.md(每个 Q 一份) | 7 文件 |
| 4 | 更新 MYSQL_COMPAT_STATUS.md + RELEASE_NOTES.md + README | 3 文件 diff |
| 5 | 父 issue 加 cross-engine 验证报告 + sha256 匹配 | comment |
| 6–7 | develop 合并 + release commit 触发父 issue 自动关闭 | #4374 closed |

**Sprint 4 退出条件**: 父 issue #4374 closed;v3.12 GA 不被 #4374/#4375-#4381 阻塞。

---

## 4. 风险与依赖

### 4.1 跨 Sprint 风险

| 风险 | 影响 | 缓解 |
|------|------|------|
| optimizer 接入执行路径后破坏现有 696 个 executor 测试 | 高 | Sprint 1 收尾跑 `cargo test -p sqlrustgo-executor --lib` 必须 0 fail |
| decorrelate 改造引入新 bug | 高 | 用 `diag_q22_minimal.rs` 风格 mini fixture 隔离验证 |
| hash join 改造导致 OOM | 中 | Sprint 3 强制加 per-query memory cap + sf=0.1 先验 |
| expiry 2026-09-30 逼近 | 中 | 每个 Sprint 必须有可发布 demo;不可"全做完才合并" |

### 4.2 Sprint 间依赖

```
Sprint 1 (S1: #4375 #4376)
    ↓ 谓词下推框架就绪
Sprint 2 (S2: #4377 #4378)
    ↓ optimizer 完整接入
Sprint 3 (S3: #4379 #4380 #4381)
    ↓ 全 7 子项 fix-merged
Sprint 4 (S4: #4374 父 issue 关闭)
```

任何 Sprint 失败必须:1) 不进入下一 Sprint;2) 评估是否需要增加 Sprint。

---

## 5. 工时汇总

| Sprint | Issues | 工时 | 风险等级 |
|--------|--------|------|---------|
| S1 | #4375 #4376 | 5–7 天 | Low |
| S2 | #4377 #4378 | 7–10 天 | Medium |
| S3 | #4379 #4380 #4381 | 15–30 天 | High |
| S4 | #4374 | 5–7 天 | Low |
| **总计** | **8 issues** | **32–54 天**(≈ 6.5–11 周) | Mixed |

按 expiry 2026-09-30(距今 5 周),**S1 + S2 必须 ≤ 2 周完成**,否则 v3.12 GA 阻塞。
若 S3 进度滞后,**风险**:
- 选项 A: 允许 #4379/#4380/#4381 中 1 个 defer 到 v3.13(违反父 issue #4374 "不允许 defer" 条款)
- 选项 B: v3.12.0 GA 推迟到 2026-10-31 后,给 S3 留足时间
- 选项 C: 接受 v3.12.0 GA 时 7 个子项中 3 个仍 TIMEOUT,降低 v3.12.0 兼容性等级(政治敏感)

---

## 6. 关联文件 + 既有证据

- 父 umbrella 修复:**#4408** (commit 2fbdf3f0c3f8) — Q7_TRACE gate 已修复(PTY 死锁)
- 父 umbrella 修复审计:`evidence/v312-58/q-test-hang-risk-audit.md`
- Q7 EXTRACT 根因:`evidence/v312-58/issue-4376-root-cause.md`
- Q7 PTY 死锁 fix:`evidence/v312-58/engine-select-q7trace-gate-fix.md`
- 优化器代码:`crates/optimizer/src/{decorrelate.rs, rules.rs, join_reorder.rs, join_cost_model.rs}`
- 优化器接入点:`src/execution_engine.rs:28-32`(`use sqlrustgo_optimizer::{rules, stats, unified_cost}`)
- 优化器 trait 定义:`crates/planner/src/optimizer.rs` + `crates/optimizer/src/lib.rs`
- 既有 Q regression:`tests/integration/oracle/{q2_5way_comma_limit_regression, q8_8way_date_range_regression, q16_notin_subquery_regression}.rs`
- 既有 diag_q*:`tests/integration/oracle/diag_q{6,11,12,14,22,7}*.rs`

---

## 7. 验证清单(Sprint 退出必做)

每 Sprint 退出前必须:

1. [ ] `cargo build --all-features` exit 0
2. [ ] `cargo clippy --all-features -- -D warnings` exit 0
3. [ ] `cargo test -p sqlrustgo-executor --lib` exit 0(696 tests)
4. [ ] `cargo test --all-features` — 全部 pass,无回归
5. [ ] 本 Sprint issue 关联的回归测试 opt-in 跑通
6. [ ] SF=1 4-way cross-engine 一致性(本 Sprint 新修复的 Q)
7. [ ] sha256 oracle 匹配
8. [ ] evidence doc 写齐,含 provenance + 命令 + 输出
9. [ ] Gitea PR 已合并到 develop/v3.12.0
10. [ ] Gitea issue 已 close + comment 引用 PR

---

## 8. 推荐的 Issue 关闭策略

按 Round-20/24/25/26/27 一贯做法:**单一 PR auto-close + 手动 close 父 issue**

- 子 issue (#4375–#4381):每个 Sprint 一个或多个 PR,PR body 含 `Closes #4375` 等 → auto-close
- 父 issue (#4374):所有子 issue close 后**手动 close**(comment 引用全部子 issue close time + sha256)
- 父 issue **不依赖** develop merge 自动关闭(因为其本质是 "7 子项 4 状态机 + 4-way 一致性" 的协调,不是一个 PR 的代码改动)

---

**总结**:7 子 issue 在已有 optimizer 代码基础上 + EXTRACT 解析/Eval 修补,**全部可在 v3.12 GA deadline (2026-09-30) 前收口**,前提是 S1+S2 必须 2 周内完成,S3 必须从 day 1 并行启动 decorrelate + semi-join 改造。
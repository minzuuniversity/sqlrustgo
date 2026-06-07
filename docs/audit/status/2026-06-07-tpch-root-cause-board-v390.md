# TPC-H Root Cause Board (v3.9.0)

> **目标**: 停止按 Query 管理 Issue，改按 Root Cause 收敛
>
> **Date**: 2026-06-07 | **Branch**: develop/v3.9.0
>
> **上游**: Sprint 1 (Failure Matrix) → Sprint 1.5 (Cell Diff) → Sprint 2 (Subsystem Classification) → Sprint 3 (Operator Regression Suite) → Sprint 4 (Fix-by-Operator)

---

## 一、根因归类表

| Root Cause  | TPC-H Query 受影响 | Sprint 3 失败 | 主修复入口 | 关联 Issue |
|-------------|--------------------|---------------|------------|------------|
| **Aggregate** | Q01, Q05, Q07, Q08, Q17 | `aggregate.rs` 7/9 fail (含 sum_real/sum_empty) | `src/engine_select.rs:599` (Sum 聚合) | #3276, #3278 |
| **Multi-Join** | Q03, Q10, Q18 | `join.rs` 3/4 fail (3-table chain = 0 rows) | `src/engine_select.rs:750+` (execute_joins) | #3277 |
| **Correlated Subquery (EXISTS)** | Q20, Q21 | `exists.rs` 0/4 fail | `src/engine_select.rs` (subquery executor) | #3248 |
| **Date Filter** | Q06, Q19 | (无独立测试) | `src/expr_utils.rs` (date comparison) | (待办 Issue) |
| **Corrupt Fixture** | Q14 (p_type='Type 1' not 'PROMO*') | (N/A — 数据问题) | `~/sqlrustgo-tpch/data/` | #3256 |
| **CHAR(N) Trim** | Q01, Q05, Q07, Q08, Q17 (cosmetic) | 已修 PR #3296 | (merged) | (已合) |
| **SUM(empty) NULL** | Q14 (语义差异) | 已修 PR #3288 | (merged) | (已合) |

**说明**:
- 22 Query → 5 Root Cause 收敛率 4.4×
- 旧式 12+ 个 P0/P1 Issue 全部归入上述 5 类
- **不应再按 Query 新建 Issue**（如"Q06 错"），而应按 Operator 新建（如"Date Filter 错"）

---

## 二、Sprint 3 状态（Operator Regression Suite）

> **本套件是基础设施**，用于：
> 1. 把 22 Query 问题降维到 5 Operator 问题
> 2. 修 bug 时防止 A 好 B 坏
> 3. Sprint 5 Cell Diff 重跑前快速验证

### 测试套件位置

```
tests/operators/
├── aggregate.rs   (9 tests: COUNT/SUM/AVG/MIN/MAX)
├── join.rs        (4 tests: 2-table, 3-table, alias, WHERE-in-ON)
├── exists.rs      (4 tests: basic, correlated, multi-row, NOT EXISTS)
└── (待) corpus/   (20+ cases per operator — Sprint 3.3 目标)
```

### 当前结果

| 套件 | PASS | FAIL | 通过率 | 关键 bug |
|------|------|------|--------|----------|
| `aggregate.rs` | 7 | 2 | 78% | `sum_real_column_returns_real` (返回 Integer 301)，`sum_empty_table_returns_zero_not_null` (返回 NULL) |
| `join.rs` | 3 | 1 | 75% | `diag_3_table_chain` (3-table 返回 0 行) |
| `exists.rs` | 0 | 4 | 0% | **整个 correlated subquery 路径未工作** |
| **总计** | **10** | **7** | **59%** | (Sprint 3.1+ 目标 100%) |

---

## 三、Sprint 3.x 路线图

> 严格按 chatGPT P4 建议：**先修 Join (有 minimal repro)，不先修 EXISTS**

### Sprint 3.0 — Root Cause Board 建立 ✅ (本会话)
- 写本文件
- 7 个失败测试按 root cause 重新归类
- AGENTS.md / Issue 模板同步"不要按 Query 管"

### Sprint 3.1 — 修 Aggregate 剩余 2 测试
- `sum_real_column_returns_real` — Sum(REAL) 应返回 Float 而非 Integer
- `sum_empty_table_returns_zero_not_null` — 空表 SUM 应返回 0 而非 NULL
- 目标: aggregate.rs 9/9 pass
- **预计 1-2 天**

### Sprint 3.2 — 修 Multi-Join (#3277)
- `diag_3_table_chain` 修复: 3-table comma-list 应返回 1 行非 0 行
- 调查路径: `execute_joins` → `extra_tables` 扩展逻辑
- 目标: join.rs 4/4 pass
- **预计 2-3 天**

### Sprint 3.3 — 扩大 Join Regression Suite
- 添加: LEFT JOIN, NULL JOIN KEY, DUPLICATE KEY, 4-TABLE CHAIN
- 目标: join.rs 20+ cases
- 同步存在到 corpus/ 子目录

### Sprint 4 — 修 EXISTS (#3248)
- 4/4 EXISTS operator tests 修复
- 涉及 Binder + Optimizer + Executor 三层（更复杂）
- 预计 3-5 天

### Sprint 5 — Cell Diff 重跑
- 跑 `tests/four_way_cell_diff_test.rs`
- 目标: 17 cell_diff → 0 (或全部已记录)
- 同时跑 4-way TPC-H
- 预计 < 1 天

### Gate-C — Correctness Gate (v3.9.0 新增)
- `scripts/gate/check_tpch_correctness.sh`
- 要求:
  - 0 row_count_mismatch
  - 0 undocumented cell_mismatch
- 与 Gate-D9 并列

---

## 四、治理原则（强制）

### ✅ 正确做法
```text
新建 Issue: "Aggregate operator: SUM(REAL) type confusion"
新建 Issue: "Multi-Join operator: 3-table comma-list returns 0 rows"
新建 Issue: "EXISTS correlated: 0/4 operator tests pass"
```

### ❌ 错误做法
```text
新建 Issue: "Q03 错误"      ← 这是 query-level，不收
新建 Issue: "TPC-H 失败"    ← 太宽泛，无 actionable
新建 Issue: "PR #3276 后续"  ← PR 已合，不应再开
```

### Issue 模板（建议）
```markdown
## Operator: [Aggregate/Multi-Join/EXISTS/Date/Fixture]
## Symptom
- 失败测试: tests/operators/xxx.rs::test_name
- 期望: ...
- 实际: ...
## Minimal Repro
```sql
SELECT ...
FROM ...
WHERE ...;
```
## Root Cause (if known)
- 文件: src/xxx.rs:line
- 函数: fn_xxx
## 关联 Sprint
- Sprint 3.1 / 3.2 / 3.3 / 4 / 5
```

---

## 五、未决决策

1. **Q14 是 Fixture 还是代码 bug？**
   - chatGPT 倾向: Fixture (#3256)
   - 当前: 已作为 #3256 duplicate 处理
   - **决定**: 维持 #3256 处理，等待 canonical TPC-H 数据替换

2. **PR #3296 (CHAR trim) 是否需要 backport 到 develop/v3.8.0？**
   - 当前: 已在 develop/v3.9.0
   - **决定**: 仅 v3.9.0，不 backport（v3.8.0 是收敛版本）

3. **Sprint 3.3 扩大 Join Suite 时机：修完 3.2 立即扩，还是 Sprint 5 前统一扩？**
   - **建议**: 修完 3.2 立即扩（chatGPT P2）

---

## 变更日志

| 日期 | 改动 | 提交 |
|------|------|------|
| 2026-06-07 | 初版 (Sprint 3.0 启动) | (TBD) |

---

## Sprint 3 完成报告 (2026-06-07)

### 最终结果
- **aggregate.rs**: 7/9 → **9/9** pass (Sprint 3.1)
  - 修复: `sum_real_column_returns_real` 改用 type check 而非 string contains
  - 修复: `sum_empty_table_returns_zero_not_null` 改名为 `_returns_null_per_sql_standard`
  - **结论**: 生产代码完全正确, 失败的 2 个测试是测试预期错
- **join.rs**: 3/4 → **20/20** pass (Sprint 3.2 + 3.3)
  - Sprint 3.2 修复: 3-table comma-list 现在返回 1 行
  - **根因**: `find_join_key_index` 在 chained join 时用 bare `lookup_column` 拿到首个匹配 (`a.id` 而非 `b.id`)
  - **修复**: 加 `lookup_qualified_column` 用 qualifier 精确匹配
  - Sprint 3.3 扩 suite: 4 → 20 cases (LEFT JOIN, NULL KEY, DUP KEY, 4-TABLE 等)
- **exists.rs**: 0/4 pass (Sprint 4 待办, 不在本会话)
  - Pre-existing 限制, 与 Sprint 3.2 修复无关

### Sprint 3.2 修复详情 (核心)
- **文件**: `src/engine_select.rs`
- **函数**: `find_join_key_index` (3-table chained join 的 column resolution)
- **问题**: `lookup_column` 剥最后一段返回首个匹配 → 3-table ON `b.id = c.b_id` 实际变成 `a.id = c.b_id` (1 vs 10) → 0 行
- **修复**: 加 `lookup_qualified_column(info, qualifier, col_name)` 用 qualifier 精确匹配 (`b.id` vs `a.id`), 然后才 fallback 到 bare lookup
- **影响**: TPC-H Q03/Q10/Q18 等多表查询的 row_count 预期会显著提升 (待 Sprint 5 验证)

### 已知 Sprint 3 限制
- `o.amount` (qualified alias projection) 在 JOIN 后返回字面字符串. 表是 `c.c.id, c.c.name, o.o.id, o.o.cust_id, o.o.amount` (double prefix). `eval_identifier` 找不到精确 match → fallback `Value::Text("o.amount")`. **已记录为单独 Issue**, 不在 Sprint 3 范围
- Self-join `emp e, emp m WHERE e.mgr_id = m.id` (comma-list 形式) 返回 self-matches. 显式 JOIN 形式工作正常. Parser comma-list auto-rewrite 在 alias 情况下不产生 JoinClause. **已记录为单独 Issue**

### Total Sprint 3
- 测试: 17 → 33 (新增 16)
- 通过: 10 → 29 (88%)
- 失败: 7 → 4 (从 41% 通过率提升到 88%)

### 关联提交
- Sprint 3 PR #3234: 初始 17 unit tests (合并 d28551cbf822)
- Sprint 3.1: aggregate 测试预期修正
- Sprint 3.2: Multi-Join 修复 (3-table chain) - 待合并
- Sprint 3.3: Join suite 扩展 4→20 - 待合并

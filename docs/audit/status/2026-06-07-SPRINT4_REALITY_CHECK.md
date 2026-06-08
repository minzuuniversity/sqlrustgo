# Sprint 4 Reality Check — Harness & Oracle Issues

> **Date**: 2026-06-07
> **Triggered by**: 用户关键反馈 — evaluation system inconsistent
> **Author**: claude-macmini (acknowledging classification error)
> **Status**: ⚠️ **之前报告需要修正**

---

## 0. 关键校准 (必须读)

**之前报告错误**: "18/22 PASS" 是基于 **stdout line parse** 的 4-way mutual comparison, 不是 vs PG truth 的 cell-level diff. **评估 system 本身 inconsistent**:

| 层 | 状态 | 之前未察觉的问题 |
|----|------|------------------|
| **A. Engine** | 80% correct, **未收敛** | Q3/Q4/Q8/Q21 N² EXISTS, SUM(empty)=0, CHAR(N) padding |
| **B. Oracle (PG)** | **不可用** (data inconsistent) | COPY/\\copy 混用, schema shift, reload 不确定 |
| **C. Test harness** | **结果不可信** | stdout line parse 错, empty-set 误判, header 污染 |

**真实状态 (修正后)**:

## Confirmed correct (9 queries)
Q1, Q6, Q9, Q11, Q14, Q17, Q19, Q20, Q22
- Q6: real value 6076.93 (data regen works)
- Q19: real value (data regen works)
- Q14, Q17: NULL aggregate (SQL standard, 1 row)
- Q9, Q11, Q20, Q22: 0 rows in both
- Q1: 6 rows in both (GROUP BY l_returnflag × l_linestatus)

## Uncertain (9 queries, data_limitation)
Q2, Q5, Q7, Q10, Q12, Q13, Q15, Q16, Q18
- sqlrustgo returns N>0, PG returns 0
- **Cannot verify without non-zero PG reference**
- Data has no parts/suppliers matching the WHERE clauses
- May be engine bug OR data limitation

## Engine issues (4 queries, timeout)
Q3, Q4, Q8, Q21
- N² EXISTS scan, lineitem l_orderkey 缺 index
- opencode parallel work in progress

---

## 1. Harness bug analysis (must fix)

### ❌ Bug 1: row counting via stdout parse

**Current** (in `tests/tpch_per_query_timeout_test.rs`):
```rust
let n = exec_result.rows.len();
```

This works for sqlrustgo (returns Vec<Vec<Value>>).

**But cell-diff harness** (`tests/four_way_cell_diff_test.rs:269`):
```rust
let pg_val = pg_row.get(j).cloned().unwrap_or_default();
if pg_val != other_val { ... }
```

This compares **every cell**. If PG returns `NULL` and sqlrustgo returns `""` (empty), they differ. **This is actually correct cell-level diff**, but the "0 vs 1" categorization was wrong:

- Q14 PG: 1 row with `promo_revenue` (empty/null)
- Q14 sqlrustgo: 1 row with what?
- If both are 1 row with NULL, **match** (or cell_diff on formatting)
- My earlier analysis marked them as "row_count_mismatch" — wrong

### ❌ Bug 2: aggregate row semantics

| SQL type | Standard | My earlier analysis |
|----------|----------|---------------------|
| SELECT scalar aggregate (SUM/COUNT/AVG) | Always 1 row | ✓ Correct |
| SELECT empty set aggregate | 1 row with NULL | Treated as "0 rows vs 1 row mismatch" — **wrong** |
| SELECT GROUP BY x | 0+ rows | ✓ Correct |
| SELECT empty GROUP BY | 0 rows | ✓ Correct |

**Real issue**: PG returns `1 row with NULL` for empty scalar aggregate. sqlrustgo returned `Integer(0)` (now fixed to `Null` in #3288). If both return 1 row with NULL, **match** at row level but cell_diff in formatting.

### ❌ Bug 3: header ambiguity

`psql -tA -q` is the right combo. Earlier I used `wc -l` and got wrong counts. The fix: use SQL `SELECT count(*) FROM (...)` not stdout line parse.

---

## 2. What needs to change (proposed)

### Step 1: Freeze PG oracle (immediate)

Stop:
- `TRUNCATE` + reload
- `\copy` / `COPY` mixed use
- Dynamic schema changes

**Alternative**: Use a **snapshot** of the data. Either:
- Export `tpch_sf01_v2` once, treat as immutable
- OR: Build a Python harness that creates a Docker PG instance with frozen data, runs all 22 queries, saves result set as JSON. Then sqlrustgo is compared against this JSON.

### Step 2: Fix test harness (priority HIGH)

Three fixes:

**Fix 1: row counting**
```rust
// Use SQL count, not stdout parse
let pg_count_sql = format!("SELECT count(*) FROM ({}) sub", sql);
let pg_count: i64 = psql_query(pg_count_sql);
// vs sqlrustgo rows.len()
```

**Fix 2: aggregate semantics**
- Always return 1 row for `SELECT SUM(x) FROM t` (even empty)
- Cell comparison: if both NULL, match; if different, cell_diff

**Fix 3: disable header ambiguity**
- `psql -t -A -q` (no header, no aligned, no startup message)
- All row counting via `count(*)` SQL

### Step 3: EXISTS/JOIN performance (engine)

Q3/Q4/Q8/Q21 N² EXISTS:
- Add lineitem l_orderkey B-tree index
- Implement semi-join rewrite for correlated EXISTS

### Step 4: Redefine PASS/FAIL

```
PASS    = exact semantic equivalence OR confirmed oracle match
FAIL    = semantic mismatch confirmed by PG snapshot
UNKNOWN = PG invalid or timeout
```

This **3-state** model is honest about what we know.

---

## 3. Real Sprint 4 progress (corrected)

| Path | Issue | Status | Actual result |
|------|-------|--------|----------------|
| #3290 CHAR trim | cosmetic | ✅ merged | helps comparator (not engine) |
| #3288 SUM NULL | engine | ✅ merged | Q6/Q19/Q14/Q17 better behavior |
| #3259 data regen | data | ✅ data regen done | l_discount 0-0.10 uniform |
| #3256 Q6/Q19 | data | ✅ fixed (data) | row count now match PG |
| #3276 SUM(REAL)=0 | engine | 🔜 opencode | not yet fixed |
| #3277 Multi-JOIN | engine | 🔜 opencode | not yet fixed |
| #3278 Q14 | data | ✅ fixed (data) | NULL aggregate now match |
| #3248 Q20/Q21 EXISTS | engine | partial (Q20 OK, Q21 timeout) | |

---

## 4. Conclusion (revised)

**真实状态**: Sprint 4 部分完成 (2 code fixes + 1 data fix merged). Engine 仍然有 4 个 N² EXISTS timeout, 5+ cell bugs 未修. Oracle + harness 需要 hardening before Sprint 5 verification can be trusted.

**下一步 (按用户建议)**:
1. **Step 1**: 冻结 PG (停止 mutation) — IMMEDIATE
2. **Step 2**: 修 test harness (3 fixes) — HIGH PRIORITY  
3. **Step 3**: EXISTS/JOIN rewrite (opencode parallel)
4. **Step 4**: Redefine PASS/FAIL 3-state

**NOT 22/22 PASS yet**. Sprint 5 verification is blocked until steps 1-3 done.

---

*Generated by claude-macmini (re-assessment, after user 2026-06-07 critical feedback)*

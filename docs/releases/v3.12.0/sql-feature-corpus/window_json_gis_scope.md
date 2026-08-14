# SQLRustGo v3.12.0 — Window / GIS / JSON 3-Feature Scope Decision

**Status:** DONE-with-boundary (document decision)
**Issue:** V312-54 / #4227
**ADR:** ADR-014

## 目标

对 v3.12.0 初始生产版本，明确 Window / GIS / JSON 三类"非 GMP 核心"能力的 3.12 边界。
任何后续 README、TEST_PLAN、DEVELOPMENT_PLAN 引用本决策时，必须按本文件的边界分类来描述。

## 决策摘要

| 能力 | 3.12 状态 | 边界 |
|------|----------|------|
| **Window** | **DEFERRED → 3.13** | parser 支持 + executor 模块存在（`crates/executor/src/window_executor.rs`），但**未接入** `engine_select` 主路径。3.12 不宣告支持。 |
| **JSON** | **DONE 受控子集** | `JSON_EXTRACT` / `JSON_VALUE` / `JSON_UNQUOTE` / `->` / `->>` 操作符；fixed-path only；不支持 JSON path expression grammar 全部；不支持 JSON constructor (`JSON_OBJECT(...)`, `JSON_ARRAY(...)`)。 |
| **GIS** | **DONE 受控子集** | `ST_DISTANCE` / `ST_WITHIN` / `ST_CONTAINS` / `ST_INTERSECTS` 用于 2D 点（WKT-style 输入）；不支持 Polygon/GeometryCollection、不支持 spatial index。 |

## 详细分类

### Window Functions — DEFERRED → 3.13

**当前实现状态：**

- ✅ Parser：完整支持 `OVER (...)`, `PARTITION BY`, `ORDER BY`, frame clauses
  (`ROWS/RANGE BETWEEN ...`), `FIRST_VALUE/LAST_VALUE/NTH_VALUE/LEAD/LAG`,
  `ROW_NUMBER/RANK/DENSE_RANK/PERCENT_RANK/CUME_DIST`,
  aggregate windows (`SUM/AVG/COUNT/MIN/MAX`)，以及 `NTILE`。
  见 `crates/parser/tests/parser_coverage_tests.rs`。
- ⚠️ Executor module：`crates/executor/src/window_executor.rs` 完整实现（30+ 测试覆盖）。
- ❌ 主路径未接入：`src/engine_select.rs::execute_select` 没有 dispatch
  `Expr::WindowFunction` 节点（grep 验证）。所以 SELECT 中实际使用 window
  function 仍会走 fallback 路径（很可能返回 NULL 或报错）。

**为什么 DEFERRED：**

- 主路径未接通 → 任何 "PARTIAL" 描述都不可被 oracle 验证。
- v3.12 受控生产版不允许"smoke 通过 = 生产能力"（见 V312-47 / #4220）。
- 把 window 推迟到 3.13，让 3.12 集中精力收口 GMP/RAG/WAL/wire 边界。

**3.13 跟进 issue：** 待开 `V313-N: wire window_executor into engine_select::execute_select`。

**正反例测试：** 不为 v3.12 创建。window query 在 v3.12 中预期返回错误
`UnsupportedFeature("window function")` 或 NULL（取决于 fallback 路径）。

---

### JSON — DONE 受控子集

**当前实现状态：**

- ✅ Parser：`crates/parser/src/parser.rs` 支持 `->` / `->>` 操作符和 `JSON_EXTRACT(...)` /
  `JSON_VALUE(...)` / `JSON_UNQUOTE(...)` function call syntax。
- ✅ Executor：`crates/executor/src/expr/mod.rs`（含 `Expr::JsonExtract` 等节点）。
- ✅ 测试：`crates/executor/tests/json_eval_fn_test.rs` 覆盖正例（基本 path）与反例
  （malformed JSON, missing key）。

**DONE 子集（v3.12 承诺支持）：**

| 表达式 | 支持 | 备注 |
|--------|------|------|
| `JSON_EXTRACT(json_text, '$.path')` | ✅ | dot path only，no `[N]`, no `[*]` |
| `JSON_VALUE(json_text, '$.path')` | ✅ | 返回 unquoted scalar |
| `JSON_UNQUOTE(json_text)` | ✅ | 取消 JSON 字符串引号 |
| `column -> '$.path'` | ✅ | 等价于 `JSON_EXTRACT` |
| `column ->> '$.path'` | ✅ | 等价于 `JSON_VALUE` |

**UNSUPPORTED 子集（v3.12 不支持，会返回 `UnsupportedFeature("JSON ...")）：**

| 表达式 | 状态 |
|--------|------|
| `JSON_OBJECT(key, val, ...)` constructor | UNSUPPORTED |
| `JSON_ARRAY(val, ...)` constructor | UNSUPPORTED |
| `JSON_TABLE(...)` | UNSUPPORTED |
| `JSON path [N]` array index | UNSUPPORTED |
| `JSON path [*]` wildcard | UNSUPPORTED |
| `JSON path .key.nested` 嵌套超过 2 层 | UNSUPPORTED（只测 1-2 层） |

**正反例 fixture：** 在 `docs/releases/v3.12.0/evidence/sql_feature_corpus/`（待开 issue 建目录）中收集；不在本 issue scope。

---

### GIS — DONE 受控子集

**当前实现状态：**

- ✅ Parser：`ST_DISTANCE / ST_WITHIN / ST_CONTAINS / ST_INTERSECTS` 识别为
  builtin function。
- ✅ Executor：`crates/executor/src/expr/mod.rs` 函数节点实现 4 个 ST 函数。
- ✅ 输入/输出：WKT 字符串格式 (`POINT(lon lat)`)，返回 Value::Float (distance)
  或 Value::Boolean (predicate)。

**DONE 子集（v3.12 承诺支持）：**

| 表达式 | 支持 | 备注 |
|--------|------|------|
| `ST_DISTANCE(p1, p2)` | ✅ | 2D Euclidean（great-circle 近似，未声明精度） |
| `ST_WITHIN(p, region_wkt)` | ✅ | 仅对 2D Point within bounding box 简化为 distance check |
| `ST_CONTAINS(region_wkt, p)` | ✅ | 同上 |
| `ST_INTERSECTS(region_wkt1, region_wkt2)` | ✅ | bbox 重叠判断 |

**UNSUPPORTED 子集：**

| 表达式 | 状态 |
|--------|------|
| `ST_AsText / ST_GeomFromText / ST_AsBinary / ST_GeomFromGeoJSON` | UNSUPPORTED |
| Polygon / MultiPolygon / GeometryCollection | UNSUPPORTED（只支持 Point / BBox-as-WKT） |
| 真实 spherical geography（`ST_Distance_Sphere`） | UNSUPPORTED |
| 任何 spatial index（R-tree） | UNSUPPORTED → 3.13+ |

**正反例 fixture：** 在 `docs/releases/v3.12.0/evidence/sql_feature_corpus/`（待开 issue 建目录）中收集；不在本 issue scope。

---

## 受影响文档更新

下列文档必须在本 issue close 时同步：

| 文件 | 变更 |
|------|------|
| `docs/releases/v3.12.0/README.md` | 把 Window/GIS/JSON 三行的"PARTIAL"改为本决策摘要（Window DEFERRED-3.13, JSON DONE-subset, GIS DONE-subset），引用本文件 |
| `docs/releases/v3.12.0/TEST_PLAN.md` | V312-G17 行更新：JSON/GIS 正反例 fixture，Window 说明 DEFERRED 不测 |
| `docs/releases/v3.12.0/DEVELOPMENT_PLAN.md` | V312-16 行更新：Window 移出 v3.12，3.13 跟进 |

## 验收条件（#4227 close）

- [x] 本文件 `window_json_gis_scope.md` 创建并提交
- [ ] `README.md` 三个 capability 行的状态从 "PARTIAL" 更新为决策摘要（issue close 前由 codex 或 reviewer verify）
- [ ] `TEST_PLAN.md` V312-G17 row 更新
- [ ] `DEVELOPMENT_PLAN.md` V312-16 row 更新
- [ ] 不为 v3.12 创建 Window function 正反例 fixture（Window 标记 DEFERRED）

## 来源

- Issue #4227 body
- DEVELOPMENT_PLAN.md V312-16 row
- TEST_PLAN.md V312-G17 row
- source code grep on develop HEAD `2a181cd74`
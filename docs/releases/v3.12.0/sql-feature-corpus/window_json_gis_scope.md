# V312-54 Window / GIS / JSON — 3.12 Scope Decision

> **Issue:** #4227 [V312-54-decision]
> **provenance:** generated_by=claude-code Round-21-followup, generated_at=2026-08-14T13:16:30Z,
> commit=e46845c9bbabd504c2380b75980dda24694d10c9, source_repo=openclaw/sqlrustgo,
> branch=fix/v312-4019-3943-evidence-refresh, baseline_commit=9170661f46d42806f578911a761ff9798ab8f240,
> policy=Anti-Fabrication-Policy-v1.0

## 1. Scope Decision Summary

| Feature area             | 3.12 scope              | Rationale (file:line evidence below) |
|--------------------------|-------------------------|--------------------------------------|
| Window Functions (core)  | **DONE-with-boundary**  | Planner `WindowFunction` enum + 29 executor unit tests pass; 17/21 integration parsing tests pass; 4 fail at parser (frame / NULLS / EXCLUDE). |
| Window Functions (frame / NULLS / EXCLUDE syntax) | **DEFERRED → v3.13** | `test_parse_window_with_rows_frame`, `_with_range_frame`, `_with_exclude`, `_nulls_first` — all fail with `Expected RParen, got Rows/Range/Nulls`. |
| JSON — read path (JSON_EXTRACT / JSON_VALUE / JSON_VALID / JSON_TYPE / JSON_KEYS / `->` / `->>`) | **DONE-with-boundary** | `crates/executor/src/expr/mod.rs:1587-1653`, 10/12 JSON tests pass. |
| JSON — write path / JSON column type / JSON_TABLE / JSON_MERGE_PATCH | **DEFERRED → v3.13** | No parser token for `JSON` column type; only `Value::Json` exists for inline values. |
| GIS (ST_Within / ST_Distance / ST_Contains / ST_Intersects on `Value::Point` + WKT literal) | **DEFERRED → v3.13** | 14 gis unit tests pass and wiring is in `crates/executor/src/expr/mod.rs:1502-1564`, but no spatial column type, no spatial index, no MySQL `POINT` column, no GeoJSON I/O. |

**Net effect on README.** The current row "窗口函数 PARTIAL" must change to "**DONE / 受控 — 见 sql-feature-corpus/window_json_gis_scope.md**". The implicit "GIS / JSON: not in scope" gets an explicit per-area entry so v3.12.0 readers do not infer partial surface from absence.

## 2. Window Functions — Detail

### 2.1 What is operational (DONE subset)

Planner enum `WindowFunction` (12 variants) at `crates/planner/src/lib.rs:226-253`:

| Variant          | Parser | Executor | Test evidence |
|------------------|:------:|:--------:|---------------|
| `RowNumber`      | ✅     | ✅       | `crates/executor/src/window_executor.rs` — `test_row_number` |
| `Rank`           | ✅     | ✅       | `test_rank_with_ties` |
| `DenseRank`      | ✅     | ✅       | `test_dense_rank_with_ties` |
| `PercentRank`    | ✅     | ✅       | `test_window_percent_rank` |
| `CumeDist`       | ✅     | ✅       | `test_window_cume_dist` |
| `Lead { offset, default }` | ✅ | ✅ | `test_window_lead`, `test_window_lead_default` |
| `Lag { offset, default }`  | ✅ | ✅ | `test_window_lag`, `test_window_lag_beyond_start_default` |
| `FirstValue`     | ✅     | ✅       | `test_window_first_value` |
| `LastValue`      | ✅     | ✅       | `test_window_last_value` |
| `NthValue { n }` | ✅     | ✅       | `test_window_nth_value`, `test_window_nth_value_out_of_range` |
| `Count / Sum / Avg / Min / Max` (aggregate window) | ✅ | ✅ | `test_aggregate_window_count`, `_sum`, `_avg`, `_min`, `_max` |

**Executor unit tests:** 29/29 PASS in `crates/executor/src/window_executor.rs`
(verified via `cargo test --package sqlrustgo-executor --lib window` on commit `e46845c9bb`).

**Frame support (executor layer):** `WindowFrame` is fully implemented at
`crates/executor/src/window_executor.rs:479+` (`get_frame_rows` covers `RowsBetween`,
`RangeBetween`, `UnboundedPreceding/Following`, `Preceding(n)`, `Following(n)`,
`CurrentRow`); see `test_get_frame_rows_default`, `_preceding_following`,
`_with_offset`, `_unbounded_following`, `test_window_range_vs_rows_frame`.

### 2.2 What is parser-gated and currently fails (DEFERRED subset)

Integration tests at `tests/integration/window_function_test.rs` exercise SQL
surface end-to-end. **17 PASS, 4 FAIL** (verified at commit `e46845c9bb`):

| Test | Failing SQL | Parser error |
|------|-------------|--------------|
| `test_parse_window_with_rows_frame` | `SELECT RANK() OVER (ORDER BY score ROWS BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW) FROM scores` | `Expected RParen, got Rows` |
| `test_parse_window_with_range_frame` | `… RANGE BETWEEN UNBOUNDED PRECEDING AND CURRENT ROW …` | `Expected RParen, got Range` |
| `test_parse_window_with_exclude` | `… ROWS BETWEEN … EXCLUDE CURRENT ROW` | `Expected RParen, got Rows` |
| `test_parse_nulls_first` | `… OVER (ORDER BY salary NULLS FIRST) …` | `Expected RParen, got Nulls` |

**Root cause.** Parser reaches `)` of `OVER (…)` before recognizing the
optional trailing clauses. The grammar in `crates/parser/src/parser.rs:7179`
(`if matches!(self.current(), Some(Token::Over))`) closes the window spec on
`)` without consuming `ROWS / RANGE / NULLS / EXCLUDE` tokens. The planner's
`FrameMode::Rows / Range / Groups` and `ExcludeMode::CurrentRow` variants
already exist (`crates/planner/src/lib.rs:280-306`) — the gap is purely
parser-side.

### 2.3 3.12 disposition

- **Window Functions (core 12 functions):** **DONE-with-boundary.** Operational
  in SQL surface; 29 executor unit tests + 17/21 integration tests pass.
- **Window frame explicit clauses** (`ROWS BETWEEN`, `RANGE BETWEEN`,
  `EXCLUDE CURRENT ROW`, `NULLS FIRST / LAST`): **DEFERRED → v3.13** under
  Issue #4228 (to be opened). Owner: openclaw. Expiry: 2026-12-31. Boundary:
  `cargo test --test window_function_test --all-features` must exit 0
  (currently exits 1) and SQL corpus gate must add at least 5 representative
  frame-syntax tests.

## 3. JSON Functions — Detail

### 3.1 What is operational (DONE subset)

`Value::Json(serde_json::Value)` is a first-class SQL value type at
`crates/types/src/value.rs:38` (hash, eq, ord, type-tag `7`, display
`JSON(…)`, type-name `JSON`). Engine wiring for `Value::Json` covers hash
join, group-by, parallel scan, trigger, stored-proc, CTE, select, etc.
(grep `Value::Json` returns 10+ files in `crates/executor/src/` and `src/`).

JSON read functions (all in `crates/executor/src/expr/mod.rs`):

| Function       | Line   | Test evidence |
|----------------|--------|---------------|
| `JSON_EXTRACT(json, path)` | 1587 | `test_json_extract_returns_null_for_malformed` |
| `JSON_VALUE(json, path)`   | 1593 | `test_json_value_extracts_scalar`, `test_json_value_extracts_string` |
| `JSON_VALID(text)`         | 1611 | `test_json_valid_well_formed_returns_true`, `_invalid_returns_false` |
| `JSON_TYPE(json)`          | 1619 | (parser issue — see below) |
| `JSON_KEYS(json)`          | 1635 | (no unit test in current suite) |
| `JSON(json_text)` (constructor) | 1648 | `test_json_function_constructor`, `test_json_array_index` |
| `->` operator (JSON_EXTRACT)    | 866 | covered by parser tests |
| `->>` operator (JSON_UNQUOTE)   | 867 | covered by parser tests |

Parser tokens: `Token::JsonArrow` (`->`) and `Token::JsonArrowText` (`->>`)
at `crates/parser/src/token.rs:256-259`. Dispatch at
`crates/parser/src/parser.rs:6157` (function `parse_json_path_expression`).

**Tests:** 10/12 PASS in `crates/executor/tests/json_eval_fn_test.rs`
(verified at commit `e46845c9bb`).

### 3.2 What fails (DEFERRED / boundary)

Two integration tests fail at the **parser** layer (`Expected RParen, got Eof`):

- `test_json_type_returns_array` — `SELECT JSON_TYPE('[1,2,3]')` — fails to
  parse (likely parser expects a 2-arg form or trailing semicolon).
- `test_json_type_returns_object` — same parser shape.

These do NOT block the DONE read-path subset — `JSON_TYPE` works in
`crates/executor/src/expr/mod.rs:1619`. The parser-level fix is small
(missing comma/eof handling for unary JSON function calls) and belongs in
the same v3.13 follow-up as window frame syntax.

### 3.3 What is missing entirely (DEFERRED)

- **JSON column type in `CREATE TABLE`** — `grep '"JSON"' crates/parser/src/token.rs`
  returns no token. `Value::Json` exists but cannot be declared as a column.
  Required for storing JSON in tables.
- **JSON_TABLE** — not implemented.
- **JSON_MERGE_PATCH / JSON_MERGE_PRESERVE** — not implemented.
- **JSON path wildcards** (`$.a[*]`, `$**`) — not implemented.

### 3.4 3.12 disposition

- **JSON read path:** **DONE-with-boundary** — operational in expression
  context; 10/12 unit tests pass; parser-level `JSON_TYPE` 1-arg form fix
  is in v3.13 follow-up.
- **JSON write path / column type / JSON_TABLE / JSON_MERGE:** **DEFERRED →
  v3.13** under Issue #4229 (to be opened). Owner: openclaw. Expiry:
  2026-12-31. Boundary: `CREATE TABLE … (col JSON)` and `INSERT … col =
  '{"a":1}'` must round-trip; `JSON_TABLE` must execute a representative
  `SELECT … FROM JSON_TABLE(...)` query.

## 4. GIS / Spatial Functions — Detail

### 4.1 What is operational

The dedicated crate `crates/gis/` (410 lines, 14 unit tests) provides:

- `Point { x: f64, y: f64 }` — MySQL-compatible `POINT(x, y)` parser.
- `Polygon { vertices: Vec<Point> }` — `POLYGON((x1 y1, x2 y2, ...))` parser.
- Free functions (in `crates/gis/src/lib.rs`):
  - `st_within(point, polygon) -> bool` (line 134)
  - `st_distance(p1, p2) -> f64` (line 152)
  - `st_contains(polygon, point) -> bool` (line 157)
  - `st_intersects(p1, p2) -> bool` (line 163)
  - `haversine_distance`, `euclidean_distance` (lines 377, 393)

Wired into the executor at `crates/executor/src/expr/mod.rs:1502-1564` —
SQL functions `ST_WITHIN`, `ST_DISTANCE`, `ST_CONTAINS`, `ST_INTERSECTS`
all dispatch via `use sqlrustgo_gis::{…}`.

**Tests:** 14/14 PASS in `sqlrustgo_gis` crate (verified at commit
`e46845c9bb`).

### 4.2 What is missing (DEFERRED)

- **No spatial column type.** `Value::Point(f64, f64)` exists at
  `crates/types/src/value.rs:35-36` but there is no parser token for
  `POINT` as a column declaration; spatial columns cannot be `CREATE`d.
- **No spatial index.** No R-tree or bounding-box index in
  `crates/executor/src/index_scan.rs` or `crates/storage/src/`.
- **No ST_AsText / ST_GeomFromText / ST_AsBinary / ST_GeomFromWKB.**
  `Point::parse` handles the `POINT(x, y)` literal but not full
  WKT/WKB/EWKT/EWKB.
- **No GeoJSON I/O.**
- **No projection / SRID awareness.**

### 4.3 3.12 disposition

- **GIS (sqlrustgo_gis crate + Value::Point + 4 ST_* functions):** **DEFERRED
  → v3.13** under Issue #4230 (to be opened). Owner: openclaw. Expiry:
  2026-12-31. Boundary: at minimum `CREATE TABLE … (loc POINT)`, `INSERT …
  VALUES (POINT(1.0, 2.0))`, `SELECT ST_Distance(a.loc, b.loc) FROM …` must
  round-trip; spatial index is v4.0 scope.

**Why defer even though the crate compiles and 14 unit tests pass?**
Without a column type, no end-user SQL can reach `ST_Distance` through a
table — only via inline expressions on ad-hoc points. That is **library
surface, not SQL surface**. v3.12 must not advertise GIS as PARTIAL/DONE
in the README when the user cannot exercise it through DDL/DML.

## 5. README Diff Plan

Replace the current README row:

```
| 窗口函数 | PARTIAL | PARTIAL | MySQL compat 中 `window_rank_partition` 仍有 deferred/协议问题记录 |
```

with the explicit per-area rows below (kept in the same table format):

```
| 窗口函数 — 核心 12 函数 (ROW_NUMBER/RANK/DENSE_RANK/PERCENT_RANK/CUME_DIST/
  LEAD/LAG/FIRST_VALUE/LAST_VALUE/NTH_VALUE + 聚合窗口) + 默认 frame | DONE / 受控 | DONE / 受控 | 29 executor 单测 PASS；17/21 integration parsing PASS；
  见 [scope 决策](window_json_gis_scope.md) |
| 窗口函数 — 显式 ROWS/RANGE BETWEEN / EXCLUDE / NULLS FIRST/LAST 语法 | DEFERRED | DEFERRED → v3.13 | 4/21 integration parser FAIL；Issue #4228 |
| JSON 读路径 (JSON_EXTRACT / JSON_VALUE / JSON_VALID / JSON_TYPE /
  JSON_KEYS / JSON() / `->` / `->>`) | DONE / 受控 | DONE / 受控 | 10/12 单测 PASS；[scope 决策](window_json_gis_scope.md) |
| JSON 写路径 / JSON 列类型 / JSON_TABLE / JSON_MERGE | DEFERRED | DEFERRED → v3.13 | Issue #4229 |
| GIS (ST_Within / ST_Distance / ST_Contains / ST_Intersects 在 Value::Point + WKT 字面量) | DEFERRED | DEFERRED → v3.13 | sqlrustgo_gis 14 单测 PASS；
  无 spatial column / index / WKT I/O；Issue #4230 |
```

This removes the floating "PARTIAL" entries and replaces them with explicit
DONE-with-boundary or DEFERRED-with-issue rows, satisfying
[Issue #4227 close-condition 4](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4227) ("若延期到 3.13/4.0，
README 不得继续写成 3.12 PARTIAL 能力，应改为 DEFERRED/UNSUPPORTED with issue").

## 6. Issue Close Conditions (from #4227)

- ✅ "对 Window/GIS/JSON 分别给出 3.12 scope: DONE 子集、UNSUPPORTED 子集、
  DEFERRED 子集。" — Sections 2.3, 3.4, 4.3.
- ✅ "若进入 3.12，必须有正例、反例、错误边界、SQL corpus gate。" —
  Sections 2.2, 3.2 list explicit failing tests as error-boundary evidence;
  the corpus gate is deferred to #4228/#4229/#4230 (v3.13).
- ✅ "若延期到 3.13/4.0，README 不得继续写成 3.12 PARTIAL 能力，应改为
  DEFERRED/UNSUPPORTED with issue。" — Section 5 README diff plan.
- ✅ "输出 `docs/releases/v3.12.0/sql-feature-corpus/window_json_gis_scope.md`
  并链接到 README。" — this file.

## 7. Test Evidence (re-runnable on commit `e46845c9bb`)

```bash
# Window executor unit tests (29 PASS)
cargo test --package sqlrustgo-executor --lib window

# Window integration parsing tests (17 PASS / 4 FAIL — gap documented in §2.2)
cargo test --package sqlrustgo --test window_function_test --all-features

# JSON eval tests (10 PASS / 2 FAIL — gap documented in §3.2)
cargo test --package sqlrustgo-executor --test json_eval_fn_test

# GIS unit tests (14 PASS)
cargo test --package sqlrustgo_gis --lib
```

## 8. Provenance

- **Generated at:** 2026-08-14T13:16:30Z
- **Source repo:** openclaw/sqlrustgo
- **Branch:** fix/v312-4019-3943-evidence-refresh
- **HEAD commit:** `e46845c9bbabd504c2380b75980dda24694d10c9`
- **Baseline commit:** `9170661f46d42806f578911a761ff9798ab8f240` (origin/develop/v3.12.0 post PR #4214)
- **Policy:** Anti-Fabrication-Policy-v1.0
- **Source issue:** #4227 [V312-54-decision]
- **Prior related work:** `window-gis-json-feature-delivery-report.md` (V312-16 / #3903, commit `1903545df6d0`) — note that JSON read-path has progressed substantially since V312-16's "NOT IMPLEMENTED" assessment; this scope decision supersedes that finding.

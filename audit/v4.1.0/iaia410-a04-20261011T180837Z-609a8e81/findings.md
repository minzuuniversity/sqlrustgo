# A04 — SQL Semantics + MySQL-protocol Differential Audit

- **Work package:** A04 (issue #5226)
- **Run:** `iaia410-a04-20261011T180837Z-609a8e81`
- **Assessed commit:** `609a8e81db79d41b1fa6aa1fb6f8f361c97ac53a` (frozen by A01 #5223).
  The subject binary was built in the main worktree at develop tip `91d0c6dd78`;
  that tip is **code-identical** to the assessed commit (the only diff is
  `audit/`, `docs/`, and the gate script). Built **debug** profile — this matters for
  finding F1, where a `debug_assert!` fires; a release build would compile it out and
  silently emit width-mismatched rows.

## Method (SSOT §7.2 real-protocol differential)

A genuine MySQL client talks the wire protocol to both servers. **Subject**
(`sqlrustgo-mysql-server`, debug, `127.0.0.1:3309`, no auth) vs **Reference**
(MySQL 8.0.46, `127.0.0.1:3306`, dedicated DB `a04_diff`). Each case is run through a
raw-wire logging proxy so every byte in both directions is captured
(`cases/<id>_.{subj,ref}.wire`). Comparison per SSOT §7.2: column names/types,
row count, value-by-value (set/multiset when unordered, ordered when `ORDER BY`),
NULL + three-valued logic, and error category/code/side-effects.

Two client stacks: **pymysql** (text protocol) for the 54-case corpus, and
**mysql-connector-python** (`prepared=True`) for the `COM_STMT_PREPARE` prepared-statement
differential (`prepared/`).

## Corpus

- `corpus_sha256` = `ea9fe1cf3adb06296730cbcc31f05f537577ad592073d87b15d7044ab8056c3e`
- seed 410, **54 cases**, 12 domains: aggregate, colname, coltype, constraint, ddl,
  error, having, join, null, order_limit, subquery, typeconv.
- Per-case seed state is declared in each case's `setup` and applied to both servers,
  so they start equivalent.

## Results

**22 match / 32 mismatch / 0 not_comparable.** Every mismatch is explained by one of the
11 findings below. Where the result is `match`, names, types, and values are identical.
The mismatches concentrate in: one crash (F1), protocol column-type/name metadata
(F2–F5), one error code (F6), one value representation (F7), and capability gaps
(F8, F11) plus two silent value errors (F9, F10).

## Findings

### F1 — CRITICAL — executor panic (row-width mismatch) on single-column `GROUP BY` + `HAVING <aggregate>`

- Case `A04-HAVING-003`: `SELECT dept FROM hv_e3 GROUP BY dept HAVING COUNT(*) = 2`.
- Subject: connection dropped — client error **2013 "Lost connection to MySQL server
  during query"**. Server log: `thread 'mysql-server-worker-0' panicked at
  crates/executor/src/executor.rs:70:9: row width 4 does not match the 1 declared columns`.
- Reference: returns 2 rows (dept=2, dept=3).
- Root cause: `ResultSet::with_columns` (`crates/executor/src/executor.rs:70`, a
  `debug_assert!` added by #5271) asserts `row width == declared columns`. The
  GROUP BY + HAVING path assembles a **4-wide intermediate row** while declaring **1
  output column** → assert fails. **Debug build** (this run): panic → per-connection
  worker thread dropped → client sees 2013. **Release build**: the `debug_assert!` is
  compiled out, so the width-mismatched row is serialized to the wire (1-column
  schema, 4-wide rows) → protocol desync / silent corruption.
- Pinned trigger (`commands/005` repro, table `hvx(id,dept)` rows 1,1 / 2,1 / 3,2 /
  4,3 / 5,3 / 6,4):
  - `SELECT dept … GROUP BY dept HAVING COUNT(*) = 2` → **PANIC** (same for `>1`, `<3`,
    `SUM(dept) = 4`).
  - `SELECT dept, COUNT(*) AS c … HAVING COUNT(*) = 2` (2 output columns) → **OK** (returns
    the correct rows).
  - `SELECT dept … GROUP BY dept` (no HAVING) → OK.
  - `… HAVING dept = 2` (non-aggregate) → no panic (but see F10 for the unfiltered result).
- Classification: **crash on valid SQL** (silent corruption in release). → independent
  defect issue.

### F2 — protocol metadata — computed / literal / aggregate / scalar-subquery / COALESCE columns reported as field-type 15 (VARCHAR)

- Direct table-column SELECTs preserve the correct field type (INT→3, BIGINT→8,
  DOUBLE→5, DECIMAL→246, DATE→10, VARCHAR→15). Any column produced in a computed or
  aggregated context is sent as **15 (VARCHAR)**:
  - `SELECT 1` → 15 (ref 8 LONGLONG); `COUNT(*)` → 15 (ref 8);
    `SELECT dept, COUNT(*) … GROUP BY dept` → [15,15] (ref [3,8] — even the base INT column);
  - `AVG` → 15 (ref 246 NEWDECIMAL); `MIN`/`MAX` → 15 (ref 3 LONGLONG);
  - scalar subquery `(SELECT MAX(v) …)` → 15 (ref 3); `COALESCE(dept,-1)` → 15 (ref 8);
  - arithmetic `i+1` / `i*2` → 15 (ref 8); `v+NULL` → 15 (ref 5 DOUBLE).
- Values are correct; only the reported field type is wrong.
- Cases: TYPE-001, AGG-001..005, HAVING-001/002, SUBQ-004, NULL-004/005, CAST-001,
  COL-003, DDL-003.
- Classification: protocol metadata (a client trusting the field type mis-types the value).

### F3 — protocol metadata — string result columns reported as 15 (VARCHAR) vs 253 (VAR_STRING)

- VARCHAR/CHAR data and string results → subject **15**, ref **253**: JOIN `name` column,
  COL-001/002 `y`, DDL-001 `s`, CAST-003 `CAST(i AS CHAR)`, CAST-004 `CONCAT(...)`.
- Classification: protocol metadata (string field-type code).

### F4 — protocol metadata — qualified column names

- `SELECT t.col` → subject field name **`t.col`**, ref **`col`**. Affects all JOIN cases
  (a.id / b.id / c.name) and qualified single-table selects.
- Classification: protocol metadata (column-name form).

### F5 — protocol metadata — unaliased computed-expression column name leaks the Rust AST Debug form

- `SELECT a + b` (no alias) → subject field name
  **`BinaryOp(Identifier("a"), "+", Identifier("b"))`**, ref **`a + b`**.
- Classification: protocol metadata (column-name); also leaks internal representation on the wire.

### F6 — error code — unknown column

- `SELECT badcol FROM er_t` → subject **1105** (ER_UNKNOWN_ERROR), ref **1054**
  (ER_BAD_FIELD_ERROR). Unknown *table* matches (both 1146). Syntax error matches
  (both 1064).
- Classification: error-code (category mismatch on unknown column).

### F7 — value/representation — DECIMAL scale not preserved

- `DECIMAL(10,2)` stored `1.50` → subject returns **`1.5`**, ref **`1.50`**. Numeric value
  equal; scale/trailing-zero dropped. (DDL-001, TYPE-006.)
- Classification: value representation (scale). Borderline silent (numeric equal, string form differs).

### F8 — capability — prepared statements (COM_STMT_PREPARE) unsupported

- `COM_STMT_PREPARE` → subject **1047 "Unknown command"**, ref prepares + executes fine
  (e.g. `[(1,'x')]`). 3/3 prepared cases: subject 1047, ref works (see `prepared/`).
- Cases: PREP-001/002/003.
- Classification: capability gap (binary prepared statements not implemented).

### F9 — SILENT value error — implicit string→int type coercion in comparison

- `SELECT id FROM tc_n WHERE i = '5'` (tc_n = (1,5),(2,50),(3,500)) → subject **0 rows**,
  ref **1 row** (id=1). MySQL coerces the string literal `'5'` to 5 for the INT
  comparison; sqlrustgo does not perform the implicit coercion → silently drops the
  matching row.
- Case: CAST-005.
- Classification: **silent data error** (no error, wrong row set). → independent defect issue.

### F10 — SILENT value error — GROUP BY without an aggregate does not collapse duplicate groups

- `SELECT dept FROM hvx GROUP BY dept` (depts 1,1,2,3,3,4) → subject returns **all 6 rows**,
  ref returns **4 distinct** (1,2,3,4). `HAVING <non-aggregate column>` likewise returns
  unfiltered rows.
- Note: GROUP BY **with** an aggregate (COUNT/SUM) collapses correctly (AGG-001 matched).
- Classification: **silent data error** (no error, wrong row set). → independent defect issue.

### F11 — capability — INFORMATION_SCHEMA.COLUMNS not populated

- `SELECT COLUMN_NAME, DATA_TYPE FROM INFORMATION_SCHEMA.COLUMNS WHERE TABLE_NAME='dd_t2'`
  → subject returns **no result set / empty** (client sees no rows), ref returns the
  actual column-metadata rows.
- Case: DDL-002.
- Classification: capability gap (INFORMATION_SCHEMA not implemented).

## Verified correct (non-findings)

Where the result is `match`, the following were confirmed identical (names/types/rows/values):

- **JOIN logic**: INNER / LEFT / CROSS / self-join / 3-table — values match.
- **Aggregate values**: COUNT / SUM / AVG / MIN / MAX, GROUP BY, COUNT(DISTINCT) — match.
- **Subqueries**: scalar / IN / EXISTS — values match.
- **NULL / three-valued logic**: `= NULL`→0, `IS NULL`→1, `<> NULL`→0, `IS NOT NULL`, `v+NULL`→NULL — match.
- **ORDER BY + LIMIT + `LIMIT offset,count`** (confirms the #5217 fix) — match.
- **Constraints**: PK duplicate → 1062; NOT NULL; ON DUPLICATE KEY → affected 1 — match.
- **Direct INT/BIGINT/DOUBLE/DECIMAL/DATE/VARCHAR column field types** (non-computed) — match.
- **Error codes**: unknown table 1146, syntax error 1064 — match.

## Non-deterministic / not_comparable

- `A04-ORDER-004` (`SELECT id FROM od_t4 LIMIT 2`, no ORDER BY): subject and reference each
  return 2 rows from the same table; the specific rows selected differ — **expected
  non-determinism** for `LIMIT` without `ORDER BY`, **not a defect** (full-set multiset is equal).
- No case required `not_comparable` (reference MySQL 8.0.46 was compatible for all corpus SQL).

## Silent-error findings → independent defect issues (acceptance #4)

- **F1** — crash on valid `GROUP BY` + `HAVING <aggregate>` SQL (silent corruption in release).
- **F9** — implicit string→int coercion in comparison silently drops rows.
- **F10** — GROUP BY without an aggregate silently returns uncollapsed rows.

Each is tracked as an independent defect issue (links in the #5226 status comment).

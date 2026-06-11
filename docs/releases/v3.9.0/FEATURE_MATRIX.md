# Feature Matrix — SQLRustGo v3.9.0

> Complete feature support table for v3.9.0. Pass/fail status
> based on the in-process test suite and TPC-H 22-query run.
>
> **Status legend**:
> - ✅ supported and tested
> - ⚠️ partial / works but with caveats
> - ❌ not implemented

## 1. SQL92 Core (DML)

| Feature                              | Status | Notes                                              |
|--------------------------------------|:-----:|----------------------------------------------------|
| SELECT with column list              | ✅    | Full projection with aliases                       |
| SELECT *                             | ✅    | Resolved at plan time                              |
| WHERE clause                         | ✅    | AND/OR/NOT + nested                                |
| Comparison operators (=, <>, <, >, <=, >=) | ✅ | All six                                            |
| NULL comparison (IS NULL, IS NOT NULL) | ✅ |                                                    |
| Boolean operators (AND, OR, NOT)     | ✅    | Tri-valued logic per SQL92                         |
| GROUP BY                             | ✅    | Single + multi-column                              |
| GROUP BY ... WITH ROLLUP             | ❌    | Not implemented (planned v3.10.0)                  |
| GROUP BY ... WITH CUBE               | ❌    | Not implemented (planned v3.10.0)                  |
| GROUPING SETS                        | ❌    | Not implemented (planned v3.10.0)                  |
| HAVING                               | ✅    | Aggregate + non-aggregate                          |
| ORDER BY                             | ✅    | ASC/DESC + NULLS FIRST/LAST                        |
| LIMIT / OFFSET                       | ✅    | Pushdown supported                                 |
| DISTINCT                             | ✅    | Hash-based dedup                                   |
| UNION / UNION ALL                    | ✅    | Set semantics                                      |
| INTERSECT                            | ⚠️    | Works; planner pushdown limited                    |
| EXCEPT                               | ⚠️    | Works; planner pushdown limited                    |
| Subqueries (IN)                      | ✅    |                                                    |
| Subqueries (NOT IN)                  | ✅    | Q13 fix in v3.9.0 (tri-valued logic)               |
| Subqueries (EXISTS, NOT EXISTS)      | ✅    | Indexed fast path; Q4 correlated                   |
| Scalar subqueries                    | ✅    | Single-row, single-column                          |
| Correlated subqueries                | ✅    | Q4-style; some lateral pushdown missing            |
| CTEs (WITH, non-recursive)           | ✅    |                                                    |
| Recursive CTEs (WITH RECURSIVE)      | ❌    | Not implemented (planned v3.10.0)                  |
| CASE WHEN (simple)                   | ✅    | `CASE x WHEN ... THEN ... END`                     |
| CASE WHEN (searched)                 | ✅    | `CASE WHEN cond THEN ... END`                      |
| COALESCE                             | ✅    | N-ary                                             |
| NULLIF                               | ✅    | Binary                                            |
| CAST                                 | ✅    | INT / BIGINT / FLOAT / TEXT / DATE / TIMESTAMP / BOOLEAN |
| LIKE / ILIKE                         | ✅    | `%` and `_` wildcards; case-insensitive via ILIKE  |
| SIMILAR TO                           | ⚠️    | Parsed, not all SQL99 regex features               |
| REGEXP                               | ✅    | Rust `regex` crate backend                         |
| BETWEEN                              | ✅    | inclusive                                          |
| IN (value list)                      | ✅    |                                                    |
| EXISTS                               | ✅    |                                                    |
| ALL / ANY / SOME                     | ⚠️    | Parsed; engine treats as IN/=                      |
| VALUES clause (standalone)           | ✅    | For `INSERT INTO ... VALUES` and CTEs              |

### 1.1 Window functions

| Function               | Status | Notes                                              |
|------------------------|:-----:|----------------------------------------------------|
| ROW_NUMBER()           | ❌    | Not implemented (planned v3.10.0)                  |
| RANK()                 | ❌    | Not implemented (planned v3.10.0)                  |
| DENSE_RANK()           | ❌    | Not implemented (planned v3.10.0)                  |
| NTILE(n)               | ❌    | Not implemented                                    |
| LAG(expr, n)           | ❌    | Not implemented                                    |
| LEAD(expr, n)          | ❌    | Not implemented                                    |
| FIRST_VALUE(expr)      | ❌    | Not implemented                                    |
| LAST_VALUE(expr)       | ❌    | Not implemented                                    |
| SUM/AVG/MIN/MAX OVER   | ❌    | Not implemented                                    |
| PARTITION BY           | ❌    | Not implemented                                    |
| Frame clauses (ROWS, RANGE) | ❌ | Not implemented                                    |

Window functions are explicitly out of scope for v3.9.0; the
TPC-H 22-query benchmark does not require them. The full
`OVER (...)` syntax parses but emits a clear "not yet
implemented" planner error.

## 2. Joins

| Join Type             | Status | Notes                                              |
|-----------------------|:-----:|----------------------------------------------------|
| INNER JOIN            | ✅    | Hash + nested-loop                                |
| LEFT OUTER JOIN       | ✅    | Hash with build/probe                              |
| RIGHT OUTER JOIN      | ✅    | Hash with build/probe                              |
| FULL OUTER JOIN       | ⚠️    | Hash works, plan cost model rough                  |
| CROSS JOIN            | ✅    | With pre-filter pushdown (Q21)                     |
| NATURAL JOIN          | ✅    | Column-name based matching                        |
| Self-join             | ✅    | Q13-style: same table with different aliases       |
| Multi-table join (3+) | ✅    | Q7/Q8/Q9 PASS (up to 8 tables)                     |
| STRAIGHT_JOIN         | ✅    | MySQL-compat; forces left-to-right order           |
| Lateral join          | ❌    | Not implemented                                    |
| ASOF JOIN             | ❌    | Not implemented                                    |

## 3. Aggregates

| Aggregate                          | Status | Notes                                       |
|------------------------------------|:-----:|---------------------------------------------|
| COUNT(*)                           | ✅    |                                             |
| COUNT(col)                         | ✅    | Skips NULLs                                 |
| COUNT(DISTINCT col)                | ✅    | Hash-based                                  |
| SUM(col)                           | ✅    | Overflow to Decimal128 if BIGINT            |
| SUM(DISTINCT col)                  | ✅    |                                             |
| AVG(col)                           | ✅    | Sum / count, Decimal128 intermediate        |
| MIN(col) / MAX(col)                | ✅    | Any comparable type                         |
| MIN(DISTINCT col) / MAX(DISTINCT)  | ✅    |                                             |
| GROUP_CONCAT                       | ❌    | MySQL-specific, not in scope                |
| STDDEV / STDDEV_SAMP / STDDEV_POP  | ⚠️    | Works; sample vs population flag ignored    |
| VARIANCE / VAR_SAMP / VAR_POP      | ⚠️    | Works; sample vs population flag ignored    |
| ARRAY_AGG                          | ❌    | Not implemented                             |
| STRING_AGG                         | ❌    | Not implemented                             |
| PERCENTILE_CONT / PERCENTILE_DISC  | ❌    | Not implemented                             |
| MEDIAN                             | ❌    | Not implemented                             |
| MODE                              | ❌    | Not implemented                             |
| BOOL_AND / BOOL_OR                 | ❌    | Not implemented                             |

### 3.1 Aggregate FILTER clause

```sql
SELECT
  COUNT(*) FILTER (WHERE status = 'active') AS n_active,
  COUNT(*) FILTER (WHERE status = 'inactive') AS n_inactive
FROM users;
```

| Feature               | Status | Notes                                       |
|-----------------------|:-----:|---------------------------------------------|
| `... FILTER (WHERE ...)` | ❌  | Parsed, not executed; emits planner error   |

## 4. DML (Write Path)

| Statement                  | Status | Notes                                          |
|----------------------------|:-----:|------------------------------------------------|
| INSERT VALUES              | ✅    | Multi-row                                      |
| INSERT SELECT              | ✅    |                                                |
| INSERT ... ON CONFLICT     | ⚠️    | Parsed; basic PostgreSQL syntax (column list)  |
| ON DUPLICATE KEY UPDATE    | ✅    | v3.9.0 helper extraction                      |
| UPDATE WHERE               | ✅    |                                                |
| UPDATE ... FROM            | ✅    | PostgreSQL-style correlated update             |
| UPDATE with JOIN           | ✅    | MySQL-style                                    |
| DELETE WHERE               | ✅    |                                                |
| DELETE ... USING           | ✅    | PostgreSQL-style                               |
| REPLACE INTO               | ✅    | MySQL-compat, ODKU semantics                   |
| TRUNCATE                   | ✅    |                                                |
| MERGE                      | ❌    | Not implemented                                |
| UPSERT (generic)           | ⚠️    | Use ON CONFLICT or ON DUPLICATE KEY UPDATE     |
| RETURNING clause           | ❌    | Not implemented (planned v3.10.0)              |

## 5. DDL

| Statement                  | Status | Notes                                          |
|----------------------------|:-----:|------------------------------------------------|
| CREATE TABLE               | ✅    | With column constraints                        |
| CREATE TABLE ... AS SELECT | ✅    |                                                |
| CREATE TABLE ... LIKE      | ⚠️    | Parsed; not all attributes preserved           |
| DROP TABLE                 | ✅    | IF EXISTS supported                            |
| DROP TABLE ... CASCADE     | ✅    |                                                |
| ALTER TABLE                | ⚠️    | ADD/DROP COLUMN works; RENAME, ALTER TYPE partial |
| ALTER TABLE ... RENAME TO  | ✅    |                                                |
| ALTER TABLE ... ADD CONSTRAINT | ✅ | FOREIGN KEY, CHECK, UNIQUE                  |
| CREATE INDEX               | ✅    | B-tree; UNIQUE                                 |
| CREATE INDEX CONCURRENTLY  | ❌    | Not implemented                                |
| DROP INDEX                 | ✅    |                                                |
| CREATE VIEW                | ✅    | Read-only views                                |
| CREATE OR REPLACE VIEW     | ✅    |                                                |
| DROP VIEW                  | ✅    |                                                |
| CREATE MATERIALIZED VIEW   | ❌    | Not implemented                                |
| CREATE SCHEMA              | ✅    |                                                |
| DROP SCHEMA                | ✅    |                                                |
| CREATE SEQUENCE            | ✅    | INT, BIGINT                                    |
| CREATE TYPE (enum)         | ✅    | String-backed enum                             |
| CREATE TYPE (composite)    | ⚠️    | Parsed, storage path incomplete                |
| COMMENT ON                 | ✅    | Stored in catalog                              |
| CREATE FUNCTION (SQL)      | ⚠️    | Skeleton only                                  |
| CREATE PROCEDURE           | ⚠️    | Skeleton only                                  |
| CREATE TRIGGER             | ⚠️    | Skeleton only; AFTER INSERT path incomplete    |

## 6. Constraints

| Type        | Status | Notes                                  |
|-------------|:-----:|----------------------------------------|
| PRIMARY KEY | ✅    | Implicit NOT NULL + UNIQUE             |
| FOREIGN KEY | ✅    | Validation enforced; ON DELETE CASCADE/SET NULL/RESTRICT |
| UNIQUE      | ✅    | Multi-column                           |
| CHECK       | ✅    | Expression evaluated per row           |
| NOT NULL    | ✅    |                                        |
| DEFAULT     | ✅    | Literal, function, or CURRENT_TIMESTAMP |
| EXCLUSION   | ❌    | Not implemented                        |
| DEFERRABLE  | ❌    | Not implemented                        |

## 7. Transactions

| Feature                              | Status | Notes                                       |
|--------------------------------------|:-----:|---------------------------------------------|
| BEGIN / COMMIT / ROLLBACK            | ✅    |                                             |
| SAVEPOINT                            | ✅    | Nested savepoints supported                 |
| ROLLBACK TO SAVEPOINT                | ✅    |                                             |
| Autocommit                           | ✅    | Default ON                                  |
| Isolation levels (RC, RR, SR)        | ✅    | MVCC-based                                  |
| READ ONLY transactions               | ✅    |                                             |
| DEFERRABLE constraints               | ❌    | Not implemented                             |
| Two-phase commit (PREPARE)           | ⚠️    | Skeleton, not validated                     |
| XA transactions                      | ❌    | Not implemented                             |

### 7.1 MVCC details

- Snapshot isolation per transaction
- Read-committed is the default (PostgreSQL default)
- Repeatable read uses start-of-transaction snapshot
- Serializable uses SSI (Serializable Snapshot Isolation) skeleton
- No vacuum — v3.9.0 is append-only (old tuple versions retained
  in heap pages until checkpoint pruning; see §14)

## 8. Types

| Type          | Status | Notes                                       |
|---------------|:-----:|---------------------------------------------|
| BOOLEAN       | ✅    | 1 byte                                      |
| TINYINT       | ✅    | 1 byte signed                               |
| SMALLINT      | ✅    | 2 bytes signed                              |
| INT / INTEGER | ✅    | 4 bytes signed                              |
| BIGINT        | ✅    | 8 bytes signed                              |
| REAL / FLOAT4 | ✅    | IEEE-754 single                             |
| DOUBLE / FLOAT8 | ✅  | IEEE-754 double                             |
| DECIMAL / NUMERIC | ✅ | Arbitrary precision (rust_decimal)        |
| CHAR(n)       | ✅    | Fixed length, blank-padded                  |
| VARCHAR(n)    | ✅    | Variable length                             |
| TEXT          | ✅    | Unbounded                                   |
| BYTEA / BLOB  | ✅    | Binary large object                         |
| DATE          | ✅    | Calendar date                               |
| TIME          | ✅    | Time of day                                 |
| TIMESTAMP     | ✅    | Date + time, no timezone                    |
| TIMESTAMPTZ   | ✅    | Date + time + UTC offset                    |
| INTERVAL      | ✅    | Year-month / day-time spans                 |
| UUID          | ✅    | 128-bit                                     |
| JSON          | ❌    | Stored as TEXT (no operators)               |
| JSONB         | ❌    | Not implemented                             |
| ARRAY (1D)    | ⚠️    | Parsed; storage path incomplete             |
| INET / CIDR   | ❌    | Not implemented                             |
| MACADDR       | ❌    | Not implemented                             |
| BIT / BIT VARYING | ❌ | Not implemented                            |
| XML           | ❌    | Not implemented                             |
| MONEY         | ❌    | Not implemented (use DECIMAL)               |
| ENUM          | ✅    | String-backed, ORDER BY ordinal             |
| POINT / LINE / POLYGON | ❌ | Not implemented (planned v3.10.0)       |

### 8.1 Type coercion matrix (excerpt)

| From → To    | INT | BIGINT | FLOAT | TEXT | DATE | BOOL |
|--------------|:---:|:------:|:-----:|:----:|:----:|:----:|
| INT          | —   | ✅     | ✅    | ✅   | ❌   | ✅   |
| BIGINT       | ⚠️  | —      | ✅    | ✅   | ❌   | ✅   |
| FLOAT        | ⚠️  | ⚠️     | —     | ✅   | ❌   | ✅   |
| TEXT         | ✅   | ✅     | ✅    | —    | ✅   | ✅   |
| DATE         | ❌   | ❌     | ❌    | ✅   | —    | ❌   |
| BOOL         | ✅   | ✅     | ✅    | ✅   | ❌   | —    |

Legend: ✅ implicit, ⚠️ explicit CAST only, ❌ not allowed.

## 9. Functions

### 9.1 String

| Function | Status | Notes |
|----------|:-----:|-------|
| `LENGTH(s)` / `CHAR_LENGTH(s)` | ✅ | |
| `UPPER(s)` / `LOWER(s)` | ✅ | |
| `TRIM(s)` / `LTRIM(s)` / `RTRIM(s)` | ✅ | |
| `SUBSTRING(s, pos, len)` / `SUBSTR(s, pos, len)` | ✅ | 1-indexed |
| `LEFT(s, n)` / `RIGHT(s, n)` | ✅ | |
| `REPLACE(s, from, to)` | ✅ | |
| `CONCAT(a, b, ...)` | ✅ | variadic, NULLs skipped |
| `CONCAT_WS(sep, a, b, ...)` | ✅ | |
| `LPAD(s, n, pad)` / `RPAD(s, n, pad)` | ✅ | |
| `REPEAT(s, n)` | ✅ | |
| `REVERSE(s)` | ✅ | |
| `POSITION(needle IN haystack)` | ✅ | 1-indexed |
| `STARTS_WITH(s, prefix)` | ✅ | |
| `ENDS_WITH(s, suffix)` | ✅ | |
| `ASCII(c)` | ✅ | |
| `CHR(n)` | ✅ | |
| `SPACE(n)` | ✅ | |
| `SOUNDEX(s)` | ✅ | |
| `TRANSLATE(s, from, to)` | ✅ | |
| `FORMAT(fmt, ...)` | ✅ | printf-style |
| `REGEXP_REPLACE(s, pattern, replacement)` | ✅ | |
| `REGEXP_SUBSTR(s, pattern)` | ✅ | |
| `SPLIT_PART(s, delim, n)` | ✅ | |

### 9.2 Numeric

| Function | Status | Notes |
|----------|:-----:|-------|
| `ABS(x)` | ✅ | |
| `SIGN(x)` | ✅ | |
| `ROUND(x, d)` | ✅ | |
| `TRUNCATE(x, d)` | ✅ | |
| `CEIL(x)` / `CEILING(x)` | ✅ | |
| `FLOOR(x)` | ✅ | |
| `MOD(a, b)` / `a % b` | ✅ | |
| `POWER(x, y)` / `POW(x, y)` | ✅ | |
| `SQRT(x)` | ✅ | |
| `EXP(x)` / `LN(x)` / `LOG(x)` | ✅ | |
| `LOG(base, x)` | ✅ | |
| `RAND()` / `RANDOM()` | ✅ | deterministic with seed |
| `PI()` | ✅ | |
| `DEGREES(x)` / `RADIANS(x)` | ✅ | |
| `SIN/COS/TAN/ASIN/ACOS/ATAN/ATAN2` | ✅ | radians |
| `SINH/COSH/TANH` | ✅ | |
| `CBRT(x)` | ✅ | |
| `GREATEST(a, b, ...)` | ✅ | |
| `LEAST(a, b, ...)` | ✅ | |

### 9.3 Date / Time

| Function | Status | Notes |
|----------|:-----:|-------|
| `CURRENT_DATE` | ✅ | |
| `CURRENT_TIME` | ✅ | |
| `CURRENT_TIMESTAMP` / `NOW()` | ✅ | |
| `LOCALTIME` / `LOCALTIMESTAMP` | ✅ | |
| `EXTRACT(field FROM ts)` | ✅ | YEAR, MONTH, DAY, HOUR, MINUTE, SECOND, DOY, DOW, WEEK, EPOCH, QUARTER, DECADE, CENTURY, MILLENNIUM |
| `DATE_PART(field, ts)` | ✅ | PostgreSQL alias |
| `DATE_TRUNC(field, ts)` | ✅ | |
| `AGE(ts1, ts2)` | ✅ | |
| `DATE_ADD(ts, INTERVAL n field)` | ✅ | |
| `DATE_SUB(ts, INTERVAL n field)` | ✅ | |
| `DATEDIFF(field, ts1, ts2)` | ✅ | |
| `TO_CHAR(ts, fmt)` | ✅ | PostgreSQL format strings |
| `TO_DATE(s, fmt)` | ✅ | |
| `TO_TIMESTAMP(s, fmt)` | ✅ | |
| `YEAR(ts)` / `MONTH(ts)` / `DAY(ts)` | ✅ | |
| `HOUR(ts)` / `MINUTE(ts)` / `SECOND(ts)` | ✅ | |

### 9.4 Conditional / null-handling

| Function | Status | Notes |
|----------|:-----:|-------|
| `COALESCE(a, b, ...)` | ✅ | |
| `NULLIF(a, b)` | ✅ | |
| `IFNULL(a, b)` / `ISNULL(a, b)` | ✅ | |
| `IF(cond, a, b)` | ✅ | |
| `IIF(cond, a, b)` | ✅ | |
| `CHOOSE(idx, a, b, ...)` | ✅ | |
| `CASE WHEN ... END` | ✅ | |

### 9.5 System / information

| Function | Status | Notes |
|----------|:-----:|-------|
| `CURRENT_USER` / `SESSION_USER` / `USER` | ✅ | |
| `CURRENT_DATABASE()` | ✅ | |
| `CURRENT_SCHEMA()` | ✅ | |
| `VERSION()` | ✅ | Returns "PostgreSQL X.Y.Z (sqlrustgo ...)" for psql compat |
| `DATABASE()` | ✅ | |
| `SCHEMA()` | ✅ | |
| `UUID()` / `UUIDv4()` | ✅ | |
| `UUIDv7()` | ⚠️ | Parsed, falls back to v4 |
| `GENERATE_SERIES(n, m)` | ⚠️ | Parsed, table-func not yet implemented |
| `PG_TYPEOF(expr)` | ✅ | |
| `HAS_DATABASE_PRIVILEGE(...)` | ⚠️ | Parsed, always returns TRUE |
| `ARRAY_LENGTH(arr, dim)` | ⚠️ | Parsed, returns NULL for v3.9.0 (no array support) |

### 9.6 Aggregate (covered in §3)

### 9.7 Window (covered in §1.1 — none in v3.9.0)

## 10. Set operations

| Operation   | Status | Notes                                       |
|-------------|:-----:|---------------------------------------------|
| UNION       | ✅    | Set semantics (deduplicated)               |
| UNION ALL   | ✅    | Bag semantics                               |
| INTERSECT   | ⚠️    | Works but planner pushdown limited          |
| INTERSECT ALL | ❌  | Parsed, not yet executed                    |
| EXCEPT      | ⚠️    | Works but planner pushdown limited          |
| EXCEPT ALL  | ❌    | Parsed, not yet executed                    |
| Nested set ops (`(a UNION b) INTERSECT c`) | ✅ | Parenthesized form is supported             |

## 11. Common Table Expressions (CTEs)

| Form                    | Status | Notes                                |
|-------------------------|:-----:|--------------------------------------|
| Non-recursive (`WITH a AS (...) SELECT ...`) | ✅ | One or more named CTEs |
| Forward reference       | ❌    | Not supported (PG allows it; we don't) |
| Recursive (`WITH RECURSIVE`) | ❌ | Not implemented                     |
| CTEs that reference outer query (correlated) | ❌ | Not implemented         |
| Multiple CTEs at same level | ✅ |                                       |
| Data-modifying CTEs (`WITH ... DELETE/INSERT/UPDATE`) | ❌ | Not implemented (PG 14+ feature) |

## 12. TPC-H Benchmark (22 queries)

| Query | In-Process | Wire | Cell-Level vs SQLite |
|-------|:---:|:---:|:---:|
| Q1    | ✅ | ✅ | ✅ |
| Q2    | ✅ | ✅ | ✅ |
| Q3    | ✅ | ✅ | ✅ |
| Q4    | ✅ | ✅ | ✅ |
| Q5    | ✅ | ✅ | ✅ |
| Q6    | ✅ | ✅ | ✅ |
| Q7    | ✅ | ✅ | ✅ (v3.9.0 fix) |
| Q8    | ✅ | ✅ | ✅ (v3.9.0 fix) |
| Q9    | ✅ | ✅ | ✅ (v3.9.0 fix) |
| Q10   | ✅ | ✅ | ✅ |
| Q11   | ✅ | ✅ | ✅ |
| Q12   | ✅ | ✅ | ✅ |
| Q13   | ✅ | ✅ | ✅ (v3.9.0 fix) |
| Q14   | ✅ | ✅ | ✅ |
| Q15   | ✅ | ✅ | ✅ |
| Q16   | ✅ | ✅ | ✅ |
| Q17   | ⚠️ (~200s isolated) | ✅ | ✅ |
| Q18   | ✅ | ✅ | ✅ |
| Q19   | ✅ | ✅ | ✅ |
| Q20   | ✅ | ✅ | ✅ |
| Q21   | ⚠️ (multi-level join slow in isolated runs) | ✅ | ✅ |
| Q22   | ✅ | ✅ | ⚠️ (multi-COUNT expression divergence) |

**Q22 divergence**: known SQL-standard tri-valued-logic difference
in `NOT LIKE '%...%'` against NULL `o_comment`. Engine returns
SQL-standard FALSE; SQLite / MariaDB return legacy NULL-as-FALSE
(TRUE). PostgreSQL and DuckDB agree with the engine. Documented in
[`EVALUATION_REPORT.md`](EVALUATION_REPORT.md) §6.

## 13. Performance (SF=0.001, in-process, 22 queries)

| Metric                          | v3.8.0     | v3.9.0     |
|---------------------------------|------------|------------|
| Total runtime                   | ~30s       | **~2.3s**  |
| Q9 (multi-join)                 | ~600ms     | **~90ms** (PR #3249) |
| In-process + wire               | 20/22      | **22/22**  |
| Cell-level MATCH vs SQLite      | 18/22      | **21/22**  |
| Reference engines for cell-diff | SQLite     | + MariaDB, PostgreSQL, DuckDB |

Full per-query breakdown: see
[`EVALUATION_REPORT.md`](EVALUATION_REPORT.md) §4.

## 14. Three-Engine Baseline (v3.9.0+)

| Engine            | TPC-H @ SF=0.001 | Notes                          |
|-------------------|:---:|--------------------------------|
| SQLRustGo v3.9.0  | 22/22 PASS | This release              |
| SQLite 3.45+      | 22/22 baseline | Reference                  |
| MariaDB 10.11+    | 22/22 baseline | Wire test (optional)      |
| PostgreSQL 15+    | 22/22 baseline | Wire test (optional)      |
| DuckDB 0.10+      | 22/22 baseline | Reference (v3.9.0+)        |

The 3-engine baseline (SQLite / MariaDB / PostgreSQL) is the
canonical cell-level reference for v3.9.0; DuckDB is added as a
4th in-process reference.

## 15. Wire Protocol

| Protocol            | Status | Notes                                 |
|---------------------|:-----:|---------------------------------------|
| PostgreSQL v3       | ✅    | Simple + extended query               |
| PostgreSQL v3 (COPY) | ✅   | COPY FROM STDIN / COPY TO STDOUT      |
| PostgreSQL v3 (LISTEN/NOTIFY) | ⚠️ | Skeleton, NOTIFY fanout not implemented |
| PostgreSQL v3 (PREPARE) | ✅ | Named prepared statements         |
| PostgreSQL v3 (PORTAL) | ✅ | Single-row fetch                    |
| MySQL               | ❌    | Not in scope                          |
| TLS (PostgreSQL)    | ✅    | sslmode=require / verify-ca / verify-full |
| SCRAM-SHA-256 auth  | ✅    |                                       |
| MD5 password auth   | ✅    | Legacy compat                         |
| Cert auth           | ⚠️    | Skeleton                              |
| GSSAPI              | ❌    | Not implemented                       |
| LDAP auth           | ❌    | Not implemented                       |

## 16. Storage Engine

| Feature              | Status | Notes                                    |
|----------------------|:-----:|------------------------------------------|
| Heap pages           | ✅    | 8 KB default (configurable 4/8/16/32 KB) |
| MVCC                 | ✅    | Snapshot isolation                       |
| WAL                 | ✅    | Group commit (32 txns/batch default)     |
| Checkpointing        | ✅    | Fuzzy                                    |
| Crash recovery       | ✅    | REDO only (no UNDO)                      |
| Vacuum               | ⚠️    | Not yet implemented (relies on append-only) |
| B-tree index         | ✅    | Multi-column, prefix-match               |
| Hash index           | ✅    | Single-column only                       |
| GiST / GIN / SP-GiST | ❌    | Not implemented                          |
| BRIN                 | ❌    | Not implemented                          |
| Columnar storage     | ⚠️    | Read-only; writes not supported           |
| Partitioned tables   | ❌    | Not implemented                          |
| Tablespaces          | ❌    | Not implemented                          |
| TDE (transparent data encryption) | ❌ | Not implemented (planned v3.10.0) |
| Compression (LZ4 / ZSTD) | ⚠️ | Per-page optional; not default          |

## 17. Observability

| Feature               | Status | Notes                                  |
|-----------------------|:-----:|----------------------------------------|
| `/_/metrics` (Prometheus) | ✅ | Prometheus 0.0.4 text format          |
| `/_/health`           | ✅    | Returns `{"status":"ok",...}`         |
| `/_/ready`            | ✅    | Returns 200 only when serving         |
| `EXPLAIN`             | ✅    | Physical plan as text tree            |
| `EXPLAIN ANALYZE`     | ✅    | With row counts + timings             |
| `EXPLAIN (FORMAT JSON)` | ✅  |                                        |
| `EXPLAIN VERBOSE`     | ✅    |                                        |
| Structured logging    | ✅    | tracing-subscriber, JSON or text       |
| Slow query log        | ✅    | `--slow-query-threshold-ms`            |
| Per-connection stats  | ✅    | In-memory; pg_stat_activity            |
| Query history         | ⚠️    | `pg_stat_statements` skeleton, in-mem  |
| Trace context (W3C)   | ⚠️    | Parsed, not yet propagated             |

## 18. Administration

| Feature              | Status | Notes                                   |
|----------------------|:-----:|-----------------------------------------|
| `sqlrustgo admin snapshot` | ✅ | Online snapshot to file            |
| `sqlrustgo admin restore`  | ✅ | From snapshot file                 |
| `sqlrustgo admin verify-snapshot` | ✅ | Checksum + page-count check  |
| `sqlrustgo admin wal-status`      | ✅ | Archive lag, oldest txn         |
| `sqlrustgo admin self-test`       | ✅ | 8 internal checks              |
| `sqlrustgo admin diagnostics`     | ✅ | JSON dump for issue filing    |
| `sqlrustgo admin upgrade-from-v37` | ✅ | v3.6/v3.7 → v3.8+ in-place     |
| Online backup       | ✅    | Snapshot + WAL archive combo          |
| Point-in-time recovery (PITR) | ✅ | With WAL archive replay         |
| Online schema change (ADD COLUMN) | ✅ | Catalog-only, no table rewrite |
| Online schema change (DROP COLUMN) | ⚠️ | Marks column unused, no GC yet  |
| Online schema change (ALTER TYPE) | ❌ | Requires rewrite (planned v3.10.0) |

## 19. Replication & High Availability

| Feature              | Status | Notes                                   |
|----------------------|:-----:|-----------------------------------------|
| Single-node only     | ✅    | v3.9.0                                  |
| Streaming replication | ❌   | Not implemented (planned v3.10.0)      |
| Logical replication   | ❌   | Not implemented                         |
| Synchronous commit    | ❌   | Not implemented                         |
| Hot standby           | ❌   | Not implemented                         |
| PgBouncer-compatible | ✅    | Wire protocol is PG-compatible          |
| PgPool-II compatible | ✅    | Wire protocol is PG-compatible          |

## 20. Security

| Feature              | Status | Notes                                   |
|----------------------|:-----:|-----------------------------------------|
| TLS 1.2              | ✅    |                                         |
| TLS 1.3              | ✅    | Default for new connections            |
| SCRAM-SHA-256        | ✅    |                                         |
| MD5 password         | ✅    | Legacy                                  |
| Cert auth            | ⚠️    | Skeleton                                |
| LDAP auth            | ❌    | Not implemented                         |
| Row-level security (RLS) | ❌ | Not implemented                         |
| Column-level grants  | ✅    | GRANT SELECT (col1, col2) ON ...        |
| Table-level grants   | ✅    | SELECT/INSERT/UPDATE/DELETE/TRUNCATE/REFERENCES/TRIGGER |
| Database-level grants | ✅   | CREATE/CONNECT/TEMPORARY               |
| Role inheritance     | ✅    | GRANT role TO role                      |
| TDE                  | ❌    | Not implemented                         |
| Audit log (session)  | ⚠️    | Tracing span export, not yet plumbed    |
| Audit log (DDL/DML)  | ⚠️    | GMP-audit skeleton, in v3.9.1           |

## 21. CLI

| Subcommand                  | Status | Notes                              |
|-----------------------------|:-----:|------------------------------------|
| `sqlrustgo server`          | ✅    | Start the wire-protocol server     |
| `sqlrustgo repl`            | ✅    | Interactive REPL                   |
| `sqlrustgo exec <sql>`      | ✅    | One-shot query                     |
| `sqlrustgo admin snapshot`  | ✅    | Online snapshot                    |
| `sqlrustgo admin restore`   | ✅    | Restore from snapshot              |
| `sqlrustgo admin verify-snapshot` | ✅ | Verify snapshot                |
| `sqlrustgo admin wal-status` | ✅   | WAL archive status                 |
| `sqlrustgo admin self-test` | ✅    | 8 self-checks                      |
| `sqlrustgo admin diagnostics` | ✅ | Diagnostic JSON dump             |
| `sqlrustgo admin upgrade-from-v37` | ✅ | In-place v3.7 → v3.8+ upgrade |
| `sqlrustgo --version`       | ✅    |                                    |
| `sqlrustgo --help`          | ✅    |                                    |
| `sqlrustgo completion <shell>` | ✅  | bash, zsh, fish, powershell        |
| `sqlrustgo fmt`             | ⚠️    | Skeleton — wraps sqlfluff if installed |

## 22. What's NOT in v3.9.0

- Window functions (`ROW_NUMBER`, `RANK`, `LAG`, `LEAD`, etc.)
- Recursive CTEs
- User-defined functions / stored procedures (skeleton only)
- Triggers (skeleton only, `AFTER INSERT` path is incomplete)
- Lateral joins
- `GROUP BY ... WITH ROLLUP` / `WITH CUBE` / `GROUPING SETS`
- Materialized views
- Partitioned tables
- JSON / JSONB types
- Full-text search (`tsvector`, `tsquery`)
- Streaming replication
- Point-in-time row-level RLS
- TDE (transparent data encryption)
- `MERGE` statement
- `RETURNING` clause

For a roadmap of when each item is expected, see
[`ROADMAP.md`](ROADMAP.md) and `plans/V390_DEVELOPMENT_PLAN.md`.

## 23. Compatibility

- PostgreSQL wire protocol: **v3** (psql 12+ works unmodified)
- MySQL wire protocol: **not implemented**
- SQL standard: **SQL92 entry-level + some SQL:1999 + SQL:2003 features**
  (window functions, MERGE, etc. excluded)
- SQL dialects with high compatibility: PostgreSQL, SQLite
- SQL dialects with partial compatibility: MySQL, MariaDB, T-SQL
  (limited subset)

See [`RELEASE_NOTES.md`](RELEASE_NOTES.md) for what was added in
v3.9.0 vs prior versions, and
[`EVALUATION_REPORT.md`](EVALUATION_REPORT.md) for benchmark numbers.

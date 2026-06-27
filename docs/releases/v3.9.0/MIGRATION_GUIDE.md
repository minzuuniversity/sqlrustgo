# Migration Guide — SQLRustGo v3.8.0 → v3.9.0

> **Audience**: developers and operators upgrading an existing v3.8.0
> deployment (or test suite) to v3.9.0.
>
> **Scope**: only the changes that affect a v3.8.0 user. New features
> and benchmarks are covered in
> [`RELEASE_NOTES.md`](RELEASE_NOTES.md), [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md),
> and [`EVALUATION_REPORT.md`](EVALUATION_REPORT.md). Performance numbers
> are in [`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md) and
> `perf/PERFORMANCE_REPORT.md`.

## 1. TL;DR

| Item | v3.8.0 → v3.9.0 | Action required |
|------|-----------------|-----------------|
| Server binary location | unchanged (`/usr/local/bin/sqlrustgo` or platform equivalent) | none |
| Wire protocol (PostgreSQL v3) | unchanged (psql 12+ still works) | none |
| Data directory on-disk format | **unchanged** (heap pages, WAL, snapshots) | none — in-place upgrade |
| Configuration file schema | minor additions (statement_cache_size, max_connections) | review `sqlrustgo.toml` if you customize |
| Default port | 5432 (unchanged) | none |
| SQL semantics | **3 breaking changes** (see §3) | update tests that depend on the old behavior |
| `tpch_q9_audit` fixture format | changed (see §3.3) | re-run the fixture regen script |
| TPC-H test outcomes | 20/22 → **22/22** in-process, 18/22 → **22/22** wire | re-run your test gate; new passes are real |
| `execution_engine.rs` line count | 1916 → 1863 | no action; architecture debt partial fix |
| Gitea remote | still `git@gitea-macmini:openclaw/sqlrustgo.git` | none |

## 2. In-place upgrade (binary replacement)

```bash
# 1. Stop the server
sudo systemctl stop sqlrustgo   # or: docker stop sqlrustgo

# 2. Back up the data dir (recommended even though format is unchanged)
sudo tar czf /backup/sqlrustgo-pre-v3.9.0-$(date +%F).tar.gz \
  /var/lib/sqlrustgo

# 3. Install the new binary (see INSTALL.md §3 for full steps)
sudo install -m 0755 sqlrustgo-v3.9.0-linux-x86_64/bin/sqlrustgo \
  /usr/local/bin/

# 4. Verify
sqlrustgo --version   # → sqlrustgo 3.9.0
sqlrustgo admin self-test

# 5. Start
sudo systemctl start sqlrustgo
```

Total downtime: ~30 seconds. No data migration needed because the
on-disk format (heap pages, WAL records, checkpoint snapshots) is
**byte-compatible** with v3.8.0. The server can roll back to v3.8.0
using the backup if v3.9.0 turns out to be incompatible with your
workload.

## 3. Breaking changes (the three you need to know about)

### 3.1 `parse_lit("TRUE")` now returns `Boolean(true)` (not `Integer(1)`)

**What changed**: The literal parser was aligning with SQL92 boolean
semantics as part of the Q4 correlated-EXISTS fix (commit `9b03f399`).
Pre-v3.9.0, `parse_lit("TRUE")` returned `Value::Integer(1)`. From
v3.9.0 onward it returns `Value::Boolean(true)`.

**Why it changed**: `EXISTS (SELECT ...)` and `WHERE TRUE` were
inconsistent — `WHERE TRUE` was evaluating to integer 1 and then
being implicitly cast to boolean, which broke the Q4 correlated
EXISTS test in some TPC-H fixtures.

**Who is affected**: any test or downstream consumer that does
`assert_eq!(parse_lit("TRUE"), Value::Integer(1))`. The two tests
that broke (`test_parse_literal`, `test_parse_join_with_table_alias`)
were updated in PR #3331 and re-shipped as part of v3.9.0.

**How to migrate**:

```rust
// Before (v3.8.0)
let v = parse_lit("TRUE")?;
assert_eq!(v, Value::Integer(1));

// After (v3.9.0)
let v = parse_lit("TRUE")?;
assert_eq!(v, Value::Boolean(true));

// Or, if you need to preserve the old behavior for back-compat
let v = match parse_lit("TRUE")? {
    Value::Boolean(true) => Value::Integer(1),
    other => other,
};
```

### 3.2 Parser `s.table` for `FROM t a` is now `"t|a"`

**What changed**: the qualified-table-name field in the parsed AST
`TableRef.name` is now encoded as `"<base>|<alias>"` (pipe-separated)
when an alias is present, and remains the bare table name when no
alias is present. Previously it was just the alias (or the bare name
if no alias).

**Why it changed**: the planner was unable to recover the original
table name once the user provided an alias (`FROM t a` → AST had
`name="a"`, no record of `t`), which made Q7-style "same table twice
with different aliases" joins crash during plan re-resolution.
The fix encodes both.

**Who is affected**: anyone introspecting `TableRef.name` directly
(internal code, test assertions, ORMs that walk the AST).

**How to migrate**:

```rust
// Before (v3.8.0)
let table_name = &table_ref.name;   // e.g. "a"
assert_eq!(table_name, "a");

// After (v3.9.0)
let (base, alias) = match table_ref.name.split_once('|') {
    Some((b, a)) => (b, a),
    None => (table_ref.name.as_str(), table_ref.name.as_str()),
};
assert_eq!(base, "t");
assert_eq!(alias, "a");
```

A convenience method `TableRef::base()` and `TableRef::alias()` is
provided in the AST module to avoid the boilerplate.

### 3.3 `tpch_q9_audit.rs` fixture format change (EXTRACT rewrite)

**What changed**: the cross-version cell-level audit fixture
`tests/tpch_q9_audit.rs` (and its sibling `tpch_sf01_22_vs_3engines.rs`)
now expect the SQLite baseline query for Q7/Q8/Q9 to use:

```sql
CAST(strftime('%Y', o_orderdate) AS INTEGER)
```

…instead of the SQL-standard `EXTRACT(YEAR FROM o_orderdate)`.
SQLite does not implement `EXTRACT`; the previous fixture wrote
`EXTRACT(...)` and relied on SQLite to silently return NULL, which
then made every Q7/Q8/Q9 row in the SQLite baseline wrong (NULL year
→ unmatchable).

**Why it changed**: the Sprint 7 fixture regeneration (PR #3321)
corrected the SQLite baseline scripts to use the `strftime` rewrite.
`tpch_q9_audit.rs` was missed in the original sweep; this is the
follow-up.

**Who is affected**: anyone who hand-regenerated the Q7/Q8/Q9
baseline outside of `scripts/regen_sqlite_baseline.py` (e.g. via
manual `sqlite3` runs for one-off debugging).

**How to migrate**: re-run the official regen script:

```bash
python3 scripts/regen_sqlite_baseline.py --sf 0.001 \
  --output tests/data/tpch-sf001/expected/
```

This regenerates all 22 `Q*_three_way.json` files using the
`EXTRACT → strftime` rewrite and the Sprint 7 column order.

## 4. SQL compatibility

### 4.1 Strictly preserved

- All DDL that v3.8.0 accepted is still accepted (CREATE / DROP /
  ALTER TABLE, CREATE / DROP INDEX, CREATE VIEW, CREATE SCHEMA).
- All DML that v3.8.0 accepted is still accepted (INSERT VALUES,
  INSERT SELECT, UPDATE, DELETE, REPLACE, ON DUPLICATE KEY UPDATE,
  TRUNCATE).
- All joins (INNER / LEFT / RIGHT / FULL / CROSS / NATURAL / self)
  are still accepted; FULL OUTER JOIN cost model is still rough
  (status ⚠️ in [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md)).
- All aggregates (COUNT, SUM, AVG, MIN, MAX) are still accepted.
- All TPC-H 22 queries return identical row counts (verified
  cell-level in `eval_22_v_auth_sqlite.rs`).

### 4.2 New SQL accepted in v3.9.0

- `EXTRACT(YEAR FROM x)` — full SQL92 form. SQLite-rewritten to
  `CAST(strftime('%Y', x) AS INTEGER)` only inside the test
  baseline; engine accepts the SQL-standard form natively.
- Q13-pattern `NOT IN (subquery_with_LIKE)` is now tri-valued-logic
  correct (3.1).
- `EXISTS (correlated subquery)` — full correlated form, used by
  Q4 / Q17 / Q20 / Q21.

### 4.3 Still NOT accepted (v3.9.0 same as v3.8.0)

- `WITH RECURSIVE`
- `LATERAL` joins
- `ROW_NUMBER()`, `RANK()`, `LAG()`, `LEAD()` (window functions)
- `GROUP BY ... WITH ROLLUP` / `WITH CUBE`
- `JSON` / `JSONB` types and operators
- `MATCH()` full-text search
- `CREATE TRIGGER` (skeleton only, see Q-Q9 trigger path caveat)
- `CREATE FUNCTION` / `CREATE PROCEDURE` (skeleton only)
- Materialized views, partitioned tables

For the full list see [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §
"What's NOT in v3.9.0".

## 5. Storage migration

**No on-disk migration is required.** The page format, WAL record
format, and checkpoint snapshot format are byte-compatible with
v3.8.0. The engine will boot against a v3.8.0 data directory and
start serving queries immediately. The new code paths (Q9 hash-join
pre-filter, Q13 NOT IN fix) are written to produce records that
both v3.8.0 and v3.9.0 can read, so **downgrade is also safe**:
you can roll back from v3.9.0 to v3.8.0 against a v3.9.0 data
directory.

**However** — if you opt into the v3.9.0-only statement cache
(`--statement-cache-size 1024`, default), the cache is rebuilt on
the first query and the old cache file (if any) is ignored. No
manual cleanup is required.

If you have an old `pg_wal/` style directory from a v3.6 or v3.7
install, the engine will refuse to start and require a manual
`sqlrustgo admin upgrade-from-v37`. This is unchanged from v3.8.0.

## 6. Wire-protocol compatibility

The PostgreSQL v3 wire protocol is **fully compatible** in both
directions:

- A v3.8.0 `psql` (or any libpq 12+) works against a v3.9.0
  server without changes.
- A v3.9.0 `psql` (or any libpq 12+) works against a v3.8.0 server
  without changes.

New in v3.9.0 at the protocol level:

- `sslmode=verify-full` is now honored (was silently downgraded to
  `require` in v3.8.0).
- The `application_name` connection parameter is propagated to
  `pg_stat_activity` (was discarded in v3.8.0).
- `LISTEN` / `NOTIFY` (skeleton) — see
  [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md).

The TLS cipher list expanded from 4 to 8 ciphers; the old 4 are
still in the list, so no negotiation change is observable by
clients.

## 7. Configuration migration

If you have a custom `sqlrustgo.toml`, the new options (with
defaults) are:

```toml
[server]
max_connections = 200          # was: hardcoded 100
statement_cache_size = 1024   # was: hardcoded 256
listen_backlog = 256           # was: hardcoded 128

[storage]
wal_group_commit_size = 32     # new
wal_archive_dir = "/archive/wal"   # new
wal_archive_compress = "zstd"  # new, options: none|zstd|lz4

[observability]
metrics_path = "/_/metrics"   # unchanged
health_path = "/_/health"     # unchanged
log_format = "text"           # unchanged, options: text|json
```

v3.9.0 reads the v3.8.0 config silently and only emits a warning
if an unknown key is present. A new key is added (the v3.9.0
default applies) without warning.

## 8. Test-suite migration (if you maintain a custom TPC-H harness)

If you are not using the in-tree `tests/eval_22_v_auth_sqlite.rs`
or `tests/tpch_full_22_test.rs`, you may need to update the
following:

1. **Fixture size**: Sprint 7 (PR #3321) regenerated the SF=0.001
   fixture. Row counts: `customer=50, part=50, partsupp=200,
   orders=500, lineitem=501`. Update `expected_counts` accordingly.
2. **Q7/Q8/Q9 SQLite baseline**: use the `EXTRACT → strftime`
   rewrite (see §3.3) and the Sprint 7 column order.
3. **Q13 expected result**: 11/11 customers (not 60/60).
4. **Wire test mode**: `LOAD DATA LOCAL INFILE` is the canonical
   data-loading path for SF=0.001; in-process `load_table()` is
   faster for unit tests but does not exercise the wire path.

## 9. Versioning and roll-back

The engine advertises version `3.9.0` in the `server_version`
GUC (PostgreSQL convention). `psql` clients can check with
`SHOW server_version`.

To roll back from v3.9.0 to v3.8.0:

```bash
sudo systemctl stop sqlrustgo
sudo install -m 0755 sqlrustgo-v3.8.0/bin/sqlrustgo /usr/local/bin/
sudo systemctl start sqlrustgo
```

No data migration is needed (see §5). The v3.8.0 server will accept
the v3.9.0-produced WAL records and the in-memory statement cache
will be rebuilt on first query.

If you used the v3.9.0-only `application_name` propagation, that
data is just discarded by v3.8.0 — no error, no migration.

## 10. Common migration questions

**Q: I have a v3.7 database. Can I jump straight to v3.9.0?**
A: Yes, but the v3.7→v3.8 migration is mandatory first. Run
`sqlrustgo admin upgrade-from-v37` against the v3.7 data dir, then
upgrade the binary to v3.8.0, then to v3.9.0. Skipping the v3.8
step is not supported.

**Q: Do I need to recompile my application?**
A: No, unless you use the `parse_lit("TRUE")` or `TableRef.name`
APIs directly. Wire-protocol clients (psql, libpq, JDBC, ODBC)
need no recompilation.

**Q: Is there a `pg_upgrade`-style in-place tool?**
A: No. The on-disk format is unchanged, so a simple binary swap
is sufficient. A `pg_upgrade`-style tool is on the v3.10.0
roadmap.

**Q: Will v3.9.0 work with my TPC-H results from v3.8.0?**
A: No — the v3.8.0 results were generated against 20/22 PASS; the
v3.9.0 fixture has 22/22 PASS with the Q7/Q8/Q9 SQLite baseline
fixed and the Q13 fix applied. Regenerate the baseline.

**Q: What about the wire protocol's `application_name` change?**
A: If your v3.8.0 monitoring scrapes `pg_stat_activity`, the
`application_name` column was always empty. From v3.9.0 it is
populated. Adjust your dashboards.

**Q: Where is the Gitea mirror?**
A: `http://192.168.0.252:3000/openclaw/sqlrustgo`. The Git remote
is `git@gitea-macmini:openclaw/sqlrustgo.git` (SSH alias for
`192.168.0.252:222`).

## 11. See also

- [`CHANGELOG.md`](CHANGELOG.md) — full version metadata
- [`RELEASE_NOTES.md`](RELEASE_NOTES.md) — feature highlights
- [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) — current SQL support
- [`EVALUATION_REPORT.md`](EVALUATION_REPORT.md) — TPC-H numbers
- [`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md) — production deploy
- [`INSTALL.md`](INSTALL.md) — install paths
- `WIRED-22-VERIFICATION-REPORT.md` — wire-protocol 22/22
- `Q9-FIX-GATE-REPORT.md` — Q9 perf fix details
- `ROADMAP.md` — Phase 0-6 plan

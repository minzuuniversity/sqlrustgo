# SQLRustGo v4.1.0 — Migration Guide

> **Date**: 2026-10-23
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Purpose**: Migration instructions from v4.0.0 and other database systems to v4.1.0.

## 1. Migration overview

### 1.1 From v4.0.0 to v4.1.0

v4.1.0 is a **drop-in upgrade** from v4.0.0. The storage format is
backward-compatible, so existing v4.0.0 data directories can be opened
directly by v4.1.0 without conversion.

```
v4.0.0 data dir ──► v4.1.0 server opens it directly
                    (no migration script needed)
```

**Steps**:
1. Stop the v4.0.0 server
2. Replace the binary with the v4.1.0 build
3. Start the v4.1.0 server with the same data directory
4. Verify: `SELECT 1;`

**Rollback**: Replace the binary back with v4.0.0 and restart.
The data format is compatible in both directions.

### 1.2 From MySQL to v4.1.0

v4.1.0 speaks the **MySQL wire protocol** (protocol version 10).
Existing MySQL client applications can connect to v4.1.0 without
code changes, with the following caveats:

| Feature | MySQL | v4.1.0 | Notes |
|---|---|---|---|
| SELECT/INSERT/UPDATE/DELETE | ✓ | ✓ | Full support |
| CREATE TABLE / ALTER TABLE | ✓ | ✓ | Basic DDL support |
| PRIMARY KEY | ✓ | ✓ | Fast-path lookup (19×) |
| Secondary indexes | ✓ | ✗ | Not yet supported |
| Transactions (BEGIN/COMMIT) | ✓ | ✓ | Basic ACID |
| Foreign keys | ✓ | ✗ | Not enforced |
| Views | ✓ | ✗ | Not supported |
| Stored procedures | ✓ | ✗ | Not supported |
| Triggers | ✓ | ✗ | Not supported |
| Subqueries | ✓ | ✗ | WP-D deferred |
| JOIN (multi-table) | ✓ | ✗ | WP-D deferred |
| GROUP BY / HAVING | ✓ | ✗ | WP-D deferred |
| Prepared statements | ✓ | Partial | Basic support |
| INFORMATION_SCHEMA | ✓ | Partial | Limited tables |

**Migration steps (MySQL → v4.1.0)**:
1. Dump the MySQL database: `mysqldump --routines --triggers db.sql`
2. Review the SQL for unsupported features (JOIN, subqueries, etc.)
3. Rewrite unsupported queries to application-level logic
4. Import the schema: `mysql -h 127.0.0.1 -P 3306 < schema.sql`
5. Import the data: `mysql -h 127.0.0.1 -P 3306 < data.sql`
6. Run application integration tests
7. Monitor for errors in the first 24 hours

### 1.3 From PostgreSQL to v4.1.0

PostgreSQL is **not** directly compatible with the MySQL wire protocol.
Migration requires:
1. Dump PostgreSQL: `pg_dump --schema-only db > schema.sql`
2. Convert SQL syntax (see §3)
3. Import into v4.1.0
4. Rewrite application connection strings

## 2. Data format compatibility

### 2.1 Storage format

v4.1.0 uses the same on-disk format as v4.0.0:
- **WAL files**: Append-only log, 4KB sectors
- **Table files**: One file per table, row-oriented storage
- **Catalog**: JSON-based schema catalog
- **Checkpoints**: Periodic snapshot of table state

### 2.2 Version compatibility matrix

| Source version | Target v4.1.0 | Compatible | Notes |
|---|---|---|---|
| v4.0.0 | v4.1.0 | ✓ | Drop-in upgrade |
| v3.12.0 | v4.1.0 | ✓ | WAL format changed in v4.0.0, data needs re-import |
| v3.x | v4.1.0 | ✗ | Pre-WAL format, requires full re-import |

## 3. SQL syntax conversion (PostgreSQL → MySQL dialect)

| PostgreSQL | MySQL (v4.1.0) | Notes |
|---|---|---|
| `SERIAL` | `INT AUTO_INCREMENT PRIMARY KEY` | Auto-increment |
| `BOOLEAN` | `TINYINT(1)` | Boolean as integer |
| `TIMESTAMP` | `DATETIME` | Timestamp precision |
| `INTERVAL` | Not supported | Use numeric seconds |
| `ARRAY` | Not supported | Use JSON or separate table |
| `JSONB` | `JSON` | JSON column type |
| `ILIKE` | `LIKE` | Case-insensitive (MySQL default) |
| `LIMIT/OFFSET` | `LIMIT offset, count` | Syntax difference |
| `::type` | `CAST(x AS type)` | Type cast syntax |
| `NOW()` | `NOW()` | Same |
| `CURRENT_TIMESTAMP` | `CURRENT_TIMESTAMP` | Same |

## 4. Application migration checklist

| Step | Action | Verified |
|---|---|---|
| 1 | Audit current SQL queries for unsupported features | ☐ |
| 2 | Identify JOIN/subquery queries that need rewriting | ☐ |
| 3 | Plan data import order (tables with FKs first) | ☐ |
| 4 | Test schema creation on v4.1.0 | ☐ |
| 5 | Test data import with sample data | ☐ |
| 6 | Run application integration tests | ☐ |
| 7 | Run performance benchmark | ☐ |
| 8 | Set up monitoring and alerting | ☐ |
| 9 | Document rollback procedure | ☐ |
| 10 | Schedule cutover window | ☐ |

## 5. Rollback procedure

### 5.1 Rollback from v4.1.0 to v4.0.0

```bash
# 1. Stop v4.1.0
sudo systemctl stop sqlrustgo

# 2. Replace binary with v4.0.0
sudo cp /usr/local/bin/sqlrustgo.v4.0.0 /usr/local/bin/sqlrustgo

# 3. Start v4.0.0
sudo systemctl start sqlrustgo

# 4. Verify
mysql -h 127.0.0.1 -e "SELECT 1;"
```

### 5.2 Rollback from v4.1.0 to MySQL

If v4.1.0 fails in production:
1. Stop the v4.1.0 service
2. Start the MySQL service (if still running)
3. Point application connection strings back to MySQL
4. Verify data consistency

## 6. Data integrity verification

After migration, run these checks:

```sql
-- 1. Verify table count
SELECT COUNT(*) FROM information_schema.tables
WHERE table_schema NOT IN ('mysql', 'information_schema');

-- 2. Verify row counts per table
SELECT table_name, table_rows
FROM information_schema.tables
WHERE table_schema NOT IN ('mysql', 'information_schema')
ORDER BY table_rows DESC;

-- 3. Spot-check data
SELECT * FROM users LIMIT 10;
SELECT COUNT(*) FROM users;

-- 4. Verify indexes exist
SHOW INDEX FROM users;
```

## 7. Known limitations

- **No secondary indexes**: Only primary keys are supported
- **No foreign key enforcement**: Referential integrity is not enforced
- **No JOINs**: Multi-table queries must be rewritten
- **No subqueries**: Nested queries are not supported
- **No views**: View definitions are not supported
- **No stored procedures**: Procedural SQL is not supported
- **No triggers**: Trigger definitions are not supported

See `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` for the full list of
19 deferred items (WP-C..G).

## 8. Performance expectations

After migration, expect:
- **Point lookups** (by PK): 19× faster than v4.0.0 pre-fix
- **Full table scans**: Comparable to v4.0.0
- **INSERT throughput**: Comparable to v4.0.0
- **UPDATE/DELETE**: 10× faster for PK-targeted operations

See `docs/releases/v4.1.0/EVALUATION_REPORT.md` for detailed benchmarks.

## 9. Migration testing plan

### 9.1 Functional testing

| Test | Method | Pass criteria |
|---|---|---|
| Schema creation | Import schema.sql | All tables created |
| Data import | Import data.sql | Row counts match source |
| CRUD operations | Run application test suite | All tests pass |
| Concurrent access | sysbench 8-client, 1h | No data loss, no crashes |
| Recovery | Kill -9, restart | WAL replay succeeds |
| Backup/restore | Backup, restore, verify | Data identical |

### 9.2 Performance testing

| Benchmark | Target | Method |
|---|---|---|
| Point lookup (PK) | < 1 ms (20K rows) | Custom benchmark |
| INSERT throughput | > 1000 rows/s | sysbench write-only |
| SELECT throughput | > 5000 QPS | sysbench read-only |
| Mixed workload TPS | > 10 TPS | sysbench oltp_read_write |
| 8h SOAK TPS | > 10 TPS avg | SOAK loop script |

### 9.3 Acceptance criteria

The migration is accepted when:
1. All functional tests pass
2. Performance meets targets in §9.2
3. No data loss in 8h SOAK test
4. No crashes in 8h SOAK test
5. Backup/restore verified
6. Application team signs off

## 10. Migration risk assessment

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Data loss during import | Low | High | Verify row counts after import; keep source DB until verified |
| Performance regression | Medium | Medium | Run benchmarks before cutover; have rollback ready |
| Application incompatibility | Medium | High | Audit SQL queries early; rewrite unsupported features |
| Downtime exceeds window | Low | High | Test cutover in staging; prepare scripts |
| Data corruption in WAL | Very low | High | Verify WAL integrity before cutover |
| v4.1.0 bug in production | Low | High | Monitor closely in first 24h; have v4.0.0 rollback ready |

## 11. Migration timeline

| Phase | Duration | Activities |
|---|---|---|
| Assessment | 1-2 days | Audit SQL, identify unsupported features, plan rewrites |
| Development | 3-5 days | Rewrite queries, prepare import scripts, test in staging |
| Testing | 2-3 days | Functional + performance testing in staging |
| Pilot | 1 day | Import subset of data, run with real traffic |
| Cutover | 4-8 hours | Full import, switch traffic, monitor |
| Stabilization | 1-2 weeks | Monitor, fix issues, tune performance |

## 12. FAQ

**Q: Can I run v4.0.0 and v4.1.0 side by side?**
A: Yes. They use different ports. Run v4.0.0 on port 3306 and v4.1.0 on
port 3307 for A/B testing.

**Q: Do I need to re-import data when upgrading from v4.0.0?**
A: No. The storage format is backward-compatible. Just replace the binary
and restart.

**Q: What if my application uses JOINs?**
A: You'll need to rewrite JOIN queries to application-level logic. v4.1.0
does not support multi-table JOINs (WP-D deferred).

**Q: Is there a way to test migration without affecting production?**
A: Yes. Clone the data directory, run v4.1.0 on a test port, and run your
test suite against it.

**Q: How long does the cutover take?**
A: Typically 4-8 hours for a database up to 100 GB. Larger databases
may take longer depending on network speed and data volume.

**Q: Can I migrate incrementally (table by table)?**
A: Yes. Import tables one at a time, verify each, then switch application
traffic table by table. This reduces cutover window risk.

**Q: What's the minimum downtime for cutover?**
A: With a warm standby (v4.0.0 running alongside), cutover can be
under 1 hour. With cold import, expect 4-8 hours.

## 13. References

- `docs/releases/v4.1.0/INSTALL.md` — installation
- `docs/releases/v4.1.0/DEPLOYMENT_GUIDE.md` — deployment
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` — defect catalog
- `docs/releases/v4.1.0/FEATURE_MATRIX.md` — feature matrix
- `docs/releases/v4.1.0/EVALUATION_REPORT.md` — evaluation

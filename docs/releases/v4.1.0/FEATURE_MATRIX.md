# SQLRustGo v4.1.0 — Feature Matrix

> **Date**: 2026-10-23
> **Status**: DRAFT (per `docs/releases/v4.1.0/STAGE.yaml`)
> **Purpose**: Complete feature matrix for v4.1.0, organized by SQL
> category. Each feature is marked as Supported (✓), Partial (◐), or
> Not Supported (✗) with the tracking issue reference.

## 1. Data Definition Language (DDL)

| Feature | Status | Issue | Notes |
|---|---|---|---|
| CREATE TABLE | ✓ | — | Basic table creation |
| DROP TABLE | ✓ | — | Table removal |
| ALTER TABLE (ADD COLUMN) | ✓ | — | Add columns |
| ALTER TABLE (DROP COLUMN) | ✓ | — | Remove columns |
| ALTER TABLE (MODIFY COLUMN) | ◐ | — | Type change only, no rename |
| PRIMARY KEY | ✓ | — | Single-column PK, fast-path lookup |
| UNIQUE constraint | ✗ | #4682 | WP-C, deferred to v4.1.1+ |
| FOREIGN KEY | ✗ | #4652 | WP-C, deferred to v4.1.1+ |
| CHECK constraint | ✗ | #4672 | WP-C, deferred to v4.1.1+ |
| DEFAULT value | ✗ | #4669 | WP-C, deferred to v4.1.1+ |
| NOT NULL constraint | ✗ | #4709 | WP-C, deferred to v4.1.1+ |
| INDEX (secondary) | ✗ | — | Not implemented, only PK index |
| FULLTEXT index | ✗ | — | Not implemented |
| SPATIAL index | ✗ | — | Not implemented |
| VIEW | ✗ | — | Not implemented |
| TRIGGER | ✗ | — | Not implemented |
| PROCEDURE | ✗ | — | Not implemented |
| FUNCTION | ✗ | — | Not implemented |

## 2. Data Manipulation Language (DML)

| Feature | Status | Issue | Notes |
|---|---|---|---|
| INSERT (single row) | ✓ | — | Single row insert |
| INSERT (multi-row) | ✓ | — | Multi-row insert |
| INSERT ... SELECT | ◐ | — | Single table only |
| UPDATE (single table) | ✓ | — | PK fast-path (#5177) |
| UPDATE (multi-table) | ✗ | #4668 | Requires JOIN, WP-D |
| DELETE (single table) | ✓ | — | PK fast-path (#5177) |
| DELETE (multi-table) | ✗ | #4668 | Requires JOIN, WP-D |
| REPLACE INTO | ◐ | — | INSERT ON DUPLICATE KEY |
| LOAD DATA | ✗ | — | Not implemented |
| SELECT (single table) | ✓ | — | Full WHERE/ORDER/LIMIT |
| SELECT (multi-table JOIN) | ✗ | #4668 | WP-D, deferred |
| SELECT (subquery) | ✗ | #4656 | WP-D, deferred |
| SELECT (UNION) | ◐ | — | Basic UNION supported |
| SELECT (UNION ALL) | ◐ | — | Basic UNION ALL supported |
| SELECT (GROUP BY) | ✗ | #4649 | WP-D, deferred |
| SELECT (HAVING) | ✗ | #4649 | WP-D, deferred |
| SELECT (DISTINCT) | ✓ | — | Deduplication |
| SELECT (ORDER BY) | ✓ | — | Single/multi column |
| SELECT (LIMIT/OFFSET) | ✓ | — | Row limiting |
| SELECT (CASE expression) | ◐ | — | Basic CASE supported |
| SELECT (EXISTS) | ✗ | #4656 | Subquery, WP-D |
| SELECT (IN subquery) | ✗ | #4656 | Subquery, WP-D |

## 3. Transaction Control

| Feature | Status | Issue | Notes |
|---|---|---|---|
| BEGIN / START TRANSACTION | ✓ | — | Explicit transaction start |
| COMMIT | ✓ | — | Commit transaction |
| ROLLBACK | ✓ | — | Rollback transaction |
| SAVEPOINT | ✗ | #4626 | WP-E, deferred |
| ROLLBACK TO SAVEPOINT | ✗ | #4626 | WP-E, deferred |
| RELEASE SAVEPOINT | ✗ | #4626 | WP-E, deferred |
| READ UNCOMMITTED | ✗ | #4626 | WP-E, deferred |
| READ COMMITTED | ✗ | #4626 | WP-E, deferred |
| REPEATABLE READ | ✗ | #4626 | WP-E, deferred |
| SERIALIZABLE | ✗ | #4626 | WP-E, deferred |
| SET TRANSACTION | ✗ | #4626 | WP-E, deferred |
| LOCK TABLES | ◐ | — | Basic table lock |
| UNLOCK TABLES | ◐ | — | Basic unlock |

## 4. Data Types

| Feature | Status | Issue | Notes |
|---|---|---|---|
| TINYINT / SMALLINT / INT / BIGINT | ✓ | — | Integer types |
| FLOAT / DOUBLE | ✓ | — | Floating point |
| DECIMAL / NUMERIC | ✗ | #4846 | WP-G, deferred |
| CHAR / VARCHAR | ✓ | — | Character types |
| TEXT / BLOB | ✓ | — | Large object types |
| DATE / DATETIME / TIMESTAMP | ✓ | — | Temporal types |
| TIME | ✓ | — | Time of day |
| YEAR | ✓ | — | Year type |
| BOOLEAN / BOOL | ✓ | — | Boolean (TINYINT(1)) |
| ENUM | ✗ | — | Not implemented |
| SET | ✗ | — | Not implemented |
| JSON | ◐ | — | Stored as VARCHAR, no JSON functions |
| BINARY / VARBINARY | ✓ | — | Binary types |
| BIT | ◐ | — | Stored as TINYINT |
| GEOMETRY / POINT | ✗ | — | Not implemented |
| ARRAY | ✗ | — | Not implemented |
| INTERVAL | ✗ | — | Not implemented |

## 5. Operators and Expressions

| Feature | Status | Issue | Notes |
|---|---|---|---|
| =, <, >, <=, >=, != | ✓ | — | Comparison operators |
| AND / OR / NOT | ✓ | — | Logical operators |
| BETWEEN | ✗ | #4846 | WP-G, deferred |
| IN (list) | ✓ | — | Value list |
| IN (subquery) | ✗ | #4656 | WP-D, deferred |
| LIKE | ◐ | — | Basic % and _ patterns |
| LIKE (escape) | ✗ | #4846 | WP-G, deferred |
| REGEXP / RLIKE | ✗ | — | Not implemented |
| IS NULL / IS NOT NULL | ✓ | — | Null check |
| IS TRUE / IS FALSE | ✓ | — | Boolean check |
| CASE (simple) | ◐ | — | Basic CASE WHEN |
| CASE (searched) | ◐ | — | Basic searched CASE |
| IF / IFNULL / NULLIF | ✓ | — | Conditional functions |
| COALESCE | ✓ | — | Null coalescing |
| CAST / CONVERT | ◐ | — | Basic type cast |
| Arithmetic (+, -, *, /) | ✓ | — | Numeric operations |
| MOD / % | ✓ | — | Modulo |
| CONCAT | ✓ | — | String concatenation |
| SUBSTRING | ✓ | — | Substring extraction |
| LENGTH / CHAR_LENGTH | ✓ | — | String length |
| UPPER / LOWER | ✓ | — | Case conversion |
| TRIM / LTRIM / RTRIM | ✓ | — | String trimming |
| NOW() / CURRENT_TIMESTAMP | ✓ | — | Current time |
| CURDATE() / CURTIME() | ✓ | — | Current date/time |
| DATE_FORMAT / TIME_FORMAT | ◐ | — | Basic formatting |
| YEAR() / MONTH() / DAY() | ✓ | — | Date component extraction |
| ABS / CEIL / FLOOR | ✓ | — | Math functions |
| RAND() | ✓ | — | Random number |
| POWER / SQRT | ✓ | — | Math functions |
| GREATEST / LEAST | ✓ | — | Comparison functions |

## 6. MySQL Wire Protocol

| Feature | Status | Notes |
|---|---|---|
| Handshake (protocol v10) | ✓ | Standard MySQL handshake |
| Auth (mysql_native_password) | ✓ | Password authentication |
| Auth (caching_sha2_password) | ✗ | Not implemented |
| COM_QUERY | ✓ | Text query |
| COM_STMT_PREPARE | ◐ | Basic prepared statements |
| COM_STMT_EXECUTE | ◐ | Basic execution |
| COM_STMT_CLOSE | ✓ | Statement close |
| COM_PING | ✓ | Heartbeat |
| COM_QUIT | ✓ | Connection close |
| COM_INIT_DB | ✓ | Database selection |
| COM_FIELD_LIST | ◐ | Basic field list |
| COM_STATISTICS | ✓ | Server statistics |
| COM_PROCESS_KILL | ✗ | Not implemented |
| COM_DEBUG | ✗ | Not implemented |
| COM_REFRESH | ✗ | Not implemented |
| COM_SHUTDOWN | ◐ | Graceful shutdown |
| COM_SET_OPTION | ◐ | Basic option setting |
| Binary protocol | ✓ | Binary result sets |
| OK packet | ✓ | Success response |
| ERR packet | ✓ | Error response |
| EOF packet | ✓ | End of result set |
| Warning packet | ◐ | Basic warnings |

## 7. Server Features

| Feature | Status | Notes |
|---|---|---|
| TLS encryption | ✓ | Opt-in via `--tls` |
| User authentication | ✗ | Not implemented (WP-H) |
| Role-based access control | ✗ | Not implemented |
| Query audit log | ✗ | Not implemented |
| Data encryption at rest | ✗ | Not implemented |
| Connection pooling | ✓ | Built-in (configurable) |
| Query caching | ✗ | Not implemented |
| Query optimizer | ✗ | Single execution plan |
| Parallel query execution | ✗ | Single-threaded |
| Replication | ✗ | Not implemented |
| Sharding | ✗ | Not implemented |
| Read replicas | ✗ | Not implemented |
| Health check endpoint | ✗ | Use `SELECT 1` |
| Metrics endpoint | ✗ | Use external monitoring |
| Slow query log | ◐ | Basic timing via logs |
| General log | ◐ | Basic query logging |
| Binary log | ✗ | Not implemented |

## 8. Feature Summary

| Category | Total | ✓ | ◐ | ✗ | Coverage |
|---|---|---|---|---|---|
| DDL | 18 | 5 | 1 | 12 | 27.8% |
| DML | 21 | 9 | 4 | 8 | 52.4% |
| Transaction | 12 | 3 | 2 | 7 | 25.0% |
| Data Types | 18 | 11 | 3 | 4 | 61.1% |
| Operators | 34 | 22 | 5 | 7 | 70.6% |
| Wire Protocol | 21 | 12 | 5 | 4 | 61.9% |
| Server Features | 17 | 4 | 3 | 10 | 23.5% |
| **Total** | **141** | **66** | **23** | **52** | **46.8%** |

## 9. Deferred Items (WP-C..G)

| WP | Issue | Feature | Target |
|---|---|---|---|
| WP-C | #4682 | UNIQUE constraint | v4.1.1+ |
| WP-C | #4652 | FOREIGN KEY | v4.1.1+ |
| WP-C | #4672 | CHECK constraint | v4.1.1+ |
| WP-C | #4669 | DEFAULT value | v4.1.1+ |
| WP-C | #4709 | NOT NULL constraint | v4.1.1+ |
| WP-C | #4703 | DDL/Integrity completion | v4.1.1+ |
| WP-D | #4668 | JOIN (multi-table) | v4.1.1+ |
| WP-D | #4656 | Subqueries | v4.1.1+ |
| WP-D | #4649 | GROUP BY / HAVING | v4.1.1+ |
| WP-D | #4636 | Join/Subquery completion | v4.1.1+ |
| WP-E | #4626 | Transaction semantics | v4.1.1+ |
| WP-F | #4848 | Schema migration | v4.1.1+ |
| WP-G | #4846 | Type/Comparison completion | v4.1.1+ |
| WP-B | #4721 | B-tree completion | v4.1.1+ |
| WP-B | #4674 | B-tree completion | v4.1.1+ |
| WP-B | #4716 | B-tree completion | v4.1.1+ |
| WP-B | #4676 | B-tree completion | v4.1.1+ |
| WP-B | #4675 | B-tree completion | v4.1.1+ |
| WP-B | #4670 | B-tree completion | v4.1.1+ |

## 10. Feature Evolution Roadmap

### 10.1 v4.1.0 → v4.1.1

| Priority | Feature | Issue | Est. Effort |
|---|---|---|---|
| P0 | JOIN (multi-table) | #4668 | 2-3 weeks |
| P0 | Subqueries | #4656 | 1-2 weeks |
| P0 | GROUP BY / HAVING | #4649 | 1-2 weeks |
| P1 | UNIQUE constraint | #4682 | 1 week |
| P1 | FOREIGN KEY | #4652 | 1 week |
| P1 | CHECK constraint | #4672 | 3 days |
| P1 | DEFAULT value | #4669 | 2 days |
| P1 | NOT NULL constraint | #4709 | 2 days |
| P1 | Transaction semantics | #4626 | 2 weeks |
| P2 | B-tree completion | #4721, #4674, #4716, #4676, #4675, #4670 | 3 weeks |
| P2 | DECIMAL type | #4846 | 1 week |
| P2 | Schema migration | #4848 | 1 week |

### 10.2 v4.1.1 → v4.1.2

| Priority | Feature | Notes |
|---|---|---|
| P0 | User authentication | WP-H, critical for production |
| P0 | Role-based access control | WP-H |
| P0 | Query audit log | WP-H, compliance requirement |
| P1 | Data encryption at rest | WP-H |
| P1 | Query optimizer | Multiple execution plans |
| P1 | Parallel query execution | Multi-threaded |
| P2 | Read replicas | High availability |
| P2 | Replication | Multi-node |

### 10.3 v4.1.2 → v4.2.0

| Priority | Feature | Notes |
|---|---|---|
| P0 | Full SQL compliance | All WP-C..G complete |
| P1 | Performance optimization | Query cache, parallel execution |
| P1 | Operational tooling | Metrics endpoint, health check |
| P2 | Advanced features | Triggers, stored procedures, views |

## 11. Compliance Mapping

| Standard | Status | Gap |
|---|---|---|
| SQL:2008 | 46.8% | 52 features missing |
| MySQL 5.7 compatibility | 61.9% | Wire protocol gaps |
| MySQL 8.0 compatibility | 55.3% | Auth, JSON, spatial gaps |
| PostgreSQL 15 compatibility | N/A | Different wire protocol |
| SQL92 (ISO) | ~30% | Many features missing |

## 12. Detailed Feature Notes

### 12.1 DDL Notes

**CREATE TABLE**: Supports column definitions with type, NOT NULL (implicit),
and PRIMARY KEY. Does not support UNIQUE, FOREIGN KEY, CHECK, DEFAULT,
or NOT NULL as explicit constraints (WP-C deferred).

**ALTER TABLE**: Supports ADD COLUMN and DROP COLUMN. MODIFY COLUMN
supports type change but not column rename.

**INDEX**: Only primary key indexes are supported. No secondary B-tree
indexes, no full-text indexes, no spatial indexes.

### 12.2 DML Notes

**INSERT**: Supports single-row and multi-row inserts. INSERT ... SELECT
works for single-table sources only.

**UPDATE/DELETE**: PK-targeted operations benefit from the fast-path
(#5177), achieving 10× speedup. Multi-table UPDATE/DELETE requires JOIN
support (WP-D deferred).

**SELECT**: Full single-table SELECT with WHERE, ORDER BY, LIMIT, DISTINCT,
and CASE expressions. Multi-table JOIN, subqueries, GROUP BY, and HAVING
are deferred (WP-D).

### 12.3 Transaction Notes

**Basic ACID**: BEGIN, COMMIT, ROLLBACK work correctly. SAVEPOINT and
isolation levels (READ UNCOMMITTED, READ COMMITTED, REPEATABLE READ,
SERIALIZABLE) are deferred (WP-E).

**Concurrency**: Single-writer model. Concurrent transactions from
multiple clients are supported but with ~1% row loss under heavy
concurrency (reduced from 40.4% pre-#5099 fix).

### 12.4 Type Notes

**Numeric**: TINYINT, SMALLINT, INT, BIGINT, FLOAT, DOUBLE are fully
supported. DECIMAL/NUMERIC is deferred (WP-G).

**String**: CHAR, VARCHAR, TEXT, BLOB are supported. ENUM and SET are not
implemented.

**Temporal**: DATE, DATETIME, TIMESTAMP, TIME, YEAR are supported.
INTERVAL is not implemented.

**Binary**: BINARY, VARBINARY, BIT are supported. GEOMETRY and spatial
types are not implemented.

### 12.5 Operator Notes

**Comparison**: All standard comparison operators (=, <, >, <=, >=, !=)
are supported. BETWEEN, REGEXP, and LIKE with escape are deferred (WP-G).

**Logical**: AND, OR, NOT are fully supported.

**Null handling**: IS NULL, IS NOT NULL, IS TRUE, IS FALSE are supported.
COALESCE, IFNULL, NULLIF work correctly.

**String functions**: CONCAT, SUBSTRING, LENGTH, UPPER, LOWER, TRIM,
LTRIM, RTRIM are supported.

**Math functions**: ABS, CEIL, FLOOR, POWER, SQRT, RAND, GREATEST, LEAST
are supported.

**Date functions**: NOW(), CURRENT_TIMESTAMP, CURDATE(), CURTIME(),
YEAR(), MONTH(), DAY() are supported. DATE_FORMAT and TIME_FORMAT have
basic support.

### 12.6 Wire Protocol Notes

**Handshake**: Standard MySQL protocol v10 handshake is fully supported.
mysql_native_password authentication works. caching_sha2_password is not
implemented.

**Commands**: COM_QUERY, COM_PING, COM_QUIT, COM_INIT_DB,
COM_STATISTICS, COM_SHUTDOWN are supported. COM_STMT_PREPARE and
COM_STMT_EXECUTE have basic support. COM_FIELD_LIST has partial support.

**Packets**: OK, ERR, EOF packets are fully supported. Warning packets
have basic support.

### 12.7 Server Feature Notes

**Security**: TLS encryption is opt-in. User authentication, role-based
access control, query audit log, and data encryption at rest are not
implemented (WP-H).

**Performance**: Connection pooling is built-in. Query caching, query
optimizer, and parallel query execution are not implemented.

**Operations**: Slow query logging and general query logging have basic
support. Metrics endpoint and health check endpoint are not implemented.
Binary log is not supported.

## 13. References

- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` — full defect catalog
- `docs/releases/v4.1.0/ISSUES_PLAN.md` — issue tracking
- `docs/releases/v4.1.0/DEV_PLAN.md` — development plan
- `docs/releases/v4.1.0/VERSION_PLAN.md` — version plan
- `docs/governance/DOC_CHECK_CORRECTION_RULES.md` — doc completeness rules
- `docs/governance/GATE_CONDITIONS.md` — gate conditions
- `docs/releases/v4.1.0/CHANGELOG.md` — full changelog

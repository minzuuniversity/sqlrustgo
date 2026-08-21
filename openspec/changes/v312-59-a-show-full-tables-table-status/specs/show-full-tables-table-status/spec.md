# Spec — show-full-tables-table-status (V312-59-A)

> **Capability**: MySQL `SHOW [FULL] TABLES` and `SHOW TABLE STATUS` statements, fully implemented in-v3.12 (anti-deferral per issue #4384).
> **Status**: VERIFIED IMPLEMENTED (PR #4392 @ commit `78e23ddf8`, merged in develop/v3.12.0).
> **Default state**: ✅ Working — no code changes needed; this spec serves as audit record.

## ADDED Requirements

### Requirement: SHOW FULL TABLES parsing

The system SHALL parse `SHOW [FULL] TABLES [FROM db] [LIKE 'pat'] [WHERE expr]` and produce a `Statement::Show(ShowStatement::FullTables { full, db, like, where_clause })`.

#### Scenario: bare SHOW TABLES
- **WHEN** parsing `SHOW TABLES;`
- **THEN** the parser produces `ShowStatement::FullTables { full: false, db: None, like: None, where_clause: None }`
- **AND** execution returns one row per table with column `Name`

#### Scenario: SHOW FULL TABLES
- **WHEN** parsing `SHOW FULL TABLES;`
- **THEN** `full: true`
- **AND** execution returns rows with columns `Name`, `Type` (`BASE TABLE` or `VIEW`)

#### Scenario: SHOW FULL TABLES FROM db
- **WHEN** parsing `SHOW FULL TABLES FROM mydb;`
- **THEN** `db: Some("mydb")`
- **AND** execution scopes table list to `mydb` schema

#### Scenario: SHOW FULL TABLES LIKE 'pattern'
- **WHEN** parsing `SHOW FULL TABLES LIKE 'foo%';`
- **THEN** `like: Some("foo%")`
- **AND** execution applies LIKE filter

#### Scenario: SHOW FULL TABLES WHERE Table_type != 'VIEW'
- **WHEN** parsing `SHOW FULL TABLES WHERE Table_type != 'VIEW';`
- **THEN** `where_clause: Some(<binop !=>)`
- **AND** execution applies the WHERE filter against the `Table_type` column

### Requirement: SHOW TABLE STATUS parsing

The system SHALL parse `SHOW TABLE STATUS [FROM db] [LIKE 'pat'] [WHERE expr]` and produce a `Statement::Show(ShowStatement::TableStatus { db, like, where_clause })`.

#### Scenario: bare SHOW TABLE STATUS
- **WHEN** parsing `SHOW TABLE STATUS;`
- **THEN** all fields are `None`
- **AND** execution returns MySQL-compatible 18-column rows for every table

#### Scenario: SHOW TABLE STATUS FROM db LIKE 'pattern'
- **WHEN** parsing `SHOW TABLE STATUS FROM mydb LIKE 'foo%';`
- **THEN** `db: Some("mydb")`, `like: Some("foo%")`
- **AND** execution scopes + filters

#### Scenario: SHOW TABLE STATUS WHERE Name = 'foo'
- **WHEN** parsing `SHOW TABLE STATUS WHERE Name = 'foo';`
- **THEN** `where_clause: Some(<binop =>)`
- **AND** execution applies the filter

### Requirement: SHOW FULL TABLES execution

The system SHALL execute `SHOW FULL TABLES` by iterating the catalog's table list, projecting:
- `Name` (table name)
- `Type` (`BASE TABLE` for ordinary tables, `VIEW` for views — required by FULL form)
- All `WHERE`/`LIKE` filters applied against the projected rows.

#### Scenario: empty catalog
- **WHEN** the catalog has no tables
- **THEN** `SHOW FULL TABLES` returns zero rows

#### Scenario: view + table distinction
- **WHEN** the catalog contains both a regular table `t1` and a view `v1`
- **THEN** the FULL form returns 2 rows: `t1|BASE TABLE` and `v1|VIEW`

#### Scenario: WHERE filters out VIEW
- **WHEN** `SHOW FULL TABLES WHERE Table_type != 'VIEW'` runs against `t1` + `v1`
- **THEN** only `t1|BASE TABLE` is returned

### Requirement: SHOW TABLE STATUS execution (18 MySQL-compatible columns)

The system SHALL execute `SHOW TABLE STATUS` by returning rows with the following 18 MySQL-compatible columns in this exact order:

| # | Column | Type | Source |
|---|---|---|---|
| 1 | Name | TEXT | catalog table name |
| 2 | Engine | TEXT | constant `InnoDB` (v3.12 controlled subset) |
| 3 | Version | INT | constant `10` |
| 4 | Row_format | TEXT | constant `Dynamic` |
| 5 | Rows | BIGINT | live scanned row count from storage |
| 6 | Avg_row_length | BIGINT | computed: `Data_length / Rows` |
| 7 | Data_length | BIGINT | live from storage |
| 8 | Max_data_length | BIGINT | constant `0` (unlimited in v3.12) |
| 9 | Index_length | BIGINT | live from storage (0 if no indexes) |
| 10 | Data_free | BIGINT | constant `0` |
| 11 | Auto_increment | BIGINT | next auto-inc value or NULL |
| 12 | Create_time | DATETIME | catalog create timestamp |
| 13 | Update_time | DATETIME | catalog update timestamp |
| 14 | Check_time | DATETIME | NULL in v3.12 (no CHECK TABLE support yet) |
| 15 | Collation | TEXT | constant `utf8mb4_general_ci` |
| 16 | Checksum | BIGINT | NULL in v3.12 |
| 17 | Create_options | TEXT | empty string in v3.12 |
| 18 | Comment | TEXT | empty string (or table comment if set) |

#### Scenario: single empty table
- **WHEN** `t1` has 0 rows, no indexes
- **THEN** `SHOW TABLE STATUS` returns 1 row with `Rows=0`, `Avg_row_length=0`, `Data_length=0`, `Index_length=0`, `Check_time=NULL`, `Checksum=NULL`

#### Scenario: WHERE Name filter
- **WHEN** `SHOW TABLE STATUS WHERE Name = 't1'` runs against `t1` + `t2`
- **THEN** only `t1`'s row is returned

### Requirement: BETA gate integration as B6_V312_56A_R3

The system SHALL include `B6_V312_56A_R3` in `scripts/gate/check_beta_v3.12.0.sh`, asserting all five preconditions:
- `MYSQL_COMPAT_STATUS.md` mentions `SHOW FULL TABLES`
- `MYSQL_COMPAT_STATUS.md` mentions `SHOW TABLE STATUS`
- `evidence/v312-56/V312-56-VERIFICATION.md` contains `56A-R3 closed`
- `tests/integration/sql/show_full_tables_test.rs` exists
- `tests/integration/sql/show_table_status_test.rs` exists

#### Scenario: all preconditions met
- **WHEN** every precondition holds
- **THEN** `B6_V312_56A_R3` reports PASS

#### Scenario: any precondition missing
- **WHEN** any of the 5 preconditions fails
- **THEN** `B6_V312_56A_R3` reports the missing precondition by name and exits non-zero

### Requirement: MYSQL_COMPAT_STATUS.md DONE marker

The system SHALL mark the 56A-R3 row in `MYSQL_COMPAT_STATUS.md` as `✅ Supported` (not `DEFERRED → v3.13+`), with citation `V312-59-A / #4384 (56A-R3 anti-deferral)`.

#### Scenario: 56A-R3 marked DONE
- **WHEN** `grep -E "SHOW FULL TABLES|SHOW TABLE STATUS" docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` matches both
- **AND** the row contains `✅ Supported`
- **AND** the row references `#4384`
- **THEN** B6_V312_56A_R3 PASS

#### Scenario: marker regressed to DEFERRED
- **WHEN** the row shows `DEFERRED` again
- **THEN** B6_V312_56A_R3 reports FAIL with "marker regressed to DEFERRED" reason

### Requirement: V312-56-VERIFICATION.md closure marker

The system SHALL include the string `56A-R3 closed` in `docs/releases/v3.12.0/evidence/v312-56/V312-56-VERIFICATION.md`.

#### Scenario: closure marker present
- **WHEN** `grep "56A-R3 closed" V312-56-VERIFICATION.md` matches
- **THEN** the audit record is verified (B6_V312_56A_R3 PASS)

### Requirement: PR workflow with anti-deferral policy prefix

Any PR implementing this work MUST use the commit-message prefix `[V312-59-A][V312-56A-R3]` to signal anti-deferral policy compliance per Anti-Fabrication-Policy-v1.0 §5.

#### Scenario: prefix present
- **WHEN** commit message starts with `[V312-59-A][V312-56A-R3]`
- **THEN** the commit is compliant with the anti-deferral audit policy

#### Scenario: prefix absent
- **WHEN** commit message lacks the `[V312-59-A][V312-56A-R3]` prefix
- **THEN** CI must reject the PR (deferral-policy gate)
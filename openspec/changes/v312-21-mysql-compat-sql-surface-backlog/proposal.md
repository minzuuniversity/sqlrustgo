## Why

V312-21 / ISSUE #3908 re-verifies the v3.7-v3.10 documented MySQL-compatibility and SQL-surface gaps, with the explicit rule from the master issue (#3887): "对 GMP/生产路径相关子集给出 fixture PASS；非目标项必须输出 explicit unsupported 或 deferred decision，不得在 release note 中无边界宣称支持". The historical record (per `docs/releases/v3.11.0/ARCHITECTURE.md` line 55, `DEVELOPMENT_PLAN.md` §4.4, and the v3.7.0/v3.8.0 frozen-list) lists 10 surfaces that may or may not still hold: SHOW TABLES / metadata, empty-password auth edge, prepared statements, ALTER TABLE RENAME/MODIFY/ADD/DROP, TIMESTAMP, connection pool, stored procedure tokens, column-level permissions, ROLLUP/CUBE/REPLACE/RANK, advanced aggregates. v3.11.0 already shipped V311-09 (column-level permissions) and V311-13 (ALTER RENAME/MODIFY), so this change is *re-verification + non-target disposition*, not first-time implementation.

## What Changes

- **Disposition table** at `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` with one row per surface: `surface | previous_version_claim | current_evidence | decision (PASS | unsupported | deferred) | evidence_hash | owner | expiry`.
- **Fixtures for GMP-critical subset** (`SHOW TABLES`, `ALTER TABLE RENAME`, `ALTER TABLE ADD COLUMN`, `ALTER TABLE DROP COLUMN`, `empty-password auth edge`, `prepared statement round-trip`): each as a `.sql` + expected `.out` file under `tests/compat/mysql_v3_12/` with a runner at `scripts/gate/check_v312_21_mysql_compat.sh`.
- **Explicit unsupported decisions** for `stored procedure tokens`, `column-level permissions on non-V311-09 columns`, `ROLLUP/CUBE/REPLACE/RANK`, advanced aggregates beyond V311-15/V311-16/V311-17 scope. Each decision is paired with: a failing fixture that proves the surface is *not* silently mishandled, and a one-line release-note boundary.
- **Deferral decisions with owner + expiry** for surfaces that have a real cost/benefit (e.g., TIMESTAMP type expansion, connection pool rewire): each deferral includes a future issue link, owner login, and an expiry date ≤ 2027-06-30.
- **Wire-out non-target boundary in release notes**: the v3.12.0 `RELEASE_NOTES.md` "MySQL compatibility" section must reference the disposition table, not duplicate the v3.11.0 claim.
- **No new SQL features** are added. If a fixture unexpectedly PASSes for a previously-unsupported surface, the change opens a follow-up issue instead of silently upgrading the disposition row.

## Capabilities

### New Capabilities

- `mysql-compat-surface-disposition`: machine-readable and human-readable disposition table for the 10 v3.7-v3.10 historical surfaces, with explicit `unsupported` and `deferred` decisions and ownership.
- `mysql-compat-fixture-suite`: SQL fixture + expected output pairs for the GMP-critical subset, runnable as a gate.

### Modified Capabilities

- `release-notes-mysql-compat-section`: must reference the disposition table; the previous "supports X" wording is removed unless X has a PASS row.

## Impact

- **New**: `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` (generated).
- **New**: `tests/compat/mysql_v3_12/*.sql` + `*.out` (fixtures for the GMP-critical subset).
- **New**: `scripts/gate/check_v312_21_mysql_compat.sh` (driver).
- **Modified**: `docs/releases/v3.12.0/RELEASE_NOTES.md` MySQL compatibility section to point at the disposition table.
- **No new crate deps**, no parser/optimizer changes. The change is *test + governance*, not implementation.
- **Risk**: if a fixture reveals that a v3.11.0-claimed surface is *actually* broken in v3.12.0, this change records that and opens a follow-up issue, rather than masking the regression. Per the master issue, the disposition must be honest even when inconvenient.

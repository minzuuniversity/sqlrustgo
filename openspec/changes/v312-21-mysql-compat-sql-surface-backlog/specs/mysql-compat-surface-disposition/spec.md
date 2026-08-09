## ADDED Requirements

### Requirement: Disposition table schema MUST be parseable

`docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` SHALL
be a markdown table with these columns, in this order:

| surface | previous_claim | current_evidence | decision | evidence_hash | owner | expiry |

The `decision` column SHALL be one of:

- `PASS` — the surface is exercised by a fixture in `tests/compat/mysql_v3_12/`
  and the fixture passes.
- `unsupported` — the surface is exercised by a fixture and the fixture
  returns a `UNSUPPORTED: <reason>` line; the test runner still records
  the row, not as a failure.
- `deferred` — the surface is recorded with a follow-up issue link, an
  owner (gitea login), and an expiry date (ISO8601) ≤ 2027-06-30.

A row SHALL NOT exist without one of those three `decision` values.

#### Scenario: Disposition file is parseable

- **WHEN** `assert_reviewer_signoff.sh` (or equivalent) reads
  `SURFACE_DISPOSITION.md`
- **THEN** every row SHALL have a `decision` in `{PASS, unsupported, deferred}`
- **AND** rows with `decision=deferred` SHALL have a non-empty `owner` and
  an `expiry` ≤ 2027-06-30

### Requirement: Disposition MUST cover the 10 v3.7-v3.10 surfaces

The disposition SHALL contain at least these 10 surfaces from the
v3.7-v3.10 historical record:

1. `SHOW TABLES` / metadata
2. empty-password auth edge
3. prepared statements (V311-vintage round-trip)
4. ALTER TABLE RENAME / MODIFY / ADD / DROP
5. TIMESTAMP
6. connection pool
7. stored procedure tokens
8. column-level permissions
9. ROLLUP / CUBE / REPLACE / RANK
10. advanced aggregates

If a surface is intentionally not re-verified in v3.12.0, the row's
`decision=deferred` or `decision=unsupported` with a one-line rationale.

#### Scenario: All 10 surfaces present

- **WHEN** the disposition is generated
- **THEN** it SHALL contain exactly one row per surface in the list above

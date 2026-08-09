## ADDED Requirements

### Requirement: empty_password_auth fixture MUST test empty password authentication

The fixture file `tests/compat/mysql_v3_12/empty_password_auth.sql` SHALL:

1. Declare `# name: empty_password_auth` directive.
2. Declare `# expect: PASS` or `# expect: UNSUPPORTED:<reason>` directive.
3. Contain SQL statements that verify authentication behavior with empty password.

#### Scenario: Fixture tests empty password connection

- **WHEN** the runner executes `empty_password_auth.sql`
- **THEN** it SHALL connect using an empty password credential
- **AND** record the authentication result (allowed or denied)
- **AND** update the `SURFACE_DISPOSITION.md` row accordingly

### Requirement: Fixture output MUST be deterministic

The `tests/compat/mysql_v3_12/empty_password_auth.out` file SHALL:

1. Contain no timestamps, sequence numbers, or non-deterministic values.
2. Contain the exact row output expected from the SQL execution.
3. Match byte-for-byte when re-run against the same server state.

#### Scenario: Fixture output is deterministic

- **GIVEN** `empty_password_auth.sql` with `# expect: PASS`
- **WHEN** the runner executes the fixture twice with the same server state
- **THEN** both runs SHALL produce identical `.out` content
- **AND** both runs SHALL produce the same `evidence_hash`

### Requirement: Disposition row MUST reflect actual fixture run

The `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` row for `empty_password_auth` SHALL:

1. Have `decision` column updated from `deferred` to the fixture result (`PASS` or `unsupported`).
2. Have `evidence_hash` column contain the SHA256 of the fixture run log.
3. Have `owner` column remain `openclaw`.
4. Have `expiry` column updated or removed based on decision.

#### Scenario: Disposition row reflects PASS result

- **GIVEN** `empty_password_auth.sql` with `# expect: PASS` and matching `.out`
- **WHEN** the runner completes successfully
- **THEN** the disposition row SHALL show `decision=PASS`
- **AND** `evidence_hash` SHALL contain the SHA256 of the run log

#### Scenario: Disposition row reflects unsupported result

- **GIVEN** `empty_password_auth.sql` with `# expect: UNSUPPORTED: empty password auth not implemented`
- **WHEN** the runner captures the unsupported error
- **THEN** the disposition row SHALL show `decision=unsupported`
- **AND** `evidence_hash` SHALL contain the SHA256 of the error output

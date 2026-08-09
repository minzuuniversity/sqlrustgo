## ADDED Requirements

### Requirement: Fixture grammar MUST be self-describing

A fixture file `tests/compat/mysql_v3_12/<name>.sql` SHALL be a UTF-8
text file consisting of:

1. Optional `# name: <short name>` directive.
2. A required `# expect: <decision>` directive where `<decision>` is
   `PASS` or `UNSUPPORTED:<reason>` or `DEFERRED:<issue-link>`.
3. SQL statements, semicolon-separated.
4. Optional trailing blank lines.

The expected output file `tests/compat/mysql_v3_12/<name>.out` SHALL be
the canonical row output of running the SQL through `MySqlTestClient`.

#### Scenario: Fixture parses

- **WHEN** the runner opens `show_tables.sql`
- **THEN** it SHALL extract the `# expect:` directive and the SQL body
- **AND** reject fixtures without the directive (exit 2)

### Requirement: Runner semantics MUST distinguish PASS from UNSUPPORTED

The runner (either `tests/compat_runner.rs` or a new binary) SHALL:

1. For each `.sql` file in the fixture directory:
   1. Read the `# expect:` directive.
   2. Connect to an ephemeral server.
   3. Execute the SQL.
   4. Compare the captured row output (semicolon-joined, no trailing
      whitespace) against the `.out` file content.
   5. If `# expect: PASS`, the comparison MUST match exactly.
   6. If `# expect: UNSUPPORTED:*`, the comparison is informational; the
      runner records the row and the test still passes when the server
      returns the documented unsupported error message.
   7. If `# expect: DEFERRED:*`, same as UNSUPPORTED, but the runner
      additionally verifies the linked issue is reachable.

#### Scenario: PASS fixture runs

- **GIVEN** `show_tables.sql` with `# expect: PASS` and a matching `show_tables.out`
- **WHEN** the runner executes the fixture
- **THEN** it records a `PASS` row in `SURFACE_DISPOSITION.md`
- **AND** exits 0

#### Scenario: UNSUPPORTED fixture runs

- **GIVEN** `with_rollup_unsupported.sql` with `# expect: UNSUPPORTED: WITH ROLLUP not implemented`
- **WHEN** the runner executes the fixture
- **THEN** the server SHALL return the documented error
- **AND** the runner records an `unsupported` row with the reason in
  `SURFACE_DISPOSITION.md`
- **AND** the runner exits 0 (this is *not* a test failure)

### Requirement: Coverage of GMP-critical subset MUST be exercised

The fixture suite SHALL include `.sql` files for at least:

- `show_tables.sql`
- `alter_rename.sql`
- `alter_add_column.sql`
- `alter_drop_column.sql`
- `alter_modify_column.sql`
- `empty_password_auth.sql`
- `prepared_stmt_roundtrip.sql`

Each of these SHALL have `# expect: PASS` and a matching `.out` file.

#### Scenario: GMP-critical subset is exercised

- **WHEN** the runner walks the fixture directory
- **THEN** the 7 fixture files above SHALL each produce a `PASS` row

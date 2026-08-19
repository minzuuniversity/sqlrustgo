# sqlite3-like 一体化教学 CLI Specification

## Purpose

Define the contract for `sqlrustgo` / `sqlrustgo-cli` sqlite3-like local mode that supports BustubX-EDU 前 4-6 周 acceptance without requiring a MySQL server, port, or user account.

## Requirements

### Requirement: Local CLI Subcommand

The `sqlrustgo-cli` binary MUST support a `local` subcommand (or positional db-path dispatch) that:
1. Accepts a single positional db-path argument (e.g., `edu.db`).
2. Treats the path as a SQLRustGo-managed directory (`FileStorage`).
3. Persists state across invocations.
4. Does NOT require a running MySQL server, port, or user account.

#### Scenario: db-path positional dispatch
- **WHEN** the user invokes `sqlrustgo edu.db`
- **THEN** the binary MUST treat `edu.db` as a single-path database handle
- **AND** MUST create the `edu.db/` directory if it does not exist
- **AND** MUST instantiate `ExecutionEngine<FileStorage>` against that path
- **AND** MUST drop to interactive readline mode if stdin is a TTY
- **AND** MUST drop to batch mode if stdin is not a TTY

#### Scenario: Single SQL argument
- **WHEN** the user invokes `sqlrustgo edu.db "SELECT 1;"`
- **THEN** the binary MUST execute the SQL as a single statement
- **AND** MUST print results in default (table) mode to stdout
- **AND** MUST exit with code 0 on success, non-zero on parse/bind/runtime error

#### Scenario: Batch from stdin
- **WHEN** the user invokes `sqlrustgo edu.db < script.sql`
- **THEN** the binary MUST read all stdin lines
- **AND** MUST execute each statement as it is read
- **AND** MUST exit with code 0 only if all statements succeed (default fail-fast)
- **AND** MUST exit with non-zero code if any statement fails (default)

#### Scenario: --continue-on-error flag
- **WHEN** the user invokes `sqlrustgo --continue-on-error edu.db < script.sql`
- **THEN** the binary MUST continue executing statements after an error
- **AND** MUST write each error to stderr with stable prefix
- **AND** MUST exit with non-zero code if any statement failed

### Requirement: Output Modes

The CLI MUST support 4 output modes: `table` (default), `list`, `csv`, `json`. Each mode MUST be activated via `.mode MODE` meta-command or `--json` / `--csv` flags.

#### Scenario: Default table mode
- **WHEN** the user runs `SELECT id, name FROM t` in default mode
- **THEN** output MUST be:
  ```
  id | name
  ---+----
  1  | alice
  2  | bob
  ```
- **AND** column widths MUST auto-size to the data
- **AND** headers MUST be on by default

#### Scenario: List mode
- **WHEN** the user runs `.mode list` then `SELECT id, name FROM t`
- **THEN** output MUST be one row per line with `|` separator (no headers by default unless `.headers on`)
- **AND** field separator MUST be `|`

#### Scenario: CSV mode
- **WHEN** the user runs `.mode csv` then `SELECT id, name FROM t`
- **THEN** output MUST follow RFC 4180:
  - First row MUST be column headers (unless `.headers off`)
  - Fields MUST be quoted if they contain `,`, `"`, or newline
  - Embedded `"` MUST be escaped as `""`
  - Row terminator MUST be `\n`

#### Scenario: JSON mode
- **WHEN** the user runs `--json` or `.mode json` then `SELECT id, name FROM t`
- **THEN** output MUST be valid JSON:
  ```json
  {"columns":["id","name"],"rows":[[1,"alice"],[2,"bob"]]}
  ```
- **AND** MUST be deterministic (stable field order via serde)

### Requirement: Meta-Commands

The CLI MUST recognize the following meta-commands when input starts with `.`:

| Command | Effect |
|----------|--------|
| `.help` | Print usage to stderr; continue |
| `.quit` / `.exit` | Exit with code 0 |
| `.tables` | List all user tables to stdout |
| `.schema [TABLE]` | Print CREATE TABLE for TABLE (or all if omitted) |
| `.mode MODE` | Switch output mode (table/list/csv/json) |
| `.headers on|off` | Toggle column headers |
| `.read FILE` | Execute SQL from FILE |
| `.output FILE` | Redirect subsequent output to FILE |
| `.output stdout` | Resume stdout output |
| `.timer on|off` | Toggle wall-clock timing to stderr |
| `.explain on|off` | Prepend EXPLAIN to subsequent SELECTs |

#### Scenario: `.tables` meta-command
- **WHEN** the user types `.tables` in interactive mode
- **THEN** the CLI MUST execute the equivalent of `SELECT name FROM <catalog> ORDER BY name`
- **AND** MUST print results in current output mode

#### Scenario: `.schema users` meta-command
- **WHEN** the user types `.schema users` after `CREATE TABLE users (id INT, name TEXT)`
- **THEN** the CLI MUST print `CREATE TABLE users (id INT, name TEXT)` to stdout
- **AND** MUST exit with code 0

#### Scenario: `.read other.sql` meta-command
- **WHEN** the user types `.read other.sql` and the file exists
- **THEN** the CLI MUST execute each statement in `other.sql` in order
- **AND** MUST respect current output mode and `--continue-on-error`

#### Scenario: Unknown meta-command
- **WHEN** the user types `.foobar` (unknown)
- **THEN** the CLI MUST write to stderr: `sqlrustgo:error:meta: unknown command '.foobar'`
- **AND** MUST exit with code 1

### Requirement: Stable Error Prefixes

The CLI MUST emit stable stderr prefixes for error classes:

| Class | Prefix | Exit code |
|-------|--------|-----------|
| Parse | `sqlrustgo:error:parse:` | 1 |
| Bind | `sqlrustgo:error:bind:` | 1 |
| Runtime | `sqlrustgo:error:runtime:` | 1 |
| Meta | `sqlrustgo:error:meta:` | 1 |

#### Scenario: Parse error
- **WHEN** the user invokes `sqlrustgo edu.db "SELECT FROM;"` (syntax error)
- **THEN** stderr MUST start with `sqlrustgo:error:parse:`
- **AND** exit code MUST be 1

#### Scenario: Bind error (column not found)
- **WHEN** the user runs `SELECT nonexistent_col FROM t`
- **THEN** stderr MUST start with `sqlrustgo:error:bind:`
- **AND** exit code MUST be 1

#### Scenario: Runtime error (table not found)
- **WHEN** the user runs `SELECT * FROM nonexistent_table`
- **THEN** stderr MUST start with `sqlrustgo:error:runtime:`
- **AND** exit code MUST be 1

### Requirement: Persistence Across Processes

The CLI MUST persist CREATE/INSERT/UPDATE/DELETE state across independent process invocations.

#### Scenario: Process-1 writes, Process-2 reads
- **WHEN** Process 1 runs `sqlrustgo edu.db "CREATE TABLE t(id INT); INSERT INTO t VALUES(1);"`
- **AND** Process 2 runs `sqlrustgo edu.db "SELECT id FROM t;"`
- **THEN** Process 2 MUST return `1` (the row inserted by Process 1)
- **AND** both processes MUST exit with code 0

### Requirement: --help Shows Local Mode

When the user runs `sqlrustgo --help` or `sqlrustgo-cli --help`, the output MUST include sqlite3-like local mode documentation.

#### Scenario: --help lists local mode
- **WHEN** the user runs `sqlrustgo --help`
- **THEN** the output MUST include a section titled "Local database (sqlite3-like)" or equivalent
- **AND** MUST list at least the positional db-path, `--batch`, `--json`, `--csv`, `--continue-on-error` flags

### Requirement: V312-57 Gate Script Exists and Passes

The repository MUST contain `scripts/gate/check_bustubx_edu_cli_v312.sh` which exits 0 when run against the current codebase.

#### Scenario: Gate script passes
- **WHEN** `bash scripts/gate/check_bustubx_edu_cli_v312.sh` runs
- **THEN** the script MUST execute all manifest cases
- **AND** MUST produce `target/check_bustubx_edu_cli_v312.log`
- **AND** MUST exit with code 0

#### Scenario: Gate script catches real failures
- **WHEN** a manifest case has wrong expected output
- **THEN** the gate MUST exit non-zero
- **AND** the log MUST show which case failed and why

### Requirement: Manifest Contains week01-week04

`tests/compat/bustubx_edu_sqlite_cli/manifest.yml` MUST list at least 12 cases covering week01-week04.

#### Scenario: manifest has week01-week04
- **WHEN** the manifest is parsed
- **THEN** it MUST contain cases for week01 (install / first SQL), week02 (CRUD), week03 (persistence + meta-commands), week04 (parser/binder/catalog errors)
- **AND** each case MUST have: case_id, week, sql_file, expected_output_file (or oracle_mode=none), expected_exit_code

### Requirement: No SQLite Binary Format Compatibility

The CLI MUST NOT claim or implement SQLite binary file format compatibility.

#### Scenario: Documentation clarifies scope
- **WHEN** a user reads the V312-57 docs
- **THEN** the docs MUST state: "This CLI replaces the `sqlite3` CLI usage experience; it does NOT replace the SQLite file format"
- **AND** the docs MUST NOT claim `.sqlite` file compatibility
# V312-57: sqlite3-like 一体化教学 CLI — design

## Architecture Overview

Add a new `local` mode to `sqlrustgo-cli` that bypasses the MySQL server entirely and drives `ExecutionEngine<FileStorage>` directly. The local mode owns:

- FileStorage lifecycle (per db-path)
- Output formatting (table / list / csv / json)
- Meta-command parsing (lines starting with `.`)
- Stable exit codes (0 / 1)
- Stable stderr prefixes (parse / bind / runtime / meta)

```
crates/sqlrustgo-cli/
  Cargo.toml:  + [[bin]] name="sqlrustgo" (alongside existing sqlrustgo-cli)
  Cargo.toml:  + deps: sqlrustgo-storage, sqlrustgo-parser, anyhow, serde, serde_json, chrono
  src/lib.rs:  + SubCmd::Local { db_path, sql, cmd, batch, json, csv, continue_on_error }
  src/lib.rs:  ~ route Local → run_local()
  src/local.rs:    NEW — FileStorage bind + ExecutionEngine + output + meta dispatch
  src/output.rs:   NEW — format_table / format_list / format_csv / format_json
  src/meta.rs:     NEW — parse_meta_command + dispatcher
  src/error.rs:    NEW — Error enum with stable stderr prefixes
```

## Data flow

```
$ sqlrustgo edu.db "SELECT id FROM t;"
  ↓
clap parse → SubCmd::Local { db_path="edu.db", sql=Some("SELECT id FROM t;"), .. }
  ↓
run_local()
  ├─ FileStorage::new("edu.db/")                  ← creates dir if missing
  ├─ ExecutionEngine<FileStorage>::new(arc)
  ├─ parse("SELECT id FROM t;") → SelectStatement  (parse_error if fails)
  ├─ engine.execute_select(&select)                (bind_error / runtime_error if fails)
  ├─ get_columns_from_table_info("t")              ← vec!["id"]
  └─ output::format_table(columns, rows, headers=true) → stdout
  exit code: 0
```

## FileStorage as db-path

`FileStorage::new(PathBuf)` accepts a directory path. We adopt `edu.db/` as the canonical storage layout:

```
edu.db/                          # CLI treats "edu.db" as a directory handle
├── catalog.json                   # table metadata
├── <table_name>.json              # table data
└── wal/                           # WAL files (if enabled)
```

If the user runs `sqlrustgo edu.db` and `edu.db` exists as a regular file (not a directory), the CLI MUST exit code 1 + stderr `sqlrustgo:error:runtime: edu.db exists but is not a directory`. This is documented in § Acceptance criteria.

## Output formatting contract

| Mode | Header row | Field separator | Row terminator | Value escaping |
|------|-----------|-----------------|----------------|----------------|
| table | column names | ` \| ` | `\n` | none (raw Value Display) |
| list | off by default | `\|` | `\n` | none |
| csv | first row if `.headers on` (default) | `,` | `\n` | RFC 4180: `"` → `""`, wrap with `"` if `,` or `"` or `\n` |
| json | n/a (object) | n/a | `\n` | serde_json::to_string |

Headers default: ON for table / csv, OFF for list / json (matching sqlite3 CLI behavior).

## Meta-command set

`MetaCommand::parse(input)` returns `Some(meta)` for lines starting with `.`, `None` for SQL. State mutations happen on the `LocalContext` struct:

```rust
struct LocalContext {
    mode: OutputMode,
    headers: bool,
    timer: bool,
    explain: bool,
    output: OutputSink,        // stdout | file path
    last_error: Option<Error>,
    statement_count: usize,
}
```

| Command | Mutates | Notes |
|---------|---------|-------|
| `.help` | none | print to stderr |
| `.quit` / `.exit` | none | return ControlFlow::Break |
| `.tables` | output (read-only) | execute internal SELECT |
| `.schema [table]` | output (read-only) | execute internal SELECT (filtered) |
| `.mode MODE` | mode | parse OutputMode enum |
| `.headers on|off` | headers | parse bool |
| `.read FILE` | executes side effects | read file, execute each line |
| `.output FILE` | output | open file, redirect subsequent stdout |
| `.output stdout` | output | restore stdout |
| `.timer on|off` | timer | measure wall-clock per statement |
| `.explain on|off` | explain | prepend EXPLAIN to SELECT |

## Error class → stderr prefix

```rust
enum Error {
    Parse(String),    // → stderr "sqlrustgo:error:parse: ..."
    Bind(String),     // → stderr "sqlrustgo:error:bind: ..."
    Runtime(String),  // → stderr "sqlrustgo:error:runtime: ..."
    Meta(String),     // → stderr "sqlrustgo:error:meta: ..."
    Io(String),       // → stderr "sqlrustgo:error:io: ..."
}
```

All Error variants exit code 1. Unknown meta-commands exit code 1.

## `--continue-on-error` semantics

```rust
for stmt in stdin.lines() {
    let start = Instant::now();
    let result = engine.execute(&stmt);
    if result.is_err() {
        eprintln!("{}", Error::from(result.unwrap_err()));
        last_exit_code = 1;
        if !continue_on_error {
            return last_exit_code;
        }
    }
    if timer {
        eprintln!("Run Time: {:?}", start.elapsed());
    }
}
return last_exit_code;
```

Default behavior: fail-fast (`!continue_on_error`).

## Gate script architecture

`scripts/gate/check_bustubx_edu_cli_v312.sh`:

```
1. cargo build -p sqlrustgo-cli --all-features (exit 0?)
2. cargo run -p sqlrustgo-cli -- --help (contains "local" or "sqlite3"?)
3. parse manifest.yml (≥ 12 cases, week01-week04 present?)
4. for each case in manifest:
   4.1. tmp_db = $(mktemp -d)/edu.db
   4.2. actual_stdout = $(sqlrustgo "$tmp_db" < case.sql 2>/dev/null)
   4.3. actual_exit = $?
   4.4. compare actual_stdout to case.expected_output_file (or run sqlite3 oracle)
   4.5. compare actual_exit to case.expected_exit_code
5. sha256sum all artifacts → target/check_bustubx_edu_cli_v312.log
6. exit 0 (if all pass)
```

No `|| true` masking; each step's failure MUST halt the gate.

## Test fixture format

`tests/compat/bustubx_edu_sqlite_cli/manifest.yml`:

```yaml
cases:
  - case_id: week01_help
    week: week01
    sql_file: week01/week01_help.sql
    expected_output_file: week01/week01_help.golden
    expected_exit_code: 0
    oracle_mode: golden
    notes: "First SELECT 1"
  # ... 11 more cases
```

Each `.golden` file contains expected stdout. Comparison is byte-exact.

## Dependencies

- `sqlrustgo-storage` — `FileStorage` (already exists)
- `sqlrustgo-parser` — `parse()` (already exists)
- `ExecutionEngine<FileStorage>` — `engine.execute()` (already exists)
- No new crates

## Risk analysis

1. **Path conflict** — `edu.db` as both file and directory. We mandate directory-only; conflicting file → exit 1.
2. **Error prefix stability** — exact strings matter for golden tests; lock down in spec.
3. **Stdin detection** — `is_terminal()` from Rust stdlib; non-TTY → batch mode. Document interactive invocation explicitly.
4. **Concurrent processes** — FileStorage uses WAL; two processes writing same path is unsupported but not catastrophic. Document this limitation.
5. **Long-running statements** — no statement timeout in V312-57 scope; document as known limitation.

## OpenSpec compliance

This change follows the openspec pattern:
- `proposal.md` — Why / What / Acceptance / Disposition
- `tasks.md` — Phased checklist (Phase 1-6)
- `design.md` — Architecture + contracts (this file)
- `specs/sqlite-style-edu-cli/spec.md` — Requirement/Scenario format

On merge to `develop/v3.12.0`, this change will be archived to `openspec/specs/sqlite-style-edu-cli/`.
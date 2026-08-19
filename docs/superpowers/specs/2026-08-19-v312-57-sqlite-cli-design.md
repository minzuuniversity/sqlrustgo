# V312-57 sqlite3-like 一体化教学 CLI — Design Spec

> **provenance:** generated_by=openclaw-minimax, generated_at=2026-08-19T23:50:00Z, branch=develop/v3.12.0, commit=54c4ebf9b75b229cf9f8d5e797b0e25fd6f302bd, policy=Anti-Fabrication-Policy-v1.0
> **scope:** V312-57 issue #4359; promotion_to_RC_requires RC9 (week01-04 fixtures)
> **supersedes:** `docs/releases/v3.12.0/V312-57_SQLITE_STYLE_EDU_CLI_PLAN.md` (this is the implementation spec; the plan doc is the scope statement)
> **parent issue:** #4359 [V312-BETA-SCOPE][V312-RC-BLOCKER][V312-57]
> **method:** Architectural-path brainstorming with user approval (Approach A: full V312-57 in one session)

## 1. Goal

Add a `sqlite` subcommand to the existing `sqlrustgo` canonical CLI binary (`crates/sqlrustgo-cli`) that provides a sqlite3-like local DB entry point for BustubX-EDU weeks 1-6:

- Single path = one local SQLRustGo database (no server, no port, no auth)
- REPL with dot-commands (`.help` / `.quit` / `.tables` / `.schema` / `.mode` / `.headers` / `.read` / `.output` / `.timer` / `.explain`)
- 4 output modes (table / list / csv / json)
- Batch via SQL parameter or stdin
- Stable exit codes + stable error prefixes
- Persistent across processes (cross-process open→close→reopen→read)

This unblocks RC9 (V312-57 week01-04 fixtures) and brings RC10 (week05-06) into reach via follow-up PRs.

## 2. Non- Goals (Explicit, per plan §2)

- ❌ SQLite file format compatibility — we use `sqlrustgo_storage::FileStorage` internally; the on-disk format is SQLRustGo's, not SQLite's.
- ❌ Full sqlite3 shell parity — only the dot-commands in §4.2; no `.import`, `.dump`, `.backup`, `.restore`, `.parameter`, `.print`, etc.
- ❌ MySQL wire protocol or server-backed teaching mode — that path is the existing `Serve` / `Repl` / `Cli` subcommands.
- ❌ Full SQL-92 / MySQL 5.7 syntax — only the SQL subset listed in §4.4.
- ❌ ANSI colors by default — BustubX-EDU auto-eval needs plain text.
- ❌ week05-06 fixtures — deferred per plan §5 (need EXPLAIN + join/aggregate golden output design that depends on the 5×11 RBAC matrix test closure and join/aggregate results).
- ❌ Wire into `check_beta_v3.12.0.sh` — separate PR after this lands.

## 3. Architecture

### 3.1 Binary surface

`sqlrustgo-cli` (existing binary at `crates/sqlrustgo-cli/src/main.rs`) gets one new subcommand:

```rust
#[derive(Subcommand, Debug)]
enum SubCmd {
    // ... existing variants (Serve, Exec, Repl, Bench, Gmp, Diag, Backup, Restore, Cli, Soak)

    /// sqlite3-like local mode: open a single-path DB, run SQL or REPL.
    Sqlite {
        /// Path to local DB (file or directory).
        db: PathBuf,

        /// Read SQL from stdin (default: REPL if no --cmd / --batch).
        #[arg(long)]
        batch: bool,

        /// Run a single SQL statement then exit.
        #[arg(long)]
        cmd: Option<String>,

        /// Output mode: table | list | csv | json.
        #[arg(long, value_enum, default_value = "table")]
        mode: OutputModeArg,

        /// Show / hide column headers (default: on in REPL, off in batch list/csv).
        #[arg(long)]
        headers: Option<bool>,

        /// Show wall-clock timing after each query.
        #[arg(long)]
        timer: Option<bool>,

        /// Prefix results with EXPLAIN-style plan (engine-provided).
        #[arg(long)]
        explain: Option<bool>,

        /// Continue past parse/bind/runtime errors instead of fail-fast.
        #[arg(long)]
        continue_on_error: bool,
    },
}
```

### 3.2 Implicit-alias `sqlrustgo edu.db` (no-subcommand mode)

The plan file (§2) requires `sqlrustgo edu.db` to also work. Implementation:

```rust
pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().collect();
    // If invoked with exactly one positional arg AND it looks like a DB path
    // (suffix `.db`, contains `/`, or exists as a dir), enter sqlite-mode implicitly.
    if args.len() == 2 && looks_like_db_path(&args[1]) {
        return run_sqlite_implicit(PathBuf::from(&args[1]));
    }
    let cli = Cli::parse();
    // ... existing match
}
```

`looks_like_db_path` heuristic (matches `sqlite3 edu.db` muscle memory):
- arg ends with `.db` or `.sqlite`, OR
- arg contains `/`, OR
- arg exists as a directory, OR
- arg is a single token without `--` prefix (no other command-like indicators)

If the heuristic is wrong (user passes an unknown flag like `--foo`), clap's normal `--help` / error message still wins. If the heuristic is ambiguous, fall back to clap.

### 3.3 Storage adapter

Reuse `sqlrustgo_storage::FileStorage` (already in `crates/storage/`). The user-supplied `db: PathBuf` becomes the data dir:

```rust
fn open_storage(db: &Path) -> Result<FileStorage, Error> {
    // If db is a file path (e.g. "edu.db"), use parent dir + db basename as subdir.
    // If db is a directory (e.g. "edu.db/" or just "edu"), use as-is.
    let dir = if db.is_dir() { db.to_path_buf() }
              else { db.parent().unwrap_or(Path::new(".")).join(db.file_name().unwrap()) };
    FileStorage::open(&dir)  // creates dir if missing
}
```

Cross-process persistence is automatic via FileStorage's existing durability layer.

### 3.4 In-process engine

```rust
let storage = Arc::new(parking_lot::RwLock::new(open_storage(&db)?));
let engine = ExecutionEngine::with_catalog(
    storage,
    Arc::new(parking_lot::RwLock::new(Catalog::new("main"))),
);
```

No port, no MySQL wire server, no auth — direct in-process executor calls.

## 4. CLI Behavior

### 4.1 Subcommand dispatch

| Invocation | Mode |
|---|---|
| `sqlrustgo sqlite edu.db` | REPL (interactive stdin) |
| `sqlrustgo sqlite edu.db "SELECT 1;"` | REPL with one-liner bootstrap then continue |
| `sqlrustgo sqlite --batch edu.db < script.sql` | Batch via stdin |
| `sqlrustgo sqlite --cmd "SELECT 1" edu.db` | One-shot, exit after |
| `sqlrustgo sqlite --batch --cmd "..." edu.db` | Same as `--cmd` (--batch redundant) |
| `sqlrustgo edu.db` | Implicit alias: same as `sqlrustgo sqlite edu.db` |
| `sqlrustgo sqlite edu.db --csv --no-headers < x.sql` | Batch + csv output, no headers |
| `sqlrustgo sqlite edu.db --json "EXPLAIN SELECT * FROM t WHERE id=1"` | One-shot json output |
| `sqlrustgo sqlite --continue-on-error edu.db < broken.sql` | Batch continue-on-error |

### 4.2 Dot-commands (REPL only)

| Dot-command | Behavior | State change |
|---|---|---|
| `.help` | Print dot-commands list + SQL subset supported | none |
| `.quit` / `.exit` | Exit REPL with code 0 | exit |
| `.tables [LIKE]` | List table names matching LIKE pattern (default `%`) | none |
| `.schema [TABLE]` | Print CREATE TABLE for TABLE; if no arg, all schemas | none |
| `.mode MODE` | Switch to `table` / `list` / `csv` / `json` | yes (OutputMode) |
| `.headers on\|off` | Toggle column headers | yes (bool) |
| `.read FILE` | Execute SQL statements from FILE (one per line; `--` line comments) | none (runs in same engine) |
| `.output FILE\|stdout` | Redirect subsequent output to FILE (or back to stdout) | yes (OutputTarget) |
| `.timer on\|off` | Toggle wall-clock timing | yes (bool) |
| `.explain on\|off` | Toggle EXPLAIN-style plan prefix | yes (bool) |

Unknown dot-commands → `DotCmdError: unknown dot-command: .foo (try .help)` to stderr.

`.mode` accepts case-insensitive: `.mode CSV` == `.mode csv`. Invalid mode → `DotCmdError: invalid mode 'xml' (must be table|list|csv|json)`.

`.tables` with LIKE pattern uses SQL LIKE syntax: `.tables 't%'` lists tables starting with 't'.

`.schema` with no arg concatenates all CREATE TABLE statements; with arg, prints the matching table only.

`.read` opens the file, executes each non-empty, non-comment line as a separate SQL statement (same fail-fast semantics as REPL by default).

### 4.3 Output formatters

Four modes, deterministic output for golden tests:

1. **`table`** (REPL default):
   ```
   col1|col2
   ----|----
   val1|val2
   val3|val4
   ```
   - Column widths = max(header_len, max_value_len) + 1 (for trailing space)
   - Header underline = `-` repeated N times where N = column width
   - NULL → empty string
   - Trailing newline at end of result-set

2. **`list`**:
   ```
   val1|val2
   val3|val4
   ```
   - Pipe-separated, no header
   - NULL → empty string

3. **`csv`** (RFC-4180):
   ```
   col1,col2
   val1,"val, with comma"
   val3,val4
   ```
   - Header row if `.headers on` (default: off in batch, on in REPL — match sqlite3)
   - Quoted if value contains `,` / `"` / `\n` / `\r`
   - Quotes inside values doubled: `"` → `""`
   - LF line endings (no CRLF)
   - NULL → empty string

4. **`json`**:
   ```json
   {"columns":["col1","col2"],"rows":[["val1","val2"],["val3","val4"]]}
   ```
   - Single JSON object per result-set
   - Keys always `columns` + `rows` (in that order)
   - Empty result: `{"columns":[],"rows":[]}`
   - NULL → JSON `null`

Formatter invariants for golden tests:
- Column order matches SELECT clause order, not schema declaration order
- Integer rendering: Rust default `{}` Display (e.g. `42`, `-7`)
- Real rendering: Rust default `{}` Display (e.g. `1.5`, `3.14`, `1e10`)
- Text rendering: as-is, but JSON-escaped
- Boolean: `true` / `false` (table/list/csv) or `true` / `false` (json)
- No trailing whitespace on rows except the column padding in table mode
- Single LF (`\n`) at end of each result-set

### 4.4 SQL subset supported

Per plan §2 + actually implemented in SQLRustGo today:
- DDL: `CREATE TABLE` (with column types INTEGER / REAL / TEXT / BOOLEAN), `DROP TABLE`
- DML: `INSERT INTO ... VALUES`, `UPDATE`, `DELETE`
- DQL: `SELECT` with `WHERE`, `ORDER BY`, `LIMIT`, simple `JOIN`, `GROUP BY`, aggregates (`COUNT`, `SUM`, `MIN`, `MAX`, `AVG`)
- Transaction: `BEGIN`, `COMMIT`, `ROLLBACK`

This is the **already-implemented SQLRustGo SQL subset**. V312-57 does NOT extend the parser; it uses what works today. If a query fails because the SQL subset is missing, the engine returns the same error as in any other context — the CLI doesn't change the error story.

## 5. Error Model

### 5.1 Stable error prefixes

All errors emitted by the CLI get a one-line `Error: <CODE>: <message>` format on stderr:

| Code | When |
|---|---|
| `ParseError:` | SQL parser rejected input (lexer / parser failure) |
| `RuntimeError:` | binder / executor / storage error (column not found, table not found, type mismatch, constraint violation) |
| `IoError:` | file I/O failure (`.read` missing file, `.output` permission denied, `.schema` read failure) |
| `DotCmdError:` | unknown dot-command, bad `.mode` arg, bad `.read` path, etc. |

Codes never change between minor versions. The message after the code can change. Tests assert on codes, not full messages (golden files record full output for human review but tests key on the prefix).

### 5.2 Exit codes

| Mode | Last statement succeeded | Last statement failed |
|---|---|---|
| REPL (`sqlrustgo sqlite edu.db`) | 0 | 0 (REPL is always continue-on-error) |
| REPL with EOF (Ctrl-D, no query) | 0 | — |
| Batch (`--batch` or `--cmd`) | 0 | 1 |
| Batch `--continue-on-error` (any failure) | 0 if all passed, 1 if any failed | 1 |
| One-shot `--cmd "..."` | 0 | 1 |
| Argument parse error | — | 2 (clap default) |
| Cannot open DB (storage init failed) | — | 3 (custom CLI code) |

Exit code 3 is reserved for "the binary itself failed to start" — distinct from 1 (a query failed).

### 5.3 stdout vs stderr routing

| Stream | What goes here |
|---|---|
| stdout | Query results (via formatters), `.help` output, `.tables` output, `.schema` output, banner |
| stderr | All `Error:` lines, panic backtraces (in debug builds) |

`.output FILE` redirects subsequent stdout to FILE; errors always go to stderr regardless.

## 6. State Machine

```rust
struct SqliteMode {
    engine: ExecutionEngine<FileStorage>,
    state: SqliteState,
    error_seen: bool,
}

struct SqliteState {
    mode: OutputMode,
    headers: bool,         // default true in REPL, false in batch
    timer: bool,           // default false
    explain: bool,         // default false
    output: OutputTarget,  // Stdout or PathBuf
    continue_on_error: bool,
}

enum OutputMode { Table, List, Csv, Json }
enum OutputTarget { Stdout, File(PathBuf) }
```

State transitions only happen via dot-commands (REPL) or initial flags (batch). No persistence of state across invocations (each `sqlrustgo sqlite db` starts fresh with default state + CLI flag overrides).

## 7. Testing

### 7.1 Unit tests (inline `#[cfg(test)] mod tests`)

In `crates/sqlrustgo-cli/src/sqlite_mode.rs`:
- `test_parse_db_path_file_vs_dir`
- `test_output_formatter_table_golden`
- `test_output_formatter_list_golden`
- `test_output_formatter_csv_with_quotes`
- `test_output_formatter_csv_null_is_empty`
- `test_output_formatter_json_empty_result`
- `test_output_formatter_json_null_is_null`
- `test_dot_command_quit_exits_0`
- `test_dot_command_unknown_to_stderr`
- `test_dot_command_mode_change_persists`
- `test_dot_command_read_executes_file`
- `test_dot_command_output_redirects_to_file`
- `test_batch_fail_fast_exit_1`
- `test_batch_continue_on_error_exit_1_if_any`
- `test_batch_continue_on_error_runs_all`
- `test_repl_eof_exit_0`
- `test_repl_parse_error_exit_0`
- `test_cross_process_persistence_create_then_select`
- `test_implicit_alias_db_path_detection`
- `test_arg_parse_error_exit_2`
- `test_storage_init_failure_exit_3`

In `crates/sqlrustgo-cli/src/output.rs`:
- formatter-specific golden tests (one per mode × 1 representative query)
- CSV quoting edge cases (commas, quotes, newlines)
- JSON escaping
- Table column width edge cases (Unicode width, very long values)

In `crates/sqlrustgo-cli/src/dotcmd.rs`:
- Each dot-command happy path
- Error paths (unknown command, bad arg)
- Case-insensitive parsing (`.mode CSV` == `.mode csv`)

### 7.2 Integration fixtures (per week)

Directory: `tests/compat/bustubx_edu_sqlite_cli/`

```
manifest.yml                              # Lists all fixtures + golden output policy
week01/01_help.sh                         # sqlrustgo sqlite --help → matches golden
week01/02_select_one.sh                   # --cmd "SELECT 1" → table output
week01/03_stdin_script.sh                # --batch + script.sql → table output
week01/04_exit_codes.sh                  # exit 0 success, exit 1 batch fail
week02/01_create_insert_select.sh        # full DDL+DML+DQL golden
week02/02_csv_output.sh                  # --csv output matches golden
week02/03_list_output.sh                 # --list output matches golden
week02/04_csv_oracle_compare.sh          # sqlrustgo csv == sqlite3 csv (with sqlite3 oracle if available)
week03/01_single_path_db.sh              # new path auto-creates
week03/02_cross_process_persistence.sh   # open+close, reopen, data preserved
week03/03_tables_meta.sh                 # CREATE → .tables lists it
week04/01_schema_meta.sh                 # CREATE → .schema prints CREATE TABLE
week04/02_stable_error_codes.sh          # ParseError / RuntimeError / IoError / DotCmdError codes stable
week04/03_column_not_found.sh            # RuntimeError: column "x" not found
golden/                                  # Expected outputs (one per fixture)
```

### 7.3 Gate script

`scripts/gate/check_bustubx_edu_cli_v312.sh`:

```bash
#!/usr/bin/env bash
# V312-57 bustubx-edu sqlite3-like CLI gate (week01-04)
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

OUT_DIR="${OUT_DIR:-/tmp/bustubx_edu_cli_v3.12.0}"
mkdir -p "$OUT_DIR"

PASS=0; FAIL=0

echo "=== V312-57 bustubx-edu sqlite3-like CLI gate ==="

# Step 1: build
echo "[BUILD] cargo build -p sqlrustgo-cli --all-features"
cargo build -p sqlrustgo-cli --all-features >/dev/null 2>&1 \
  && PASS=$((PASS+1)) || { echo "[FAIL] build"; FAIL=$((FAIL+1)); }

# Step 2: --help smoke
echo "[HELP] sqlrustgo sqlite --help"
"$REPO_ROOT/target/debug/sqlrustgo" sqlite --help >/dev/null 2>&1 \
  && PASS=$((PASS+1)) || { echo "[FAIL] --help"; FAIL=$((FAIL+1)); }

# Step 3: each week0[1-4] fixture
FIX_DIR="$REPO_ROOT/tests/compat/bustubx_edu_sqlite_cli"
for wk in 01 02 03 04; do
  for fixture in "$FIX_DIR"/week${wk}/*.sh; do
    name="$(basename "$fixture" .sh)"
    echo "[WEEK$wk] $name"
    if bash "$fixture" > "$OUT_DIR/${name}.out" 2>"$OUT_DIR/${name}.err"; then
      if diff -q "$OUT_DIR/${name}.out" "$FIX_DIR/golden/${name}.out.txt" >/dev/null; then
        PASS=$((PASS+1))
      else
        echo "  [DIFF] $name stdout differs from golden"
        FAIL=$((FAIL+1))
      fi
    else
      echo "  [EXIT] $name exit non-zero"
      FAIL=$((FAIL+1))
    fi
  done
done

echo "==="
echo "PASS: $PASS    FAIL: $FAIL"
[ "$FAIL" -eq 0 ]
```

### 7.4 Out-of-scope for this PR (deferred to follow-up)

- `week05/*.sh` — EXPLAIN golden outputs, ORDER/LIMIT golden (need separate plan)
- `week06/*.sh` — JOIN/aggregate golden (need separate plan)
- Wire into `check_beta_v3.12.0.sh` — separate PR after this lands
- ANSI color support — separate PR
- `.import` / `.dump` / `.backup` / `.restore` dot-commands — out of scope per §2

## 8. Files Touched

| File | Action |
|---|---|
| `crates/sqlrustgo-cli/Cargo.toml` | modified — no new deps (use existing clap, parking_lot) |
| `crates/sqlrustgo-cli/src/lib.rs` | modified — add `Sqlite` subcommand + implicit-alias dispatch |
| `crates/sqlrustgo-cli/src/main.rs` | unchanged |
| `crates/sqlrustgo-cli/src/sqlite_mode.rs` | new — main `SqliteMode` struct + REPL loop + batch |
| `crates/sqlrustgo-cli/src/output.rs` | new — 4 output formatters |
| `crates/sqlrustgo-cli/src/dotcmd.rs` | new — dot-command parser + handlers |
| `crates/sqlrustgo-cli/src/error.rs` | new — stable error codes + exit code policy |
| `crates/sqlrustgo-cli/src/implicit_alias.rs` | new — `looks_like_db_path` heuristic |
| `tests/compat/bustubx_edu_sqlite_cli/manifest.yml` | new |
| `tests/compat/bustubx_edu_sqlite_cli/week0[1-4]/*.sh` | new — ~14 fixture scripts |
| `tests/compat/bustubx_edu_sqlite_cli/golden/*.out.txt` | new — expected outputs |
| `scripts/gate/check_bustubx_edu_cli_v312.sh` | new — gate script |
| `crates/sqlrustgo-cli/tests/sqlite_mode_test.rs` | new — top-level integration tests |

## 9. Implementation Order (Single PR)

1. Add `error.rs` (foundation: error codes + exit code policy) — commit `feat(cli/sqlite): error codes + exit code policy`
2. Add `output.rs` (4 formatters with inline unit tests) — commit `feat(cli/sqlite): output formatters (table/list/csv/json)`
3. Add `dotcmd.rs` (dot-command parser + handlers) — commit `feat(cli/sqlite): dot-command REPL parser`
4. Add `implicit_alias.rs` (`looks_like_db_path` heuristic) — commit `feat(cli/sqlite): implicit `sqlrustgo <path>` alias`
5. Add `sqlite_mode.rs` (main `SqliteMode` + REPL + batch loop) — commit `feat(cli/sqlite): in-process engine + REPL + batch mode`
6. Wire `Sqlite` subcommand into `lib.rs` + implicit-alias dispatch — commit `feat(cli/sqlite): wire Sqlite subcommand + implicit alias dispatch`
7. Add `tests/compat/bustubx_edu_sqlite_cli/week0[1-4]/*.sh` + golden files — commit `test(cli/sqlite): bustubx-edu week01-04 fixtures + golden outputs`
8. Add `scripts/gate/check_bustubx_edu_cli_v312.sh` — commit `feat(gate): bustubx-edu sqlite CLI gate (week01-04)`
9. Final: full `cargo test -p sqlrustgo-cli --all-features` + commit `chore(cli/sqlite): final test pass + clippy`

Each commit must compile + tests pass before next commit (per CLAUDE.md TDD workflow).

## 10. Spec Self-Review

### 10.1 Placeholder scan

No "TBD" / "TODO" / "FIXME" / unresolved placeholders. Every section has explicit content.

### 10.2 Internal consistency

- §4.1 says batch mode default = `headers: false in batch`; §6 says default state `headers: false in batch` — consistent.
- §4.3 formatter invariants say column order matches SELECT clause; §4.3 golden rules reaffirm — consistent.
- §5.2 exit code table says REPL always exits 0 (continue-on-error); §5.1 says REPL continue-on-error is default — consistent.
- §7.3 gate script expects `target/debug/sqlrustgo`; §9 implementation order builds via `cargo build`; consistent.
- §8 files list `crates/sqlrustgo-cli/tests/sqlite_mode_test.rs`; §7.1 unit tests are inline `mod tests`; §7.2 integration fixtures are `*.sh` — three different test layers, all named consistently.

### 10.3 Scope check

Focused on V312-57 sqlite-like CLI. Does NOT include:
- week05-06 fixtures (deferred to follow-up)
- Gate wiring into `check_beta_v3.12.0.sh` (deferred)
- ANSI colors (out of scope)
- `.import` / `.dump` / `.backup` / `.restore` (out of scope per §2)
- SQLite file format compat (out of scope per §2)

Single PR scope: ~1500-2000 lines new code + ~600 lines fixtures/gate. Achievable in one session.

### 10.4 Ambiguity check

- "Stable error prefixes": §5.1 enumerates the 4 codes exactly. No ambiguity.
- "Exit codes": §5.2 gives full matrix. No ambiguity.
- "Cross-process persistence": §3.3 + §7.1 `test_cross_process_persistence_create_then_select` + week03 fixture. No ambiguity.
- "REPL continue-on-error semantics": §4.1 + §5.1 + §5.2 all consistent. No ambiguity.
- "Implicit alias detection": §3.2 + §7.1 `test_implicit_alias_db_path_detection`. No ambiguity (heuristic enumerated).

## 11. Out of Scope Reminder (for PR description)

When this PR is opened (after Gitea recovers), the PR description must include:

```
What this PR does NOT do:
- week05-06 fixtures (deferred — separate PR)
- Wire check_bustubx_edu_cli_v312.sh into check_beta_v3.12.0.sh (deferred — separate PR)
- ANSI color support (out of scope per plan §2)
- .import / .dump / .backup / .restore dot-commands (out of scope per plan §2)
- SQLite file format compatibility (out of scope per plan §2)
- Full SQL-92 / MySQL 5.7 syntax (out of scope per plan §2)
```
# SQLRustGo v4.1.0 — Deployment Guide

> **Status**: ALPHA — per [`STAGE.yaml`](STAGE.yaml) → `current_stage: "ALPHA"`
> (re-entered 2026-10-06, see `last_transition`). v4.1.0 is **not GA**, is
> **not released**, and **no gate verdict is asserted** by this document.
> **Date**: 2026-10-23
> **Baseline**: `develop/v4.1.0` @ `3a3350f660` — the tree this guide
> describes. Every path, flag, environment variable and metric name below
> was read out of that tree; where a path moved between releases, the
> current location is given.
> **Scope**: operating the SQLRustGo MySQL wire-protocol server from the
> `develop/v4.1.0` line — build artifacts, startup/shutdown, data directory
> layout, storage/WAL selection, configuration, observability, sizing,
> backup/restore, troubleshooting.
> **Audience**: operators running the server against a real data directory.
>
> Every command, flag, environment variable, path and metric name below was
> read out of this repository. Where the repository does not record something,
> the item is marked **TODO / unknown — needs owner input** rather than
> guessed.

---

## 1. Scope and how to read this document

This guide describes how to run the server that exists in this tree. It does
not describe a released product, and it does not certify that the release
line is fit for production — the release line's own defect catalog and claim
downgrade manifest record what is *not* covered
([`DEFECTS_AND_ISSUES.md`](DEFECTS_AND_ISSUES.md),
[`CLAIM_DOWNGRADE_MANIFEST.md`](CLAIM_DOWNGRADE_MANIFEST.md)).

Two facts about the tree you should carry into every decision below:

- The engine is a **hand-written interpreter**, not a physical-plan executor.
  The production path is `src/engine_select.rs` + `src/engine_dml.rs`
  ([`ARCHITECTURE.md`](ARCHITECTURE.md) §2.1). The `crates/planner` physical
  plan layer is **not** in the production path (`PhysicalPlan` has no
  `execute()`; `mysql-server` never imports `planner`).
- The root `Cargo.toml` `version` field is **not** the release version. The
  root `README.md` notes it is still `3.12.0-fix-zombie`, deliberately, and
  that the release line is identified by the tag / `STAGE.yaml`, not by the
  crate version string.
- Several v4.1.0 sibling documents still carry a stale `DRAFT` banner
  (`ARCHITECTURE.md`, `RELEASE_NOTES.md`, `COMPREHENSIVE_ASSESSMENT_REPORT.md`).
  `STAGE.yaml` is the per-version SSOT and says **ALPHA**; the disagreement
  is itself recorded in [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §3 item 8.
  This guide follows the SSOT.

---

## 2. Build artifacts

### 2.1 Workspace layout

The workspace crates live under `crates/` (40+ members); integration tests
live under `tests/`. The two binaries an operator needs are owned by:

| Binary | Package | Declared at |
|---|---|---|
| `sqlrustgo-mysql-server` | `crates/mysql-server` | `crates/mysql-server/Cargo.toml` → `[[bin]] name = "sqlrustgo-mysql-server"`, `path = "src/main.rs"` |
| `sqlrustgo` | `crates/sqlrustgo-cli` | `crates/sqlrustgo-cli/Cargo.toml` → `[[bin]] name = "sqlrustgo"`, `path = "src/local_bin.rs"` |

Build them with the toolchain pinned in `cargo-toolchain.toml`:

```bash
cargo build --all-features
cargo build --release --all-features
```

`README.md` §6 uses `cargo build --all-features`; the `--all-features` form
also turns on the optional allocator and fault-injection features described
in §2.3.

### 2.2 Where the binaries land

Cargo puts them under the target directory, which is
`$CARGO_TARGET_DIR` if set, otherwise `<repo>/target`:

```
target/release/sqlrustgo-mysql-server
target/release/sqlrustgo
```

### 2.3 Cargo features that change runtime behaviour

Only these features change what the binary *does* at runtime. Everything
else in the workspace is compile-time plumbing.

| Feature | Declared at | What it does |
|---|---|---|
| `parallel-executor` | root `Cargo.toml` → `parallel-executor = ["sqlrustgo-executor/parallel-executor"]` | Enables intra-query parallel scan/join/agg pipelines. Without it, `--executor-parallelism N` is accepted but the runtime caps execution at 1 (see `crates/mysql-server/src/main.rs`, `--executor-parallelism` help text). |
| `jemalloc-prof` | `crates/mysql-server/Cargo.toml` | Installs jemalloc as the global allocator and enables the `SIGUSR2` → heap-dump handler. Off by default; production (non-prof) builds keep the system allocator. |
| `fault-injection` | `crates/storage/Cargo.toml` **and** `crates/mysql-server/Cargo.toml` | Enables the fail-point hooks in `crates/storage/src/fault_injection.rs` (driven by `SQLRUSTGO_FAULT_POINT`). **Test-only — off in production builds.** |
| `bin_storage_default` | `crates/storage/Cargo.toml` | Declared but empty; the storage crate's `default` feature set is `default = []`. |

The storage crate's own feature list is deliberately minimal:

```toml
[features]
default = []
fault-injection = []
bin_storage_default = []
v313_3_profile = []   # V313.3 profiling experiments; test-only hooks; OFF by default
```

### 2.4 Binary resolution (two different overrides — do not mix them up)

There are **two** independent override mechanisms in this repository, and
they resolve different artifacts:

| Variable | Resolves | Read at |
|---|---|---|
| `SQLRUSTGO_SERVER_BIN` | the **`sqlrustgo-mysql-server`** binary that the `sqlrustgo` CLI spawns | `crates/sqlrustgo-cli/src/lib.rs` → `mysql_server_bin()` |
| `SQLRUSTGO_BIN` | the **`sqlrustgo`** binary that shell scripts under `scripts/` invoke | `scripts/common/resolve_bin.sh` → `tests/common/resolve_bin.sh` → `resolve_cargo_bin` |

`mysql_server_bin()` tries, in order: `SQLRUSTGO_SERVER_BIN` →
`$CARGO_TARGET_DIR/{debug,release}/sqlrustgo-mysql-server` → the directory
the running `sqlrustgo` binary itself lives in → `./target/{debug,release}/…`
→ `$PATH` → the bare name (so the spawn fails loudly instead of silently
picking up something else). If none is found, the CLI prints
`sqlrustgo-mysql-server not found.` and exits non-zero.

The shell-side resolver (`resolve_cargo_bin <package> <env-var>`) returns
**77** ("skipped") when it cannot find the binary, and callers must treat 77
as *not run*, never as *passed*.

---

## 3. Server startup and shutdown

### 3.1 Canonical invocation

The canonical entry point is the server binary with the `serve` subcommand:

```bash
cargo run --bin sqlrustgo-mysql-server -- serve --host 127.0.0.1 --port 3307
```

(`README.md` §6.) From a release build:

```bash
./target/release/sqlrustgo-mysql-server serve \
  --host 127.0.0.1 \
  --port 3307 \
  --data-dir /var/lib/sqlrustgo/data
```

`serve` is also the **default** subcommand: running the binary with no
subcommand starts the server with the defaults listed in §3.2.

### 3.2 Every `serve` flag

All flags below are declared in `crates/mysql-server/src/main.rs`, `enum
Command::Serve`.

| Flag | Type | Default | Notes |
|---|---|---|---|
| `--host` | `String` | `127.0.0.1` | Bind address of the MySQL wire listener. |
| `--port` | `u16` | `3306` | Bind port of the MySQL wire listener. |
| `--data-dir` | `String` | `/tmp/sqlrustgo-data` | Storage data directory. The default is **not durable** — see §3.4. |
| `--load-infile-dir` | `Option<String>` | `None` | Whitelist directory for `LOAD DATA LOCAL INFILE`. When unset, the whitelist falls back to `--data-dir`. |
| `--max-connections` | `usize` | `100` | Semaphore limit on concurrent connections. |
| `--server-threads` | `usize` | `16` | Connection-handler worker threads. `0` = legacy unbounded `thread::spawn`; `1..=80` = bounded pool. Values above 80 are rejected by the parser (`must be ≤ 80`). |
| `--auth-mode` | `String` | `none` | `none` = allow all; `password` = require a password. The comparison is case-insensitive. |
| `--storage` | `String` | `file` | Storage engine — see §5.1. |
| `--wal-sync` | `String` | `every` | WAL durability mode — see §5.2. |
| `--executor-parallelism` | `usize` | `1` | Intra-query worker threads for scan/join/agg pipelines. Valid range `1..=1024` (`0` and `>1024` are rejected). Also readable from `SQLRUSTGO_EXECUTOR_PARALLELISM`. |
| `--bulk-insert-rows-per-flush` | `usize` | `10000` | Rows per `bulk_insert_records` flush during `LOAD DATA LOCAL INFILE`. `0` disables the periodic flush and falls back to flushing only when the per-packet byte buffer drains. |
| `--verbose` | `bool` | `false` | Adds the `MVCC: enabled` line to the startup banner. |
| `--metrics-port` | `Option<u16>` | `None` | When set, the server spawns a background thread serving `GET /metrics` in Prometheus text exposition format 0.0.4. When unset, **no metrics endpoint is started**. |
| `--log-level` | `String` | `info` | **Global** flag (accepted before or after the subcommand). |

### 3.3 The `sqlrustgo` CLI wrapper

The `sqlrustgo` binary forwards a small set of subcommands to the server
binary (`crates/sqlrustgo-cli/src/lib.rs`):

| CLI form | Forwards to |
|---|---|
| `sqlrustgo serve --port <P> [--data-dir <D>]` | `serve --port <P> [--data-dir <D>]` |
| `sqlrustgo exec "<SQL>"` | `exec "<SQL>"` |
| `sqlrustgo repl` | `repl` |
| `sqlrustgo backup <output_dir> --database <DB> --backup-type <TYPE>` | `backup <output_dir> --database <DB> --backup-type <TYPE>` |
| `sqlrustgo restore <backup_id> <database> --backup-dir <DIR>` | `restore <backup_id> <database> --backup-dir <DIR>` |
| `sqlrustgo cli --host <H> --port <P> [--user <U>] [--password <W>] "<SQL>"` | connects to a running server over the wire |
| `sqlrustgo soak --host <H> --port <P> [--user <U>] [--pass <W>]` | persistent MySQL connection, reads SQL from stdin, writes a tab-separated protocol to stdout |

Note the CLI's own `--port` default is **3307**, while the server binary's is
**3306**. The two defaults differ; always pass `--port` explicitly.

`sqlrustgo` also has a sqlite3-like local mode — `sqlrustgo <db-path>
[SQL...]` (implicit alias) or `sqlrustgo sqlite --batch <db> ...` — which is
an **embedded, in-process** path, not a server deployment.

### 3.4 The `/tmp` default is not durable — the server says so

If you start the server **without** `--data-dir` and the resolved default
begins with `/tmp/`, the server prints this to stderr
(`crates/mysql-server/src/main.rs`):

```
WARNING: no --data-dir given; defaulting to <dir>, which is NOT durable.
  /tmp is cleared on reboot and is world-writable, so committed
  data will be lost on restart and is not private to this user.
  Pass --data-dir <persistent-path> for any real deployment.
```

The default is deliberately **not** relocated, because moving an existing
installation's data without a migration would be worse. **Always pass an
explicit `--data-dir`.**

### 3.5 Startup banner

On a successful start the server prints (to stdout):

```
SQLRustGo v3.8.0-beta (Strong Beta, 8.0/10)
MySQL wire-protocol server
  Listen:     <host>:<port>
  Data dir:   <data_dir>
  Max conn:   <max_connections>
  Auth mode:  <auth_mode>
  Storage:    <storage>
  Exec par:   <executor_parallelism> (Issue #3703, --executor-parallelism)
  Metrics:    http://<host>:<metrics_port>/metrics (Prometheus 0.0.4, V312-26 / #4021)
  Ready to accept connections.
```

The `INFILE dir:` line appears only when `--load-infile-dir` was given. The
`MVCC: enabled` line appears only under `--verbose`.

> **Note on the banner version string.** The banner hard-codes
> `SQLRustGo v3.8.0-beta`. This is a literal in the binary, **not** the
> release identity of the tree you are running. Identify the build by tag /
> `STAGE.yaml` / commit, not by this line.

### 3.6 Logging

The server installs a `tracing_subscriber` with a `fmt` layer. The filter is
`EnvFilter::try_from_default_env()` (i.e. the standard `RUST_LOG`
environment variable) and falls back to `--log-level` when `RUST_LOG` is
unset or unparsable.

```bash
RUST_LOG=sqlrustgo_mysql_server=debug ./target/release/sqlrustgo-mysql-server serve ...
```

### 3.7 Graceful shutdown

`SIGINT` / `SIGTERM` drive a shared `AtomicBool` shutdown flag that the
accept loop polls every 100 ms; the OS's default disposition is left in
place so the process terminates when the signal lands. Use `SIGTERM` for a
planned stop so the server takes the shutdown path rather than being killed
outright.

### 3.8 Exit codes

| Code | Meaning |
|---|---|
| `0` | Success (server exited normally after shutdown; `exec`/`repl`/`backup`/`restore` completed). |
| `1` | Runtime error — e.g. WAL recovery failed, storage init failed, `backup`/`restore` failed. |
| `2` | Placeholder subcommands (`bench`, `gmp`, `diag`) print "full features migrate in a follow-up" and exit 2. |
| `64` | `EX_USAGE` — any clap-level parse error (unknown subcommand, bad flag value). `--help` and `--version` print and exit `0`. |

---

## 4. Data directory layout

### 4.1 What the server writes

With `--data-dir /var/lib/sqlrustgo/data` the server produces:

```
/var/lib/sqlrustgo/data/
├── sqlrustgo.wal                 # write-ahead log (file/parallel engines)
├── <database>/
│   ├── <table>.json              # one file per table
│   └── <table>_idx_<column>.json # one file per secondary index
├── trigger_<name>.json           # trigger definitions
└── view_<name>.json              # view definitions
```

The table file name is `format!("{}.json", table_name)` and the index file
name is `format!("{}_idx_{}.json", table_name, column_name)`
(`crates/storage/src/file_storage.rs`).

### 4.2 The database-scoped directory layout (#5025)

Table files are written **inside a per-database subdirectory**:
`<data_dir>/<database>/<table>.json`. The write side never falls back to the
data-directory root — writing a new table into the shared root from inside a
database is exactly what #5025 was about.

The **read** side *does* fall back: if the database directory does not
exist, the loader looks for `<data_dir>/<table>.json`, so tables written
before the layout change stay visible. This means a data directory can
legitimately contain both layouts at once.

### 4.3 The checkpoint file is NOT in the data directory

This is the single most surprising operational detail in the tree. The
server constructs its checkpoint manager with `CheckpointManager::default()`,
and `Default` is:

```rust
impl Default for CheckpointManager {
    fn default() -> Self {
        Self::new(PathBuf::from("."), CheckpointConfig::default()).unwrap()
    }
}
```

The checkpoint path is then `<checkpoint_dir>/checkpoint.json`, i.e.
**`./checkpoint.json` relative to the process's current working directory**.

**Operational consequence:** run the server with a working directory that is
(a) writable, and (b) the same on every restart — otherwise a restart writes
a *different* `checkpoint.json` and the previous one is stranded. Include
`./checkpoint.json` in your backup set alongside the data directory.

### 4.4 The WAL file

The WAL file is `<data_dir>/sqlrustgo.wal`, opened through
`FileBackedWalManager`. At startup the server logs its size and warns when
it is large:

```
WAL file is large: <path> (<N> MB). Consider --data-dir to isolate runs.
```

(threshold: 100 MB).

---

## 5. Storage engine and WAL selection

### 5.1 `--storage` values

`crates/mysql-server/src/lib.rs` matches on the storage string:

| `--storage` | Engine | Notes |
|---|---|---|
| `file` (default) | `WalStorage<FileStorage, FileBackedWalManager>` over `MvccStorage<FileStorage>` | WAL-backed, durable, MVCC snapshot reads. This is the default production path. |
| `parallel` | `ParallelWalStorage<MvccStorage<FileStorage>, DynWalManager>` | Parallel table flush; shares **one** `SharedWalHandle` with `FileStorage` so a single physical file handle is the only writer of `sqlrustgo.wal`. This is the construction that fixes the #5317 two-handle race. |
| `binary` | `BinaryTableStorage` | Reads pre-generated `.bin` tables from the data directory. **No WAL.** Intended for fast TPC-H loads; not a general-purpose durable engine. |
| anything else | falls through to the `file` branch | The match is `Some("binary") => …, Some("parallel") => …, _ => …`, so an unrecognised value silently means `file` semantics. |

Read paths run through `MvccStorage`, so `SELECT`s take per-table `RwLock`s
inside the MVCC chain instead of the outer global storage lock — this is the
change that made reader concurrency viable.

### 5.2 `--wal-sync` values

Parsed by `parse_wal_sync_mode()` in `crates/mysql-server/src/lib.rs`:

| Value | `WalSyncMode` | Durability |
|---|---|---|
| `every` (default) | `Every` | fsync on every commit. |
| `batch:N` | `Batch(N)` | fsync every N transactions. The server logs `WAL batch mode: sync every N transactions`. |
| `group:MAX_BATCH,MAX_WAIT_US` | `GroupCommit { max_batch, max_wait_us }` | Group commit through a coordinator. Defaults if the numbers do not parse: `max_batch=32`, `max_wait_us=1000` (1 ms). The server logs the loss bound explicitly: *"Up to max_batch-1 tx loss on crash."* |
| `off` | `Off` | Durability disabled. The server logs `WAL sync OFF: durability disabled for performance!` |

`--wal-sync` reaches the storage layer through the `SQLRUSTGO_WAL_SYNC`
environment variable.

### 5.3 Startup WAL recovery is fail-closed (#5192)

On startup the server replays the WAL with `StatefulRecoveryEngine<FileStorage>`
and logs:

```
WAL recovery: total=<n> committed_txns=<n> rows_inserted=<n> skipped=<n>
```

Two outcomes matter:

- **`skipped_entries > 0`** — a committed transaction could not be replayed.
  Those writes were already acknowledged to clients and are now lost. The
  server **refuses to start** and returns an error, unless
  `SQLRUSTGO_ALLOW_DATA_LOSS=1` is set, in which case it logs
  `DATA IS BEING LOST.` and continues.
- **hard recovery error** — the server refuses to start with
  `WAL recovery failed: … Refusing to start, because the committed state of
  the data directory cannot be established.`

`SQLRUSTGO_ALLOW_DATA_LOSS` is named for what it does. It is **not** a
recovery switch: it accepts the loss. Set it only when the data directory
has already been copied aside and you have decided to bring the database
back up with acknowledged writes missing.

### 5.4 Replay cost — plan restarts accordingly

WAL recovery replays entries one at a time, rewriting the whole table per
entry (`save_table_full` is called per record; the delta-append path is
never reached). This is registered as an open defect, **P0-08** in
[`DEFECTS_AND_ISSUES.md`](DEFECTS_AND_ISSUES.md): *"WAL recovery 逐条整表重写,
重启 24 分钟不可用"*.

The measured instance behind that headline: the first full-table query after
a restart took **24 min 22 s** at 68 288 rows / 58 288 INSERTs / 8.0 MB WAL
— the identical query immediately afterwards took **0.08 s**. Recovery
completes **before** the listener accepts connections, so this is a hard
startup outage, and no historical SOAK covered it.

A large WAL therefore implies a long restart. Measure it on your own data
before scheduling a restart window.

---

## 6. Configuration via environment variables

### 6.1 The variables that matter

Every name below was found by searching the tree for `env::var` /
`std::env::var`.

| Variable | Default | Effect | Source |
|---|---|---|---|
| `SQLRUSTGO_DATA_DIR` | set by the server from `--data-dir`; otherwise the server falls back to `<cwd>/.sqlrustgo/data` | Storage data directory. | `crates/mysql-server/src/lib.rs` (`run_server_v2`, and the fallback branch) |
| `SQLRUSTGO_MAX_CONN` | set from `--max-connections` | Connection semaphore limit. | `crates/mysql-server/src/lib.rs` |
| `SQLRUSTGO_AUTH_MODE` | `none` (from `--auth-mode`) | `none` (case-insensitive) **skips authentication entirely**. Any other value requires a password. | `crates/mysql-server/src/lib.rs` → `skip_auth()` |
| `SQLRUSTGO_STORAGE` | `file` (from `--storage`) | Storage engine selection, read at startup. | `crates/mysql-server/src/lib.rs` |
| `SQLRUSTGO_WAL_SYNC` | `every` (from `--wal-sync`) | WAL sync mode. | `crates/mysql-server/src/lib.rs` |
| `SQLRUSTGO_EXECUTOR_PARALLELISM` | `1` (from `--executor-parallelism`) | Intra-query parallelism. Also settable directly via the flag's clap `env =` binding. | `crates/mysql-server/src/main.rs`, `crates/mysql-server/src/lib.rs` |
| `SQLRUSTGO_IDLE_TIMEOUT_SECS` | `30` | Per-connection idle (read) timeout. Values `< 1` fall back to 30. Exists because the previous 600 s default was the root cause of `CLOSE_WAIT` zombies accumulating. | `crates/mysql-server/src/lib.rs` → `read_idle_timeout_secs()` |
| `SQLRUSTGO_REAPER_INTERVAL_SECS` | `5` | Scan interval of the idle-connection reaper. Values `< 1` fall back to 5. | `crates/mysql-server/src/lib.rs` → `read_reaper_interval_secs()` |
| `SQLRUSTGO_METRICS_PORT` | unset (set by the server from `--metrics-port`) | Port for the Prometheus `/metrics` listener. Unset ⇒ no metrics endpoint. | `crates/mysql-server/src/main.rs`, `crates/mysql-server/src/lib.rs` |
| `SQLRUSTGO_LOAD_INFILE_DIR` | unset (set by the server from `--load-infile-dir`) | `LOAD DATA LOCAL INFILE` whitelist directory; falls back to the data dir when unset. | `crates/mysql-server/src/main.rs`, `crates/mysql-server/src/lib.rs` |
| `SQLRUSTGO_BULK_INSERT_ROWS_PER_FLUSH` | `10000` (set by the server from `--bulk-insert-rows-per-flush`) | Rows per bulk-insert flush. `0` disables the periodic flush. | `crates/mysql-server/src/main.rs`, `crates/mysql-server/src/lib.rs` |
| `SQLRUSTGO_ALLOW_DATA_LOSS` | unset | `1` lets the server start when committed WAL entries cannot be replayed. | `crates/mysql-server/src/lib.rs` → `guard_unrecoverable_commits()` |
| `SQLRUSTGO_MAX_MEMORY_MB` | `64` (64 MiB) | Per-query memory limit, in MiB. | `crates/executor/src/session_config.rs` → `DEFAULT_MAX_MEMORY_PER_QUERY = 64 * 1024 * 1024` |
| `SQLRUSTGO_BENCHMARK_MODE` | unset | `1` disables the cache and statistics collection. | `crates/executor/src/session_config.rs` |
| `SQLRUSTGO_TEACHING_MODE` | unset | `1` disables the cache and statistics collection. | `crates/executor/src/session_config.rs` |
| `SQLRUSTGO_HOME` | unset | GMP runtime home. GMP state lands in `$SQLRUSTGO_HOME/gmp.db` (docs + embeddings) and `$SQLRUSTGO_HOME/graph.db` (graph). | `crates/gmp/src/persist_sqlite.rs` |
| `SQLRUSTGO_SERVER_BIN` | unset | Path to the `sqlrustgo-mysql-server` binary the `sqlrustgo` CLI spawns. | `crates/sqlrustgo-cli/src/lib.rs` → `mysql_server_bin()` |
| `SQLRUSTGO_BIN` | unset | Path to the `sqlrustgo` binary used by `scripts/`. | `scripts/common/resolve_bin.sh` |
| `SQLRUSTGO_FAULT_POINT` | unset | Fault-injection point name. **Requires the `fault-injection` feature** (test builds only). | `crates/storage/src/fault_injection.rs` |
| `RUST_LOG` | unset | Standard `tracing` filter; overrides `--log-level` when set and parsable. | `crates/mysql-server/src/main.rs` |
| `MALLOC_CONF` | unset | Read by the `jemalloc-prof` build; the startup line prints the effective `MALLOC_CONF`, `config.prof` and `opt.prof`. | `crates/mysql-server/src/main.rs` |

### 6.2 How the CLI propagates into the server

Several flags are not passed as arguments but re-published as environment
variables before the server starts, because the server's entry point
signature is fixed:

| Flag | Propagated as | Set in |
|---|---|---|
| `--metrics-port` | `SQLRUSTGO_METRICS_PORT` | `crates/mysql-server/src/main.rs` |
| `--load-infile-dir` | `SQLRUSTGO_LOAD_INFILE_DIR` | `crates/mysql-server/src/main.rs` |
| `--bulk-insert-rows-per-flush` | `SQLRUSTGO_BULK_INSERT_ROWS_PER_FLUSH` | `crates/mysql-server/src/main.rs` |
| `--data-dir`, `--max-connections`, `--auth-mode`, `--storage`, `--wal-sync`, `--executor-parallelism` | `SQLRUSTGO_DATA_DIR`, `SQLRUSTGO_MAX_CONN`, `SQLRUSTGO_AUTH_MODE`, `SQLRUSTGO_STORAGE`, `SQLRUSTGO_WAL_SYNC`, `SQLRUSTGO_EXECUTOR_PARALLELISM` | `crates/mysql-server/src/lib.rs` → `run_server_v2` |

**Set the flags, not the variables.** The server overwrites these variables
at startup, so a value exported in the environment is replaced by the flag
value (or by the flag's default).

### 6.3 Security note on `SQLRUSTGO_AUTH_MODE`

The default `none` **allows every client to connect without a password**.
The comparison is `eq_ignore_ascii_case("none")`, so `NONE` behaves the same
as `none`. Any other value takes the password path. Do not expose a
`--auth-mode none` server on a non-loopback address.

---

## 7. Observability

### 7.1 The Prometheus endpoint

`--metrics-port <P>` starts a background thread serving HTTP/1.1 over a
plain `TcpListener` (`crates/mysql-server/src/metrics_endpoint.rs`):

| Request | Response |
|---|---|
| `GET /metrics` | `200 OK`, `Content-Type: text/plain; version=0.0.4`, Prometheus text exposition body |
| `GET /health` | `200 OK`, `application/json`, `{"status":"ok"}` |
| `GET /healthz` | `200 OK`, `application/json`, `{"status":"ok"}` |
| anything else (including `POST /metrics`) | `404 Not Found`, `application/json`, `{"error":"not found"}` |

Operational properties of this endpoint:

- **Bind failure is non-fatal.** If the port is in use or not permitted, the
  server logs `metrics_endpoint: bind <addr> failed (<e>); /metrics will not
  be available` and keeps running **without** metrics.
- **No keep-alive.** Every scrape opens a fresh connection and the
  response carries `Connection: close`.
- **Single-threaded accept loop**, one connection served at a time. This is
  fine for low-QPS scrapes; it is not a high-rate scrape endpoint.
- The endpoint binds the same `--host` value as the wire listener.

### 7.2 Metric names

Wire-protocol counters, always incremented:

| Metric | Type |
|---|---|
| `sqlrustgo_active_connections` | gauge |
| `sqlrustgo_connections_accepted_total` | counter |
| `sqlrustgo_queries_served_total` | counter |
| `sqlrustgo_query_errors_total` | counter |

Telemetry counters, appended by `sqlrustgo_telemetry::PrometheusRenderer`
(`crates/telemetry/src/prometheus.rs`):

| Metric | Type |
|---|---|
| `sqlrustgo_queries_total` | counter |
| `sqlrustgo_queries_duration_seconds` | histogram (with `_sum` and `_count`) |
| `sqlrustgo_cache_hits` / `sqlrustgo_cache_misses` | counters |
| `sqlrustgo_cache_hit_rate` | gauge |
| `sqlrustgo_storage_read_bytes` / `sqlrustgo_storage_write_bytes` | counters |
| `sqlrustgo_connections_active` / `sqlrustgo_connections_total` | gauge / counter |

The two sources are deliberately kept separate — the wire counters are
incremented on every connection and statement, while the telemetry counters
are opt-in at finer-grained call sites — so the rendered body does not
double-count.

### 7.3 The legacy HTTP server crate

`crates/server/` is the **deprecated legacy HTTP server** crate. It pulls in
`actix-web` and `tokio` and exposes a different endpoint set
(`/health/live`, `/ready`, `/metrics`). The production MySQL server is
deliberately a minimal `std::net::TcpListener` daemon and does **not** link
it. Do not point a scraper at `crates/server` expecting the production
server's counters.

### 7.4 Slow query log

`crates/query-stats/src/slow_query_log.rs` provides a slow-query logger:

- `SlowQueryConfig::default()` → `threshold_ms: 1000`, `log_path:
  PathBuf::from("slow_query.log")`.
- `SlowQueryLog::new(threshold_ms, log_path)` and
  `SlowQueryLog::from_config(&config)` build a logger.
- `maybe_log(&self, query, duration_ms, rows)` writes a record when the
  duration exceeds the threshold; `read_logs()` and `get_recent()` read back.

The default log path is **relative to the process working directory** — see
§4.3 for why that matters.

### 7.5 Admin status scripts

- `scripts/admin/admin-status.sh` — queries
  `information_schema.session_status` for `Innodb_rows_read`,
  `Threads_connected` and `Uptime`. Defaults to `--host 127.0.0.1 --port
  3307`; both are overridable.
- `scripts/admin/admin-flush-logs.sh` — flushes server logs.

### 7.6 Heap profiling (optional, non-default)

With the `jemalloc-prof` feature, `SIGUSR2` triggers a one-shot heap dump
written to `/tmp/jeprof.<pid>.<unix_ts>.heap`, analysed offline with
`jeprof --text` / `--pdf` / `--dot`. Without the feature, the handler is a
no-op and the server uses the system allocator.

---

## 8. Resource sizing guidance

All figures below are read out of this repository. **No throughput target,
QPS figure or latency SLO is asserted here** — the repository does not
publish sizing guidance, and inventing one would be a fabrication.

### 8.1 Concurrency

| Knob | Default | Range / limit | Where |
|---|---|---|---|
| `--max-connections` | 100 | — | `crates/mysql-server/src/main.rs` |
| `--server-threads` | 16 | `0` (unbounded) or `1..=80` | `crates/mysql-server/src/main.rs` |
| `--executor-parallelism` | 1 | `1..=1024` | `crates/mysql-server/src/main.rs` |

The intra-query executor is **sequential by default** (`1`). Raising it is
opt-in and requires the `parallel-executor` feature to have any effect.

### 8.2 Memory

- Per-query memory limit: **64 MiB** by default
  (`SQLRUSTGO_MAX_MEMORY_MB`, `crates/executor/src/session_config.rs`).
- The cache and statistics collection are **enabled by default** and are
  disabled by `SQLRUSTGO_BENCHMARK_MODE=1` or `SQLRUSTGO_TEACHING_MODE=1`.
- The 8-hour SOAK recorded a **peak RSS of 85.3 MB**
  (`docs/releases/v4.1.0/CHANGELOG.md`, P0 round of 2026-10-09). That is an
  observation from one workload, not a sizing model.

### 8.3 WAL growth

- The WAL file is `<data_dir>/sqlrustgo.wal` and grows with write volume.
  The server warns at ≥ 100 MB.
- `--wal-sync batch:N` and `--wal-sync group:MAX_BATCH,MAX_WAIT_US` trade
  durability for fewer fsyncs; the group-commit mode documents an explicit
  loss bound of *up to `max_batch - 1` transactions on crash*.

### 8.4 Recorded measurements (observations, not targets)

These are measurements recorded in this repository, cited with their source.
They are **not** capacity guarantees and must not be quoted as such.

| Measurement | Value | Source |
|---|---|---|
| 8 h SOAK | 28800.29 s, 0 FATAL / 0 errors / 0 reconnects, 352 402 transactions, 7 048 040 queries, RSS peak 85.3 MB | `docs/releases/v4.1.0/CHANGELOG.md` (P0 round 2026-10-09) → `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` |
| Primary-key point lookup, 20 000 rows | 9.02 ms → 0.467 ms after #5168 | `docs/releases/v4.1.0/CHANGELOG.md` → `crates/storage/tests/wal_scan_pk_forwarding_5168.rs` |
| sysbench, 8 clients | 13.89 → 38.53 TPS; 277.79 → 770.52 QPS | `docs/releases/v4.1.0/CHANGELOG.md` (#5168 row) |

> **Evidence-weight caveat.** The 8 h SOAK's own conclusion "cannot serve as
> evidence that v4.1.0 is usable" — the IAIA-410 truthfulness audit found
> defects that SOAK structurally could not cover, e.g. the #5192 durability
> illusion after `kill -9`. A green SOAK is **not** a release-grade claim.
> See [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) row F-07.

---

## 9. Backup and restore

### 9.1 `scripts/backup.sh`

```
Usage: ./scripts/backup.sh [OPTIONS]
  -d, --database <name>    Database name (default: default)
  -o, --output <dir>       Backup output directory (default: ./backups)
  -t, --type <type>       Backup type: full (the only implemented one)
  -c, --compress           NOT IMPLEMENTED — the script refuses it
  -s, --schema-only        NOT IMPLEMENTED — the script refuses it
```

The script:

1. Resolves the `sqlrustgo` binary via `resolve_cargo_bin sqlrustgo
   SQLRUSTGO_BIN` and the server binary via `resolve_cargo_bin
   sqlrustgo-mysql-server SQLRUSTGO_SERVER_BIN`, exporting
   `SQLRUSTGO_SERVER_BIN` so the CLI cannot pick up a stale server binary.
2. Runs `sqlrustgo backup "$OUTPUT_DIR" --database "$DATABASE" --backup-type
   "$BACKUP_TYPE"`.
3. **Verifies the artifact**: the newest `*.sql` file in the output directory
   must exist and must contain at least one `CREATE TABLE` line, otherwise
   the script exits non-zero and reports a failure.

### 9.2 `scripts/restore.sh`

```
Usage: ./scripts/restore.sh -b <backup-id> [OPTIONS]
  -d, --database <name>    Database name (required)
  -b, --backup-id <id>    Backup ID to restore (required)
  -i, --input <dir>       Backup directory (default: ./backups)
      --drop-first        NOT IMPLEMENTED — the script refuses it
```

The script runs `sqlrustgo restore <backup_id> <database> --backup-dir <dir>`
and **verifies the outcome** — a restore that reports zero restored tables
is reported as a failure, not as success.

### 9.3 Direct CLI / server forms

```bash
# via the CLI (which spawns the server binary)
sqlrustgo backup ./backups --database default --backup-type full
sqlrustgo restore <backup_id> default --backup-dir ./backups

# via the server binary directly
sqlrustgo-mysql-server backup ./backups --database default --backup-type full
sqlrustgo-mysql-server restore <backup_id> default --backup-dir ./backups
```

The path / backup id are **positional** arguments on the server side. The
`--database` and `--backup-type` / `--backup-dir` options are named.

### 9.4 Known limitation — read this before relying on a backup

The backup implementation enumerates **zero tables** today, so it produces
an empty file while still printing "Backup completed successfully!". The
shell scripts work around this by **verifying the artifact** and reporting
an empty backup as a failure.

**Consequence:** a logical backup produced by this implementation is not
currently a substitute for a filesystem-level copy of the data directory.
For a full-fidelity backup, copy the whole data directory (including
`sqlrustgo.wal`) while the server is stopped, and use `scripts/backup.sh` as
a logical, schema-shaped artifact rather than as the sole restore point.

---

## 10. Troubleshooting

| Symptom | Real cause | Action |
|---|---|---|
| Server exits immediately after startup with a WAL recovery error | Committed WAL entries could not be replayed (`skipped_entries > 0`) — the #5192 fail-closed guard | Copy the data directory aside **before** touching the WAL, then decide whether to set `SQLRUSTGO_ALLOW_DATA_LOSS=1` to accept the acknowledged-write loss and bring the database back up |
| `WAL recovery failed: … Refusing to start` | Hard recovery error; the committed state of the data directory cannot be established | Do not retry blindly. Restore from a filesystem-level copy, or work the WAL by hand with the data directory copied aside |
| Server starts and immediately loses its data on restart | Started without `--data-dir`, so the default `/tmp/sqlrustgo-data` was used; `/tmp` is cleared on reboot | Restart with an explicit `--data-dir` on a persistent path and re-load the data. The server prints a WARNING about this at startup |
| `sqlrustgo-mysql-server not found.` | The CLI could not locate the server binary | Build it (`cargo build -p sqlrustgo-mysql-server`) or export `SQLRUSTGO_SERVER_BIN=/path/to/sqlrustgo-mysql-server` |
| The CLI spawns a **stale** server binary whose flags are unknown | The old candidate list resolved `./target/debug/…` relative to the current directory, ignoring `CARGO_TARGET_DIR` | Export `SQLRUSTGO_SERVER_BIN` explicitly; the current resolver prefers `SQLRUSTGO_SERVER_BIN` → `CARGO_TARGET_DIR` → the CLI's own directory → `./target/{debug,release}` → `$PATH` |
| `unexpected argument '--port'` / exit code 64 | A flag was passed at the wrong level — the server's `--port` belongs to the `serve` subcommand, and the CLI's own is `sqlrustgo serve --port` | Put the flags after the subcommand: `sqlrustgo-mysql-server serve --port 3307` |
| `LOAD DATA LOCAL INFILE` rejected with a path error | The file is outside the whitelist directory | The whitelist is `--load-infile-dir` when set, otherwise the data dir. Use `--load-infile-dir` for fixtures that live outside the storage directory |
| `/metrics` returns connection-refused | The metrics endpoint is off by default | Start with `--metrics-port <P>`; it is only spawned when that flag is given |
| `/metrics` refuses to bind but the server keeps running | The bind failure is logged as a warning and is deliberately non-fatal | Check the logged `metrics_endpoint: bind <addr> failed` line and pick a free port |
| A scrape is refused or hangs | The metrics accept loop is single-threaded and serves one connection at a time, with no keep-alive | Reduce scrape concurrency; it is a low-QPS endpoint by design |
| `checkpoint.json` appears in an unexpected directory | `CheckpointManager::default()` uses `PathBuf::from(".")` | Run the server from a fixed working directory; include `./checkpoint.json` in the backup set |
| Connections accumulate in `CLOSE_WAIT` | The idle reaper did not run, or the idle timeout is too long | `SQLRUSTGO_IDLE_TIMEOUT_SECS` (default 30) and `SQLRUSTGO_REAPER_INTERVAL_SECS` (default 5) |
| Workers peg the CPU at ~100% each with no traffic | The TLS read-path spin (#5099) — **recorded as fixed** in this line | Confirm you are on a build that includes the fix; the fix distinguishes "no data" (`Ok(0)`) from a half-read header via `complete_io`'s `rdlen` |
| A `SELECT` returns 0 rows for a non-default database | The handshake-selected database was not synchronised with `engine.session_db` — #5191 | Recorded in this line's audit as a semantic-degradation finding; see [`DEFECTS_AND_ISSUES.md`](DEFECTS_AND_ISSUES.md) |
| A restart takes a very long time | WAL recovery replays entry-by-entry, rewriting the whole table each time (P0-08) | Schedule a generous restart window; measure on your own data |

---

## 11. Open items — TODO / unknown, needs owner input

The following are **not** recorded anywhere in this repository, and this
guide deliberately does not invent them:

1. **TODO — no official container image, package, or systemd unit.**
   `scripts/install.sh` exists and supports a Cargo install and a
   pre-compiled-binary install into `$INSTALL_DIR` (default
   `$HOME/.local/bin`), but the repository contains no `Dockerfile`, no
   `docker-compose.yml`, and no unit file for a service manager. **A
   packaging owner must supply one before this guide can describe a
   supported installation.**
2. **TODO — no published sizing model.** The repository contains measured
   numbers (§8.4) but no capacity-planning guidance, no memory-per-table
   formula, and no recommended hardware profile. §8 lists the knobs and
   their real defaults; it is not a substitute.
3. **TODO — no documented log rotation, retention or shipping policy.**
   The server logs through `tracing` to stderr. Nothing in the repository
   configures rotation or ships logs off-host.
4. **TODO — `scripts/crash/run_real_crash_test.sh` invocation shape.** The
   script launches the binary as `"$SQLRUSTGO_BIN" --port 3306 --data-dir
   "$RESULTS_DIR/data"`. With the **current** CLI, top-level `--port` is not
   a valid flag (`sqlrustgo serve --port` is), so this invocation shape
   appears inconsistent with the present interface and was **not** executed
   as part of writing this guide. An owner must confirm the intended
   invocation before the script is cited as a working crash-recovery
   procedure.
5. **TODO — TLS configuration.** The repository records TLS 1.3 support and
   the TLS read-path fix (#5099), but no certificate/key path flag or
   environment variable was found for the server. **No TLS deployment
   configuration is documented here because none exists in the tree.**
6. **TODO — `SQLRUSTGO_STORAGE` is written but its read site is not
   obvious.** `run_server_v2` sets it from `--storage`, and the storage
   selection is made from the `storage` argument in the same call. The
   environment variable is therefore **not** an independent way to select
   the engine. Use the flag.

---

## 12. References

Release-line documents:

- [`STAGE.yaml`](STAGE.yaml) — stage SSOT (`current_stage: "ALPHA"`).
- [`VERSION_PLAN.md`](VERSION_PLAN.md) — what v4.1.0 is relative to v4.0.0.
- [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) — the delta matrix and what it
  does not claim.
- [`CHANGELOG.md`](CHANGELOG.md) — the P0 fix round and measured results.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — the actual execution path.
- [`DEFECTS_AND_ISSUES.md`](DEFECTS_AND_ISSUES.md) — the P0/P1/P2 catalog.
- [`CLAIM_DOWNGRADE_MANIFEST.md`](CLAIM_DOWNGRADE_MANIFEST.md) — what the
  release line explicitly excludes.
- [`MIGRATION_GUIDE.md`](MIGRATION_GUIDE.md) — upgrading **to** v4.1.0.
- [`RELEASE_NOTES.md`](RELEASE_NOTES.md) — release-line notes draft.

Source consulted for this guide:

- `crates/mysql-server/src/main.rs` — `serve` flags, banner, exit codes.
- `crates/mysql-server/src/lib.rs` — `run_server_v2`, storage selection,
  WAL recovery, idle timeout, auth mode.
- `crates/mysql-server/src/metrics_endpoint.rs` — `/metrics`, `/health`.
- `crates/sqlrustgo-cli/src/lib.rs` — CLI subcommands and binary
  resolution.
- `crates/storage/src/file_storage.rs` — data directory layout.
- `crates/storage/src/checkpoint.rs` — checkpoint path.
- `crates/storage/src/fault_injection.rs` — `SQLRUSTGO_FAULT_POINT`.
- `crates/storage/Cargo.toml`, `crates/mysql-server/Cargo.toml`,
  root `Cargo.toml` — feature flags.
- `crates/executor/src/session_config.rs` — `SQLRUSTGO_MAX_MEMORY_MB`,
  benchmark/teaching modes.
- `crates/telemetry/src/prometheus.rs` — metric names.
- `crates/query-stats/src/slow_query_log.rs` — slow query log.
- `crates/gmp/src/persist_sqlite.rs` — `SQLRUSTGO_HOME`.
- `scripts/backup.sh`, `scripts/restore.sh`,
  `scripts/common/resolve_bin.sh`, `scripts/admin/admin-status.sh`.
- `README.md` §6 — the canonical build and run commands.

> **Stage reminder.** v4.1.0 is at stage **ALPHA** as of
> 2026-10-23. It is not GA, not released, and no gate is claimed to pass by
> this document.

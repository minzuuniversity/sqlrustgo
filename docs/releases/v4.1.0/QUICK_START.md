# Quick Start — SQLRustGo v4.1.0

> **Date**: 2026-10-23
> **Baseline**: `develop/v4.1.0` @ `3a3350f660` — the tree this guide was
> written against. Every command and path below was executed against it.
> **Stage**: ALPHA (`docs/releases/v4.1.0/STAGE.yaml` → `current_stage: "ALPHA"`,
> re-entered 2026-10-06 — see `last_transition` in the same file)
> **Status**: **ALPHA-stage working document.** v4.1.0 is **not** GA, is **not**
> released, and no claim is made here that the release gates pass. The v4.0.0
> GA promotion was **revoked** on 2026-09-30 (see `docs/releases/v4.0.0/GA_RELEASE_TIMELINE.md`).
> **Scope**: build from source, start the server, connect with the bundled
> client, run a minimal SQL session, stop the server, and diagnose the first
> problems a new user hits. Every command below was executed against this
> tree; the transcripts quoted are real output.

---

## 1. What is SQLRustGo?

SQLRustGo is a SQL database engine written in Rust. It exposes **one** network
protocol — the **MySQL wire protocol** — through the `sqlrustgo-mysql-server`
binary, plus a bundled MySQL-protocol client and a `sqlite3`-like local mode,
both behind the `sqlrustgo` binary.

The storage path is a WAL + file storage engine with an MVCC layer
(`crates/storage/`), and statements are executed by the hand-written
interpreter in `src/engine_select.rs` and `src/engine_dml.rs`. The
`crates/planner` physical-plan layer is **not** on the production path —
see `docs/releases/v4.1.0/ARCHITECTURE.md` §2.2 for that boundary.

v4.1.0 is the **post-v4.0.0 maintenance continuation** of SQLRustGo. It
carries v4.0.0 forward through bugfix work (transaction / AuditChain,
storage regressions, `workers.push` restore) and adds 5-remote sync tooling.
It makes **no new feature claims** relative to v4.0.0 — see
[VERSION_PLAN.md](VERSION_PLAN.md) §2 for the exact delta table and
[FEATURE_MATRIX.md](FEATURE_MATRIX.md) for the status of each delta.

> 中文注记：v4.1.0 是 v4.0.0 之后的维护延续线，不替换 v4.0.0；当前阶段以
> `STAGE.yaml` 的 `current_stage: "ALPHA"` 为准。本文件不声明 GA、不声明门禁全绿。

### 1.1 Stage and honesty boundary

- The stage SSOT is `docs/releases/v4.1.0/STAGE.yaml`. Its `current_stage`
  is `"ALPHA"`.
- **Repository contradiction, stated rather than resolved:** the root
  `README.md` (§1 and §8), `docs/releases/v4.1.0/ARCHITECTURE.md` (header),
  `docs/releases/v4.1.0/README.md` and `docs/releases/v4.1.0/PHASE_1_SCOPE.md`
  still say `DRAFT (NOT YET ALPHA)`. The `STAGE.yaml` banner dated 2026-10-21
  explains that the DRAFT wording describes the 2026-09-30 rollback state and
  was superseded by the 2026-10-06 re-entry. **This document follows the
  SSOT (`ALPHA`)** and flags the stale wording here rather than silently
  picking one side.
- The version strings the binaries print are **stale relative to this release
  line**: the server banner says `SQLRustGo v3.8.0-beta (Strong Beta, 8.0/10)`
  and the REPL banner says `SQLRustGo REPL v3.8.0`. Those strings are
  hard-coded (`crates/mysql-server/src/main.rs`). They are *not* a statement
  about v4.1.0.

---

## 2. Prerequisites

| Requirement | Notes |
|---|---|
| Rust toolchain, `stable` channel | `cargo-toolchain.toml` pins `channel = "stable"`. **No MSRV is declared** — no `rust-version` key exists in any `Cargo.toml` in this tree — so do not assume a minimum version from these docs. |
| `cargo` | Comes with the toolchain. CI installs it via `rustup install stable` / `dtolnay/rust-toolchain@stable`. |
| A C linker (`cc` / `gcc` / `clang`) | Required by the native dependencies. `libz-sys` is pulled in with its `static` feature, so **no system zlib development package is needed** (root `Cargo.toml`, `libz-sys` comment). |
| `git` | Only if you clone the repository. |
| ~2 GB of free disk | A full workspace build of this tree writes a large `target/` directory. |

No database server, no Docker, and no external MySQL installation is
required for anything in this guide.

---

## 3. Build from source

From the repository root:

```bash
cargo build --release -p sqlrustgo-mysql-server -p sqlrustgo-cli
```

This produces two binaries:

| Binary | Produced by | Location |
|---|---|---|
| `sqlrustgo-mysql-server` | `crates/mysql-server/Cargo.toml` → `[[bin]] name = "sqlrustgo-mysql-server"` | `target/release/sqlrustgo-mysql-server` |
| `sqlrustgo` | `crates/sqlrustgo-cli/Cargo.toml` → `[[bin]] name = "sqlrustgo"` | `target/release/sqlrustgo` |

> **Naming trap.** Older release documents (for example
> `docs/releases/v3.9.0/INSTALL.md`) tell you to look for
> `target/release/sqlrustgo-cli`. That name is **stale**: the current
> `[[bin]]` in `crates/sqlrustgo-cli/Cargo.toml` is `sqlrustgo`. A file named
> `sqlrustgo-cli` in `target/` is a **stale artifact from an older build**,
> not the current binary.

Two more binaries exist in the workspace and are useful later:

| Binary | Package | Location |
|---|---|---|
| `sqlrustgo-admin` | `crates/admin/Cargo.toml` | `target/release/sqlrustgo-admin` |
| `sqlrustgo-soak` | `crates/cli/Cargo.toml` (package `sqlrustgo-soak`) | `target/release/sqlrustgo-soak` |

Check that the build produced what you expect:

```bash
./target/release/sqlrustgo --help
./target/release/sqlrustgo-mysql-server --help
```

A debug build (`cargo build -p sqlrustgo-mysql-server -p sqlrustgo-cli`)
works identically and is much faster to produce; every command in this
document was verified with a debug build.

---

## 4. Start the server

```bash
mkdir -p ./srg-data
./target/release/sqlrustgo-mysql-server serve \
  --host 127.0.0.1 \
  --port 3307 \
  --data-dir ./srg-data
```

The server prints its banner and then blocks:

```text
SQLRustGo v3.8.0-beta (Strong Beta, 8.0/10)
MySQL wire-protocol server
  Listen:     127.0.0.1:3307
  Data dir:   ./srg-data
  Max conn:   100
  Auth mode:  none
  Storage:    file
  Exec par:   1 (Issue #3703, --executor-parallelism)
Ready to accept connections.
```

Run it in a second terminal, or background it, to use the client in §5.

### 4.1 The `serve` flags that matter

All of these are read from `crates/mysql-server/src/main.rs`.

| Flag | Default | Meaning |
|---|---|---|
| `--host` | `127.0.0.1` | Listen address. |
| `--port` | `3306` | Listen port. |
| `--data-dir` | `/tmp/sqlrustgo-data` | Storage / WAL directory. |
| `--max-connections` | `100` | Concurrent-connection semaphore limit. |
| `--server-threads` | `16` | Connection-handler worker threads; valid range `0..=80`. |
| `--auth-mode` | `none` | `none` allows all connections; `password` requires a password. |
| `--storage` | `file` | `file` (WAL + FileStorage) or `binary` (BinaryTableStorage). |
| `--wal-sync` | `every` | `every`, `batch:N`, or `off`. |
| `--executor-parallelism` | `1` | Intra-query worker threads, `1..=1024`. Also settable via the `SQLRUSTGO_EXECUTOR_PARALLELISM` environment variable. |
| `--bulk-insert-rows-per-flush` | `10000` | Rows per `bulk_insert_records` flush during `LOAD DATA LOCAL INFILE`. |
| `--load-infile-dir` | *(unset)* | Whitelist directory for `LOAD DATA LOCAL INFILE`. Unset means the whitelist falls back to `--data-dir`. |
| `--metrics-port` | *(unset)* | Starts a Prometheus `/metrics` endpoint (text format 0.0.4) on this port. |
| `--verbose` | off | Adds `MVCC: enabled` to the banner. |
| `--log-level` | `info` | Global tracing filter; also settable via `RUST_LOG`. |

> **Always pass `--data-dir` for anything you intend to keep.** When
> `--data-dir` is omitted and the default `/tmp/...` path applies, the server
> prints a warning that the location is cleared on reboot and is
> world-writable. The warning is emitted by
> `crates/mysql-server/src/main.rs` and is not suppressed by the default.

---

## 5. Connect with the bundled client

The repository ships its own MySQL-protocol client — no external `mysql`
binary is required. The `sqlrustgo cli` subcommand connects to a running
server and executes one query:

```bash
./target/release/sqlrustgo cli --host 127.0.0.1 --port 3307 "SELECT 1"
```

```text
Connected to 127.0.0.1:3307 (server: 8.0.33-SQLRustGo)
1
-
1

1 row(s) in set
```

Its flags are `-p/--port` (default **3307**), `--host` (default
`127.0.0.1`), `-u/--user` (default `root` when omitted), `-w/--password`
(default empty when omitted). The query itself is a positional argument.

Any other MySQL client works too — the server speaks the MySQL wire
protocol, and the bundled client in `crates/mysql-client/src/lib.rs`
documents itself as "MySQL 8.0 wire protocol" with `mysql_native_password`
authentication.

A few one-liners worth trying first, to confirm the server is alive and to
see what it reports about itself (all verified against this tree):

```bash
./target/release/sqlrustgo cli -p 3307 "SHOW DATABASES"
# col_1
# -
# default

./target/release/sqlrustgo cli -p 3307 "SHOW TABLES"
# Tables_in_db
# -
# content
# vectors
# documents

./target/release/sqlrustgo cli -p 3307 "SHOW VARIABLES"
# col_1 | col_2
# -----
# version | sqlrustgo-3.12.0-controlled-subset
# sql_mode |
# autocommit | 1
# character_set_server | utf8
```

`SHOW DATABASES` returning the single database `default`, and `SHOW TABLES`
returning the three built-in tables `content`, `vectors` and `documents`, are
a good sign that a fresh server is healthy. `SHOW VARIABLES` is the reliable
place to read the server's own version identifier.

> **端口默认值不一致（重要）**：`sqlrustgo serve` 与 `sqlrustgo cli` 的默认端口是
> **3307**，而 `sqlrustgo-mysql-server serve` 的默认端口是 **3306**，
> `sqlrustgo-admin` 的 `status` / `processlist` / `kill` 默认端口也是 **3306**。
> 两端默认值不一致，混用时容易出现 “connection refused”。见 §7。

---

## 6. A minimal SQL session

With the server running, these are the four statements that exercise the
whole path (parse → execute → WAL → file storage → read back). All four were
run against this tree and produced the output shown.

```bash
./target/release/sqlrustgo cli -p 3307 "CREATE TABLE quickstart_demo (id INT PRIMARY KEY, name VARCHAR(64))"
./target/release/sqlrustgo cli -p 3307 "INSERT INTO quickstart_demo VALUES (1, 'alpha'), (2, 'beta')"
./target/release/sqlrustgo cli -p 3307 "SELECT * FROM quickstart_demo"
```

```text
Connected to 127.0.0.1:3307 (server: 8.0.33-SQLRustGo)
id | name
-----
1 | alpha
2 | beta

2 row(s) in set
```

The data is written into `--data-dir` as one JSON file per table plus the
WAL:

```text
quickstart_demo.json
quickstart_demo_idx_id.json
sqlrustgo.wal
```

### 6.1 No server at all

Two things work without a running server:

```bash
# In-process single statement — fresh in-memory engine, nothing is persisted.
./target/release/sqlrustgo exec "SELECT 1 + 1 AS two"
# col_1
# Integer(2)
# (1 rows)

# Interactive REPL over stdin — one shared engine for the whole session.
./target/release/sqlrustgo repl
# SQLRustGo REPL v3.8.0 — type `.help` for commands, `.exit` to quit
# sqlrustgo> SELECT 42 AS answer;
# col_1
# Integer(42)
# (1 rows)
# sqlrustgo>
```

`exec` runs each statement against a **new** engine, so state does not
survive between two `exec` invocations. Use `repl` when you want several
statements to share one catalog.

### 6.2 The `sqlite3`-like local mode

A second, server-free entry point creates a local database directory:

```bash
./target/release/sqlrustgo sqlite ./demo.sqlrg --cmd "CREATE TABLE t (id INT, name TEXT); INSERT INTO t VALUES (1,'x'); SELECT * FROM t;"
# id |name
#---|---
# 1  |x
```

The path you pass is a **directory**, not a single file: after the command
above, `./demo.sqlrg/t.json` exists. The same mode is reachable with the
implicit alias form `sqlrustgo ./demo.sqlrg "<SQL>"`, and with `--batch` to
read a script from stdin.

---

## 7. Common first problems

| Symptom | What is happening | What to check / do |
|---|---|---|
| `Connection failed: ...` / connection refused | The client and the server are on different default ports. `sqlrustgo cli` defaults to **3307**; `sqlrustgo-mysql-server serve` defaults to **3306**. | Pass the same explicit `--port` to **both** sides, e.g. `--port 3307` on the server and `-p 3307` on the client. |
| `sqlrustgo-mysql-server not found` | The `sqlrustgo` CLI could not locate the server binary. It searches, in order: `$SQLRUSTGO_SERVER_BIN`, then `$CARGO_TARGET_DIR/{debug,release}`, then the directory of the running `sqlrustgo` binary, then `./target/{debug,release}`, then `$PATH`. | Build the server next to the CLI (`cargo build -p sqlrustgo-mysql-server -p sqlrustgo-cli`), or set `SQLRUSTGO_SERVER_BIN=/absolute/path/to/sqlrustgo-mysql-server`. |
| `cargo build` fails with `cannot find -lz` | A missing system zlib. This tree already depends on `libz-sys` with the `static` feature, so the vendored zlib should be used. | Make sure you are building this workspace's manifests, and that `CARGO_TARGET_DIR` is not pointing at a tree built with different features. |
| Server warns `no --data-dir given; defaulting to /tmp/...` | The default data directory is not durable (rebooted away) and is world-writable. | Pass `--data-dir <persistent-path>` explicitly. |
| Ctrl-C does not stop the server | **Verified behaviour on this tree:** SIGINT does **not** terminate `sqlrustgo-mysql-server`; the process keeps running and keeps serving connections. The comment at `crates/mysql-server/src/main.rs:452` advertises "graceful shutdown via SIGINT/SIGTERM", and `install_signal_handler()` at line 576 is an explicit placeholder that installs no handler. SIGTERM **does** stop it. | Stop it with `kill -TERM <pid>`. Do not rely on Ctrl-C. (The reason the default SIGINT disposition does not apply was not determined from the source read here — treat the SIGINT behaviour as an open item, see §9.) |
| `SELECT VERSION()` returns `NULL` | `VERSION()` is parsed as a function call but is not implemented; it returns `NULL`. The version string in the startup banner is printed by the server, not produced by a query. | Run `SHOW VARIABLES` instead — it returns `version \| sqlrustgo-3.12.0-controlled-subset`. |
| `sqlrustgo-admin status` shows all zeros | The admin CLI issues `SHOW GLOBAL STATUS`, which the server does not parse: the server log shows `SQL error 1064 (42000): Parse error: Unexpected token after SHOW: Identifier("GLOBAL")`. The zeros are the result of that failed query, not a real measurement — `SHOW STATUS` reported `Threads \| 1` where the admin CLI reported `Threads: 0`. | Treat `sqlrustgo-admin status` output as not meaningful on this tree. Run `SHOW STATUS` through the bundled client instead. |
| `sqlrustgo-admin processlist` fails with `Unknown information_schema view: 'processlist'` | The `information_schema` implementation supports `schemata`, `tables`, `columns`, `indexes` only. | Use `SHOW PROCESSLIST` through the bundled client — it is accepted and returns a (currently empty) row set. |
| `SHOW GLOBAL STATUS` / `SHOW GLOBAL VARIABLES` fail with error 1064 | `SHOW GLOBAL ...` is not in the parser. The plain forms are. | The plain forms that were verified to work are `SHOW DATABASES`, `SHOW TABLES`, `SHOW STATUS`, `SHOW VARIABLES` and `SHOW PROCESSLIST`. `SHOW STATUS` returns `Uptime`, `Threads`, `Questions`, `Slow_queries`; `SHOW VARIABLES` returns `version`, `sql_mode`, `autocommit`, `character_set_server`. |
| Data disappears after a restart | The default `/tmp` data dir was used, or the server was killed without the data being flushed. | Always use an explicit `--data-dir`; the WAL is replayed on startup (`WAL recovery: total=... committed_txns=...` appears in the server log). |

---

## 8. What v4.1.0 does and does not claim

- v4.1.0 is **ALPHA**, not GA, and not released. The GA stage still has
  missing required artefacts and the GA gate list has never been executed
  end-to-end for this version — see [FEATURE_MATRIX.md](FEATURE_MATRIX.md) §3.
- The `V400-09` 168 h SOAK is registered as `status: "NOT MET"`,
  `decision: "OPEN"` in `STAGE.yaml` → `known_gaps`.
- Seven issues are registered as open in `STAGE.yaml` → `open_issues`.
- The parser coverage gap (60.58 % measured against the 80 % ALPHA
  threshold) is an open item, not a closed one.
- The full defect catalogue for this line is
  [DEFECTS_AND_ISSUES.md](DEFECTS_AND_ISSUES.md).

---

## 9. Open items that need owner input

The following could not be resolved from the repository and are recorded here
rather than guessed:

1. **Why SIGINT does not terminate the server.** The observable behaviour is
   confirmed (twice, with a correctly captured PID); the cause is not
   determined from the source read for this document. The comment at
   `crates/mysql-server/src/main.rs:452` and the placeholder
   `install_signal_handler()` at line 576 disagree with the observed
   behaviour — that disagreement needs an owner decision.
2. **The minimum supported Rust version (MSRV).** No `rust-version` key is
   present in any `Cargo.toml` in this tree, and `cargo-toolchain.toml`
   pins only the `stable` channel. An MSRV cannot be stated from the
   repository.
3. **Pre-built release binaries / package managers for v4.1.0.** No v4.1.0
   release artifact, Homebrew tap, apt/yum repository, or container image
   exists in this tree. The install paths described in older release
   documents (for example `docs/releases/v3.9.0/INSTALL.md`) are for older
   versions and must not be assumed to exist for v4.1.0. See
   [INSTALL.md](INSTALL.md) for what this document does and does not verify.

---

## 10. Where to go next

| Document | What it answers |
|---|---|
| [INSTALL.md](INSTALL.md) | Toolchain, feature flags, build variants, test commands, platform notes. |
| [FEATURE_MATRIX.md](FEATURE_MATRIX.md) | What v4.1.0 adds over v4.0.0, with the status of each delta. |
| [VERSION_PLAN.md](VERSION_PLAN.md) | The v4.1.0 scope and its delta table over v4.0.0. |
| [ARCHITECTURE.md](ARCHITECTURE.md) | The execution path and what is *not* on it. |
| [DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) | Deployment of a built binary. |
| [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) | Upgrade paths onto this line. |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | Release notes for this line. |
| [CHANGELOG.md](CHANGELOG.md) | Commit-level changes. |
| [EVALUATION_REPORT.md](EVALUATION_REPORT.md) | The stage evaluation report. |
| [TEST_PLAN.md](TEST_PLAN.md) | The test plan for this version. |
| [ROADMAP.md](ROADMAP.md) | Milestone status. |
| [DEFECTS_AND_ISSUES.md](DEFECTS_AND_ISSUES.md) | Open defect catalogue. |
| `docs/releases/v4.1.0/STAGE.yaml` | The stage SSOT — read this for the current stage. |

---

*Generated 2026-10-23 for the v4.1.0 ALPHA stage. Every command in this
document was executed against this working tree; quoted output is real.*

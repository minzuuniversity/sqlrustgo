# SQLRustGo v4.1.0 — Migration Guide

> **Baseline**: `develop/v4.1.0` @ `3a3350f660` — the tree this guide
> describes. Every version, path and command below was read out of it.
> **Status**: ALPHA — per [`STAGE.yaml`](STAGE.yaml) → `current_stage: "ALPHA"`
> (re-entered 2026-10-06, see `last_transition`). v4.1.0 is **not GA**, is
> **not released**, and **no gate verdict is asserted** by this document.
> **Date**: 2026-10-23
> **Scope**: upgrading **to** the v4.1.0 line from the previous line.
> **Base version**: v4.0.0 — whose self-claimed GA was **revoked** on
> 2026-09-30 (see [`../v4.0.0/GA_RELEASE_TIMELINE.md`](../v4.0.0/GA_RELEASE_TIMELINE.md)).
>
> Every version, command, flag, issue number and status below was read out
> of this repository. Where the repository does not record something, the
> item is listed under **§8 "Not recorded in this release line"** or
> **§9 "TODO / unknown — needs owner input"** rather than guessed.

---

## 1. Who this guide is for

You need this guide if you are moving a running SQLRustGo deployment onto
the `develop/v4.1.0` line.

Read it in this order:

1. §2 — what the v4.1.0 release line actually is. **Do this first**: the
   stage of the target matters more than anything else in this document.
2. §3 — what changes between the previous line and this one.
3. §4 — pre-upgrade checks. Run these **before** you stop anything.
4. §5 — the upgrade procedure.
5. §6 — rollback.
6. §7 — known incompatibilities and limitations.
7. §8 — what this release line does **not** record.

---

## 2. What the v4.1.0 release line is

### 2.1 Stage facts

| Fact | Value | Source |
|---|---|---|
| Current stage | **ALPHA** | [`STAGE.yaml`](STAGE.yaml) → `current_stage: "ALPHA"` |
| Stage re-entry | 2026-10-06, DRAFT → ALPHA | `STAGE.yaml` → `last_transition` |
| Branch | `develop/v4.1.0` | [`README.md`](README.md) §3.1 |
| Alpha tag | `v4.1.0-alpha1` exists (points to `c1a73a5320`) and is **retained as an audit trail**, not as a release credential | `STAGE.yaml`; root `README.md` §1 |
| Beta / RC / GA tags | **not cut** | `STAGE.yaml`; [`CHANGELOG.md`](CHANGELOG.md) "Status" list |
| GA artefacts | `GA_GATE_REPORT.md`, `GA_RELEASE_TIMELINE.md`, `EVIDENCE_STATUS.md`, `POST_GA_PLAN.md` are **missing**; the GA gate list has never been executed end-to-end for this version | [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §3.1 |

`STAGE_CONFIG.yaml` describes ALPHA as *"功能开发阶段 (active feature
development, API may change)"* — **API may change**.

### 2.2 The base is v4.0.0, and v4.0.0's GA was revoked

`VERSION_PLAN.md` defines v4.1.0 as the **post-v4.0.0 maintenance
continuation**: it does not replace v4.0.0; v4.0.0 stays on its release
branch while v4.1.0 is the active development trunk.

Two facts a migrating operator must carry:

- **v4.0.0's GA promotion was revoked on 2026-09-30.** The recorded reason
  is a self-reported coverage of **78.28%** against the GA threshold of
  **85%** (`COVERAGE_MIN_GA`; 80% is `COVERAGE_MIN_RC`), plus an
  unsupported WP-A "DONE" claim and three inherited gate failures that had
  not actually been run. See [`../v4.0.0/GA_RELEASE_TIMELINE.md`](../v4.0.0/GA_RELEASE_TIMELINE.md).
- **v3.12.0 is the most recent line with a GA claim**
  (GA CONDITIONAL PASS, 72/72 gates), and it too carries three GA-claim
  caveats that v4.1.0 inherits verbatim (§7.1).

### 2.3 A documentation contradiction you will hit

Several v4.1.0 documents still carry the pre-correction `DRAFT` banner —
including [`README.md`](README.md), [`ARCHITECTURE.md`](ARCHITECTURE.md),
[`VERSION_PLAN.md`](VERSION_PLAN.md), [`CHANGELOG.md`](CHANGELOG.md) and
[`RELEASE_NOTES.md`](RELEASE_NOTES.md).

**The stage SSOT is `STAGE.yaml`**, and its `current_stage` is `ALPHA`. The
2026-10-21 correction banner in `STAGE.yaml` explains why the older
"deliberately LEFT AT DRAFT" note is historical: the sequence was
2026-09-29 DRAFT → ALPHA (declared), 2026-09-30 ALPHA → DRAFT (rolled back
after a real gate run failed), 2026-10-06 DRAFT → ALPHA (re-entered). The
banner states explicitly that the current stage is ALPHA and that
`current_stage` plus `last_transition` are authoritative.

Treat the older `DRAFT` banners in the sibling documents as **stale
headers**, and treat the SSOT as the fact. This is the same reconciliation
approach [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §2 uses for the ROADMAP
snapshot: where two sources disagree, both are shown rather than one being
silently preferred.

[`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §3 item 8 records this disagreement
explicitly, naming four affected documents: `ARCHITECTURE.md` (2026-10-07,
"DRAFT (NOT YET ALPHA, per `docs/releases/v4.1.0/STAGE.yaml`)"),
`COMPREHENSIVE_ASSESSMENT_REPORT.md` (2026-10-05), `RELEASE_NOTES.md`, and
`STAGE.yaml` itself — and states that `docs/governance/STAGE_CONFIG.yaml`
makes `STAGE.yaml` the per-version SSOT. It records the disagreement rather
than hiding it, and this guide follows the same rule.

---

## 3. What changes between the previous line and v4.1.0

### 3.1 Carried-forward deltas

`VERSION_PLAN.md` §2 — status column from
[`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §1.1:

| ID | Area | Delta | Status |
|---|---|---|---|
| D-01 | zombie-fix core | Cherry-pick `67b624cbd2` + merge `9c6767a512`; `gap_lock_manager` + `read_write_splitter` updated | DONE |
| D-02 | `workers.push` | Commit `b4d46e6e18` + PR #4906 + PR #3793 merges | DONE |
| D-03 | DML/storage regressions | `f3595e7361` + PR #4905 + PR #3792 merges | DONE |
| D-04 | Review queue closure | 3 cherry-picks + zombie merge + `0bbb044da3` (CLOSED status doc) | DONE |
| D-05 | 5-remote sync tooling | `scripts/sync/5remotes_sync.sh`, `scripts/sync/5remotes_drift_check.sh`, `scripts/sync/README.md` | DONE |

D-01 to D-04 are **internal** changes — they do not alter the operator-facing
interface or the on-disk format. D-05 is **tooling for maintainers**, not a
runtime change.

### 3.2 The P0 fix round of 2026-10-08

From [`CHANGELOG.md`](CHANGELOG.md) §"[Unreleased] / P0 修复 — 2026-10-08",
with status from [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §1.2. These are
the changes that actually alter observed behaviour:

| ID | Item | Effect | Status |
|---|---|---|---|
| F-01 | #5099 concurrent-transaction row loss | Transaction identity moved off a shared slot. Loss rate 40.4% → ~1%. **`ERROR 1062` and the intermittent server hang remain unresolved.** | **PARTIAL** |
| F-02 | #5099 server hang (TLS read spin) | `read_exact` no longer busy-loops on a half-read header. | **FIXED** |
| F-03 | #5168 primary-key point lookup degraded to a full table scan | `WalStorage` / `ParallelWalStorage` now forward `scan_pk` / `scan_pk_range`. 20 000-row point lookup 9.02 ms → 0.467 ms. | **FIXED** |
| F-04 | #5177 `UPDATE`/`DELETE` `WHERE` clause used a full table scan | Added `pk_point_lookup`; UPDATE at 20 000 rows 10.941 ms → 1.051 ms. Conservative predicate matching; non-matching shapes fall back to a full scan, **semantics unchanged**. | **FIXED** |
| F-05 | #5181 delete path re-allocated the whole table | `Vec::retain` removes the second allocation; **the scan itself is still O(rows)**. | **PARTIAL** |
| F-06 | #5167 `COUNT(*)` / range scan / aggregates returned 0 or NULL | Point lookups were unaffected; the issue is recorded as closed. | **FIXED** (issue closed) |
| F-07 | 8 h SOAK | 28800.29 s, 0 FATAL / 0 errors / 0 reconnects. **Evidential weight is downgraded** — see the caveat below. | RUN, downgraded |

**F-07 caveat, carried verbatim.** The 8 h SOAK conclusion "cannot serve as
evidence that v4.1.0 is usable". The IAIA-410 truthfulness audit found
defects the SOAK structurally could not cover — notably the #5192 durability
illusion after `kill -9`. The SOAK was single-database, with no `kill -9`
and no restart, so "no data lost" and "query results correct" do **not**
hold. A green SOAK is **not** a GA-grade claim.

**Migration-relevant consequence of F-03 / F-04:** these changes alter
*how* the same query is executed, not *what* it returns. The `pk_point_lookup`
path was deliberately built to be semantically conservative — it falls back
to a full scan for AND/OR predicates, `CHAR` primary keys, tables with no
primary key, and qualified names. Expect **timing** to change materially
(queries that previously scanned the whole table will get much faster);
expect **results** not to change.

### 3.3 Interface notes (not recorded as v4.1.0 deltas)

These are facts about the current tree's command surface. They are called
out because they affect how you invoke the server and the backup tools —
but the release line's own CHANGELOG does **not** list them as v4.1.0
deltas, so do not read them as "new in v4.1.0".

- **Backup/restore positional arguments.** The server's `backup` subcommand
  takes the output path as a **positional** argument and `restore` takes the
  backup id as a **positional** argument. The `sqlrustgo` CLI forwards them
  that way today. An earlier forwarding used `--output <dir>` and
  `--input <id>`, which clap rejected with "unexpected argument" — the
  comments in `crates/sqlrustgo-cli/src/lib.rs` and `scripts/backup.sh`
  record that defect (#5516) and its fix. **If you have scripts that call
  `sqlrustgo backup --output …` or `sqlrustgo restore --input …`, they will
  fail with a clap usage error (exit 64) against the current tree.**
- **The CLI's `--port` default is 3307; the server's is 3306.** The two
  defaults differ. Pass `--port` explicitly in both cases.
- **CLI subcommand set** (`crates/sqlrustgo-cli/src/lib.rs`): `serve`,
  `exec`, `repl`, `bench`, `gmp`, `diag`, `backup`, `restore`, `cli`,
  `soak`, `sqlite`.
- **Server subcommand set** (`crates/mysql-server/src/main.rs`): `serve`,
  `exec`, `repl`, `bench`, `gmp`, `diag`, `backup`, `restore`.

Full operational detail for all of these is in
[`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md).

### 3.4 Storage format

**No on-disk format change is recorded in this release line's own
documents.** `VERSION_PLAN.md` §2 and `CHANGELOG.md` describe code-level and
performance-level changes; neither records a change to the `FileStorage`
file layout, the WAL record format, or the checkpoint format. See §8.

What *is* verifiable in the current tree, and matters for an upgrade:

- **The data directory layout is database-scoped.** Table files live at
  `<data_dir>/<database>/<table>.json`; index files at
  `<data_dir>/<database>/<table>_idx_<column>.json`. The **read** path falls
  back to `<data_dir>/<table>.json` when the database directory does not
  exist, so a directory written before that layout change stays readable.
  The **write** path never falls back.
- **The WAL file** is `<data_dir>/sqlrustgo.wal`.
- **The checkpoint file is `./checkpoint.json`, relative to the process's
  working directory** — `CheckpointManager::default()` uses
  `PathBuf::from(".")`. It is **not** inside the data directory. Whatever
  you back up, back this up too, and start the server from a consistent
  working directory before and after the upgrade.

---

## 4. Pre-upgrade checks

Run every one of these **before** you stop the running server. All of them
are read-only.

### 4.1 Confirm what you are upgrading to

```bash
# The stage SSOT — this is the authoritative stage record.
sed -n '1,40p' docs/releases/v4.1.0/STAGE.yaml
grep -n 'current_stage' docs/releases/v4.1.0/STAGE.yaml
```

Expect `current_stage: "ALPHA"`. If you are expecting GA, stop — this line
is not GA and this guide does not describe a GA upgrade.

### 4.2 Record the exact current invocation

You must be able to reproduce the **exact** flags of the running server
after the swap. Capture them now:

```bash
ps -eo pid,args | grep -i 'sqlrustgo-mysql-server' | grep -v grep
```

Write down: `--host`, `--port`, `--data-dir`, `--storage`, `--wal-sync`,
`--auth-mode`, `--max-connections`, `--server-threads`,
`--executor-parallelism`, `--metrics-port`, `--load-infile-dir`. The
canonical forms are in [`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md) §3.2.

### 4.3 Confirm the data directory is durable and is not under `/tmp`

```bash
grep -n 'WARNING: no --data-dir' /var/log/sqlrustgo/server.log 2>/dev/null
ls -ld /var/lib/sqlrustgo/data
```

If the server was started without `--data-dir`, its data lives under
`/tmp/sqlrustgo-data`, which is cleared on reboot and world-writable. The
server prints an explicit WARNING about this at startup. **Migrate the data
directory off `/tmp` as part of this upgrade**, not later.

### 4.4 Size the WAL and budget the restart

```bash
ls -l /var/lib/sqlrustgo/data/sqlrustgo.wal
```

WAL recovery replays entries one at a time, rewriting the whole table per
entry. This is registered as **P0-08** in
[`DEFECTS_AND_ISSUES.md`](DEFECTS_AND_ISSUES.md): *"WAL recovery 逐条整表重写,
重启 24 分钟不可用"*. The measured instance behind that headline: the first
full-table query after a restart took **24 min 22 s** at 68 288 rows /
58 288 INSERTs / 8.0 MB WAL (the identical query immediately after: 0.08 s).
Recovery completes **before** the listener accepts connections, so this is a
hard startup outage. Measure the restart on a **copy** of your data
directory before committing to a maintenance window.

### 4.5 Take a filesystem-level backup while the server is running, then a second one while it is stopped

The logical backup is **not** sufficient on its own — see §7.4. The
filesystem copy is your real restore point.

```bash
# Logical backup (see §7.4 for its current limitation).
bash scripts/backup.sh -d default -o ./backups -t full
```

`scripts/backup.sh` **verifies its own artifact**: it fails if the produced
`*.sql` contains no `CREATE TABLE`. A failure here means the logical backup
is empty — that is a real finding, not a script bug. Do not proceed on the
strength of a logical backup alone.

### 4.6 Note the checkpoint file location

```bash
ls -l ./checkpoint.json
```

It lives in the server's working directory, not in the data directory (§3.4).

---

## 5. Upgrade procedure

### Step 1 — Build the new binaries

```bash
cargo build --release --all-features
```

Resulting artifacts:

```
target/release/sqlrustgo-mysql-server
target/release/sqlrustgo
```

Keep the **old** binaries somewhere reachable (see §6). Do not overwrite
them in place.

### Step 2 — Stop the old server gracefully

Use `SIGTERM` so the server takes its shutdown path rather than being
killed outright:

```bash
kill -TERM <pid>
```

Confirm it exited:

```bash
ps -eo pid,args | grep -i 'sqlrustgo-mysql-server' | grep -v grep
```

### Step 3 — Take the cold filesystem copy

With the server **stopped**, copy the whole data directory and the
checkpoint file:

```bash
cp -a /var/lib/sqlrustgo/data /var/lib/sqlrustgo/data.bak.$(date +%Y%m%d_%H%M%S)
cp -a ./checkpoint.json ./checkpoint.json.bak.$(date +%Y%m%d_%H%M%S)
```

This is the point of no return for rollback — the copy is your rollback
target (§6).

### Step 4 — Swap the binary

```bash
mv target/release/sqlrustgo-mysql-server target/release/sqlrustgo-mysql-server.prev
# copy or install the freshly built binary into target/release/
```

If you install to a fixed location instead, keep the previous file next to
it rather than deleting it.

### Step 5 — Start the new server with the **same** flags

```bash
./target/release/sqlrustgo-mysql-server serve \
  --host 127.0.0.1 \
  --port 3307 \
  --data-dir /var/lib/sqlrustgo/data \
  --storage file \
  --wal-sync every \
  --auth-mode none \
  --max-connections 100 \
  --server-threads 16 \
  --executor-parallelism 1 \
  --metrics-port 9100
```

Use the flags you recorded in §4.2 — the list above is an **example**, not a
recommendation. Keep `--wal-sync` at the value you were running; changing
durability semantics during an upgrade is a separate decision.

### Step 6 — Verify the startup

Watch for these lines and check each one:

```
SQLRustGo v3.8.0-beta (Strong Beta, 8.0/10)
MySQL wire-protocol server
  Listen:     127.0.0.1:3307
  Data dir:   /var/lib/sqlrustgo/data
  ...
  Ready to accept connections.
```

And, in the log, the WAL recovery line:

```
WAL recovery: total=<n> committed_txns=<n> rows_inserted=<n> skipped=<n>
```

**`skipped=0` is the pass condition.** If `skipped` is non-zero, the server
**refuses to start** with a WAL-recovery error — that is the fail-closed
guard for #5192 doing its job. **Do not "fix" it by setting
`SQLRUSTGO_ALLOW_DATA_LOSS=1`.** That variable accepts the loss of
acknowledged writes; it is not a recovery switch. Go to §6.

Also confirm the **absence** of the `/tmp` warning (§4.3).

> The banner's `v3.8.0-beta` string is a hard-coded literal in the binary,
> not the release identity. Identify the build by commit / tag /
> `STAGE.yaml`.

### Step 7 — Functional smoke test

```bash
# Wire-protocol round trip through the bundled client.
./target/release/sqlrustgo cli --host 127.0.0.1 --port 3307 --user root "SELECT 1"
```

Then, against your real data, verify:

- a **point lookup** returns the same rows as before (F-03 changed the plan,
  not the result);
- a **range scan** and `COUNT(*)` return the expected values (F-06 closed a
  defect where these returned 0 or NULL);
- an `UPDATE`/`DELETE` with a `WHERE` clause affects the expected number of
  rows (F-04 changed the plan, not the result).

### Step 8 — Watch the first hour

The TLS read-path spin (F-02) manifested as workers pegged at ~100% CPU
with no traffic. Watch CPU. The 8 h SOAK recorded a CPU average of 164.2%
and a peak of 174.0% across 950 samples — **but that run's evidential weight
is downgraded** (§3.2, F-07).

If you have `--metrics-port` enabled, scrape `GET /metrics` and confirm
`sqlrustgo_queries_served_total` is advancing and
`sqlrustgo_query_errors_total` is flat. The metric names are listed in
[`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md) §7.2.

---

## 6. Rollback

### 6.1 When to roll back

- The new server **refuses to start** with a WAL recovery error
  (`skipped > 0`) — the committed state of the data directory could not be
  established.
- A new error class appears in `sqlrustgo_query_errors_total` that was not
  present before.
- Functional smoke tests (§5, step 7) show wrong results, not just
  different timings.

### 6.2 How to roll back

The rollback is the reverse of §5, against the **cold copy** from §5 step 3:

```bash
# 1. Stop the new server.
kill -TERM <new-pid>

# 2. Restore the data directory from the cold copy.
mv /var/lib/sqlrustgo/data /var/lib/sqlrustgo/data.failed.$(date +%Y%m%d_%H%M%S)
cp -a /var/lib/sqlrustgo/data.bak.<timestamp> /var/lib/sqlrustgo/data

# 3. Restore the checkpoint file.
cp -a ./checkpoint.json.bak.<timestamp> ./checkpoint.json

# 4. Restore the previous binary.
mv target/release/sqlrustgo-mysql-server.prev target/release/sqlrustgo-mysql-server

# 5. Start the previous version with the original flags (§4.2).
```

### 6.3 Rollback caveats

- **Do not use `SQLRUSTGO_ALLOW_DATA_LOSS=1` as a rollback shortcut.** It
  starts the server while acknowledging that committed writes were dropped.
  It is appropriate only after the data directory has been copied aside and
  a human has decided to accept the loss.
- **Rollback safety depends on §5 step 3.** If you did not take a cold copy
  with the server stopped, you do not have a verified restore point, and the
  logical backup alone does not cover this (§7.4).
- **No on-disk format incompatibility is recorded** between the previous
  line and this one (§3.4), so a file-level rollback is not expected to need
  a conversion step. If you observe one, it is not recorded in this release
  line — treat it as a defect and report it.

---

## 7. Known incompatibilities and limitations

### 7.1 Inherited GA-claim caveats (3 items)

v4.1.0 inherits all three v3.12.0 GA-claim caveats unchanged, via
[`../v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md`](../v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md)
and [`../v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md`](../v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md):

| Issue | Subsystem | Exclusion |
|---|---|---|
| #4846 | Executor / type semantics | `CHAR(n)` byte-padding primary-key point lookup is excluded |
| #4847 | Transaction semantics | explicit `BEGIN` / `COMMIT` / `ROLLBACK` is excluded |
| #4848 | Storage / DDL | `ALTER TABLE ... RENAME COLUMN` is excluded |

**These are the three you are most likely to hit in an upgrade**, because
they are behavioural exclusions rather than performance limits.

### 7.2 v4.1.0-specific downgrade: #5304

[`CLAIM_DOWNGRADE_MANIFEST.md`](CLAIM_DOWNGRADE_MANIFEST.md) §3.1 excludes the
advertised `BufferPool` / `Page` / disk B+Tree capability from v4.1.0
release claims. The `sqlrustgo_storage::BufferPool` type, the
`sqlrustgo_storage::Page` type and the disk B+Tree in
`crates/storage/src/bplus_tree/index.rs` are `pub use`-exported from
`src/lib.rs` for **storage-test code only**. Production `FileStorage` is an
in-memory `HashMap` plus a JSON `insert_buffer` — **NON-paged**.

The manifest's own note (added after the #5328 merge) records that PR2 of
#5304 landed **only trait-level plumbing** (`buffer_pool_stats()` default +
`BoxStorageEngine` forwarding + `estimated_pages_for_table()` default), and
that **production v4.1.0 still does NOT page through a BufferPool**.

**Migration consequence:** do not size your deployment as if the server
pages its data. It holds working sets in memory and writes JSON table
files. A large working set is a memory concern, not a page-cache concern.

### 7.3 The durability illusion (#5192) — the guard exists, the root cause does not

The startup path is **fail-closed**: `guard_unrecoverable_commits()` refuses
to start when committed WAL entries cannot be replayed (§5, step 6). The
code comment states the underlying reason plainly: *"the underlying replay
bug (WAL entries carry no database identifier) is not fixed yet"*.

**Migration consequence:** after `kill -9`, restart the server and **read
the WAL recovery line before trusting the database**. A server that starts
cleanly after a crash has, in this release line, still not demonstrated
end-to-end crash durability for committed writes.

Also recorded in the same audit round: **#5191** — the handshake-selected
database is not synchronised with `engine.session_db`, so queries against a
non-default database can silently return 0 columns and 0 rows. Test against
your **actual** database names after the upgrade, not only `default`.

### 7.4 The logical backup implementation enumerates zero tables

`scripts/backup.sh` and `scripts/restore.sh` both **verify their outcome**
and report an empty backup / zero-table restore as a **failure** rather than
a success, precisely because the logical backup implementation enumerates
zero tables today while still printing "Backup completed successfully!".

**Migration consequence:** treat the logical backup as a secondary artifact.
Your restore point is the filesystem copy of the data directory plus
`./checkpoint.json`.

### 7.5 Registered open issues at the ALPHA stage

`STAGE.yaml` → `open_issues` recorded **seven** items that nothing in the delta
matrix resolves ([`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §1.6 gives each one
a row with the defect-catalog entry it corresponds to). **Five of them were
closed on 2026-10-23**; the closures are recorded in
[`evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md`](evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md)
and in `STAGE.yaml` → `resolved_issues_2026_10_23`. **Two remain open.**

| Item | Summary | Status |
|---|---|---|
| O-01 / P0-03 | The **v4.0.0 "GA (CONDITIONAL PASS)" wording conflict** — `CURRENT_VERSION.md` and `docs/releases/v4.0.0/RELEASE_NOTES.md` still write v4.0.0 as "GA (CONDITIONAL PASS)", contradicting `docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status.verdict=REVOKED` (v4.0.0's GA was revoked on 2026-09-30) | **RESOLVED 2026-10-23** — both files now carry the SSOT's wording: the code was published (2026-09-19, tag `v4.0.0-final` @ `54571eeca0`, on all five remotes' `main`), but v4.0.0 never legitimately reached GA and is not a GA release |
| P0-01 | `check_anti_ignore_gate.sh` is a **no-op gate** — it reads two numbers out of the registry JSON and compares them to hard-coded limits; it never scans the source tree. The `active` count keys on a `status` field that no registry entry carries, so `active` is structurally always 0 | **RESOLVED 2026-10-23** — the gate recounts the tree with the registry's own rule and fails closed on drift; measured tree count 119 == registry `total_allowed` 119, exit 0 |
| P0-02 | #4708 is a false "DONE" — block comments (`/* */`) are not supported by the parser at all, masked by an unregistered `#[ignore]` at `crates/parser/tests/wp_a_legacy.rs:68` | **RESOLVED 2026-10-23** — the parser has a `/* ... */` branch (`crates/parser/src/lexer.rs:78`) and the `#[ignore]` at `wp_a_legacy.rs:68` is gone |
| P0-05 | `docs/governance/debt/debt-registry.yaml` `SEM-4`'s closure root cause is still the literal placeholder `XYZ`; `closing_pr` is `[]`, and the registry's `parser 75.16%` is not reproducible (measured 60.58 %) | **RESOLVED 2026-10-23** — `SEM-4` is `CLOSED` (`closed_at: "2026-10-23"`, `closing_pr: [5126, 5537]`) and the placeholder root cause is replaced by a measurement (the coverage had been taken with parallel test threads, so the reading depended on machine load). Measured: 8/8 crates, L1_8 **87.80 %**, `parser 86.46 %` |
| P1-01 | parser coverage gap — see the table below; the figure **and** the threshold are both unsettled | **RESOLVED 2026-10-23** for the fail-open half: a crate that cannot be measured now fails the gate (`check_coverage_v312.sh:315-349`), and the measured parser figure is **86.46 %** (L1_8 87.80 %), not 60.58 %. The *threshold* question stays with the stage framework |
| P1-02 | #4540 Q20 oracle is not closed — `sqlrustgo/q20.tsv` does not exist (21/22 files present) and `CROSS_ENGINE_HASH_CHECK.json` records `present:false`, contradicting the "Q20 PASS" in `docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_REPORT.md` | **OPEN** |
| P0-08 | WAL recovery rewrites the whole table per record — the measured instance is a **24 min 22 s** first query after restart, and recovery completes before the listener accepts connections | **OPEN** |

Plus the wider catalog in [`DEFECTS_AND_ISSUES.md`](DEFECTS_AND_ISSUES.md) and
the **24-issue** backlog in `ISSUES_PLAN.md` §4.

**Defect counts — two readings, both recorded.**
`DEFECTS_AND_ISSUES.md` §0 records **P0 × 8 / P1 × 7 / P2 × 7 = 22**;
`RELEASE_NOTES.md` and `ARCHITECTURE.md` both quote **"P0×6 + P1×7 + P2×7"**.
[`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) §3 item 9 records the disagreement
and deliberately does not pick one. Either way, **eight or six P0 items are
open**, and the catalog explicitly says P0 blocks any release.

**Parser coverage — four figures coexist, and the threshold is disputed too.**

| Figure | Source | Date |
|---|---|---|
| **60.58 %** (682 passed / 0 failed) | `DEFECTS_AND_ISSUES.md` P1-01; root `README.md` §1 | 2026-10-01 |
| **74.51 %** | `STAGE.yaml` → `last_transition` (alongside storage 81.45 %, executor ~82 %, mysql-server ~75 %) | 2026-10-06 |
| **86.20 %** | `evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`; recorded as **COV-01 `DONE`** in `STAGE.yaml` | 2026-10-07 |
| **86.46 %** (full L1_8 run, L1_8 average 87.80 %) | `evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3 — taken with `--test-threads=1`, which is what makes the reading independent of machine load | 2026-10-23 |

`FEATURE_MATRIX.md` §3 items 6 and 7 quote the 60.58 % and 86.20 %
measurements with their dates and call **neither "the" number**. The
threshold is equally unsettled: `docs/governance/STAGE_CONFIG.yaml` sets
`COVERAGE_MIN_ALPHA: 50` and `STAGE.yaml` → `exit_criteria.alpha_to_beta`
says "Coverage >= 50 %", while `DEFECTS_AND_ISSUES.md` P1-01 and
`ARCHITECTURE.md` §1 use **80 %** — which is the *Beta* figure
(`COVERAGE_MIN_BETA: 75`; `ALPHA_TO_BETA_GATE_PLAN.md` §2 applies the
stricter G17 Beta 80 %).

**Do not treat parser coverage as a settled upgrade gate.** The readings point
in different directions, and the repository itself declines to choose — though
the 2026-10-23 measurement is the first one taken under a measurement
**convention** that does not depend on machine load.

### 7.6 V400-09 — the 168 h SOAK is NOT MET

`STAGE.yaml` → `known_gaps.V400_09_soak` records `status: "NOT MET"` and
`decision: "OPEN"` — whether to re-run it is an architect decision,
deliberately left open. The `alpha_to_beta` exit criterion *"FINAL_REPORT
exists OR explicit deferral recorded"* is therefore not satisfied by the
8 h SOAK in §3.2 (which is a different run, with downgraded evidential
weight).

### 7.7 Governance facts that constrain any upgrade

- **`main` and `develop/v4.1.0` have diverged.** `LEGACY_ISSUES.md` §3.2
  records `main` behind `develop/v4.0.0` by 16905 commits and **not** an
  ancestor of `develop/v4.1.0`. The divergence was recorded as a known
  state pending an explicit decision.
- **The 5-remote sync tooling requires SSH access** to `z440@192.168.0.250`
  and `liying@192.168.0.252` for the protected-branch update-ref path
  (`LEGACY_ISSUES.md` §3.3). Without it, protected branches cannot be
  advanced via the sync tooling.
- **`STAGE_CONFIG.yaml` describes ALPHA as a stage where the API may
  change.** Treat this release line as moving.

---

## 8. Not recorded in this release line

The following are things an operator would reasonably want in a migration
guide that **this repository does not record**. They are listed here
explicitly so that nobody fills the gap with a guess:

1. **No on-disk format migration is recorded.** Neither `VERSION_PLAN.md` §2
   nor `CHANGELOG.md` records a change to the `FileStorage` file layout, the
   WAL record format, or the checkpoint format between the previous line and
   v4.1.0. §3.4 describes the layout as it exists in the current tree and
   notes that the read path tolerates both the database-scoped and the
   pre-#5025 root layouts — which is why no conversion step is described.
2. **No wire-protocol change is recorded.** No change to the MySQL
   handshake, capability flags, or command set is recorded in this release
   line's documents.
3. **No client-library API change is recorded.** The `crates/mysql-client`
   surface is not mentioned in the v4.1.0 delta documents.
4. **No configuration-file format change is recorded.** The server is
   configured by CLI flags and environment variables; there is no config
   file, and no migration of one is described.
5. **No minimum client-version requirement is recorded**, and no
   compatibility matrix against a specific `mysql` client version is
   published in this release line.
6. **No recorded procedure for migrating a database *between* two v4.1.0
   builds**, or for downgrading from v4.1.0 to the previous line with data
   written by v4.1.0. §6 describes a file-level rollback; no logical
   down-migration is recorded.
7. **No recorded SLA, support window, or end-of-life policy** for the
   v4.1.0 line.

---

## 9. TODO / unknown — needs owner input

1. **TODO — the release line's `CHANGELOG.md` and `RELEASE_NOTES.md` still
   carry a stale `DRAFT` banner.** §2.3 explains the reconciliation, but the
   documents themselves have not been re-bannered. An owner must decide
   whether to update them or to leave the correction solely in `STAGE.yaml`.
2. **TODO — the `v4.1.0-alpha1` tag does not name a current stage
   credential.** `STAGE.yaml` retains the tag as an audit trail and records
   the disposition as *"ACCEPTED AS-IS (human decision, 2026-09-30)"*. It is
   **not** a GA credential and there is no `v4.1.0-alpha2`. An owner must
   decide whether to cut one.
3. **TODO — no release artefact exists for a deployment.** This guide
   describes running the server from a build tree. There is no image, no
   package, no service unit, and no published artefact checksum in this
   repository. An owner must supply the packaging and the artefact
   provenance before an operator can install a *known* v4.1.0 build rather
   than a locally built one.
4. **TODO — the migration path from a v3.12.0 GA deployment specifically is
   not written.** v3.12.0 is the last line with a GA claim, but the delta
   documents in this repository describe v4.1.0 **relative to v4.0.0**, not
   relative to v3.12.0. A v3.12.0 → v4.1.0 upgrade would traverse the
   v4.0.0 deltas, which this release line does not enumerate.
5. **TODO — `scripts/crash/run_real_crash_test.sh` invocation shape.** The
   script launches the binary as `"$SQLRUSTGO_BIN" --port 3306 --data-dir
   "$RESULTS_DIR/data"`. With the current CLI, top-level `--port` is not a
   valid flag (`sqlrustgo serve --port` is), so this invocation shape
   appears inconsistent with the present interface. It was **not** executed
   while writing this guide. An owner must confirm the intended invocation
   before the script is cited as a working crash-recovery procedure for the
   upgrade.
6. **TODO — no performance regression budget is defined.** F-03/F-04 change
   timings substantially in the *good* direction, and F-05 leaves the delete
   path O(rows). The repository defines no acceptable-regression threshold
   that an upgrade could be measured against.

---

## 10. References

Release-line documents:

- [`STAGE.yaml`](STAGE.yaml) — the stage SSOT: `current_stage`,
  `last_transition`, `known_gaps`, `open_issues`.
- [`VERSION_PLAN.md`](VERSION_PLAN.md) — what v4.1.0 is relative to v4.0.0.
- [`CHANGELOG.md`](CHANGELOG.md) — the P0 fix round and the measured
  results.
- [`FEATURE_MATRIX.md`](FEATURE_MATRIX.md) — the delta matrix, the
  milestone reconciliation, and what the line does not claim.
- [`PHASE_1_SCOPE.md`](PHASE_1_SCOPE.md) — the DRAFT → ALPHA work plan.
- [`LEGACY_ISSUES.md`](LEGACY_ISSUES.md) — inherited WP catalog, the SSOT
  contradiction, the `main` divergence, the sync-tooling SSH constraint.
- [`CORE_PATH_INVENTORY.md`](CORE_PATH_INVENTORY.md) — what is actually in
  the production path.
- [`CLAIM_DOWNGRADE_MANIFEST.md`](CLAIM_DOWNGRADE_MANIFEST.md) — the three
  inherited caveats and the #5304 exclusion.
- [`DEFECTS_AND_ISSUES.md`](DEFECTS_AND_ISSUES.md) — P0×6 + P1×7 + P2×7.
- [`ARCHITECTURE.md`](ARCHITECTURE.md) — the actual execution path.
- [`DEPLOYMENT_GUIDE.md`](DEPLOYMENT_GUIDE.md) — operating the server.
- [`../v4.0.0/GA_RELEASE_TIMELINE.md`](../v4.0.0/GA_RELEASE_TIMELINE.md) —
  the v4.0.0 GA revocation and its three promotion paths.
- [`../v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md`](../v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md)
  and [`../v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md`](../v3.12.0/CLAIM_DOWNGRADE_MANIFEST.md)
  — the origin of the three inherited caveats.
- [`../../../docs/governance/STAGE_CONFIG.yaml`](../../../docs/governance/STAGE_CONFIG.yaml)
  — the version-agnostic stage framework.

Source consulted for this guide:

- `crates/mysql-server/src/main.rs` — subcommands, flags, exit codes.
- `crates/mysql-server/src/lib.rs` — WAL recovery and the fail-closed
  guard, storage selection, auth mode.
- `crates/sqlrustgo-cli/src/lib.rs` — CLI surface and binary resolution.
- `crates/storage/src/file_storage.rs` — the data directory layout and the
  read-side fallback.
- `crates/storage/src/checkpoint.rs` — the checkpoint path.
- `scripts/backup.sh`, `scripts/restore.sh` — backup/restore behaviour and
  artifact verification.

> **Stage reminder.** v4.1.0 is at stage **ALPHA** as of 2026-10-23. It is
> not GA, not released, and no gate is claimed to pass by this document.
> `STAGE_CONFIG.yaml` warns that at this stage the API may change.

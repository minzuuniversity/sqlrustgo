# Install — SQLRustGo v4.1.0

> **Date**: 2026-10-23
> **Baseline**: `develop/v4.1.0` @ `3a3350f660` — the tree this guide was
> written against. Every path, feature name and command below was read out
> of it.
> **Stage**: ALPHA (`docs/releases/v4.1.0/STAGE.yaml` → `current_stage: "ALPHA"`,
> re-entered 2026-10-06 — see `last_transition` in the same file)
> **Status**: **ALPHA-stage working document.** v4.1.0 is **not** GA and **not**
> released. This document describes how to build the v4.1.0 line **from
> source in this repository**. It does **not** describe a downloadable
> release artifact, because no v4.1.0 release artifact exists in this tree.
> **Scope**: prerequisites, toolchain, build variants, the binaries that are
> produced and where they land, the real Cargo feature names, how to run the
> test suite, and platform notes.
>
> Every command, flag, feature name and path below was read out of this
> repository (manifests, source, CI configuration) or executed against this
> working tree.

---

## 1. Read this first

Three facts about this release line that affect installation:

1. **There is no v4.1.0 pre-built binary, package-manager tap, or container
   image in this repository.** The release directories under `docs/releases/`
   contain documents only. Older release documents (for example
   `docs/releases/v3.9.0/INSTALL.md`) describe Homebrew, apt, yum, scoop and
   Docker install paths for *those* versions; **none of those paths is
   verified to exist for v4.1.0**, and none of them is reproduced here.
   Section 8 lists exactly what this document does and does not cover.
2. **v4.0.0's GA promotion was revoked on 2026-09-30** (root `README.md` §1;
   `docs/releases/v4.0.0/GA_RELEASE_TIMELINE.md`). v4.1.0 inherits that
   boundary and is at the ALPHA stage.
3. **The version numbers are inconsistent across the tree, by design and by
   oversight.** The root `Cargo.toml` carries `version = "3.12.0-fix-zombie"`
   (the root `README.md` explains this is a deliberate, retained release
   convention), `crates/sqlrustgo-cli/Cargo.toml` declares `version = "3.9.0"`,
   and `crates/mysql-server/Cargo.toml` declares `version = "0.1.0"`. The
   release line is defined by `docs/releases/v4.1.0/STAGE.yaml`, not by any
   `Cargo.toml` version field.

---

## 2. Prerequisites

| Requirement | Version | Source of truth |
|---|---|---|
| Rust toolchain | `stable` channel | `cargo-toolchain.toml` → `[toolchain] channel = "stable"` |
| `cargo` | from the same toolchain | CI: `rustup install stable`, or `dtolnay/rust-toolchain@stable` |
| C linker | `cc` / `gcc` / `clang` | native dependencies of the workspace |
| System zlib dev package | **not required** | root `Cargo.toml` declares `libz-sys = { version = "1.1", features = ["static"] }`, so zlib is built from vendored source |
| `git` | any recent version | only for cloning |
| Disk | ~2 GB free | full workspace build |

### 2.1 Minimum supported Rust version (MSRV)

**No MSRV can be stated from this repository.** No `Cargo.toml` in the tree —
root or member — declares a `rust-version` key (verified by searching every
manifest). `cargo-toolchain.toml` pins only the `stable` channel and a fixed
component set:

```toml
[toolchain]
channel = "stable"
components = ["rust-src", "rust-analyzer", "clippy", "rustfmt", "llvm-tools-preview"]
targets = ["aarch64-apple-darwin", "x86_64-unknown-linux-gnu"]
profile = "default"
```

Install with `rustup` and let it pick the pinned toolchain automatically:

```bash
rustup show          # confirms the channel and the installed targets
cargo --version
```

### 2.2 Inspecting the resolved dependency graph

```bash
cargo tree --workspace
cargo metadata --format-version 1 --no-deps > /tmp/sqlrustgo-metadata.json
```

---

## 3. Build from source

### 3.1 Release build (what you want for running the server)

```bash
cargo build --release -p sqlrustgo-mysql-server -p sqlrustgo-cli
```

### 3.2 Debug build (what you want while developing)

```bash
cargo build -p sqlrustgo-mysql-server -p sqlrustgo-cli
```

### 3.3 Whole-workspace build — and a trap worth knowing

```bash
cargo build --all-features
```

This builds the **root package** `sqlrustgo` and its features. It does
**not** build the member packages' binaries. The CI configuration states
this explicitly (`.gitea/workflows/ci.yml`, the `Build` step): *"cargo build
inside a workspace that has a root package builds ONLY that root package —
the member packages' binaries are never produced."* The root package
`sqlrustgo` has no package-local `main.rs` entry point, so even
`target/debug/sqlrustgo` comes from the member package `crates/sqlrustgo-cli`
(entry file `crates/sqlrustgo-cli/src/local_bin.rs`).

The reason it matters: several test targets spawn a real server binary and
abort when it is missing — `tests/e2e/e2e_canonical_subprocess.rs` says so
verbatim ("canonical binary not found at …; run `cargo build -p
sqlrustgo-mysql-server` first"). **Always build the two member packages
explicitly**, as in §3.1 and §3.2.

### 3.4 Build time

The CI pipeline records a measured figure in `.gitea/workflows/ci.yml`:
*"a `cargo build` of this tree takes ~7m30s of CPU-bound compilation"*,
measured on the CI host on 2026-10-22. The same file records that the CI
container has **no cargo cache**, so every job rebuilds the workspace from
scratch, and that the runner enforces a **3 h container limit**.

> 中文注记：本仓库 CI 注释记录的整树构建耗时约 7 分 30 秒（2026-10-22 实测），
> CI 容器无 cargo 缓存，每次 job 都从头构建。

### 3.5 Smoke check

```bash
./target/release/sqlrustgo --help
./target/release/sqlrustgo-mysql-server --help
./target/release/sqlrustgo-mysql-server serve --help
```

---

## 4. The binaries, and where they land

| Binary | Package | Manifest | Output path |
|---|---|---|---|
| `sqlrustgo` | `sqlrustgo-cli` | `crates/sqlrustgo-cli/Cargo.toml` → `[[bin]] name = "sqlrustgo"` | `target/{debug,release}/sqlrustgo` |
| `sqlrustgo-mysql-server` | `sqlrustgo-mysql-server` | `crates/mysql-server/Cargo.toml` → `[[bin]] name = "sqlrustgo-mysql-server"` | `target/{debug,release}/sqlrustgo-mysql-server` |
| `sqlrustgo-admin` | `sqlrustgo-admin` | `crates/admin/Cargo.toml` → `[[bin]] name = "sqlrustgo-admin"` | `target/{debug,release}/sqlrustgo-admin` |
| `sqlrustgo-soak` | `sqlrustgo-soak` | `crates/cli/Cargo.toml` → `[[bin]] name = "sqlrustgo-soak"` | `target/{debug,release}/sqlrustgo-soak` |

> **The `sqlrustgo-cli` name is stale.** `crates/sqlrustgo-cli/Cargo.toml`
> declares the *package* name `sqlrustgo-cli` but the *binary* name
> `sqlrustgo`. Older release documents point at
> `target/release/sqlrustgo-cli`; a file with that name in `target/` is a
> leftover from an older build, not the current binary.

### 4.1 The CLI needs the server binary next to it

`sqlrustgo` delegates `serve`, `exec`, `repl`, `bench`, `gmp`, `diag`,
`backup` and `restore` to the `sqlrustgo-mysql-server` binary, and locates it
in this order (`crates/sqlrustgo-cli/src/lib.rs`, `mysql_server_bin()`):

1. `$SQLRUSTGO_SERVER_BIN` — explicit override, the repository-wide convention.
2. `$CARGO_TARGET_DIR/{debug,release}/sqlrustgo-mysql-server`.
3. The directory the running `sqlrustgo` binary itself lives in.
4. `./target/{debug,release}/sqlrustgo-mysql-server`, relative to the current
   working directory.
5. `$PATH`.
6. If none match, the bare name is kept so the spawn fails loudly with
   "No such file or directory" rather than silently running a stale binary.

Build both packages together (§3.1) so that rules 3 and 4 resolve.

### 4.2 Installing the binaries onto `PATH`

There is no packaging recipe in this repository. If you want the binaries on
`PATH`, copy them yourself and keep them in the same directory so that rule 3
above keeps working:

```bash
sudo install -m 0755 target/release/sqlrustgo target/release/sqlrustgo-mysql-server /usr/local/bin/
```

---

## 5. Optional features — the real names

Features are per-package. Enabling one requires selecting the package that
declares it.

### 5.1 Root package `sqlrustgo` (root `Cargo.toml`, `[features]`)

| Feature | Effect |
|---|---|
| `bin_storage_default` | Enables the `bin_storage_default` path in `sqlrustgo-storage`. |
| `v313_3_profile` | Enables the `v313_3_profile` path in `sqlrustgo-storage` (profiling hooks; off by default). |
| `parallel-executor` | Turns on `sqlrustgo-executor/parallel-executor` — the intra-query parallel scan/join/agg pipelines. Without it, `--executor-parallelism=N` silently degrades to sequential execution. |
| `q7-trace` | GROUP BY debug trace (formerly the `Q7_TRACE` environment variable, which was read per grouped query on the hot path). |

### 5.2 `sqlrustgo-executor` (`crates/executor/Cargo.toml`)

| Feature | Effect |
|---|---|
| `default` | Empty; the parallel and SIMD paths are off. |
| `parallel-executor` | Activates the `parallel_executor` / `parallel_vector_executor` modules and wires `LocalExecutor::execute_select_parallel` into the production path. |
| `simd` | SIMD batch filter evaluation (experimental). |

### 5.3 `sqlrustgo-storage` (`crates/storage/Cargo.toml`)

| Feature | Effect |
|---|---|
| `default` | Empty. |
| `fault-injection` | Enables the fault-injection hooks used by the crash-recovery matrix. **OFF in production** by design. |
| `bin_storage_default` | Binary table storage default path. |
| `v313_3_profile` | Profiling hooks; test-only, off by default. |

### 5.4 `sqlrustgo-mysql-server` (`crates/mysql-server/Cargo.toml`)

| Feature | Effect |
|---|---|
| `jemalloc-prof` | Installs jemalloc as the global allocator **with profiling**. **Requires `JEMALLOC_SYS_WITH_MALLOC_CONF="prof:true,prof_active:true"` at build time** — `tikv-jemalloc-sys` 0.7.x always passes `--with-malloc-conf` to jemalloc's configure, and the runtime `MALLOC_CONF` variable is ignored once the build-time setting is baked in. With the feature on, `SIGUSR2` writes `/tmp/jeprof.<pid>.<unix_ts>.heap`. The default build keeps the system allocator. |
| `fault-injection` | Pass-through to `sqlrustgo-storage/fault-injection`. Non-default; test builds only. |

### 5.5 Enabling them — with the syntax this tree requires

```bash
# The root package's own features (this is what `cargo build` at the root selects):
cargo build --release --features parallel-executor

# Enabling a feature on a *dependency* of the package you are building.
# `parallel-executor` is NOT a feature of sqlrustgo-mysql-server — it lives on
# the root `sqlrustgo` package and on `sqlrustgo-executor`:
cargo build --release -p sqlrustgo-mysql-server --features sqlrustgo/parallel-executor
```

The `package/feature` syntax is required because `sqlrustgo-mysql-server`
does not declare `parallel-executor` itself. Passing the bare name to a
`-p` build is rejected by cargo:

```text
error: the package 'sqlrustgo-mysql-server' does not contain this feature: parallel-executor
```

> **Do not build the member packages with `--all-features`.** That would
> activate `sqlrustgo-mysql-server/jemalloc-prof`, which requires the
> `JEMALLOC_SYS_WITH_MALLOC_CONF` build-time variable. The CI configuration
> makes the same call deliberately, and says so in the `Build` step of
> `.gitea/workflows/ci.yml`.

To see what a package actually exposes:

```bash
cargo tree -p sqlrustgo-executor -e features
cargo metadata --format-version 1 --no-deps | python3 -c "import json,sys; \
  [print(p['name'], sorted(p['features'])) for p in json.load(sys.stdin)['packages']]"
```

---

## 6. Running the test suite

### 6.1 The commands this repository actually uses

| Command | Source |
|---|---|
| `cargo fmt --check --all` | `.gitea/workflows/ci.yml` |
| `cargo clippy --all-features --all-targets -- -D warnings` | `.gitea/workflows/ci.yml` |
| `cargo clippy --all-features -- -D warnings` | `AGENTS.md` §7 (minimal dev loop) |
| `cargo build --all-features` | `.gitea/workflows/ci.yml` |
| `cargo build -p sqlrustgo-mysql-server -p sqlrustgo-cli` | `.gitea/workflows/ci.yml` |
| `cargo test --all-features --no-fail-fast` | `AGENTS.md` §7 |
| `cargo test --workspace --exclude sqlrustgo --no-fail-fast` | `.gitea/workflows/ci.yml`, `test-crates` job |
| `cargo test --test <name> --all-features` | `AGENTS.md` §7 (single integration test) |

`--no-fail-fast` is required by `AGENTS.md` §7. The CI configuration records
why: without it the run stopped at the first failing target — on one CI run
only 64 of the 446 declared test targets ever executed while the summary
still printed "passed=70 failed=1" and hid the other 382.

### 6.2 `cargo test` at the root does not test the whole workspace

This is the single most important thing to know before you trust a green
local run. `.gitea/workflows/ci.yml` states it plainly: *"cargo test at the
workspace root selects the root package only (the workspace root is itself a
package), so the 301 test targets under `crates/*` never ran in CI — and
nothing reported it."*

So:

```bash
# Root package only:
cargo test --all-features --no-fail-fast

# Member crates (the crates/ targets):
cargo test --workspace --exclude sqlrustgo --no-fail-fast
```

CI splits these into two separate jobs because the root package's own run
alone needs 86–147 minutes on the runner and the container limit is 3 hours.

### 6.3 A few useful narrower invocations

```bash
cargo test -p sqlrustgo-mysql-server --lib              # server library unit tests
cargo test -p sqlrustgo-cli                              # the CLI crate
cargo test --test prometheus_test                        # one integration target
cargo test -p sqlrustgo-storage --lib --release
```

### 6.4 Gate scripts

The repository's hard gates live in `scripts/gate/` and are listed in
`docs/governance/GATE_CONDITIONS.md`. A representative, self-contained one:

```bash
bash scripts/gate/check_document_completeness.sh v4.1.0
```

It checks that the mandatory release documents exist, meet their size
thresholds, and that sampled internal links resolve. It writes a report to
`artifacts/gate/v4.1.0/document_completeness/`.

---

## 7. Platform notes

### 7.1 What is actually exercised in this repository

| Platform | Evidence in the tree | Confidence |
|---|---|---|
| Linux x86_64 | `cargo-toolchain.toml` lists the target `x86_64-unknown-linux-gnu`; the CI runner is a self-hosted Linux host (`hp-z6g4`); the root `Cargo.toml` records that the CI image is Alpine/musl and ships no static zlib. | **Verified in CI.** |
| macOS aarch64 (Apple Silicon) | `cargo-toolchain.toml` lists the target `aarch64-apple-darwin`. | Target is declared and the tree builds on it; **no CI job runs on macOS**. |
| Linux aarch64 | Not in `cargo-toolchain.toml`'s target list. | **Not verified in this repository.** |
| Windows (any) | No target, no workflow, and no platform-specific code path was found. | **Not verified in this repository.** |
| Other platforms (FreeBSD, …) | Nothing in the tree. | **Not verified in this repository.** |

> This tree is verified on **Linux x86_64** only. Treat the macOS line as
> "the toolchain target is declared and the code compiles", not as a tested
> platform. Do not read a platform as supported because an older release
> document listed a pre-built binary for it — no v4.1.0 pre-built binary
> exists here.

### 7.2 musl / Alpine

`libz-sys` is depended on with the `static` feature across the workspace, so
the binaries do not link a system `libz` and no `zlib1g-dev` package is
required. The root `Cargo.toml` explains the motivation: a musl target links
statically by default, and the CI image had no static zlib, so every job that
had to link a test binary failed with `ld: cannot find -lz`.

### 7.3 macOS notes

- The `cargo-toolchain.toml` target list includes `aarch64-apple-darwin`.
- Gatekeeper quarantine applies to binaries copied onto the machine rather
  than built locally. Nothing in this repository automates the removal of
  the quarantine attribute.

### 7.4 `jemalloc-prof` is not portable-by-default

The jemalloc feature is opt-in and requires a build-time environment
variable (§5.4). It is also the reason CI does not use `--all-features` on
the member packages (§5.5).

---

## 8. What this document does NOT cover

Stated explicitly so that nothing here is read as a promise:

- **Pre-built binaries** for v4.1.0 — none exist in this repository.
- **Homebrew / apt / yum / dnf / apk / winget / chocolatey / scoop** install
  paths — no packaging recipe for v4.1.0 exists in this tree.
- **Docker / container images** — no Dockerfile or image definition for
  v4.1.0 was found in the repository.
- **GPG signature verification of a release artifact** — there is no release
  artifact to verify, so no fingerprint is quoted here.
- **A supported-platform matrix with pre-built binaries** — §7.1 gives the
  platforms this tree exercises, which is a narrower claim.
- **An MSRV** — no `rust-version` is declared anywhere in the tree (§2.1).
- **Upgrade instructions from an older install** — the on-disk format
  compatibility question belongs to [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md),
  not to this document.

---

## 9. Open items that need owner input

1. **MSRV.** No `rust-version` is declared in any manifest. If a minimum Rust
   version is intended to be supported, it has to be declared in the
   workspace `Cargo.toml` (`[workspace.package] rust-version = "…"`) and
   cannot be inferred from the current tree.
2. **Packaging.** No release-artifact pipeline output, package-manager
   recipe, or container image definition for v4.1.0 was found. Whether
   v4.1.0 will ship one is an owner decision; this document cannot state
   one.
3. **Windows and Linux-aarch64.** Neither is exercised by anything in this
   repository. If either is meant to be supported, that needs a CI runner
   and a statement in the release documentation.

---

## 10. Related documents

| Document | What it answers |
|---|---|
| [QUICK_START.md](QUICK_START.md) | The five-minute path: build, start, connect, query, stop. |
| [FEATURE_MATRIX.md](FEATURE_MATRIX.md) | What v4.1.0 adds over v4.0.0, with status. |
| [VERSION_PLAN.md](VERSION_PLAN.md) | The v4.1.0 scope and delta table. |
| [ARCHITECTURE.md](ARCHITECTURE.md) | The execution path and what is not on it. |
| [DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) | Deploying a built binary. |
| [MIGRATION_GUIDE.md](MIGRATION_GUIDE.md) | Upgrading onto this line. |
| [TEST_PLAN.md](TEST_PLAN.md) | The test plan for this version. |
| [RELEASE_NOTES.md](RELEASE_NOTES.md) | Release notes for this line. |
| [CHANGELOG.md](CHANGELOG.md) | Commit-level changes. |
| [EVALUATION_REPORT.md](EVALUATION_REPORT.md) | The stage evaluation report. |
| [DEFECTS_AND_ISSUES.md](DEFECTS_AND_ISSUES.md) | Open defect catalogue. |
| `docs/releases/v4.1.0/STAGE.yaml` | The stage SSOT. |

---

*Generated 2026-10-23 for the v4.1.0 ALPHA stage. Commands, flags, feature
names and paths were read from this repository's manifests, source and CI
configuration, or executed against this working tree.*

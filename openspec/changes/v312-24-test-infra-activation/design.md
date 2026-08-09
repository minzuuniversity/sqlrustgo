# V312-24: Test Infrastructure Activation — design

## Architecture Overview

The V312-24 activation pivots three skeleton crates from "library with self-tests" to "executable tool producing a gate artifact". The contract is one tool → one binary → one artifact → one gate input.

```
crates/sqlancer/
  Cargo.toml:  + [[bin]] name="sqlancer"
  src/bin/sqlancer.rs:  + new — wires Fuzzer::new(config).run(120s)
  src/bin/sqlancer.rs:  + writes target/sqlancer-report.json
  src/lib.rs:  no change (Fuzzer / FuzzerConfig / FuzzerResult API stable)

crates/test-runner/
  Cargo.toml:  + [[bin]] name="test-runner"
  src/lib.rs:  ~ timeout_per_test_ms via tokio::time::timeout (currently ignored)
  src/lib.rs:  ~ run_tests via JoinSet honoring max_parallel (currently plain for-loop)
  src/lib.rs:  + writes target/test-runner-report.json via serde_json
  src/bin/test-runner.rs:  + new — CLI args + TestRunner::new(config).run_all()

crates/test-registry/
  Cargo.toml:  + [[bin]] name="test-registry-cli"
  src/lib.rs:  + TestRegistry::from_toml(path) using the unused `toml` dep
  src/lib.rs:  + TestRegistry::write_toml(path)
  src/bin/test-registry-cli.rs:  + new — `test-registry-cli init | list | register <bin-path>`
  src/lib.rs:  ~ run_test shell to consume registered manifests

crates/sqlrustgo_sqllogictest/
  no change (binary already active; Phase 5 corpus task improves it)
```

## Wiring Diagram

```
┌─────────────────┐
│ sqlancer binary │── writes ──▶ target/sqlancer-report.json
└─────────────────┘                            │
                                              ▼
┌─────────────────┐                  scripts/gate/check_beta_gate.sh
│ test-runner bin │── writes ──▶ target/test-runner-report.json  (B10_SQLANCER check)
└─────────────────┘                            │
                                              ▼
┌─────────────────┐                  scripts/test/run-regression.sh
│ test-registry   │── reads ───▶ test-registry.toml (managed entries list)
│ binary          │                       │
└─────────────────┘                       │
                                       ▼
                                cargo run --release -p sqlancer -- --duration 120
                                (replaces || true masking)
```

## Design Decisions

### Decision 1: sqlancer binary entry — minimal CLI

**Choice**: Add `crates/sqlancer/src/bin/sqlancer.rs` with `fn main()` that:
1. Parses `--duration <secs>` (default 30), `--seed <u64>` (default 0), `--out <path>` (default `target/sqlancer-report.json`)
2. Constructs `FuzzerConfig::default()` + overrides from args
3. Calls `fuzzer.run(duration.as_secs())`
4. Writes `FuzzerResult` as JSON via serde (add `serde`, `serde_json` to dev-deps)

**Why minimal**: Avoid feature creep; reuse the existing `Fuzzer::run` API which already supports DDL/DML generation + TLP oracle. The binary's only job is to expose `Fuzzer::run` as an executable + write the result to disk.

**Alternative considered**: Promote `Fuzzer` to a public binary inside `crates/sqlancer/src/bin/sqlancer.rs` (one file). Rejected: existing public surface is sufficient; no need to add new API.

### Decision 2: test-runner timeout enforcement via `tokio::time::timeout`

**Choice**: Wrap `Command::spawn` output polling in `tokio::time::timeout(Duration::from_millis(timeout_per_test_ms))`. On timeout, kill the child process and mark test as `TestStatus::Timeout`.

**Why**: `tokio` is already a dependency; current `run_test` reads child stdout until EOF (no timeout) — a hung test hangs the runner indefinitely. Timeout is a correctness invariant per `TestRunConfig::timeout_per_test_ms`.

**Code sketch**:
```rust
pub async fn run_test(&self, test: &TestMetadata) -> TestResult {
    let mut child = Command::new(&test.binary)
        .args(&test.args)
        .stdout(Stdio::piped())
        .spawn()?;
    let timeout = Duration::from_millis(self.config.timeout_per_test_ms);
    match tokio::time::timeout(timeout, async {
        child.wait().await
    }).await {
        Ok(status) => TestResult { status: TestStatus::from(status), ... },
        Err(_) => { let _ = child.kill().await; TestResult { status: TestStatus::Timeout, ... } }
    }
}
```

### Decision 3: test-registry TOML persistence using existing dep

**Choice**: Add `TestRegistry::from_toml(path: &Path)` and `TestRegistry::write_toml(&self, path: &Path)`. Use the existing `toml` crate already in `Cargo.toml` (currently unused per audit).

**Schema** (`test-registry.toml`):
```toml
[[test]]
name = "sqlancer"
binary = "target/release/sqlancer"
args = ["--duration", "120", "--out", "target/sqlancer-report.json"]
timeout_ms = 600_000
priority = "p1"
category = "fuzz"

[[test]]
name = "test-runner"
binary = "target/release/test-runner"
args = []
timeout_ms = 300_000
priority = "p0"
category = "integration"
```

**Why TOML not JSON**: Matches Rust ecosystem convention; human-editable; serde already in deps.

### Decision 4: E2E script de-duplication strategy

**Choice**: Delete the 4 stale mirrors in `tests/e2e/` (`startup_connect`, `tpch_sf01`, `kill9_recovery`) + the 4 in `scripts/gate/e2e/` (`e2e_01..04`). Update `scripts/gate/check_rc_gate_v3.10.0.sh` R4 substring match to use only the post-retirement 4 active scripts (`alter_rename`, `rollback_mvcc`, `union_set_ops`, `e2e_runner_exec`).

**Why de-duplicate vs rename**: The `tests/e2e/*.sh` are the canonical R4 scripts (issue-3399 authored them). The `scripts/gate/e2e/e2e_*.sh` are stale copies from before the v3.10.0 test directory restructure. Deleting the stale copies is safer than renaming the live ones (renames break downstream callers).

### Decision 5: Anti-fabrication enforcement pattern

**Choice**: Extend `scripts/gate/check_gate_test_integrity.sh` to scan `scripts/gate/*.sh` for `|| true` after `cargo test …` invocations. If found, log warning + exit 1.

**Why add gate**: Manual review found 6 violations; gate prevents future recurrence.

**Detection regex**: `grep -B1 -A0 '|| true' scripts/gate/*.sh | grep -B0 'cargo test'`

### Decision 6: SQL corpus activation budget (deferred to V312-24-corpus)

**Choice**: Phase 5 corpus activation (16h) marked carried → V312-24-corpus follow-up. Owner: opencode-z440, expiry 2026-09-30.

**Why defer**: Corpus pass-rate is currently 27.3% (6/16 subcategories). Bringing it to 80% requires either fixing ~75 subcategory-specific SimpleExecutor gaps OR lowering the gate threshold with explicit owner/expiry attestation. Both are 16h scope. Per V312_DAG_ANALYSIS budget of 16h for V312-24, deferring the corpus to a follow-up task is consistent with the V312 master plan structure.

## Goals / Non-Goals

**Goals**: All Phase 1-4 + 6-8 done in 60h with 100% gate output, no `|| true` masking, no `#[ignore]` on a now-passing test.

**Non-Goals**: SQLite official corpus download (network-blocked per V310-14b), sysbench installation on CI runners (out of scope), multi-language integration (Python/MySQL interop testing — separate issue #3429 track).

## File-by-file change list

### crates/sqlancer/Cargo.toml
```diff
+[[bin]]
+name = "sqlancer"
+path = "src/bin/sqlancer.rs"
+
 [dev-dependencies]
+serde = { version = "1.0", features = ["derive"] }
+serde_json = "1.0"
```

### crates/sqlancer/src/bin/sqlancer.rs (new, ~80 lines)
```rust
use std::time::Duration;
use sqlancer::{Fuzzer, FuzzerConfig, FuzzerResult};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut duration = Duration::from_secs(30);
    let mut out = "target/sqlancer-report.json".to_string();
    let mut seed = 0u64;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--duration" => duration = Duration::from_secs(args.next().unwrap().parse()?),
            "--out" => out = args.next().unwrap(),
            "--seed" => seed = args.next().unwrap().parse()?,
            _ => eprintln!("unknown arg: {}", arg),
        }
    }
    std::fs::create_dir_all(std::path::Path::new(&out).parent().unwrap())?;
    let result: FuzzerResult = Fuzzer::new(FuzzerConfig { seed, duration, ..Default::default() }).run()?;
    std::fs::write(&out, serde_json::to_string_pretty(&result)?)?;
    println!("sqlancer: wrote {} ({} queries)", out, result.total_queries);
    Ok(())
}
```

### crates/test-runner/src/lib.rs
```diff
-    pub async fn run_test(&mut self, test: &TestMetadata) -> TestResult {
-        // ...plain Command::spawn().wait().await... no timeout
-    }
+    pub async fn run_test(&self, test: &TestMetadata) -> TestResult {
+        use tokio::time::timeout;
+        let timeout_dur = Duration::from_millis(self.config.timeout_per_test_ms);
+        let mut child = tokio::process::Command::new(&test.binary)
+            .args(&test.args)
+            .stdout(Stdio::piped()).stderr(Stdio::piped())
+            .spawn()?;
+        let id = child.id();
+        let outcome = match timeout(timeout_dur, child.wait()).await {
+            Ok(Ok(status)) => TestStatus::from(status),
+            Ok(Err(e)) => TestStatus::Error(e.to_string()),
+            Err(_) => { let _ = tokio::process::Command::new("kill").arg("-9").arg(id.unwrap().to_string()).status().await; TestStatus::Timeout },
+        };
+        TestResult { name: test.name.clone(), status: outcome, ..Default::default() }
+    }
+
+    pub async fn run_tests(&self, tests: &[TestMetadata]) -> Vec<TestResult> {
+        let sem = Arc::new(tokio::sync::Semaphore::new(self.config.max_parallel.max(1)));
+        let mut joinset = tokio::task::JoinSet::new();
+        for t in tests { let s = sem.clone(); joinset.spawn(s.acquire_owned().then(move |permit| async move { let _p = permit; self.run_test(t).await })); }
+        let mut out = Vec::new();
+        while let Some(res) = joinset.join_next().await { out.push(res?); }
+        out
+    }
```

### crates/test-registry/src/lib.rs
```diff
+    pub fn from_toml(path: &std::path::Path) -> Result<Self, toml::de::Error> {
+        let s = std::fs::read_to_string(path).map_err(|_| toml::de::Error::from(...))?;
+        toml::from_str(&s)
+    }
+    pub fn write_toml(&self, path: &std::path::Path) -> Result<(), toml::ser::Error> { ... }
```

## Migration plan

| Step | Risk | Rollback |
|------|------|----------|
| 1.6 (test-runner parallelize) | Medium — JoinSet semantics differ from for-loop | git revert |
| 4.1 (remove merge_vtu ignore) | Low — VtuGuard closed per ARCH-3 evidence | re-add #[ignore] if merge_vtu test fails |
| 4.2 (smoke assertion fix) | Low — better assertion is strictly stricter | git revert |
| 5 (corpus activation) | High — semantic fixes may break other tests | defer to follow-up (already in plan) |

## Test strategy

- **sqlancer binary**: `cargo run --release -p sqlancer -- --duration 5` (small duration for CI) must exit 0 and write non-empty `target/sqlancer-report.json`. Add an integration test in `crates/sqlancer/tests/cli_smoke.rs` that calls the binary as subprocess.
- **test-runner timeout**: integration test in `crates/test-runner/tests/timeout_enforced.rs` that spawns a `sleep 60` script with `timeout_per_test_ms: 100`, asserts status == `Timeout` within 1s.
- **test-registry TOML**: unit test in `crates/test-registry/tests/toml_round_trip.rs` that writes a registry, reads it back, asserts equality.
- **E2E retire**: `cargo run --bin e2e_runner_exec` (the active orchestrator) must still pass 8/8 — but with 4 retired scripts, the R4 substring match shrinks from 8 to 4. Update `check_rc_gate_v3.10.0.sh:132-135` accordingly.

## Verification plan

- **Phase 1**: `cargo build --workspace` + `cargo test -p sqlancer -p test-runner -p test-registry --lib` must all pass
- **Phase 2**: `bash scripts/gate/check_rc_gate_v3.10.0.sh` must still exit 0 after retire (with updated R4 substring match)
- **Phase 4**: `bash scripts/gate/check_anti_fabrication.sh` must pass (no new violations introduced); `grep -rn '|| true' crates/*/tests/*.rs tests/integration/*.rs` should return 0 hits in the 6 fixed files
- **Phase 6**: `bash scripts/gate/check_beta_gate.sh` B10_SQLANCER check transitions from `check_warn` to `check_fail` and still passes (because sqlancer now produces a valid artifact)

## Open questions

1. Where should the test-registry.toml live? Options: `crates/test-registry/data/registry.toml` (default shipped), or `tests/data/test-registry.toml` (per-project), or per-user `~/.config/sqlrustgo/test-registry.toml`. **Recommendation**: `crates/test-registry/data/registry.toml` (shipped default + per-user override via env var `SQLRUSTGO_REGISTRY`).
2. Should the sqlancer binary's report include the seed? **Recommendation**: Yes (reproducibility). Already in FuzzerConfig; just serialize in FuzzerResult.
3. Backward compatibility of retired scripts: are any external CI scripts (e.g., docs/release pipeline) calling them by name? **Action**: grep `tests/e2e/{e2e_01..08,startup_connect,tpch_sf01,kill9_recovery,alter_rename,rollback_mvcc,union_set_ops,backup_restore,sysbench_wired}` across all `.sh`, `.yaml`, `.yml`, `.toml` files; fix any callers before git rm.

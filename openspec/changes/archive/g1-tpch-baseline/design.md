# Design — g1-tpch-baseline

## Context

`v3.8.0` is GA. The TPC-H 22/22 result is now a contract: it must not regress in v3.9.0 across 12 weeks of debt-closure work. Today's evidence is scattered:

- `tests/tpch_gate_test.rs` — 22/22 inline gate (passes; cannot detect semantic drift, only panic)
- `tests/tpch_full_22_test.rs` — 22/22 full run (passes; same limitation)
- `docs/releases/v3.8.0/COVERAGE_REPORT.md` — narrative report ("22/22 PASS")
- `docs/releases/v3.8.0/historical/TPCH_22_REPORT_2026-06-05.md` — newer audit (mentions 22/22)

None of these is a *machine-checked* invariant. If a PR ships a change that makes TPC-H Q3 return a different row count but still passes `assert!(count > 0)`, nothing notices.

## Goals / Non-Goals

**Goals:**

- A single SHA-256 hash that uniquely fingerprints the 22/22 TPC-H output on `develop/v3.9.0` HEAD.
- A test that re-derives the hash on every run and fails on any drift.
- A shell gate `check_g1_tpch_baseline.sh` that wraps the test + the existing gate tests, and is the single thing CI runs.
- CI integration: the gate runs on PRs that touch SQL/executor/storage code paths.
- A hash-bump workflow (documented in `tpch_hashes_v380.json`'s header comment) that lets intentional evolutions update the baseline via a regular PR.

**Non-Goals:**

- Cross-platform hash equivalence (Linux vs macOS vs Windows). G1 only enforces on the Linux CI runner.
- Latency / throughput / cost tracking — those are P3-1..P3-5 in Phase 6.
- A web dashboard for hash history — out of scope; git log is the history.
- Bumping the baseline on a one-off `cargo test` failure — that requires human review (a follow-up PR), not an auto-bump.

## Decisions

### D1. Hash the *combined* TPC-H output, not per-query hashes

**Why:** A single SHA-256 of `Q1_output || Q2_output || ... || Q22_output` (with sorted rows, deterministic order) is the simplest invariant. Per-query hashes would let Q1+Q2 stay stable while Q3..Q22 all change silently. One hash is also a single line of `cargo test` output for humans to read.

**Alternatives considered:**

- *22 separate hashes, one per query*: rejected — partial regressions are possible; one query changing while another compensates; harder to reason about; more lines in the baseline file.
- *Manifest-of-hashes over all 22 with a "coverage" check*: rejected — overkill; the question "did anything change?" reduces to "do all 22 hashes match?", and a single combined hash answers that faster.

### D2. Sort rows before hashing; use stable string repr

**Why:** Two semantically-identical results can differ in row order due to planner choices, MVCC visibility, or buffer pool state. We need the hash to be a *logical* fingerprint, not a *physical* one. Sorting is the standard SQL-result normalization: `SELECT * FROM (...) ORDER BY 1, 2, ..., n`. String repr uses the existing `Value::Display` impl (already used by the test).

**Alternatives considered:**

- *Set-equality (commutative) hashing*: rejected — cannot detect a row being replaced by an identical-key row with different columns.
- *JSON canonical form*: rejected — adds a serde dep to the test path; sorting is sufficient.

### D3. Baseline file checked into the repo (`tests/tpch_hashes_v380.json`)

**Why:** The baseline must evolve with the code. Keeping it in-tree means every PR that legitimately changes TPC-H output (e.g., a bug fix that makes Q5 return the *right* answer) updates the baseline in the same PR. No out-of-band coordination. The file's first 5 lines are a comment explaining how to bump it (run the test once, copy the printed hash into this file, commit).

**Alternatives considered:**

- *Baseline in a separate tag/S3 bucket*: rejected — adds infra; the v3.8.0 hash is a single 64-char string.
- *Compute baseline at test time from v3.8.0 binary*: rejected — requires checking out v3.8.0 every run; slow; brittle.

### D4. Hash lives in the test, not a separate utility

**Why:** A unit test that has the hash in its source and the comparison logic inline is 30 lines. Splitting into `tpch_hash_compare.py` adds a process boundary. But we want the **shell gate** to run the same logic without `cargo test`'s overhead — that's where the Python helper earns its keep. So: hash logic is in `tpch_hash_compare.py`; the Rust test calls the same algorithm via `include_str!` of a small `.rs` template + a runtime call. Actually simpler: the test calls `Command::new("python3").arg("tpch_hash_compare.py")` and asserts the exit code. This means the gate and the test share the implementation literally.

**Alternatives considered:**

- *Two implementations (Rust for test, Python for gate) that must stay in sync*: rejected — every change requires updating two files; drift is inevitable.
- *Subprocess approach (`Command::new("python3")`)*: chosen — single source of truth, gate and test are guaranteed to use the same hash function.
- *Move all logic into a `bin/hash_main.rs` and have both call it*: rejected — adds a binary, complicates `cargo test` invocation.

### D5. CI integration via existing `.gitea/workflows/ci.yml`, not a new workflow file

**Why:** Gitea CI convention in this repo is one `ci.yml`. Adding a new workflow file for a single gate is inconsistent and makes the PR noise larger than the change. The new step is one `bash` invocation, gated on `paths:` matching the test/prod code directories.

**Alternatives considered:**

- *New `g1-tpch.yml`*: rejected — repository convention is one `ci.yml`; multi-workflow is reserved for unrelated concerns (release, reconciliation, gate-promotion).
- *GitHub-Actions-style matrix on the hash*: rejected — over-engineered for a single 22-query run.

## Risks

- **Test flakiness from non-determinism in TPC-H** — if any query returns rows in non-deterministic order, the hash will flap. Mitigation: sort-by-all-columns; if a query has ties that the sort doesn't break, the test prints a diagnostic and the operator must investigate. No known such cases in v3.8.0.
- **Bumping the baseline by accident** — a careless PR could update `tpch_hashes_v380.json` with a wrong hash and pass CI while silently accepting a regression. Mitigation: the hash is in the PR diff, the PR template's "evidence" section requires hash printout, and the G1 gate exit code on hash change is non-zero with a clear "hash changed intentionally?" prompt (which CI does not auto-confirm).

## Open Questions

- (None — this is a self-contained gate; it does not depend on any in-flight v3.9.0 change.)

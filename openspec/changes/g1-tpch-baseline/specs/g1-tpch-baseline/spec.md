# Spec: g1-tpch-baseline

## ADDED Requirements

### Requirement: Single SHA-256 baseline hash for TPC-H 22/22

The project MUST maintain a single SHA-256 hash, stored in `tests/tpch_hashes_v380.json` under the key `tpc_h_hash_sha256`, that uniquely identifies the v3.8.0 TPC-H 22/22 PASS result. The hash is computed over the sorted, deterministic concatenation of the outputs of queries Q1..Q22 run against an ephemeral `sqlrustgo-mysql-server` instance.

#### Scenario: Hash file is valid JSON with a 64-char hex value
- **WHEN** the file `tests/tpch_hashes_v380.json` is parsed
- **THEN** the result is a JSON object whose `tpc_h_hash_sha256` field is a lowercase hexadecimal string of exactly 64 characters
- **AND** the `version` field equals `"v3.8.0"`
- **AND** the `tpc_h_queries` field equals `22`

#### Scenario: Hash is reproducible on the same tree
- **WHEN** `python3 scripts/gate/tpch_hash_compare.py --capture` is run twice on the same `develop/v3.9.0` HEAD, with no intervening code changes
- **THEN** both invocations produce the same 64-char hex string byte-for-byte
- **AND** the value matches `tests/tpch_hashes_v380.json`'s `tpc_h_hash_sha256`

#### Scenario: Hash changes when any query output changes
- **WHEN** any of the 22 TPC-H queries produces a different row count, different column values, or different row order (before sorting) than the v3.8.0 baseline
- **THEN** the recomputed hash differs from the stored value
- **AND** `cargo test --test tpch_hash_test` fails with a message of the form `Q<n> changed: ...` (when `--check` is used with a non-matching hash, the diff is reported to stderr)

### Requirement: Hash test as part of the standard test suite

The project MUST ship a Rust regression test `tests/tpch_hash_test.rs` that, on every `cargo test` invocation, recomputes the TPC-H hash and asserts it matches the baseline.

#### Scenario: Hash test passes on v3.9.0 HEAD
- **WHEN** `cargo test --test tpch_hash_test` is run against an unmodified checkout of `develop/v3.9.0` (post-merge of this change)
- **THEN** all tests in the file pass
- **AND** the test prints the recomputed hash to stdout for human verification

#### Scenario: Hash test runs in under 5 minutes
- **WHEN** `cargo test --test tpch_hash_test` is run on the Z6G4 CI runner
- **THEN** wall-clock time from invocation to completion is strictly less than 300 seconds

#### Scenario: Hash test uses the same algorithm as the shell gate
- **WHEN** the test and the gate both compute the hash on the same tree
- **THEN** the two values are byte-identical
- **BECAUSE** both call `python3 scripts/gate/tpch_hash_compare.py` (the test does so via `Command::new`)

### Requirement: Shell gate wrapping the baseline check

The project MUST ship a shell script `scripts/gate/check_g1_tpch_baseline.sh` that orchestrates the four checks required for G1: (a) `tpch_gate_test` 22/22, (b) `tpch_full_22_test` 22/22, (c) `tpch_hash_test` matches baseline, (d) `tpch_hash_compare.py --check` confirms the file's hash.

#### Scenario: Gate passes on a fresh checkout
- **WHEN** `bash scripts/gate/check_g1_tpch_baseline.sh` is run on a clean clone of `develop/v3.9.0` at the merge commit of this change
- **THEN** the script exits 0
- **AND** prints a summary line `G1 PASS: TPC-H 22/22 baseline verified, hash=<short-hash>`

#### Scenario: Gate fails clearly on hash drift
- **WHEN** the code is modified so any of the 22 TPC-H queries returns a different result
- **AND** `bash scripts/gate/check_g1_tpch_baseline.sh` is run without updating the baseline
- **THEN** the script exits 1
- **AND** prints a line of the form `G1 FAIL: tpch_hash_test — <reason>`
- **AND** the underlying `tpch_hash_compare.py --check` reports a per-query diff to stderr

#### Scenario: Gate supports dry-run
- **WHEN** `bash scripts/gate/check_g1_tpch_baseline.sh --dry-run` is run
- **THEN** the script prints the four steps it would execute
- **AND** exits 0 without running any of them
- **AND** no `cargo`/`python3` subprocess is spawned

### Requirement: CI integration on PRs that touch prod code

The repository's CI workflow `.gitea/workflows/ci.yml` MUST run `bash scripts/gate/check_g1_tpch_baseline.sh` as a step on every PR that modifies any of: `crates/**`, `tests/**`, `Cargo.toml`, `Cargo.lock`.

#### Scenario: Gate runs on a PR that touches the executor
- **WHEN** a PR is opened with a diff that includes `crates/executor/src/lib.rs`
- **THEN** the CI workflow runs the `g1-tpch-baseline` step
- **AND** a non-zero exit from the gate fails the PR

#### Scenario: Gate does NOT run on a docs-only PR
- **WHEN** a PR is opened with a diff that touches only `docs/**` (and no `crates/**`, `tests/**`, `Cargo.toml`, or `Cargo.lock`)
- **THEN** the `g1-tpch-baseline` step is skipped
- **AND** the PR can still be merged without running the gate

#### Scenario: Gate run produces a single summary line
- **WHEN** the `g1-tpch-baseline` step runs and exits 0
- **THEN** the step's output includes the line `G1 PASS: TPC-H 22/22 baseline verified, hash=<short-hash>` for the PR reviewer's quick verification

### Requirement: Bump workflow is documented in-tree

The repo MUST document, in plain prose at the top of `tests/tpch_hashes_v380.json`, the steps required to intentionally update the baseline when a TPC-H query's output legitimately changes (e.g., a correctness bug fix).

#### Scenario: A maintainer can bump the baseline without external docs
- **WHEN** a maintainer opens `tests/tpch_hashes_v380.json` in a text editor
- **THEN** the first 5 lines explain the bump workflow in 3-4 sentences
- **AND** mention the requirement that the PR diff show the hash change as evidence

#### Scenario: An accidental bump is detectable
- **WHEN** a PR updates `tpc_h_hash_sha256` to a new value
- **AND** the PR body does not reference a legitimate TPC-H output change
- **THEN** a reviewer can reject the PR based on the unexplained diff
- **BECAUSE** the `tpc_h_hash_sha256` value is short (64 hex chars) and stands out in the diff

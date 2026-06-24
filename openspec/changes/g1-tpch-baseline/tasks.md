# Tasks — g1-tpch-baseline

## 1. Worktree + baseline verification

- [ ] 1.1 Verify clean worktree at `feature/g1-tpch-regression` from `develop/v3.9.0` (HEAD = `3ed72a3e`)
- [ ] 1.2 Run `cargo build --workspace --all-features` and confirm exit 0
- [ ] 1.3 Run `cargo test --test tpch_gate_test --test tpch_full_22_test` and confirm 22/22 PASS on the unmodified tree (this *is* the v3.8.0 hash)
- [ ] 1.4 Capture the baseline hash: run `python3 scripts/gate/tpch_hash_compare.py --capture > /tmp/v380_hash.txt` and inspect it (this becomes the value in step 4.1)

## 2. Write the Python hash helper

- [ ] 2.1 Create `scripts/gate/tpch_hash_compare.py` with two subcommands: `--capture` (writes the hash to stdout) and `--check <expected>` (compares and exits 0/1)
- [ ] 2.2 Implement the hash: for each TPC-H query Q1..Q22, run it against an ephemeral `sqlrustgo-mysql-server` (in-process via `start_ephemeral`), capture the result, sort rows by all columns, format as `Value::Display` strings, concatenate with `---Q<n>---` separators, and SHA-256 the whole thing
- [ ] 2.3 Handle empty results (Q1 with no rows is valid): emit `---Q<n>---\n[empty]\n`
- [ ] 2.4 Exit codes: `--capture` always exits 0 (even on no-data); `--check` exits 0 on match, 1 on mismatch with diff to stderr
- [ ] 2.5 Unit smoke: `python3 scripts/gate/tpch_hash_compare.py --capture` on `develop/v3.9.0` HEAD produces a 64-char hex string and exits 0

## 3. Write the Rust regression test

- [ ] 3.1 Create `tests/tpch_hash_test.rs` with one `#[test] fn tpch_hash_matches_v380_baseline()`
- [ ] 3.2 The test calls `Command::new("python3").arg("scripts/gate/tpch_hash_compare.py").arg("--check").arg(EXPECTED_HASH).status()` and asserts `ExitStatus.success()`
- [ ] 3.3 The `EXPECTED_HASH` constant is `include_str!("tpch_hashes_v380.json")` parsed via a `const fn` (or, if that's too tricky, a `lazy_static!`/`OnceLock` for simplicity) — the hash file is 64 hex chars + comment lines
- [ ] 3.4 Add a second `#[test] fn tpch_hash_test_runs_in_under_5_minutes()` that asserts wall-clock < 300s (regression guard for the hash step becoming accidentally expensive)
- [ ] 3.5 Run `cargo test --test tpch_hash_test` on the unmodified tree → both tests pass

## 4. Create the baseline hash file

- [ ] 4.1 Create `tests/tpch_hashes_v380.json` with the captured hash from step 1.4:
  ```json
  {
    "version": "v3.8.0",
    "captured_at": "2026-06-05",
    "captured_from": "develop/v3.9.0 @ 3ed72a3e (post Phase 0)",
    "tpc_h_hash_sha256": "<64-char hex>",
    "tpc_h_queries": 22,
    "tpc_h_test_files": ["tpch_gate_test.rs", "tpch_full_22_test.rs"]
  }
  ```
- [ ] 4.2 Add a 5-line comment header to the file explaining the bump workflow:
  ```
  # To bump: run `python3 scripts/gate/tpch_hash_compare.py --capture` and replace
  # `tpc_h_hash_sha256`. Required for any legitimate TPC-H output change
  # (bug fix that returns correct rows, etc.). The PR diff must show the hash change.
  ```
- [ ] 4.3 Verify the file is valid JSON: `python3 -c "import json; json.load(open('tests/tpch_hashes_v380.json'))"`

## 5. Write the shell gate

- [ ] 5.1 Create `scripts/gate/check_g1_tpch_baseline.sh` (chmod +x) that runs, in order:
  1. `cargo test --test tpch_gate_test` (must be 22/22 PASS)
  2. `cargo test --test tpch_full_22_test` (must be 22/22 PASS)
  3. `cargo test --test tpch_hash_test` (must match baseline)
  4. `python3 scripts/gate/tpch_hash_compare.py --check "$(jq -r .tpc_h_hash_sha256 tests/tpch_hashes_v380.json)"` (must exit 0)
- [ ] 5.2 Exit codes: 0 if all four pass, 1 on any failure with a clear "G1 FAIL: <which step> <error>" message
- [ ] 5.3 Add a `--dry-run` flag that prints what it would do without executing (for the docs and for quick CI smoke)
- [ ] 5.4 Run the gate: `bash scripts/gate/check_g1_tpch_baseline.sh` exits 0 on the unmodified tree

## 6. Wire the gate into CI

- [ ] 6.1 Add a new step `g1-tpch-baseline` to `.gitea/workflows/ci.yml` after the existing `cargo build` step
- [ ] 6.2 The step runs `bash scripts/gate/check_g1_tpch_baseline.sh` and fails the workflow on non-zero exit
- [ ] 6.3 Add a `paths:` trigger so the step only runs on PRs that touch:
  - `crates/**`
  - `tests/**`
  - `Cargo.toml`
  - `Cargo.lock`
- [ ] 6.4 Verify the YAML parses: `gitea-actions-lint` or equivalent (if available); otherwise manual review

## 7. Documentation

- [ ] 7.1 Add a "G1: TPC-H Baseline" section to `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md` if not already covered (it is — just verify the section is up-to-date)
- [ ] 7.2 Add the gate to `docs/governance/GA_SCRIPTS_SKILLS_REGISTRY.md` §1 (gates) and §3 (skills) so future AI agents can find it
- [ ] 7.3 Update `docs/governance/INDEX.md` §3 (Gate 脚本与门禁) with the new gate

## 8. Verify and ship

- [ ] 8.1 Run `bash scripts/gate/check_full_gate_verification.sh` to confirm D1-D9 still PASS
- [ ] 8.2 Run `cargo clippy --all-features -- -D warnings` and `cargo fmt --check --all` — both must pass
- [ ] 8.3 Run `bash scripts/gate/check_docs_links.sh` — must pass (validates `INDEX.md` link to the new gate)
- [ ] 8.4 `git add` the 5 new files + the modified `ci.yml` / `INDEX.md` / `GA_SCRIPTS_SKILLS_REGISTRY.md`
- [ ] 8.5 Commit with message `test(g1): TPC-H 22/22 baseline hash + CI gate (#3186)`
- [ ] 8.6 Push branch `feature/g1-tpch-regression` to Gitea
- [ ] 8.7 Open PR targeting `develop/v3.9.0`, title `test(g1): TPC-H 22/22 baseline hash + CI gate (#3186)`, body linking #3186
- [ ] 8.8 On PR merge, close #3186 with a comment containing the merged commit SHA and a link to the gate run

## Done criteria (DoD)

- All checkboxes above ticked.
- The PR is merged to `develop/v3.9.0`.
- #3186 is `closed` with a `state_reason: completed`.
- `bash scripts/gate/check_g1_tpch_baseline.sh` exits 0 on the merge commit.
- The CI workflow file is parseable and the new step appears in the PR run.

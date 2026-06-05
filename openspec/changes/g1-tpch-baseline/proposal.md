# Proposal — g1-tpch-baseline

## Why

SQLRustGo v3.8.0 GA achieved 22/22 TPC-H PASS in `tpch_gate_test` and `tpch_full_22_test` (PR #3132, #3142, #3152). For v3.9.0, **Gate G1** requires this 22/22 result to **never regress** across all 12 weeks / 6 Phases of v3.9.0 development. Today there is no automated enforcement: a PR that breaks even one TPC-H query would land without alarm. Without a frozen baseline, perf-tier regressions (TPC-H Q1 dropping from 5s to 30s) or correctness-tier regressions (a query returning a different row count) would only be caught at GA time — far too late, after 5+ integrated phases of debt closure have masked the cause.

This change freezes the v3.8.0 TPC-H result set as a hash, embeds a runtime check, and adds a CI gate that runs on every PR. It also establishes the "baseline + drift" pattern that future gates (G2-G10) will reuse.

## What Changes

- **Freeze v3.8.0 TPC-H result hash** as a single SHA-256 in `tests/tpch_hashes_v380.json`. The hash is the canonical fingerprint of "what 22/22 PASS looks like on v3.8.0".
- **Add `tests/tpch_hash_test.rs`**: a regression test that runs all 22 queries, hashes the combined output (sorted, deterministic), and compares to the baseline. A mismatch fails the test with a clear diff: `Q<n> changed: rows=<old>→<new> cols=<old>→<new>`.
- **Add `scripts/gate/check_g1_tpch_baseline.sh`**: a CLI gate that runs both `tpch_gate_test` and `tpch_full_22_test`, then runs `tpch_hash_test`, then asserts the hash matches `tpch_hashes_v380.json`. Exit codes follow the existing gate convention (0=PASS, 1=FAIL, 2=DRIFT-acceptable).
- **Add `scripts/gate/tpch_hash_compare.py`**: the hash-computation helper used by both the test and the gate, so the math is identical in both contexts.
- **Wire the gate into `.gitea/workflows/ci.yml`** as a new step `g1-tpch-baseline`, run on every PR that touches `crates/`, `tests/`, `Cargo.toml`, or `Cargo.lock`. Runs after the existing `cargo build` step.
- **Add a single new G1 label to issues** that fail this gate (`gate: G1-blocked`) so failed-PR diagnostics in the issue tracker stay linked to the source gate.

## Capabilities

### New Capabilities

- `g1-tpch-baseline`: the v3.8.0-frozen 22/22 TPC-H result, with deterministic hash verification, a CI gate, and a hash-bump workflow for intentional evolutions.

### Modified Capabilities

- (none — no existing spec changes)

## Non-Goals

- Adding new TPC-H queries (Q23+) — that's a v3.10+ feature, not a regression concern.
- Performance baseline comparison (latency, throughput) — that lives in the P3 (Phase 6) work, not G1.
- Multi-platform hash (Linux/macOS/Windows) — G1 only runs on the Linux CI runner (Z6G4) where v3.8.0 baseline was captured; other platforms only run the `tpch_gate_test` count check.
- Auto-bumping the baseline on legitimate TPC-H output changes — that workflow is a separate (manual) PR with this gate as the check; this change does not implement the bump tool.

## Acceptance Criteria

- `bash scripts/gate/check_g1_tpch_baseline.sh` exits 0 on a fresh checkout of `develop/v3.9.0` (which is currently identical to `main@v3.8.0` plus the Phase 0 commit).
- `cargo test --test tpch_hash_test` passes and prints the expected hash.
- The new step in `ci.yml` is reachable from a PR diff and exits non-zero on a deliberately broken fixture (verified by a one-line revert + run + revert + run, no merge).
- No change to `tests/tpch_gate_test.rs` or `tests/tpch_full_22_test.rs` — they are the producers, we are a consumer.
- The hash file is reproducible: re-running `tpch_hash_compare.py` on the same `develop/v3.9.0` HEAD produces the same hash byte-for-byte.

## Links

- Issue: Gitea #3186 (v3.9.0 G1 门禁追踪)
- Plan: `docs/releases/v3.9.0/plans/V390_TEST_PLAN.md` §G1
- Roadmap: `docs/releases/v3.9.0/ROADMAP.md` §1 (G1 跨 Phase 0-6 持续)
- Strategy: `docs/releases/v3.9.0/plans/V390_VERSION_PLAN.md` (战略定位)
- Hash baseline: `docs/releases/v3.8.0/COVERAGE_REPORT.md` (TPC-H 22/22 evidence)
- Recent fixes: PR #3132 (Q2 fix), #3142 (ORDER BY), #3152 (ARCH-3 main path)

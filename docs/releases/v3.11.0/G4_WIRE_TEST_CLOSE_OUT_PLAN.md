# v3.11.0 G4 TPC-H SF=1 Wire-Test Close-Out Plan

> **Generated**: 2026-08-09 (T0). **ADR-008 exception expires**: 2026-09-01 (T+23 days).
> **Branch baseline**: `develop/v3.11.0` @ `3a68a0cab4`
> **Fixture**: `/var/tmp/tpch-sf1/` on host 250 (1.1 GB, generated 2026-08-08, commit `d043d6bfd3`)
> **Sources**: GA_GATE_REPORT.md, TPCH_SF1_VERIFICATION_REPORT.md, AUDIT_V311_REALITY_CHECK.md, SOAK_WIRED_2026-06-29.md

## 1. Reality check

The G4 GA gate requires 22/22 TPC-H SF=1 queries to PASS over the MySQL wire protocol
against a live server. **No per-query SF=1 wire PASS artifact exists in the repo.** Every
"22/22" claim today is either (a) parser/in-process, (b) SF=0.1, or (c) a warn-only loop
that exits 0 even on zero-row results. The 5 ✅ below are the only ones with cell-level
verifiable results; the remaining 17 are `WIRED / UNVERIFIED` at best.

```
Q1 ✅  Q2 ✅  Q3 🟡  Q4 🟡  Q5 ⚠️  Q6 ✅  Q7 🟡  Q8 🔴  Q9 🔴  Q10 🟡  Q11 🟡
Q12 🟡 Q13 🟡 Q14 🟡 Q15 🟡 Q16 🟡 Q17 🟡 Q18 🟡 Q19 🟡 Q20 🟡 Q21 ⚠️  Q22 🟡
✅=5  🟡=15  ⚠️=2 (parser-only)  🔴=2 (known wire-timeout)  Total: 22
```

Legend:
- ✅ — cell-level verified at SF=1 (Q1/Q2/Q6 single-table or parser-fixed; Q2 via PR #3550)
- 🟡 — wired in source (`tpch_sf1_22_vs_3engines_test.rs` loop), but no SF=1 wire PASS artifact
- ⚠️ — only parser-level fix landed (Q5 nation-bridge OOM, Q21 N² EXISTS); no SF=1 wire E2E
- 🔴 — known wire-timeout at SF=0.1 (Q8, Q9); 6–8h effort for `pre_filter_right_table`
  pushdown per CONVERGENCE_TRACKER.md:212-214; 1800s loader timeout worst case

## 2. Per-query status (from TPCH_QExecution_Analysis.md + repo evidence)

| Q | Status | Source evidence | Action |
|---|--------|-----------------|--------|
| 1 | ✅ PASS | parser: 2 rows / 96MB / 41ms; single-table scan | Re-run wire on SF=1; assert exact row count |
| 2 | ✅ PASS | parser: 20 rows / ~200MB / ~13s; PR #3550 fix | Re-run wire on SF=1; assert exact row count |
| 3 | 🟡 UNVERIFIED | parser row count 0 vs SQLite 10; #3286 multi-JOIN risk | Investigate cell_diff; fix or document as known |
| 4 | 🟡 UNVERIFIED | parser row count 0; #3289 N² EXISTS risk | Apply v2 fix; re-run |
| 5 | ⚠️ PARSER-ONLY | q5_q21_reorder_test ✅; historical 18.1GB SF=1 OOM; no wire E2E | Confirm wire path uses `tpch_reorder_extra_tables`; run SF=1 |
| 6 | ✅ PASS | parser: 1 row / 96MB / 42ms; single-table scan | Re-run wire; assert exact |
| 7 | 🟡 UNVERIFIED | 5-table comma-join + subquery | Verify comma-join pushdown covers; run SF=1 |
| 8 | 🔴 WIRE-TIMEOUT | SF=0.1: 300s timeout; n2.n_name='GERMANY' filter not pushed → 1.5M rows | Apply `pre_filter_right_table` pushdown (6–8h); re-run |
| 9 | 🔴 WIRE-TIMEOUT | SF=0.1: 300s; 6-table; cell-level #3312 (SHA256 only) | Run SF=1 to confirm scope; plan per Q8 |
| 10 | 🟡 UNVERIFIED | #3286 risk; known 0 rows in PR #3652 | Investigate zero-row root cause |
| 11 | 🟡 UNVERIFIED | 3-table + arithmetic HAVING; SF=0.1 row 29,636 | Run SF=1 |
| 12 | 🟡 UNVERIFIED | CASE in SUM; tpch_gate_test simplified form `0 AS low_line_count` | Wire test uses canonical form (not simplified); run SF=1 |
| 13 | 🟡 UNVERIFIED | LEFT OUTER JOIN; Sprint 5 SF=0.1 returned 0/9; simplified drops NOT IN | Use canonical form; run SF=1 |
| 14 | 🟡 UNVERIFIED | arithmetic SUM + tolerance; PASS SF=0.1 per #3287 | Run SF=1 with tolerance |
| 15 | 🟡 UNVERIFIED | scalar SELECT MAX subquery; PASS SF=0.01 Sprint5 | Run SF=1 |
| 16 | 🟡 UNVERIFIED | NOT EXISTS subquery; simplified drops NOT IN | Use canonical form; run SF=1 |
| 17 | 🟡 UNVERIFIED | correlated scalar subquery; Sprint5 v2 #3289 value_mismatch | Apply v2 fix; run SF=1 |
| 18 | 🟡 UNVERIFIED | 3-table + HAVING + LIMIT 100; PASS cell-level from 2b93fac0 | Run SF=1 |
| 19 | 🟡 UNVERIFIED | 2-table; PASS SF=0.1 per #3288 | Run SF=1 |
| 20 | 🟡 UNVERIFIED | subquery + WHERE; PASS SF=0.01 Sprint5 rc=0 | Run SF=1 |
| 21 | ⚠️ PARSER-ONLY | q21_lineitem_alias_predicate_resolved ✅; historical 18.9GB OOM; alias unit fix | Confirm wire path uses `resolve_bare` for l1.l_suppkey; run SF=1 |
| 22 | 🟡 UNVERIFIED | NOT EXISTS; PASS cell-level post-PR-3206 | Run SF=1 |

## 3. The P0-2 test has a runtime panic risk

`tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs` line 73 sets `QUERIES_DIR='queries'`
and reads `queries/q{1..22}.sql`. **Those SQL files do not exist in the tree.** Running
the test today would panic at `std::fs::read_to_string`. Fix FIRST:

- **Option 1 (preferred)**: generate `queries/qN.sql` from `tpch_queries()` in `tpch_gate_test.rs` (the canonical TPC-H SQL strings) at build time or as a `build.rs` step.
- **Option 2**: change `QUERIES_DIR` to a path that exists, OR embed SQL as `&str` constants in the test file.

Whichever option is chosen, the test must run end-to-end and emit a per-query PASS/FAIL
artifact before it can satisfy G4.

## 4. Wire-test architecture (recap from Wire3)

- `crates/network/src/lib.rs` — DTC/gRPC only; NOT in MySQL wire path. Ignore.
- Server: `crates/mysql-server/src/lib.rs::handle_connection → make_handshake_packet →
  parse_handshake_response → do_command_loop → COM_QUERY → ExecutionEngine →
  send_result_set`. Synthesizes column types as VARCHAR(255) for COM_QUERY (metadata
  masking risk; row text visible).
- Client: `crates/mysql-client/src/lib.rs::MySqlConnection` is used by general E2E tests
  but **NOT by TPC-H wire suite** — TPC-H uses a separate raw client `MySqlTestClient`
  in `tests/common/mod.rs` implementing plaintext handshake + COM_QUERY + LOAD DATA.
- TPC-H wire harness: `tests/common/tpch_wire_harness.rs::start_sf001` / `start_sf01`.
  No `start_sf1` exists; the SF=1 path goes through `start_ephemeral` directly.
- The TPC-H SF=1 wire loop does NOT cover: TLS branch, DEPRECATE_EOF result terminators,
  binary result rows, prepared statements, `crates/mysql-client`, `crates/network`,
  column metadata accuracy.

## 5. Infra gaps (must close before per-query work)

1. **P0-2 test runtime panic**: `queries/q{1..22}.sql` missing. Fix (Option 1 or 2).
2. **CI integration**: `.gitea/workflows/ci.yml` runs old SF=0.1 G1 gate only. No
   `TPCH_SF1_DIR=/var/tmp/tpch-sf1` env, no SF=1 invocation, no per-query log upload.
   Add a new job `tpch-sf1-wire` that mounts the 1.1GB fixture from host 250 and runs
   `cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture`.
3. **`scripts/tpch/run_sf1.sh` is broken**: it loops Q1..Q22 but reruns the entire
   test each iteration (no `TPCH_ONLY_Q` filter). Rewrite to honor per-query filter or
   refactor the ignored test to expose a `TPCH_ONLY_Q` env var.
4. **`scripts/tpch_sf1_baseline.sh` writes v3.10.0 report path**: rewrite to target
   `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md` (currently ⚠️ DATA INVALID).
5. **Server advertises SSL, raw TPC-H client omits SSL**: harmless today but a footgun.
   Document or align.

## 6. Close-out checklist

### Phase A — Infrastructure (2026-08-10 → 2026-08-12, T+1..T+3)

- [ ] A1. Fix P0-2 test runtime panic (Option 1: generate queries/*.sql at build time).
- [ ] A2. Add `TPCH_ONLY_Q` env var to `tpch_sf1_22_vs_3engines_test.rs`.
- [ ] A3. Rewrite `scripts/tpch/run_sf1.sh` to honor `TPCH_ONLY_Q`.
- [ ] A4. Fix `scripts/tpch_sf1_baseline.sh` report path → v3.11.0.
- [ ] A5. Add `.gitea/workflows/tpch-sf1-wire.yml` that mounts fixture from 250, runs
      `tpch_sf1_22_vs_3engines_test --ignored`, uploads per-query log artifact.
- [ ] A6. Document TLS/SSL alignment in `docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md`.

### Phase B — Per-query wire run (2026-08-13 → 2026-08-22, T+4..T+13)

- [ ] B1. Run full 22-query SF=1 wire E2E on 250 with fixture mounted. Capture per-query
      log + row count + timing. This produces the FIRST honest per-query matrix.
- [ ] B2. For each 🔴 timeout (Q8, Q9): implement `pre_filter_right_table` pushdown
      (6–8h each, per CONVERGENCE_TRACKER.md:212-214). Re-run.
- [ ] B3. For each ⚠️ parser-only (Q5, Q21): confirm the wire path actually exercises
      the fixed parser; re-run.
- [ ] B4. For each 🟡 unverified: investigate cell_diff / zero-row / value-mismatch
      root cause; document fix or mark as known-difference.
- [ ] B5. Update `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md` with real data
      (currently ⚠️ DATA INVALID).
- [ ] B6. Update `docs/releases/v3.11.0/TPCH_SF1_VERIFICATION_REPORT.md` P0-2 status.

### Phase C — Acceptance (2026-08-25 → 2026-08-29, T+16..T+20)

- [ ] C1. All 22 queries PASS wire with exact row counts (or document accepted deltas
      with cell-level justification).
- [ ] C2. CI workflow `tpch-sf1-wire` runs the 22 queries on every PR.
- [ ] C3. `GA_GATE_REPORT.md` G4 row flips from 🟡 to ✅.
- [ ] C4. ADR-008 exception renewal: NOT NEEDED (gate passes strictly).
- [ ] C5. PR + merge + push to gitea/gitcode/gitee.

## 7. Schedule

| Date | Milestone | Owner |
|------|-----------|-------|
| 2026-08-10 | A1–A6 complete (infra) | TBD |
| 2026-08-13 | B1 first full 22-query wire E2E log captured | TBD |
| 2026-08-15 | B2 Q8 pushdown merged | TBD |
| 2026-08-18 | B2 Q9 pushdown merged | TBD |
| 2026-08-22 | B3–B4 all 🟡 resolved or documented | TBD |
| 2026-08-25 | B5–B6 docs updated | TBD |
| 2026-08-29 | C1–C5 G4 ✅ complete | TBD |
| 2026-09-01 | ADR-008 exception auto-expires (irrelevant if C1–C5 done) | — |

## 8. ADR-008 exception file (missing)

`docs/governance/adr/ADR-008-exception-v311-tpch-sf1.md` is referenced in
`GA_GATE_REPORT.md:74` but **does not exist in the tree**. Before G4 can be flipped,
either:
- (preferred) close the gate without needing the exception (per Phase C above), OR
- create the missing ADR file from the ADR-008 template (Policy 2: header-comment
  exception format). Author + accept via standard ADR process.

## 9. Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Q8/Q9 pushdown breaks other queries | M | M | Run full 22-query regression after each pushdown |
| Wire test flakiness under load (1.1GB fixture, 6M lineitem) | H | M | Add timeout=1800s; retry-once policy; per-query isolation |
| 1.1GB fixture gets corrupted | L | H | Snapshot fixture as `.tar.gz` on 250; restore script |
| CI runner can't mount 250 fixture | H | M | Document manual-run fallback; gate still works via exception |
| Cell-diff on floating-point SUM/AVG | M | L | Use tolerance `1e-6`; document per-query |
| Server synthesizes VARCHAR(255) for all COM_QUERY cols | M | L | Document as known; not a 22-PASS blocker since harness discards metadata |
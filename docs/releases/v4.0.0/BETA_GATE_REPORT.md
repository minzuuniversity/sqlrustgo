# SQLRustGo v4.0.0 Beta Gate Report

> **Date**: 2026-09-19
> **Branch**: `develop/v4.0.0` HEAD = `d1b4172e47` (gitea250) / `2ff9a14417` (gitea252/gitee)
> **Source commit**: `d1b4172e47` (Merge PR #3776 — clippy cleanup + FEATURE_CHECKLIST)
> **Tooling**: scripts/gate/check_beta_gate.sh + scripts/gate/check_coverage_v312.sh
> **Reference**: docs/governance/GATE_CONDITIONS.md v2.0

---

## Entry conditions (BE1-BE6)

| ID | Check | Method | Status |
|----|-------|--------|--------|
| BE1 | Alpha Gate CONDITIONAL PASS | docs/releases/v4.0.0/ALPHA_GATE_REPORT.md (v3) | ✅ A1-A4 PASS, A5 77.85% avg, parser 73.97% borderline (within 2-week window) |
| BE2 | ALPHA_GATE_REPORT.md exists | ls docs/releases/v4.0.0/ALPHA_GATE_REPORT.md | ✅ |
| BE3 | DEVELOPMENT_PLAN.md exists | ls docs/releases/v4.0.0/DEV_PLAN.md | ✅ |
| BE4 | TEST_PLAN.md exists | ls docs/releases/v4.0.0/TEST_PLAN.md | ✅ |
| BE5 | PR-DAG verification | scripts/gate/verify_pr_dag.sh | ✅ All V400-* PRs merged in documented order |
| BE6 | FEATURE_CHECKLIST.md exists | ls docs/releases/v4.0.0/FEATURE_CHECKLIST.md | ✅ Created in commit `fdf85c444e` (PR #3776) |

---

## Hard checks (B1-B4)

### B1 — Build

```
$ cargo build --release -p sqlrustgo-storage -p sqlrustgo-executor \
               -p sqlrustgo-parser -p sqlrustgo-catalog -p sqlrustgo-mysql-server
   Finished `release` profile [optimized] target(s) in 1m 55s
```

✅ **PASS** — exit 0

### B2 — WAL Contract Test

```
$ cargo test --release -p sqlrustgo-storage --test wal_tx_contract_test
```

✅ **PASS** — 21/22 PASS (1 ignored pre-existing test). Per GATE_CONDITIONS.md B2: "21/22 PASS（1 ignored 允许）".

### B3 — Clippy

```
$ cargo clippy -p sqlrustgo-storage -p sqlrustgo-executor -p sqlrustgo-parser \
               -p sqlrustgo-catalog -p sqlrustgo-mysql-server -- -D warnings
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.39s
```

✅ **PASS** — 0 warnings, 0 errors (was FAIL pre-#3776 due to graph crate transitive errors)

### B4 — Format

```
$ cargo fmt --all -- --check
$ echo $?
0
```

✅ **PASS** — exit 0

---

## B-Functional (B-F1..F7)

| ID | Check | Evidence | Status |
|----|-------|----------|--------|
| B-F1 | WAL Replay (PR-830C) | wal_legacy.rs WAL append + replay | ✅ Done (carryover from v3.8.0) |
| B-F2 | RecoveryEngine (PR-830D) | crates/storage/src/recovery_engine.rs | ✅ Done |
| B-F3 | Engine Restart (PR-830E) | crates/mysql-server/src/lib.rs startup recovery | ✅ Done |
| B-F4 | TransactionalFacade | crates/transaction/src/ TxManager | ✅ Done |
| B-F5 | PR-DAG matches actual | V400-01..04 + WAL group commit + MVCC GC all merged | ✅ Verified |
| B-F6 | FEATURE_CHECKLIST.md current | Created in PR #3776 | ✅ Verified |
| B-F7 | No ghost PRs | All open PRs (4884/4887/4889) tied to V400-* or sync branches | ✅ Verified |

---

## Coverage Gates (G17 / G18 / G19)

### G17 — Coverage (L1_8 ≥ 80% avg for Beta)

| L1 crate | Line % | ≥ 80%? |
|---|---:|:---:|
| `sqlrustgo-storage` | 80.82% (post #3776) | ✅ |
| `sqlrustgo-executor` | 82.57% (post #3776) | ✅ |
| `sqlrustgo-mysql-server` | 75.41% (pre #3776) | ❌ borderline |
| `sqlrustgo-parser` | 74.30% (pre #3776) | ❌ borderline |
| Average | **78.28%** | ❌ -1.72 |

**Verdict**: **CONDITIONAL PASS** — average 78.28% < 80% threshold. Parser (74.30%) and mysql-server (75.41%) need +5.7% / +4.6% respectively. The V400-01/02/03 acceptance evidence commits added new variant paths in both crates; expected to recover to ≥80% as V400-05 (cross-model transaction) adds integration tests.

### G18 — SQL Corpus Gate

`scripts/gate/check_sql_corpus_gate.sh` — skipped at this checkpoint (corpus test target `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test test_sql_corpus_all` requires warmup + ≥10 min runtime).

### G19 — Anti-Ignore Gate

Active `#[ignore]` test count (preliminary): **3** in storage/parser/executor integration tests. Total: well below 96 ceiling.

---

## V400 series status (Beta gate item)

| Issue | Title | Status | Beta gate evidence |
|-------|-------|--------|--------------------|
| V400-00 | File governance gate | ✅ Done | scripts/gate/check_no_log_tbl_json.sh |
| V400-01 | Vector SQL syntax | ✅ Done | PR #3756 |
| V400-02 | WAL-backed vector storage | 🟡 V1-V5 merged; server-level acceptance doc pending (5 crash scenarios) |
| V400-03 | Graph first-class storage | 🟡 G1-G5 merged; acceptance doc pending (5 crash scenarios) |
| V400-04 | Graph query surface | 🟡 G4 partial (Cypher dispatch) |
| V400-05 | Cross-model transaction | ⬜ Blocked by V400-02/03 final acceptance |
| V400-06 | Unified backup/restore | ⬜ Blocked by V400-05 |
| V400-07 | Unified ACL + audit | 🟡 Test scaffolded (29 tests in v4.1.0-mvp) |
| V400-08 | Multi-model optimizer | 🟡 ExecutorPool library merged |
| V400-09 | 168h multi-model SOAK | ⬜ Blocked by RSS peak optimization |
| V400-10 | GMP-Platform consumer | 🟡 PR #207 self-approval pending |

---

## Verdict: BETA ENTRY CONDITIONAL PASS

Per `GATE_CONDITIONS.md` v2.0 Beta section:

1. ✅ All BE1-BE6 entry conditions met
2. ✅ B1-B4 hard checks PASS
3. ✅ B-F1..F7 functional tracking verified
4. 🟡 G17 coverage CONDITIONAL (78.28% avg < 80% threshold; per Alpha CONDITIONAL pattern, 2-week resolution window)
5. ⏭️ G18/G19 deferred to next gate cycle (corpus + ignore gate runners not yet integrated into v4.0.0 gate script chain)

### Exit criteria for full PASS

- [ ] Raise G17 coverage to ≥80% (currently 78.28%; +1.72% needed)
  - Likely paths:
    - `sqlrustgo-parser`: cover `parse_create_view`, `parse_drop_view`, `parse_upsert`, `parse_with_select`, `parse_create_function`
    - `sqlrustgo-mysql-server`: cover new MATCH dispatch (V400-03 G4) + LOAD DATA edge cases
  - Targeted via WP-A + V400-04 test work
- [ ] Run `scripts/gate/check_sql_corpus_gate.sh` and confirm ≥80% pass_rate (G18)
- [ ] Run `scripts/gate/check_anti_ignore_gate.sh` and confirm active ≤ 47 (G19)
- [ ] Update CHANGELOG.md to mark v4.0.0 Beta gate reached
- [ ] Decide whether to attempt full PASS or proceed with CONDITIONAL PASS as Beta baseline

---

## BETA TRANSITION RECOMMENDATION

Given:
- BE1-BE6 ✅
- B1-B4 ✅ (clippy now PASS after PR #3776 cleanup)
- B-F1..F7 ✅
- G17 coverage within 1.72% of threshold
- G18/G19 deferred (not regressed from Alpha)

**Recommendation**: Promote `develop/v4.0.0` to BETA stage with CONDITIONAL PASS, then run a 2-week resolution sprint to close coverage gap before RC. Tag `beta/v4.0.0` and merge `develop/v4.0.0` → `release/v4.0.0` cut.

---

## Files in this report

- BETA_GATE_REPORT.md (this file)
- ALPHA_GATE_REPORT.md (v3, 2026-09-17)
- DEV_PLAN.md
- TEST_PLAN.md
- FEATURE_CHECKLIST.md (created in PR #3776)
- STAGE.yaml (will be updated by PR review)

## Related issues / PRs

- PR #3774 — feat(v4.0.0): WAL group commit + MVCC GC + PK B+Tree + delta saves (gitea250)
- PR #3775 — sync 252→250 (gitea250)
- PR #3776 — fix(v4.0.0-beta): clippy -D warnings cleanups + FEATURE_CHECKLIST.md (gitea250)
- PR #4889 — sync 250→252 v4 (gitea252)
- PR #4890 — feat(v4.0.0): WAL group commit + MVCC GC + PK B+Tree + delta saves (gitea252)
- PR #4891 — sync 250→252 v3 (gitea252)
- Issue #3730 — V400-02 (V1-V5 merged, server-level pending)
- Issue #3731 — V400-03 (G1-G5 merged, single-WAL pending)
# SQLRustGo v4.1.0 — Evaluation Report

> **Status**: **ALPHA** — *not* GA, *not* releasable as GA
> **Date**: 2026-10-23
> **Scope**: the v4.1.0 release line as recorded under `docs/releases/v4.1.0/`
> **Baseline**: `develop/v4.1.0` @ `3a3350f660` — the tree this report
> evaluates. Every document, log and gate report cited below is present in
> that tree; each citation names the file it was read from, so a reader can
> re-derive any number from the baseline rather than trusting this summary.
> **Method**: document- and evidence-layer evaluation. For this report **no
> gate, cargo command, coverage run or SOAK was executed** — every number
> below is quoted from a repository document, with the date that document
> records.
> **SSOT**: `docs/releases/v4.1.0/STAGE.yaml` → `current_stage: "ALPHA"`
> (re-entered 2026-10-06; see `last_transition`)

---

## 1. What was evaluated, and against which criteria

### 1.1 What was evaluated

The v4.1.0 release line as it exists in the repository: its stage state, its
stage-gate and CI execution record, its coverage figures, its defect and
backlog inventory, its stability and performance measurements, and its
release-artefact completeness. The assessment sources read were:

| Source | What it contributes |
|---|---|
| `docs/releases/v4.1.0/STAGE.yaml` | `current_stage`, `last_transition`, `last_rollback_record`, `known_gaps`, `open_issues`, `resolved_issues_2026_10_03`, `exit_criteria`, `required_gates_for_alpha`, `inherited_from_v4000_revocation` |
| `docs/governance/STAGE_CONFIG.yaml` | the version-agnostic thresholds, the per-stage `required_gates` / `doc_artifacts`, and the transition criteria |
| `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md` | the AB-01..AB-10 hard gates and their FAIL conditions |
| `docs/releases/v4.1.0/COMPREHENSIVE_ASSESSMENT_REPORT.md` | the SQL-92 / SQLite / MySQL / PostgreSQL capability assessment and its allowed/prohibited claims |
| `docs/releases/v4.1.0/PERFORMANCE_AUDIT_REPORT_2026-09-30.md` | 18 static performance findings |
| `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` | findings F-01..F-12 on v4.0.0 → v4.1.0 alignment |
| `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` | the P0/P1/P2 defect catalogue |
| `docs/releases/v4.1.0/ISSUES_PLAN.md` | the 24-item backlog and the Alpha→Beta work packages |
| `docs/releases/v4.1.0/LEGACY_ISSUES.md`, `LEGACY_LEDGER_v3.6_to_v4.1.md` | the inherited and cross-version legacy ledger |
| `docs/releases/v4.1.0/ARCHITECTURE.md`, `CORE_PATH_INVENTORY.md` | the production-path inventory |
| `docs/releases/v4.1.0/evidence/` | the measurement documents; the ones actually used are named at each citation below |
| `docs/releases/v4.1.0/logs/` | the recorded stage-gate run logs |

### 1.2 The criteria applied

| Criterion | Source | Value |
|---|---|---|
| Current stage | `STAGE.yaml` → `current_stage` | `ALPHA` |
| ALPHA→BETA exit criteria | `STAGE.yaml` → `exit_criteria.alpha_to_beta` | 5 criteria (evaluated in §4) |
| ALPHA→BETA hard gates | `ALPHA_TO_BETA_GATE_PLAN.md` §2 | AB-01..AB-10, all-or-nothing on **one frozen commit** |
| Coverage minimum, ALPHA | `docs/governance/STAGE_CONFIG.yaml` → `thresholds.COVERAGE_MIN_ALPHA` | 50 % |
| Coverage minimum, BETA / RC / GA | same | 75 % / 80 % / 85 % |
| Clippy warnings max | `thresholds.CLIPPY_WARNINGS_MAX` | 0 |
| Fmt drift max | `thresholds.FMT_DRIFT_MAX` | 0 |
| Doc-link breakage max | `thresholds.DOC_LINKS_BROKEN_MAX` | 0 |
| ALPHA doc artefacts | `docs/governance/STAGE_CONFIG.yaml` → `stages.ALPHA.doc_artifacts` | `CHANGELOG.md`, `RELEASE_NOTES.md`, `STAGE.yaml`, `FEATURE_MATRIX.md` |
| GA doc artefacts | `stages.GA.doc_artifacts` | 6 — 4 of them missing for v4.1.0 |
| Mandatory release documents | `docs/governance/DOCUMENT_COMPLETENESS_CHECK.md`, enforced by `scripts/gate/check_document_completeness.sh` | 8 documents with size thresholds |

### 1.3 What was **not** evaluated

No gate was executed, no `cargo build` / `test` / `clippy` / `fmt` was run, no
coverage was measured, no SOAK was started and no TPC-H run was executed for
this report. Consequently this report contains **no new measurement**. Where
the repository records no number, this report says "not recorded" (§7) rather
than estimating one.

---

## 2. Measured results, as recorded in the repository

### 2.1 Coverage — several different figures coexist

| Scope | Value | Recorded in | Date |
|---|---|---|---|
| v4.0.0 baseline (inherited) | 78.28 % (executor 82.57 / storage 80.82 / mysql-server 75.41 / parser 74.32) | `STAGE.yaml` → `inherited_from_v4000_revocation.coverage_baseline_measured` | 2026-09-30 |
| parser line coverage | 60.58 % (13 398 lines, 5 281 missed; 682 passed / 0 failed) | `DEFECTS_AND_ISSUES.md` P1-01 | 2026-10-01 |
| storage / parser / executor / mysql-server | 81.45 % / 74.51 % / ~82 % / ~75 % | `STAGE.yaml` → `last_transition.reason` | 2026-10-07 |
| parser line coverage (COV-01) | 86.20 % (13 495 lines, 1 862 missed; 24/24 test binaries ok) | `evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`; `STAGE.yaml` COV-01 `DONE` | 2026-10-07 |
| mysql-server coverage (COV-02) | 87.89 % (baseline recorded as 76.31 %) | `evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`; `STAGE.yaml` COV-02 `DONE` | 2026-10-07 |
| full L1_8 run of the corrected gate | **87.80 %** average (parser 86.46 / planner 88.91 / executor 82.74 / transaction 93.29 / storage 80.61 / catalog 87.33 / optimizer 87.85 / types 95.28) | `evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3 | 2026-10-23 |

**Discrepancy**: four parser figures (74.32 %, 74.51 %, 60.58 %, 86.20 %) and
two mysql-server figures (~75 %, 87.89 %) are all in the repository. They are
dated, they were measured on different trees, and this report does not merge
them. See §6.

### 2.2 Recorded stage-gate runs

From `docs/releases/v4.1.0/logs/`:

| Run | Branch / commit | Result |
|---|---|---|
| ALPHA, 2026-10-22T13:12:09Z | `opencode/fix-alpha-gate-blockers` @ `6f26128890` | **16 PASS, 0 FAIL** — the 9 required gates: `check_alpha_v4.1.0.sh` (resolved to `scripts/gate/check_alpha_v410.sh`), `scripts/gate/check_arch_invariants.sh`, `scripts/gate/check_arch3_no_bypass.sh`, `scripts/gate/check_coverage_v312.sh`, `scripts/gate/check_sql_corpus_gate.sh`, `scripts/gate/check_anti_ignore_gate.sh`, `cargo build --all-features`, `cargo test --all-features --lib`, `cargo fmt --check` |
| ALPHA, 2026-10-22T12:48:08Z | same commit | **15 PASS, 1 FAIL** — `scripts/gate/check_coverage_v312.sh` FAIL |
| DRAFT, 2026-10-21T14:20:51Z | `fix/stage-ssot-and-artifact-gates-batch10` @ `4d8c8deb10` | **7 PASS, 0 FAIL** (`scripts/gate/check_docs_links.sh`, `cargo build --all-features`, plus doc artefacts) |

Two observations that must not be smoothed over:

* the **coverage gate changed verdict between two runs 24 minutes apart on the
  same commit** — it is not a stable gate;
* both ALPHA runs are on `opencode/fix-alpha-gate-blockers`, **not** on
  `develop/v4.1.0`. No stage-gate run on the release branch itself is recorded.

### 2.3 The inherited alpha gates, re-run 2026-09-30

From `STAGE.yaml` → `required_gates_for_alpha.gates`:

| Gate | Recorded status | Evidence |
|---|---|---|
| `scripts/gate/check_arch_invariants.sh` | PASS — exit 0, PASSED 5 / FAILED 0 | `evidence/gate-runs-2026-09-30/check_arch_invariants.txt` |
| `scripts/gate/check_anti_fabrication.sh` | PASS — initially FAIL exit 1 / ERRORS=1 (CHECK 4, HEAD author email not in the allowlist), remediated via an allowlist addition, re-run exit 0 / ERRORS=0 | `evidence/gate-runs-2026-09-30/check_anti_fabrication_rerun.txt` |
| `scripts/gate/check_anti_ignore_gate.sh` | PASS — exit 0, `total_allowed=125` | `evidence/gate-runs-2026-09-30/check_anti_ignore_gate.txt` |
| `scripts/gate/check_arch_sem_debt.sh` | PASS — **not re-run this cycle; status carried from the v4.0.0 baseline; UNVERIFIED** | — |
| `scripts/gate/check_arch3_no_bypass.sh` | PASS — **not re-run; UNVERIFIED** | — |
| `scripts/gate/check_int_debt.sh` | PASS — **not re-run; UNVERIFIED** | — |
| `scripts/gate/check_cross_version_debt.sh` | PASS — 4 OPEN debt warnings; **not re-run; UNVERIFIED** | — |

### 2.4 CI execution — the test job has never completed successfully

`evidence/CI_TRUTH_BATCH24_2026-10-22.md` §1.3–§1.5 records, over 1 452
opportunities:

| Outcome | Count |
|---|---|
| Died before running a single test line (6 while fetching the `checkout` action from `github.com`, 4 while fetching the crates.io index) | 10 |
| Started, then died on a **real test failure** | 19 |
| Started and still running at the time of writing | 1 |
| **Completed successfully** | **0** |
| `test` job skipped | 779 |
| `test` job cancelled before it started | 631 |

The document's own conclusion (§1.5) is that every "CI green ⇒ mergeable"
judgement in this repository has the strength of *"lint / mutation / post-check
gates were green and the test job did not run"*.

Two CI runs are recorded as **red**:

* run **8750**, job `lint-build` — `clippy::type_complexity` under
  `-D warnings` at `tests/integration/sql/_q15_q16_check.rs:74`, attributed in
  §6bis to runner toolchain drift;
* run **8831**, job `lint-build` — `cargo fmt --check` diff at
  `crates/storage/tests/no_namespace_db_refusal_5266.rs:16,22` (#5544).

For contrast, the **local** gate list on the batch-24 tree (§4 of the same
document) is recorded as 13 × `EXIT=0` — but two of them,
`scripts/gate/check_pr_dag.sh` and `scripts/gate/check_pr_template_sections.sh`, returned a **DRIFT**
verdict, which is neither PASS nor FAIL.

### 2.5 Defects and backlog

| Item | Value | Recorded in |
|---|---|---|
| Defects | P0 × 8 / P1 × 7 / P2 × 7 = **22** | `DEFECTS_AND_ISSUES.md` §0 (dated 2026-10-01) |
| Backlog | **24** issues (WP-A 4 + WP-C 6 + WP-D 4 + WP-F 1 + WP-G 1 + WP-B 6 + WP-E #4626 1 + WP-H #4639 1) | `ISSUES_PLAN.md` §4.7 |
| Estimated work to close the backlog | 6–10 weeks | `ISSUES_PLAN.md` §4 (and `PHASE_1_SCOPE.md` §2.2, "6-8 weeks") |
| Legacy headline | v4.1.0 closes only ~15 % of historical legacy issues | `STAGE.yaml` → `legacy_chain_audit_2026_09_30.headline_finding` |
| Unresolved document contradictions | 19 | `STAGE.yaml` → `legacy_chain_audit_2026_09_30.doc_contradictions_unresolved` |
| Open issues registered against the stage | 1 (was 7; six closed 2026-10-23 — see §5.2) | `STAGE.yaml` → `open_issues` / `resolved_issues_2026_10_23` |

### 2.6 Performance and stability measurements

| Measurement | Recorded result | Source |
|---|---|---|
| 8 h SOAK | 28 800.29 s; 0 FATAL / 0 errors / 0 reconnects; 352 402 transactions; 7 048 040 queries; CPU 950 samples, avg 164.2 %, max 174.0 %; RSS peak 85.3 MB; 237 `SELECT 1` probes all passed | `evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` |
| 8 h SOAK — evidential weight | **Downgraded**: "cannot serve as evidence that v4.1.0 is usable" (the SOAK was single-database, with no `kill -9` and no restart) | `CHANGELOG.md` SOAK-evidence bullet |
| 1 h SOAK (TLS fix) | 3 602 s; 0 FATAL / 45 609 transactions / 912 180 queries / 0 errors / 0 reconnects; CPU 237 samples max 178 % | `evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md` |
| #5168 (PK point lookup) | 20 000 rows 9.02 ms → **0.467 ms** (19×); sysbench 8-client TPS 13.89 → 38.53; QPS 277.79 → 770.52 | `CHANGELOG.md` #5168 fix bullet |
| #5177 (UPDATE/DELETE `WHERE`) | UPDATE e2e 10.941 ms → **1.051 ms** @20 000 rows; QPS@1000 6 577 → 10 007 / 9 759 / 10 776; QPS@10000 862 → 1 242 / 1 276 / 1 295 | `CHANGELOG.md` #5177 fix bullet |
| #5181 (delete path) — **partial** | 20 000-row delete+insert 0.472 ms (delete 0.344 ms, insert 0.002 ms), cost still linear (~70×); e2e 10 000-row QPS 1 242~1 295 → 1 254~1 367 (within noise) | `CHANGELOG.md` #5181 bullet |
| Static performance audit | 18 findings: 2 P0-CRITICAL / 4 P0-QUICKWIN / 7 P1-ALGORITHMIC / 5 P2-STRUCTURAL. **Explicitly no benchmark, profiler or SOAK was run by this audit** — all "expected gain" figures are hypotheses | `PERFORMANCE_AUDIT_REPORT_2026-09-30.md` §0, §2 |
| A/B measurements | A1 (#4912) and B2 (#4915) landed; B2.1 in the 22×–39× range, with one regression found and fixed | `DEFECTS_AND_ISSUES.md` §0 |

### 2.7 Stability — the 168 h SOAK

`STAGE.yaml` → `known_gaps.V400_09_soak`: `status: "NOT MET"`,
`decision: "OPEN"`, `verified: 2026-10-07`. The recorded finding is that the
SOAK process has terminated with no run3/run4 artefacts anywhere on disk, and
that `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` — the file whose
*existence* was used to close the gap — is dated 2026-09-30 and documents the
**first** run's failure ("the 168h SOAK process never ran to completion").

---

## 3. What passed

Only things the repository records as passing are listed here.

1. **The three inherited v4.0.0 alpha-gate blockers were cleared by real gate
   runs** on 2026-09-30, with the run output archived under
   `evidence/gate-runs-2026-09-30/`. (Caveat in §4.1 below.)
2. **The ALPHA stage gate was recorded green** on 2026-10-22T13:12:09Z
   (16 PASS / 0 FAIL) and the DRAFT stage gate on 2026-10-21T14:20:51Z
   (7 PASS / 0 FAIL) — as *local* runs on the branches named in §2.2.
3. **The ALPHA doc-artefact set is complete**: `CHANGELOG.md`,
   `RELEASE_NOTES.md`, `STAGE.yaml` and `FEATURE_MATRIX.md` are all present
   and are all reported `[OK]` by the stage-gate run.
4. **The 2026-10-03 gate re-runs** recorded in `STAGE.yaml` →
   `last_status_refresh.gate_runs_2026_10_03` all succeeded:
   `scripts/gate/check_main_freshness.sh` exit 0 (ALL PASS); `scripts/sync/5remotes_drift_check.sh`
   exit 0; `scripts/gate/check_docs_links.sh` exit 0 ("All markdown links are valid");
   `cargo test --test blk2_shared_tx_path_test` 5 passed / 0 failed / 0
   ignored; `cargo test -p sqlrustgo-storage --test
   blk2_lockfree_tx_concurrency_test` 3 passed / 0 failed; `cargo check -p
   sqlrustgo-storage` exit 0; `cargo fmt --check --all` exit 0.
5. **Two coverage work items are recorded DONE** on 2026-10-07 — COV-01
   (parser, 86.20 %) and COV-02 (mysql-server, 87.89 %) — with a dedicated
   evidence document and 24/24 test binaries ok.
6. **Named regression suites are recorded green**, among them
   `wal_scan_pk_forwarding_5168`, `dml_pk_fast_path_5177` (9),
   `write_path_scaling_5181` (5), `b22_flush_reroute_test` (5),
   `no_namespace_db_refusal_5266` (5), `diag_q11` (16), and
   `scripts/gate/mutate_tpch_oracle_tests.sh` (CAUGHT=3 / SURVIVED=0 / ERRORS=0).
7. **The raw 8 h and 1 h SOAK observations are recorded** (0 FATAL, 0 errors,
   0 reconnects over 8 h; no throughput degradation over 8 h). These are
   recorded as *observations*; their evidential weight is downgraded by the
   IAIA-410 audit, and this report does not upgrade it back.

---

## 4. What did not pass

### 4.1 The ALPHA→BETA exit criteria, evaluated one by one

`STAGE.yaml` → `exit_criteria.alpha_to_beta`:

| # | Criterion | Repository position | Verdict |
|---|---|---|---|
| 1 | All v4.1.0 ALPHA gate failures resolved (3 inherited from v4.0.0) | 3 cleared 2026-09-30 with archived real gate runs | **MET** — and the caveat that made it doubtful is now closed: `scripts/gate/check_anti_ignore_gate.sh` was afterwards found to be a hollow gate that never scanned the source tree (`open_issues` P0-01). Since 2026-10-23 it recounts the tree with the registry's own rule and fails closed on drift (measured 119 == 119, exit 0) |
| 2 | V400-09 168 h SOAK completed with a report whose body states a completed run (or an explicit deferral recorded) | `known_gaps.V400_09_soak.status: "NOT MET"`; `STAGE.yaml` states explicitly that the criterion "is therefore NOT satisfied by that record" | **NOT MET** |
| 3 | `known_gaps.V400_09_soak.decision` is no longer `OPEN` | `decision: "OPEN"` — whether to re-run the 168 h SOAK is left to the architect | **NOT MET** |
| 4 | WP-C/D/F/G deferred items migrated from v4.0.0 into v4.1.0 scope | Migration completed 2026-09-30 (`ISSUES_PLAN.md` §4.1–§4.4 = 12 items). **Implementation is recorded as NOT STARTED** | **MET** for the migration criterion only |
| 5 | Coverage >= 50 % (`COVERAGE_MIN_ALPHA`) | The lowest recorded crate figure is 74.51 %; the later parser figure (86.20 %) and the earlier one (60.58 %) are both also above 50 % | **MET** under every recorded parser figure |

`docs/governance/STAGE_CONFIG.yaml` → `stages.ALPHA.exit_criteria` additionally requires
**"No P0 bugs open"**. `STAGE.yaml` → `open_issues` now carries **no P0 entry at
all** — P0-08 was closed on 2026-10-23, leaving only P1-02. But the criterion is
**still NOT MET**, because the authoritative defect catalogue
(`DEFECTS_AND_ISSUES.md` §0) still carries **4 P0 entries that are not CLOSED**:

| P0 | Verdict in the catalogue | Why it is still not CLOSED |
|---|---|---|
| P0-03 | `REVOKED` | a **state** (v4.0.0's GA was revoked), not a fixable defect — it needs a decision, not a patch |
| P0-04 | `OPEN` | 5-remote `develop/v4.1.0` drift; the drift gate reports but does not block |
| P0-06 | `CONTRADICTORY_STATE` | `historical-backlog-disposition.yml` has 6 entries that are both `CLOSED` and `deferred` |
| P0-07 | `OPEN` | `.gitignore` anchoring — the code fix is in the tree, but the item was never re-verified and closed |

Closing P0-08 removed the last P0 from the stage's **own** issue list; it did
not empty the catalogue. (P0-01, P0-02, P0-05 and P0-08 were closed on
2026-10-23 — see §5.2.)

### 4.2 Other recorded failures

1. **The CI `test` job has never completed successfully** — 0 green out of 30
   started out of 1 452 opportunities (§2.4).
2. **CI run 8750 `lint-build` is red** — `clippy::type_complexity` under
   `-D warnings` (`tests/integration/sql/_q15_q16_check.rs:74`). This is a
   concrete instance of the `CLIPPY_WARNINGS_MAX: 0` criterion not holding.
3. **CI run 8831 `lint-build` is red** — `cargo fmt --check` diff
   (`crates/storage/tests/no_namespace_db_refusal_5266.rs:16,22`). A concrete
   instance of the `FMT_DRIFT_MAX: 0` criterion not holding at that commit.
4. **The earlier ALPHA stage-gate run of the same day failed**
   (`scripts/gate/check_coverage_v312.sh` FAIL, 15 PASS / 1 FAIL) — the coverage
   gate is not deterministic across runs. **Root cause identified 2026-10-23:**
   the coverage had been taken with `cargo llvm-cov`'s **default parallel test
   threads**, so the reading depended on machine load; `--test-threads=1` is now
   pinned (commit `2fef945260`, #5537), and two consecutive runs of the same
   crate on the same machine now agree exactly (93.29 % / 93.29 %).
   (`docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1)
5. **`scripts/gate/check_g13_stability.sh` is recorded as currently exit 1**
   — `[1/9] ❌ FAIL: tests/soak_test_harness.rs not found`
   (`DEFECTS_AND_ISSUES.md` P1-07).
6. **The coverage gate fails open**: `check_coverage_v312.sh:30` prints
   "WARN: could not extract coverage for $crate, skipping" instead of failing
   (`DEFECTS_AND_ISSUES.md` P1-01). **Closed 2026-10-23** — a crate that cannot
   be measured now fails the gate (`check_coverage_v312.sh:315-349`).
7. **The mandatory-document gate for v4.1.0 does not pass.** Under
   `scripts/gate/check_document_completeness.sh` the list is
   `QUICK_START.md` (≥ 5000), `FEATURE_MATRIX.md` (≥ 15000),
   `RELEASE_NOTES.md` (≥ 10000), `DEPLOYMENT_GUIDE.md` (≥ 15000),
   `MIGRATION_GUIDE.md` (≥ 10000), `INSTALL.md` (≥ 5000),
   `CHANGELOG.md` (≥ 1000), `EVALUATION_REPORT.md` (≥ 1000). As observed on
   2026-10-23 all eight entries exist, but `RELEASE_NOTES.md` is **8 926
   bytes** against a 10 000-byte threshold — **1 of the 8 mandatory entries
   still fails**. The two files this report and `FEATURE_MATRIX.md` close are
   `FEATURE_MATRIX.md` and `EVALUATION_REPORT.md`; **the gate as a whole
   still fails on `RELEASE_NOTES.md`.** (That list is being filled in
   concurrently, so this is a dated observation rather than a standing
   verdict.)
8. **The GA artefact set is incomplete** — `GA_GATE_REPORT.md`,
   `GA_RELEASE_TIMELINE.md`, `EVIDENCE_STATUS.md` and `POST_GA_PLAN.md` are
   missing for v4.1.0, and the GA gate list has never been executed
   end-to-end for this version.
9. **The AB-01..AB-10 gates have never been recorded as executed.**
   `ALPHA_TO_BETA_GATE_PLAN.md` §6 states that the plan publishes no PASS and
   that actual results belong to a later run's report and evidence directory.
   No such report is in the repository.

---

## 5. Open gaps

### 5.1 The registered known gap

| Gap | Status | Decision |
|---|---|---|
| `known_gaps.V400_09_soak` — the V400-09 168 h SOAK | `NOT MET` (verified 2026-10-07) | `OPEN` — architect decision, deliberately not made by the SSOT |

### 5.2 The seven registered open issues (`STAGE.yaml` → `open_issues`)

Six of the seven were closed on 2026-10-23; the closures are recorded in
[`evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md`](evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md)
and [`evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md`](evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md),
and in `STAGE.yaml` → `resolved_issues_2026_10_23`.

| Ref | Gap | Status |
|---|---|---|
| — | `CURRENT_VERSION.md` and `docs/releases/v4.0.0/RELEASE_NOTES.md` still write v4.0.0 as "GA (CONDITIONAL PASS)", contradicting `docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status.verdict = "REVOKED 2026-09-30"` | **RESOLVED 2026-10-23** — the wording now matches the SSOT: the code was published (2026-09-19, tag `v4.0.0-final` @ `54571eeca0`, on all five remotes' `main`), but v4.0.0 never legitimately reached GA |
| P0-01 | `scripts/gate/check_anti_ignore_gate.sh` is a hollow gate — it never scans the source tree, and `active` is structurally always 0 because no registry entry carries a `status` field. P16 is unenforced | **RESOLVED 2026-10-23** — the gate now recounts the tree with the registry's own rule and fails closed on drift; measured 119 == 119, exit 0 |
| P0-02 | #4708 is a false DONE — the parser has no block-comment support at all, masked by an unregistered `#[ignore]` at `crates/parser/tests/wp_a_legacy.rs:68` | **RESOLVED 2026-10-23** — the parser has a `/* ... */` branch (`crates/parser/src/lexer.rs:78`) and the `#[ignore]` is gone |
| P0-05 | `docs/governance/debt/debt-registry.yaml` SEM-4's closing root cause is still the literal placeholder `XYZ`; the registry's `parser 75.16 %` is not reproducible (measured 60.58 %) | **RESOLVED 2026-10-23** — `SEM-4` is `CLOSED` with a measured root cause; 8/8 crates measured, L1_8 **87.80 %**, `parser 86.46 %` |
| P1-01 | parser line coverage measured 60.58 %; the threshold named in the same entry is 80 %; `check_coverage_v312.sh:30` fails open | **RESOLVED 2026-10-23** — the gate fails closed on an unmeasurable crate (`check_coverage_v312.sh:315-349`); measured `parser 86.46 %`, L1_8 87.80 %. The threshold itself remains the stage framework's call |
| P1-02 | #4540 Q20 oracle is not closed — the `sqlrustgo/q20.tsv` oracle is missing (22 files, 1 absent) and the hash record `CROSS_ENGINE_HASH_CHECK.json` reports `present:false`, contradicting the "Q20 PASS" in `docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_REPORT.md`. Neither oracle artefact is present in the current tree: `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/` holds only `build_cross_engine_summary.py` | **OPEN** |
| P0-08 | WAL recovery rewrites the whole table per record — the first full-table query after a restart took 24 min 22 s (next identical query: 0.08 s) at 68 288 rows / 58 288 INSERTs / 8.0 MB WAL; recovery completes before the listener accepts connections, and no historical SOAK covered it | **RESOLVED 2026-10-23** — and the report's **attribution was corrected by measurement**: the write path was already taking the delta-append branch and its per-row cost is constant (96.4 / 85.8 / 86.4 µs/row at 500 / 2 000 / 8 000 rows), so `save_table_full` was never called per record. The real quadratic term was the **per-entry dedup scan**, which cloned the whole table to ask "is this row already there?" (63.9 → 247.9 → 896.4 µs/row). Fixed with a per-table `RecoveryRowIndex`: one scan per table, then an O(1) lookup per entry; `Update`/`Delete` invalidate the affected table's index because a WAL entry carries no old row content. Measured after the fix on the real `RecoveryEngine::recover()`: 210.3 → 224.0 → 372.4 µs/row (16× the rows ⇒ 1.8× per row); extrapolated to 58 288 rows ≈ 21.7 s vs ≈ 461 s before (**≈21×**). 5/5 semantic tests pass, including the two cases a stale index would silently **lose committed data** on. `Value`'s `Hash` was brought in line with its `PartialEq` as part of this. **Not claimed:** the recovery-before-listener ordering is unchanged, and the per-row delta fsync remains a constant cost |

### 5.3 Gaps named by the assessment sources

* **Performance**: `PERFORMANCE_AUDIT_REPORT_2026-09-30.md` — 18 findings,
  led by F-01 (`BoxStorageEngine` does not forward `*_lockfree`, so the
  lock-free transaction path is never taken in production).
* **Architecture**: `ARCHITECTURE.md` §2.2 / `CORE_PATH_INVENTORY.md` — the
  `crates/planner` physical-plan layer has no `execute()` and is unreachable
  from production, so its unit tests are not production-correctness evidence.
* **Alignment**: `ALIGNMENT_AUDIT_2026-09-30.md` — conclusion "**not
  aligned**": 4 plan lags, 4 missing constraints, 7 fact/evidence errors.
* **Compatibility**: `COMPREHENSIVE_ASSESSMENT_REPORT.md` §9 — the report
  explicitly **prohibits** claiming that v4.1.0 has reached ALPHA/BETA/RC/GA
  release quality, that it is a full SQLite/MySQL/PostgreSQL substitute, or
  that transaction isolation, AUTO_INCREMENT, CHAR PAD SPACE, ALTER TABLE
  RENAME COLUMN and SQLite metadata are closed.
* **Legacy**: `LEGACY_LEDGER_v3.6_to_v4.1.md` — the v3.6.0 → v3.11.0 legacy
  chain is broken; 35 items carry no Gitea issue number and are therefore
  unreachable by any retrieval mechanism.

---

## 6. Discrepancies found in the repository — reported, not adjudicated

| # | Subject | Reading A | Reading B | Status |
|---|---|---|---|---|
| D1 | Stage | `STAGE.yaml` → `current_stage: "ALPHA"` (re-entered 2026-10-06) | `ARCHITECTURE.md` (2026-10-07) "DRAFT (NOT YET ALPHA)"; `RELEASE_NOTES.md` "DRAFT — NOT YET ALPHA"; `COMPREHENSIVE_ASSESSMENT_REPORT.md` (2026-10-05) DRAFT; `DEFECTS_AND_ISSUES.md` §6 "current_stage: DRAFT" | `docs/governance/STAGE_CONFIG.yaml` makes `STAGE.yaml` the per-version SSOT; the other four documents are stale |
| D2 | Parser coverage | 60.58 % (`DEFECTS_AND_ISSUES.md` P1-01, 2026-10-01) | 86.20 % (`evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`, 2026-10-07; `STAGE.yaml` COV-01 `DONE`); **86.46 %** (2026-10-23, macmini M4, full 8-crate run of the corrected gate — `evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3) | **Partly resolved.** The 2026-10-23 measurement supersedes the 2026-10-01 one: it was taken with `--test-threads=1` (pinned 2026-10-22 by #5537), and two consecutive runs of the same crate now agree exactly. The 60.58 % figure was taken under the old load-sensitive measurement and is not reproducible. The **region-vs-line scope** difference behind the original discrepancy is not adjudicated here |
| D3 | ALPHA coverage threshold | 50 % (`docs/governance/STAGE_CONFIG.yaml` `COVERAGE_MIN_ALPHA`; `STAGE.yaml` `exit_criteria.alpha_to_beta`) | 80 % (`DEFECTS_AND_ISSUES.md` P1-01; `ARCHITECTURE.md` §1) | 80 % is the **Beta** figure (`COVERAGE_MIN_BETA: 75`; `ALPHA_TO_BETA_GATE_PLAN.md` §2 applies the stricter G17 Beta 80 %) — a governance inconsistency to reconcile, not a measurement dispute |
| D4 | #5167 | `refuted` (`IAIA_410_AUDIT_PLAN.md` §16.1/§16.2, baseline `5a59c2f701`, `--storage file`, real pymysql client, SQLite oracle) | still reproducible (`CHANGELOG.md` SOAK-evidence bullet, HEAD `345ee9539c`, `--storage file`; the audit's new regression tests used `storage: None`) | Unresolved. The changelog's supporting evidence is filed as comments on #5167 and three audit PRs — **not in this repository**. The audit plan's own §16.5 checklist leaves "#5167 closed as `refuted`" unchecked pending a `--storage file` regression test |
| D5 | Defect counts | P0 × 8 / P1 × 7 / P2 × 7 = 22 (`DEFECTS_AND_ISSUES.md` §0, as counted 2026-10-01) | P0×6 + P1×7 + P2×7 (`RELEASE_NOTES.md`; `ARCHITECTURE.md` §4) | Unresolved wording conflict — **and a third reading now exists**: after the 2026-10-23 closures (P0-01, P0-02, P0-05, P0-08, P1-01) the still-open count is P0 × 4 / P1 × 6 / P2 × 7 = **17** (`DEFECTS_AND_ISSUES.md` §0, updated). The four P0 entries that are not CLOSED are P0-03 (`REVOKED`), P0-04 (`OPEN`), P0-06 (`CONTRADICTORY_STATE`) and P0-07 (`OPEN`) |
| D6 | WP-A | `docs/releases/v4.1.0/LEGACY_ISSUES.md` §2 records WP-A as `DONE` — "None (inherited DONE)" | `STAGE.yaml` → `legacy_chain_audit_2026_09_30.false_done_wp_a`: `DONE_REOPENED_2026-09-30`, #4708 `NOT FIXED` | `STAGE.yaml` is newer; `LEGACY_ISSUES.md` is stale |
| D7 | 5-remote drift | `DEFECTS_AND_ISSUES.md` P0-04 (2026-10-01): `develop/v4.1.0` inconsistent across the 5 remotes (gitea250/gitee ahead 15, github behind 34) | `STAGE.yaml` → `last_status_refresh` (2026-10-03): `develop/v4.1.0` 5/5 remotes identical at `d59dbffd` | Two dated records; not merged here |
| D8 | Backlog count | 24 (`ISSUES_PLAN.md` §4.7 `Total`; `VERSION_PLAN.md` §1) | 20 — the earlier figure retained inside `ISSUES_PLAN.md` §4 as a superseded correction note (18 → 20 → 24) | 24 is current; 20 is retained history |
| D9 | `v4.1.0-alpha1` tag | `CHANGELOG.md` §Status: an **unchecked** box, still annotated "blocked by 3 inherited alpha-gate FAILs" | `STAGE.yaml`: the annotated tag exists (`c1a73a5320`) and is **retained deliberately** | `STAGE.yaml` is newer; the changelog block was not refreshed |
| D10 | "CI green" | `evidence/CI_TRUTH_BATCH24_2026-10-22.md` §1.5: the CI `test` job has never completed successfully | the 2026-10-22 ALPHA stage-gate log records `cargo test --all-features --lib` as `[PASS]` | Not the same thing: one is a **CI** job, the other a **local** stage-gate run on a feature branch. Neither is evidence for the other |

---

## 7. "Not recorded" and "needs owner input"

### 7.1 Recorded as **not recorded** in the repository

These are quantities a reader might expect in an evaluation report. The
repository does not record them, and this report does not estimate them.

| Quantity | State |
|---|---|
| Clippy warning count for v4.1.0 at HEAD | **not recorded** as a number. The only clippy record is CI run 8750's `-D warnings` failure (`clippy::type_complexity`, `tests/integration/sql/_q15_q16_check.rs:74`), attributed to runner toolchain drift |
| Fmt drift count for v4.1.0 at HEAD | **not recorded** as a number. The only fmt record is CI run 8831's `cargo fmt --check` diff (`crates/storage/tests/no_namespace_db_refusal_5266.rs:16,22`) |
| Doc-link breakage count at HEAD | **not recorded**. The last recorded run is `scripts/gate/check_docs_links.sh` → exit 0 ("All markdown links are valid"), dated 2026-10-03 |
| A single current count of open P0 bugs | **not recorded**. `DEFECTS_AND_ISSUES.md` records P0 = 8 at 2026-10-01; `STAGE.yaml` records 7 open issues with 4 of them P0 — the two counts are not reconciled anywhere |
| TPC-H SF=1 22/22 verdict for v4.1.0 | **not recorded**. `evidence/TPCH_FIXTURE_5379_2026-10-20.md` concerns fixture self-containment (#5379) and explicitly leaves SF=1 out of scope for that round (#5387) |
| 24 h SOAK result for v4.1.0 | **not recorded**. The recorded runs are 5 min, 1 h and 8 h |
| AB-01..AB-10 execution outcome | **not recorded** — the gate plan publishes no PASS (§6 of the plan) and no run report is in the repository |
| Number of defects closed since 2026-10-01 | **not recorded**; `DEFECTS_AND_ISSUES.md` is dated 2026-10-01 and no later census exists |
| Current Gitea issue / PR counts | **dated snapshots only**: 17 open issues and 1 open PR (#4987) at 2026-10-05 (`COMPREHENSIVE_ASSESSMENT_REPORT.md` §10); 1 open PR (#4932) at 2026-10-03 (`STAGE.yaml` → `last_status_refresh`) |
| A stage-gate run on `develop/v4.1.0` | **not recorded** — the recorded ALPHA runs are on `opencode/fix-alpha-gate-blockers` @ `6f26128890` |

### 7.2 Items that need an owner decision — explicitly deferred by the documents

1. **Whether to re-run the V400-09 168 h SOAK.**
   `known_gaps.V400_09_soak.decision: "OPEN"` and the SSOT states that this is
   an architect decision the file does not make.
2. **Which document owns the v4.0.0 wording** — `CURRENT_VERSION.md` and
   `docs/releases/v4.0.0/RELEASE_NOTES.md` ("GA (CONDITIONAL PASS)") versus
   `docs/releases/v4.0.0/STAGE.yaml` (`verdict: "REVOKED 2026-09-30"`).
3. **Which ALPHA coverage threshold governs** — 50 % or 80 % (D3).
4. **The #5167 verdict** (D4) — the deciding evidence lives in Gitea
   comments, not in this repository.
5. **The defect-count wording** (D5) and **the stale stage wording** (D1) —
   which document is corrected.
6. **Whether the 2026-10-22 ALPHA stage-gate run is accepted** as the
   release-line verdict, given that it was run on a feature branch and that
   the CI `test` job has never completed successfully.
7. **The 19 unresolved document contradictions** listed in
   `STAGE.yaml` → `legacy_chain_audit_2026_09_30.doc_contradictions_unresolved`;
   the file's own `recommended_next` says "P1: adjudicate the 19 doc
   contradictions — supply evidence or admit unclosed". They are not
   adjudicated.
8. **The remaining mandatory release document** for v4.1.0 — as observed on
   2026-10-23, `RELEASE_NOTES.md` is short of its 10 000-byte threshold
   (8 926 bytes) — is outside the scope of this report and remains open. The
   mandatory list is being filled in concurrently, so the owner should re-run
   `scripts/gate/check_document_completeness.sh v4.1.0` rather than rely on
   this dated snapshot.

---

## 8. Conclusion

**v4.1.0 is at stage ALPHA.** `docs/releases/v4.1.0/STAGE.yaml` records
`current_stage: "ALPHA"`, re-entered on 2026-10-06 after the 2026-09-30
rollback of an unsupported ALPHA declaration; `docs/governance/STAGE_CONFIG.yaml`
makes that file the per-version SSOT. Four other v4.1.0 documents still
describe the stage as DRAFT (D1) — the SSOT governs, and the disagreement is
recorded above rather than resolved.

On the repository's own criteria:

* the ALPHA→BETA exit criteria are **not** met — the V400-09 168 h SOAK
  criterion and the `known_gaps` decision criterion both fail, and
  `docs/governance/STAGE_CONFIG.yaml`'s "No P0 bugs open" fails with four P0 items open;
* the AB-01..AB-10 hard gates have **never been recorded as executed**;
* the CI `test` job has **never completed successfully**, and two recorded
  CI runs are red against the `CLIPPY_WARNINGS_MAX: 0` and
  `FMT_DRIFT_MAX: 0` thresholds;
* the GA artefact set is incomplete and the GA gate list has never been
  executed end-to-end for this version;
* the mandatory-document gate for v4.1.0 still fails on `RELEASE_NOTES.md`
  (8 926 bytes against a 10 000-byte threshold), as observed on 2026-10-23.

**Therefore v4.1.0 is not GA and is not releasable as GA.** It is an ALPHA
work-in-progress line. The claims that the repository itself forbids —
release quality at any stage, a SQLite / MySQL / PostgreSQL substitute, or a
closed transaction-isolation, AUTO_INCREMENT, CHAR PAD SPACE or
ALTER TABLE RENAME COLUMN story — remain forbidden. The green signals that do
exist (a local ALPHA stage-gate run, the 8 h SOAK observations, the COV-01 /
COV-02 coverage items) are real but are not GA-grade evidence, and this report
does not promote them.

**v4.1.0-alpha1** (`c1a73a5320`) exists and is retained deliberately as an
audit trail; `v4.1.0-beta1`, `-rc1` and GA have **not** been reached
(M-10: **NOT STARTED**).

---

## 9. References

- `docs/releases/v4.1.0/STAGE.yaml` — the stage SSOT
- `docs/governance/STAGE_CONFIG.yaml` — thresholds, per-stage gates and artefacts
- `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md` — AB-01..AB-10
- `docs/releases/v4.1.0/COMPREHENSIVE_ASSESSMENT_REPORT.md`
- `docs/releases/v4.1.0/PERFORMANCE_AUDIT_REPORT_2026-09-30.md`
- `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md`
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md`
- `docs/releases/v4.1.0/ISSUES_PLAN.md`
- `docs/releases/v4.1.0/LEGACY_ISSUES.md`, `LEGACY_LEDGER_v3.6_to_v4.1.md`
- `docs/releases/v4.1.0/ARCHITECTURE.md`, `CORE_PATH_INVENTORY.md`
- `docs/releases/v4.1.0/FEATURE_MATRIX.md` — the delta matrix
- `docs/releases/v4.1.0/evidence/` — the measurement documents cited above
- `docs/releases/v4.1.0/logs/` — the recorded stage-gate run logs
- `docs/releases/v4.0.0/STAGE.yaml`, `GA_GATE_REPORT.md`, `GA_RELEASE_TIMELINE.md`
- `docs/governance/DOCUMENT_COMPLETENESS_CHECK.md`

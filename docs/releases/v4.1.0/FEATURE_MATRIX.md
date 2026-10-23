# v4.1.0 — FEATURE_MATRIX

> **Date**: 2026-10-23
> **Baseline**: `develop/v4.1.0` @ `3a3350f660` — the tree this matrix
> describes. Every commit, PR and status below was read out of it.
> **Stage**: ALPHA (`docs/releases/v4.1.0/STAGE.yaml` → `current_stage: "ALPHA"`,
> re-entered 2026-10-06 — see `last_transition`)
> **Status**: **WORKING DRAFT** — this is the ALPHA-stage artefact. At BETA it
> must be re-derived and *frozen* (the BETA stage's `doc_artifacts` list names
> the same file; the freeze is a separate, deliberate act, not this document).
>
> This file is required by `docs/governance/STAGE_CONFIG.yaml` →
> `stages.ALPHA.doc_artifacts`. It was **missing** until 2026-10-21 (#5439:
> a required artefact that nobody checked for is a requirement nobody has to
> keep). `scripts/gate/check_release_artifacts.sh` now checks it.

## 0. What this file is, and what it is not

This file is a **matrix of the deltas v4.1.0 carries over v4.0.0**, with the
status of each delta and a pointer to the evidence for that status.

It is **not** a claim that v4.1.0 is releasable. It does not assert that the
gates pass, that the version is GA, or that the remaining defects are closed.
Section 3 lists what this file deliberately does **not** claim.

**Every status in section 1 is copied from a source document, not asserted
here.** The source column names that document. Where two sources disagree,
both are shown (see M-6..M-8 in section 2) — this file does not resolve the
disagreement by preference.

**How to re-derive this file**: every row in section 1 is either a row of
`VERSION_PLAN.md` §2 (deltas over v4.0.0), a bullet of `CHANGELOG.md`
§"[Unreleased]" (the P0 fix round of 2026-10-08 and the carry-forward
subsections), a §1 "Pending" item of `VERSION_PLAN.md`, a finding of
`IAIA_410_AUDIT_PLAN.md`, or a registered item of `STAGE.yaml` →
`open_issues`. Section 2 is `ROADMAP.md` §1 verbatim in status. Section 3 is
`STAGE.yaml` → `known_gaps` + `open_issues`, plus the artefact gaps that
`scripts/gate/check_release_artifacts.sh` reports, plus the document contradictions this
file leaves visible rather than smooths over.

---

## 1. Delta matrix (v4.1.0 over v4.0.0)

### 1.1 Carried-forward deltas — source: `VERSION_PLAN.md` §2

| ID | Area | Delta | Status | Evidence |
|---|---|---|---|---|
| D-01 | zombie-fix core | Cherry-pick `67b624cbd2` + merge `9c6767a512`; `gap_lock_manager` + `read_write_splitter` updated | DONE | `VERSION_PLAN.md` §2; `STAGE.yaml` `last_transition` (WP-C/D/F/G merged via #5041/#5043/#5045/#5047) |
| D-02 | `workers.push` | Commit `b4d46e6e18` + PR #4906 + PR #3793 merges | DONE | `VERSION_PLAN.md` §2 |
| D-03 | DML/storage regressions | `f3595e7361` + PR #4905 + PR #3792 merges | DONE | `VERSION_PLAN.md` §2 |
| D-04 | Review queue closure | 3 cherry-picks + zombie merge + `0bbb044da3` (CLOSED status doc) | DONE | `VERSION_PLAN.md` §2 |
| D-05 | 5-remote sync tooling | `scripts/sync/5remotes_sync.sh`, `scripts/sync/5remotes_drift_check.sh`, `scripts/sync/README.md` | DONE | `VERSION_PLAN.md` §2; `ROADMAP.md` M-4 |

### 1.2 P0 fix round — source: `CHANGELOG.md` §"[Unreleased] / P0 修复 — 2026-10-08"

| ID | Item | Claim in the changelog | Status | Evidence |
|---|---|---|---|---|
| F-01 | #5099 concurrent-transaction row loss | PR #5132 moved tx identity off a shared slot; loss rate 40.4% → ~1%; **`ERROR 1062` and the intermittent server hang remain unresolved** | **PARTIAL** | `docs/releases/v4.1.0/evidence/CONCURRENT_TX_ROLLBACK_DATA_LOSS_5099_2026-10-08.md` |
| F-02 | #5099 server hang (TLS read spin) | `read_exact` busy-looped on a half-read header; 100% CPU per worker | **FIXED** | `docs/releases/v4.1.0/evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md` (1 h SOAK 3602 s, 0 FATAL) |
| F-03 | #5168 PK point lookup = full scan | `WalStorage`/`ParallelWalStorage` did not forward `scan_pk` | **FIXED** | `crates/storage/tests/wal_scan_pk_forwarding_5168.rs`; changelog table 9.02 ms → 0.467 ms |
| F-04 | #5177 UPDATE/DELETE `WHERE` = full scan | Added `pk_point_lookup`; UPDATE 10.941 ms → 1.051 ms @20000 rows | **FIXED** | `tests/integration/dml/dml_pk_fast_path_5177.rs` (9 tests) |
| F-05 | #5181 delete path re-allocates the whole table | `Vec::retain` removes the second allocation; **the scan itself is still O(rows)** | **PARTIAL** | `crates/storage/tests/write_path_scaling_5181.rs` (5 tests) |
| F-06 | #5167 `COUNT(*)` / range scan / aggregates return 0 or NULL | Point lookups were unaffected | **FIXED** (issue closed) | Gitea #5167; `docs/releases/v4.1.0/evidence/TLS_READ_SPIN_HANG_5099_2026-10-09.md` §6 |
| F-07 | 8 h SOAK | 28800.29 s, 0 FATAL / 0 errors / 0 reconnects, 352 402 transactions, 7 048 040 queries; no degradation over 8 h | **RUN — but the report's evidential weight is downgraded** by the IAIA-410 audit | `docs/releases/v4.1.0/evidence/SOAK_8H_AND_CAPACITY_2026-10-09.md` |

> **F-07 caveat, carried verbatim from the changelog**: the 8 h SOAK
> conclusion "cannot serve as evidence that v4.1.0 is usable" — the
> IAIA-410 truthfulness audit found defects the SOAK structurally cannot
> cover (e.g. #5192, durability illusion after `kill -9`). A green SOAK is
> therefore **not** a GA-grade claim.

### 1.3 IAIA-410 findings and their remediation — source: `IAIA_410_AUDIT_PLAN.md` §13–§16 + `CHANGELOG.md` §"[Unreleased] / P0 修复 — 2026-10-08"

The changelog bullet "⚠️ SOAK 报告证据地位更新（2026-10-09，IAIA-410 之后）" and the
IAIA-410 audit plan name defects that section 1.2 above does not carry as rows
of their own. They are listed here with the status each source records.

| ID | Item | Claim in the source | Status | Evidence |
|---|---|---|---|---|
| F-08 | #5192 — F9 durability illusion (GA-blocking) | After `INSERT; COMMIT`, a `kill -9` + restart **loses the committed data permanently**; the service only WARNs and keeps accepting writes. Root cause named in the changelog: the WAL entry carries no database identifier (`engine.rs:1423-1441`) | **FIXED** per `IAIA_410_AUDIT_PLAN.md` §14.1/§14.4 (PR #5208 merged; verified `rows_inserted=5 skipped=0`, 3 rows recovered) — **but** `IAIA_410_SSOT.md` §10.3 explicitly does *not* assert that any listed issue is fixed, verified or closable, and the changelog records #5192 as a GA-blocking defect and carries no fix record for it | `IAIA_410_AUDIT_PLAN.md` §14.1, §14.4; `IAIA_410_SSOT.md` §10.3; `CHANGELOG.md` (SOAK-evidence bullet) |
| F-09 | #5191 — F5 silent semantic degradation | The database selected during the handshake is not propagated to `engine.session_db`; queries against a non-default database **silently return 0 columns / 0 rows** | **FIXED** per `IAIA_410_AUDIT_PLAN.md` §14.1/§14.2 (PR #5195) — **with the same section's §14.3 recording that `cargo test -p sqlrustgo --lib` did not finish, so the new tests' assertions are still unverified** (end-to-end verification with a real pymysql client is recorded as complete) | `IAIA_410_AUDIT_PLAN.md` §14.1, §14.2, §14.3; `IAIA_410_SSOT.md` §10.3 |
| F-10 | #5188 — O(log N) delete evaluation | `WriteState` cannot reach the primary-key B+Tree, so a real O(log N) delete needs a structural change. The evaluation was opened with two candidates (position resolution moved earlier / a row-slot indirection layer); the "locate by index then `Vec::remove`" option is **recorded as rejected** — it breaks ROLLBACK of multi-row deletes | **OPEN — evaluation only, no fix recorded** | `CHANGELOG.md` (#5181 bullet); `crates/storage/tests/write_path_scaling_5181.rs` |
| F-11 | #5167 — `refuted` candidate, then disputed | `IAIA_410_AUDIT_PLAN.md` §16.1/§16.2 judged #5167 **`refuted`** at baseline `5a59c2f701` (10 000 rows, `--storage file`, real pymysql client, SQLite as an independent oracle: `COUNT(*)` → 10 000, `SUM(k)` → 500 050 000, 13/13 point hits). The changelog bullet of the same date records the opposite: at HEAD `345ee9539c` under `--storage file` (the `serve` default) the defect **still reproduces**, because the audit's newly added regression tests use `storage: None` (in-memory), i.e. a non-production configuration | **DISPUTED — both readings are shown; this file does not pick one.** The changelog's supporting evidence is filed as comments on #5167 and three audit PRs, which are **not recorded in this repository**, so the repository alone cannot adjudicate. The audit plan's own §16.5 checklist leaves "#5167 closed as `refuted`" **unchecked**, pending a `--storage file` regression test | `IAIA_410_AUDIT_PLAN.md` §16.1, §16.2, §16.5; `CHANGELOG.md` (SOAK-evidence bullet) |
| F-12 | #5193 — F3 planner/executor not wired into production | `crates/planner`'s `PhysicalPlan` trait has **no `execute()` method**; `SeqScanExec::execute` is hard-coded `Ok(vec![])`; `IndexScanExec` has no `execute`; `DefaultPlanner::new()`'s only external construction sites are inside `crates/planner/tests/`; `mysql-server` never imports planner — so unit tests in that crate exercise a path production cannot reach | **CLOSED** per `IAIA_410_AUDIT_PLAN.md` §14.1 (option C adopted, PR #5207); `ARCHITECTURE.md` §2.2 records the same decision and names `ARCHITECTURE_ROUTE_DECISION_5193.md` | `IAIA_410_AUDIT_PLAN.md` §14.1; `ARCHITECTURE.md` §2.2; `ARCHITECTURE_ROUTE_DECISION_5193.md`; `CORE_PATH_INVENTORY.md` |

> **F-11 is a section-number pointer that does not resolve.** The changelog
> cites "IAIA-410 计划 §6.1" for the `refuted` candidacy of #5167; in the
> current `IAIA_410_AUDIT_PLAN.md`, §6.1 is "执行纪律" and #5167 is discussed
> in §7.1 and §16. The pointer is recorded as-is and not repaired here.

### 1.4 Changelog carry-forward rows — source: `CHANGELOG.md` §"[Unreleased]" (non-P0 subsections)

The changelog's `[Unreleased]` section contains five further subsections whose
entries are deltas v4.1.0 carries over v4.0.0 and are not represented by
sections 1.1–1.3.

| ID | Item | Claim in the changelog | Status | Evidence |
|---|---|---|---|---|
| C-01 | Phase 0 — DRAFT doc scaffolding (2026-09-29) | 8 documents created: `STAGE.yaml`, `VERSION_PLAN.md`, `DEV_PLAN.md`, `ROADMAP.md`, `TEST_PLAN.md`, `ISSUES_PLAN.md`, `LEGACY_ISSUES.md`, `README.md` | **DONE** (the changelog lists the files as created) | `CHANGELOG.md` §"Phase 0 (DRAFT doc scaffolding) — 2026-09-29" |
| C-02 | Empty-merge graph repair (2026-09-22) | 4 commits bring 3 doc-only v4.0.0 commits into the v4.1.0 graph (`fca71cb525`, `a65497e276`, `7e30fbc0cb`) plus the review-queue doc (`2bd69b223f`) | **DONE** (4 commits listed) | `CHANGELOG.md` §"Empty-merge graph repair (2026-09-22)"; `V400_TO_V410_REVIEW_QUEUE.md` |
| C-03 | Sync audit + queue closure (2026-09-23..2026-09-26) | `136f232643` (5-remote convergence report), `9c6767a512` (merge), `0bbb044da3` (review queue closed — all 4 commits resolved) | **DONE** (`0bbb044da3` is D-04's "CLOSED status doc") | `CHANGELOG.md` §"Sync audit + queue closure (2026-09-23..2026-09-26)" |
| C-04 | v4.0.0 GA gate — parent commit (2026-09-20) | 4 commits: `2e1f9bd44d` (GA_GATE_REPORT.md FINAL — promotes v4.0.0 to GA), `cceba7f330` (TAG_PROTECTION), `3005413953` (FORCE_PUSH_AUDIT), `38566af0f8` (SYNC_AUDIT final convergence) | **COMMITS LANDED — but the GA verdict they recorded was REVOKED 2026-09-30**; `docs/releases/v4.0.0/STAGE.yaml` now carries `ga_promotion_status.verdict: REVOKED` | `CHANGELOG.md` §"v4.0.0 GA gate (parent commit) — 2026-09-20"; `docs/releases/v4.0.0/STAGE.yaml`; `docs/releases/v4.0.0/GA_RELEASE_TIMELINE.md` |
| C-05 | Release-tag status | The changelog's "Status" block carries four **unchecked** boxes (alpha1 / beta1 / rc1 / ga), with alpha1 still annotated "blocked by 3 inherited alpha-gate FAILs" | **DISPUTED — the changelog block is unchecked; `STAGE.yaml` records the annotated tag `v4.1.0-alpha1` (`c1a73a5320`) as existing and **retained deliberately**, and beta1 / rc1 / ga as not reached (M-10)** | `CHANGELOG.md` §"Status"; `STAGE.yaml` `last_rollback_record`, `last_transition` |

### 1.5 Scoped-in ("Pending") items — source: `VERSION_PLAN.md` §1 "Pending"

`VERSION_PLAN.md` §1 lists what v4.1.0 takes on beyond §2's delta table. Two of
those entries were later revised by the SSOT; both readings are shown.

| ID | Item | Claim in the source | Status | Evidence |
|---|---|---|---|---|
| P-01 | WP-A parser legacy — #4708 / #4696 / #4710 / #4720 | §1: re-opened 2026-09-30, with `LEGACY_LEDGER` §2.1 / `ISSUES_PLAN` §4.0 as the basis | **RE-OPENED** — #4708 `NOT FIXED` (the parser has no block-comment support at all), #4696 / #4710 / #4720 `PARTIAL` (parser-level regression passes, E2E unverified) | `ISSUES_PLAN.md` §4.0; `STAGE.yaml` `legacy_chain_audit_2026_09_30.false_done_wp_a`; `docs/releases/v4.1.0/evidence/wp-a-reverify-2026-09-30/` |
| P-02 | WP-C/D/F/G legacy issues deferred from v4.0.0 | §1: "4 WP groups / 12 issues" | **SCOPED / MIGRATED to v4.1.0 — implementation NOT STARTED** (WP-C 6 / WP-D 4 / WP-F 1 / WP-G 1) | `ISSUES_PLAN.md` §4.1–§4.4; `STAGE.yaml` `phase_1_alpha_status.v410_09_inherit_v4000_deferred_legacy_issues` |
| P-03 | Total v4.1.0 backlog | §1: "total v4.1.0 backlog = 24 issues incl. WP-A/WP-B/WP-E/WP-H" | **24** — `ISSUES_PLAN.md` §4.7 `Total`; the earlier "20" figure is retained inside that document as a superseded correction note, and §4 states the count was raised 18 → 20 → 24 by successive corrections | `ISSUES_PLAN.md` §4 and §4.7 (both correction notes) |
| P-04 | V400-09 168 h SOAK FINAL_REPORT | §1: "~~V400-09 168h SOAK FINAL_REPORT~~ — **RESOLVED 2026-09-30**", on the grounds that `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` exists | **RETRACTED — `known_gaps.V400_09_soak.status: "NOT MET"`, `decision: "OPEN"`.** The basis was file existence; the file is dated 2026-09-30 and documents the *first* run's failure ("the 168h SOAK process never ran to completion"); the later run3/run4 left no artefacts at all | `VERSION_PLAN.md` §1; `STAGE.yaml` `known_gaps.V400_09_soak` and `resolved_issues_2026_10_03` (3rd entry) |
| P-05 | 3 inherited v4.0.0 alpha-gate FAILs | §1: "~~3 v4.0.0 alpha gate FAILs~~ — **CLEARED 2026-09-30** by real gate runs" | **CLEARED — 3 real gate runs archived** under `evidence/gate-runs-2026-09-30/`. **However** one of the three, `scripts/gate/check_anti_ignore_gate.sh`, is afterwards found to be a hollow gate that never scans the source tree (open_issues P0-01), so its recorded PASS does not mean what it appears to mean | `VERSION_PLAN.md` §1; `docs/releases/v4.1.0/evidence/gate-runs-2026-09-30/`; `STAGE.yaml` `open_issues` P0-01; `DEFECTS_AND_ISSUES.md` P0-01 |

### 1.6 Registered open issues in ALPHA scope — source: `STAGE.yaml` → `open_issues` (7 items)

Section 3 item 3 names these seven in prose. This table gives each one a row,
with the defect-catalog entry it corresponds to. The **claims** are copied from
the named source; the **statuses** are updated by the closures recorded in
`docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md`
— five of the seven were closed on 2026-10-23, two remain open.

| ID | Item | Claim in the source | Status | Evidence |
|---|---|---|---|---|
| O-01 | v4.0.0 "GA (CONDITIONAL PASS)" wording conflict | `CURRENT_VERSION.md` and `docs/releases/v4.0.0/RELEASE_NOTES.md` still write v4.0.0 as "GA (CONDITIONAL PASS)", contradicting `docs/releases/v4.0.0/STAGE.yaml` → `ga_promotion_status.verdict=REVOKED` | **RESOLVED 2026-10-23** — the wording was chosen: both files now say what the SSOT says. v4.0.0's **code** was published (2026-09-19, tag `v4.0.0-final` @ `54571eeca0` → `07178c9d66`, present on all five remotes' `main`), but the stage verdict is `REVOKED 2026-09-30` with `current_stage: "DRAFT"` ⇒ v4.0.0 never legitimately reached GA and is not a GA release. `CURRENT_VERSION.md` had 6 places carrying the old wording; all were realigned | `docs/releases/v4.0.0/STAGE.yaml:29-41`; `docs/releases/v4.0.0/GA_GATE_REPORT.md:1-25`; `docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §2; `STAGE.yaml` `resolved_issues_2026_10_23` (1st entry) |
| O-02 | P0-01 — `scripts/gate/check_anti_ignore_gate.sh` is a hollow gate | The script never scans the source tree; it compares the registry's self-declared numbers against hard-coded ceilings (`ACTIVE_MAX=47` / `TOTAL_ALLOWED_MAX=125`); `active` counts entries with `status == "ACTIVE"` although no registry entry carries that field, so `active` is structurally always 0 | **RESOLVED 2026-10-23** — the gate now recounts the tree with the same rule the registry uses (`count_tree()` at `scripts/gate/check_anti_ignore_gate.sh:123-159`) and fails closed when the two disagree. Measured: `total_allowed=119 (max=150), active=69 (max=80)` and `tree recount: real #[ignore] attributes in tests/ + crates/ = 119`, exit 0 ⇒ the tree is scanned and the registry matches it | `scripts/gate/check_anti_ignore_gate.sh:123-159`; `docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §7; `STAGE.yaml` `resolved_issues_2026_10_23` (2nd entry) |
| O-03 | P0-02 — #4708 is a false DONE | The parser has **no** block-comment support at all (`SELECT 1 /* comment */` → `Expected expression`); the gap is masked by an unregistered `#[ignore]` at `crates/parser/tests/wp_a_legacy.rs:68` | **RESOLVED 2026-10-23** — the parser has had a `/* ... */` branch since (it includes the EOF-unterminated case) at `crates/parser/src/lexer.rs:78`, and the `#[ignore]` at `crates/parser/tests/wp_a_legacy.rs:68` is gone (that line is now the comment recording its removal) ⇒ the support exists and is no longer masked | `crates/parser/src/lexer.rs:78-90`; `crates/parser/tests/wp_a_legacy.rs:68`; `docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §7; `STAGE.yaml` `resolved_issues_2026_10_23` (3rd entry) |
| O-04 | P0-05 — `docs/governance/debt/debt-registry.yaml` SEM-4 | The SEM-4 closing root cause is still the literal placeholder `XYZ` (`debt-registry.yaml:293`), `closing_pr` is `[]`, and the registry's `parser 75.16%` is not reproducible (measured 60.58 %) | **RESOLVED 2026-10-23** — SEM-4 is `IN_PROGRESS` → **CLOSED** (`closed_at: "2026-10-23"`, `closing_pr: [5126, 5537]`) and the placeholder root cause is replaced by a measurement: the coverage had been taken with `cargo llvm-cov`'s **default parallel test threads**, so the reading depended on machine load — pinned to `--test-threads=1` by commit `2fef945260` (#5537) on 2026-10-22, the measurement became deterministic. Measured: 8/8 crates, L1_8 **87.80 %** ≥ 80.0 %, `parser 86.46 %` (the 75.16 % and 60.58 % figures are not reproducible), and two consecutive runs of the same crate on the same machine agree exactly (93.29 % / 93.29 %). `check_arch_sem_debt.sh` moved from PASS-WITH-DRIFT to **PASS (all 7 CLOSED)** | `docs/governance/debt/debt-registry.yaml` (SEM-4); `docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1; `STAGE.yaml` `resolved_issues_2026_10_23` (5th entry) |
| O-05 | P1-01 — parser line coverage | Measured **60.58 %** (682 passed / 0 failed) on 2026-10-01; the ALPHA threshold named in the same entry is **80 %**; `check_coverage_v312.sh:30` fails open when it cannot extract a coverage number | **RESOLVED 2026-10-23** — the fail-open is gone: a crate that cannot be measured is recorded in `FAILED_CRATES` and the gate exits 1 (`scripts/gate/check_coverage_v312.sh:315-349`), with a separate count check that the sample is the full L1_8 set. The parser figure is **86.46 %**, not 60.58 % (measured 2026-10-23, macmini M4), and the L1_8 average is **87.80 %**. The *threshold* question is a separate matter and stays with the stage framework (`COVERAGE_MIN_ALPHA: 50`; `STAGE_CONFIG.yaml`) | `scripts/gate/check_coverage_v312.sh:315-349`; `docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md` §1.3; `STAGE.yaml` `resolved_issues_2026_10_23` (4th entry) |
| O-06 | P1-02 — #4540 Q20 oracle is not closed | The `sqlrustgo/q20.tsv` oracle is missing (22 files, 1 absent) and the hash record `CROSS_ENGINE_HASH_CHECK.json` reports `present:false`, which directly contradicts the "Q20 PASS" in `docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_REPORT.md`. Neither oracle artefact is present in the current tree: `docs/releases/v3.12.0/evidence/tpch/cross_engine_sf1/` holds only `build_cross_engine_summary.py` | **OPEN** | `DEFECTS_AND_ISSUES.md` P1-02; `docs/releases/v3.12.0/evidence/v312-59/GA5_TPCH_SF1_REPORT.md` |
| O-07 | P0-08 — WAL recovery rewrites the whole table per record | The first full-table query after a restart took **24 min 22 s** (the immediately following identical query: 0.08 s) at 68 288 rows / 58 288 INSERTs / 8.0 MB WAL; `save_table_full` is called per record and the delta append path is never reached; recovery completes **before** the listener accepts connections, so this is a hard startup outage, and no historical SOAK covered it | **RESOLVED 2026-10-23** — the report's **attribution was corrected by measurement**: the write path was already taking the delta-append branch (the base snapshot `t.json` stays at the 650-byte schema while `t.delta` carries every row) and its per-row cost is **constant** (96.4 / 85.8 / 86.4 µs/row at 500 / 2 000 / 8 000 rows) ⇒ `save_table_full` was never called per record. The real quadratic term is the **dedup scan**: every replayed `Insert` scanned and cloned the whole table to ask "is this row already there?", so the per-row cost grew with the table (63.9 → 247.9 → 896.4 µs/row). Fix: a per-table `RecoveryRowIndex` — one scan per table, then an O(1) hash lookup per entry; `Update`/`Delete` invalidate the affected table's index because a WAL entry carries no old row content. Measured after the fix on the real `RecoveryEngine::recover()`: 210.3 → 224.0 → 372.4 µs/row (500 / 2 000 / 8 000 rows) — 16× the rows costs only 1.8× per row; extrapolated to 58 288 rows ≈ 21.7 s vs ≈ 461 s before (**≈21×**). Semantics pinned by 5/5 PASS (`crates/storage/tests/wal_recovery_dedup_p0_08.rs`), including the two cases a stale index would silently **lose committed data** on. `Value`'s `Hash` was brought in line with its `PartialEq` as part of this (`0.0` vs `-0.0` previously hashed differently despite comparing equal). **Not claimed:** the recovery-before-listener ordering is unchanged, and the per-row delta fsync remains a constant cost | `crates/storage/src/recovery_engine.rs` (`RecoveryRowIndex`); `crates/storage/tests/wal_recovery_dedup_p0_08.rs`; `crates/types/src/value.rs`; `docs/releases/v4.1.0/evidence/WAL_RECOVERY_SCALE_P0_08_2026-10-23.md`; `docs/releases/v4.1.0/ISSUE-RECOVERY-QUADRATIC-REPLAY.md` §9; `STAGE.yaml` `resolved_issues_2026_10_23` (6th entry) |

### 1.7 Evidence pointers — areas this matrix names but does not detail

These are pointers, not claims: each row names an area the matrix already
refers to and the repository file that carries the detail. **No status is
asserted in this subsection.**

| ID | Area | What the pointer covers | Status | Evidence |
|---|---|---|---|---|
| EP-1 | Per-measurement evidence | One document per measurement (SOAK, coverage, perf A/B, rollback, TLS, concurrency, gate honesty, CI truth, evidence-binding provenance) | — | `docs/releases/v4.1.0/evidence/` |
| EP-2 | IAIA-410 audit | The audit protocol, the frozen-run contract, findings and their remediation tracking | — | `IAIA_410_SSOT.md` (protocol only), `IAIA_410_AUDIT_PLAN.md` (plan + execution record), `IAIA_410_FORMALIZATION_WORK_REPORT.md` |
| EP-3 | Alpha→Beta criteria | The AB-01..AB-10 hard gates and their fail conditions | — | `ALPHA_TO_BETA_GATE_PLAN.md`; work packages #5112..#5117 in `ISSUES_PLAN.md` §5 |
| EP-4 | Production-path inventory | Which files are on the production read/write path, and the traps in each | — | `ARCHITECTURE.md`, `ARCHITECTURE_ROUTE_DECISION_5193.md`, `CORE_PATH_INVENTORY.md`, `b2-off-production-path-registry.md` |
| EP-5 | Static performance audit | 18 findings in 4 levels (2 P0-CRITICAL / 4 P0-QUICKWIN / 7 P1-ALGORITHMIC / 5 P2-STRUCTURAL), measured against `develop/v4.1.0 @ 99198a5151`; a **static** audit — no benchmark or profiler was run | — | `PERFORMANCE_AUDIT_REPORT_2026-09-30.md`; `PERFORMANCE_OPTIMIZATION_PLAN.md`; `PERF_A1_4912_AB_MEASUREMENT.md`; `PERF_B2_4915_AB_MEASUREMENT.md`; `PERF_B22_CONCURRENT_MEASUREMENT.md` |
| EP-6 | v4.0.0 → v4.1.0 alignment audit | 12 findings F-01..F-12 (conclusion: **not aligned** — 4 plan lags, 4 missing constraints, 7 fact/evidence errors) | — | `ALIGNMENT_AUDIT_2026-09-30.md`; `MAIN_DIVERGENCE_RESOLUTION_2026-09-30.md` |
| EP-7 | Cross-version legacy ledger | The v3.6.0 → v4.1.0 legacy chain; 34 documents across 9 versions; document-layer findings only (no tests or gates were run) | — | `LEGACY_LEDGER_v3.6_to_v4.1.md`; `docs/releases/v4.1.0/evidence/legacy-item-reverify-2026-09-30/` |
| EP-8 | Defect catalogue | P0 × 8 / P1 × 7 / P2 × 7 = 22 items, each with verdict, evidence and reproduction | — | `DEFECTS_AND_ISSUES.md` |
| EP-9 | Gate run logs | The most recent recorded stage-gate runs for v4.1.0 | — | `docs/releases/v4.1.0/logs/` |
| EP-10 | Governance thresholds | Per-stage coverage / clippy / fmt / doc-link thresholds and the stage transition criteria | — | `docs/governance/STAGE_CONFIG.yaml`; `docs/governance/GATE_CONDITIONS.md`; `docs/governance/DOCUMENT_COMPLETENESS_CHECK.md` |

---

## 2. Milestone matrix — source: `ROADMAP.md` §1 (snapshot 2026-09-29)

| ID | Milestone | ROADMAP status | Newer SSOT position | Reconciled status |
|---|---|---|---|---|
| M-1 | PHASE_0 docs scaffolded | DONE | — | DONE |
| M-2 | Bugfix carry-forward merged | DONE | — | DONE |
| M-3 | Review queue closed | DONE | — | DONE |
| M-4 | 5-remote sync tooling | DONE | — | DONE |
| M-5 | v4.0.0 main + release branches synced to 5 ends | DONE | — | DONE |
| M-6 | 3 inherited v4.0.0 alpha-gate FAILs resolved | **NOT STARTED** | `STAGE.yaml` `last_transition` (2026-10-06) records all 3 cleared by real gate runs, with the run output archived under `evidence/gate-runs-2026-09-30/` | **CLEARED per STAGE.yaml** (ROADMAP snapshot is 7 days older) |
| M-7 | V400-09 168 h SOAK FINAL | **NOT STARTED** | `STAGE.yaml` `known_gaps.V400_09_soak` → `status: "NOT MET"`, `decision: "OPEN"` | **NOT MET — architect decision, deliberately left open** |
| M-8 | WP-C..G deferred items migrated to v4.1.0 scope | **NOT STARTED** | `STAGE.yaml` `last_transition`: merged via #5041/#5043/#5045/#5047 | **MERGED per STAGE.yaml** (ROADMAP snapshot is stale) |
| M-9 | v4.1.0-alpha1 tag cut | blocked by M-6/M-7 | tag `v4.1.0-alpha1` exists (`c1a73a5320`); `STAGE.yaml` records the rollback and the 2026-10-06 re-entry | **CUT** (the rollback is recorded in `STAGE.yaml`; the tag is retained deliberately) |
| M-10 | v4.1.0-beta1 / rc1 / ga | blocked by M-9 | not reached | **NOT STARTED** |

**M-6 / M-8 are recorded here with both readings on purpose.** The ROADMAP
snapshot (2026-09-29) and the STAGE.yaml transition record (2026-10-06) do
not agree; the newer SSOT entry is the one this file treats as current, and
the disagreement is left visible rather than smoothed over.

---

## 3. What this file does **not** claim

These are registered gaps, not omissions from this matrix.

1. **v4.1.0 is not GA, and is not close to GA.** The GA stage still has
   missing required artefacts — `GA_GATE_REPORT.md`,
   `GA_RELEASE_TIMELINE.md`, `EVIDENCE_STATUS.md`, `POST_GA_PLAN.md` — and
   the GA gate list has **never been executed end-to-end** for this
   version. `scripts/gate/check_release_artifacts.sh --version v4.1.0`
   prints the full gap.
2. **The ALPHA → BETA transition criterion is not met.** `V400-09` (168 h
   SOAK) is `status: "NOT MET"`, `decision: "OPEN"` — whether to re-run it
   is an architect decision, deliberately not made here.
3. **Seven open issues are registered in `STAGE.yaml` → `open_issues`** and
   are not resolved by anything in this matrix: the `v4.0.0` "GA (CONDITIONAL
   PASS)" wording conflict, `scripts/gate/check_anti_ignore_gate.sh` being a no-op gate
   (P0-01), the parser block-comment gap masked by an unregistered
   `#[ignore]` (P0-02), the literal `XYZ` placeholder in
   `docs/governance/debt/debt-registry.yaml` SEM-4 (P0-05), the parser coverage gap
   (60.58 % measured vs the 80 % ALPHA threshold — P1-01), the missing Q20
   oracle (P1-02), and the O(N²) WAL recovery rewrite (P0-08).
4. **No gate verdict is asserted.** Nothing in this file says any gate
   passes. The gate verdicts live in `docs/releases/v4.1.0/logs/` (run
   logs) and in the evidence directory, not here.
5. **No coverage, throughput or latency number is asserted** beyond the
   figures quoted in section 1, each of which points at the measurement
   document it came from.
6. **The parser coverage figure is not settled in the repository.** Two
   measurements exist — **60.58 %** (`DEFECTS_AND_ISSUES.md` P1-01,
   2026-10-01) and **86.20 %** (`evidence/COV_01_02_COVERAGE_GATE_2026-10-07.md`,
   2026-10-07, recorded as COV-01 `DONE` in `STAGE.yaml`). This file quotes
   both, with their dates, and calls neither "the" number.
7. **The ALPHA coverage threshold is not settled either.**
   `docs/governance/STAGE_CONFIG.yaml` sets `COVERAGE_MIN_ALPHA: 50` and
   `STAGE.yaml` `exit_criteria.alpha_to_beta` says "Coverage >= 50 %";
   `DEFECTS_AND_ISSUES.md` P1-01 and `ARCHITECTURE.md` §1 use **80 %**, which
   is the *Beta* figure (`COVERAGE_MIN_BETA: 75`;
   `ALPHA_TO_BETA_GATE_PLAN.md` §2 applies the stricter G17 Beta 80 %). All
   readings are shown; this file does not pick one.
8. **The stage recorded in four release documents disagrees.**
   `STAGE.yaml` says `current_stage: "ALPHA"` (re-entered 2026-10-06);
   `ARCHITECTURE.md` (2026-10-07) says "DRAFT (NOT YET ALPHA, per
   `docs/releases/v4.1.0/STAGE.yaml`)", `COMPREHENSIVE_ASSESSMENT_REPORT.md`
   (2026-10-05) says DRAFT, and `RELEASE_NOTES.md` says "DRAFT — NOT YET
   ALPHA". `docs/governance/STAGE_CONFIG.yaml` makes `STAGE.yaml` the per-version SSOT, so
   this file follows it — and records the disagreement rather than hiding it.
9. **The defect counts disagree.** `DEFECTS_AND_ISSUES.md` §0 records
   P0 × 8 / P1 × 7 / P2 × 7 = 22; `RELEASE_NOTES.md` and `ARCHITECTURE.md`
   both quote "P0×6 + P1×7 + P2×7". Both numbers are left exactly as they
   are.
10. **No Alpha→Beta gate verdict is asserted.** `ALPHA_TO_BETA_GATE_PLAN.md`
    §1 makes AB-01..AB-10 all-or-nothing on a single frozen commit, and §6
    states that the plan publishes no PASS; no AB verdict is recorded
    anywhere in the repository.
11. **The IAIA-410 SSOT is a protocol, not a result.**
    `IAIA_410_SSOT.md` declares `evidence_status: protocol-only` and states
    that it must not serve as run evidence for any audit PASS/FAIL.
12. **No claim about the current 5-remote sync state.** The drift figures in
    `DEFECTS_AND_ISSUES.md` P0-04 are dated 2026-10-01 and
    `STAGE.yaml` `last_status_refresh` records a different state on
    2026-10-03; both are dated records and this file does not merge them.
13. **Nothing here is a GA-grade claim.** The 8 h SOAK's evidential weight is
    downgraded by the IAIA-410 audit (F-07 caveat above); a green SOAK, a
    green stage gate and a closed issue list are three different things and
    only the first of them is recorded here.

---

## 4. Artefact completeness at the time of writing

`scripts/gate/check_release_artifacts.sh --version v4.1.0` reported, on
2026-10-21:

| Stage | Present | Missing |
|---|---|---|
| ALPHA (current, enforced) | 6/7 | — (`FEATURE_MATRIX.md` was the one missing; this file closes it) |
| BETA | 4/7 | `FEATURE_CHECKLIST.md` |
| RC | 5/9 | `RELEASE_GATE_CHECKLIST.md`, `PERFORMANCE_TARGETS.md`, `EVIDENCE_STATUS.md`, `GA_GATE_REPORT.md` |
| GA | 5/11 | `GA_GATE_REPORT.md`, `GA_RELEASE_TIMELINE.md`, `EVIDENCE_STATUS.md`, `POST_GA_PLAN.md` |

> **A second, different mandatory-document list exists.**
> `scripts/gate/check_document_completeness.sh` enforces the list in
> `docs/governance/DOCUMENT_COMPLETENESS_CHECK.md`, which is **not** the same
> list as the `doc_artifacts` table above. It additionally requires
> `QUICK_START.md` (≥ 5000 bytes), `DEPLOYMENT_GUIDE.md` (≥ 15000),
> `MIGRATION_GUIDE.md` (≥ 10000), `INSTALL.md` (≥ 5000) and
> `EVALUATION_REPORT.md` (≥ 1000), and it sizes `FEATURE_MATRIX.md` at
> ≥ 15000 bytes. As observed on 2026-10-23, all eight mandatory entries
> exist, but `RELEASE_NOTES.md` is **8 926 bytes** against its 10 000-byte
> threshold — **1 of the 8 mandatory entries still fails**. That list is
> being filled in concurrently, so this is a dated observation, not a
> standing verdict. **This file does not claim that gate passes** — it
> records the gap, and the same gate also checks internal link integrity of
> the mandatory documents.

---

## 5. Sources

- `docs/releases/v4.1.0/VERSION_PLAN.md` — §2 delta table; §1 "Pending" items
- `docs/releases/v4.1.0/CHANGELOG.md` — "[Unreleased] / P0 修复 — 2026-10-08"
  and the carry-forward subsections; §"Status"
- `docs/releases/v4.1.0/ROADMAP.md` — §1 milestones
- `docs/releases/v4.1.0/STAGE.yaml` — `current_stage`, `last_transition`,
  `last_rollback_record`, `known_gaps`, `open_issues`,
  `resolved_issues_2026_10_03`, `phase_1_alpha_status`,
  `legacy_chain_audit_2026_09_30`, `inherited_from_v4000_revocation`
- `docs/releases/v4.1.0/IAIA_410_AUDIT_PLAN.md` — §13–§16 findings and
  remediation tracking
- `docs/releases/v4.1.0/IAIA_410_SSOT.md` — audit protocol; §10.3 migration
  entries
- `docs/releases/v4.1.0/ISSUES_PLAN.md` — §4 backlog (§4.0–§4.7), §4.8–§4.9
  defect tables, §5 Alpha→Beta work packages
- `docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md` — P0/P1/P2 catalogue
- `docs/releases/v4.1.0/ARCHITECTURE.md`, `ARCHITECTURE_ROUTE_DECISION_5193.md`,
  `CORE_PATH_INVENTORY.md` — production-path inventory
- `docs/releases/v4.1.0/ALIGNMENT_AUDIT_2026-09-30.md` — F-01..F-12
- `docs/releases/v4.1.0/PERFORMANCE_AUDIT_REPORT_2026-09-30.md` — 18 static
  performance findings
- `docs/releases/v4.1.0/LEGACY_LEDGER_v3.6_to_v4.1.md` — cross-version legacy
  ledger
- `docs/releases/v4.1.0/ALPHA_TO_BETA_GATE_PLAN.md` — AB-01..AB-10
- `docs/releases/v4.1.0/evidence/` — the measurement documents named above
- `docs/releases/v4.1.0/logs/` — recorded stage-gate run logs
- `docs/releases/v4.0.0/STAGE.yaml`, `GA_GATE_REPORT.md`,
  `GA_RELEASE_TIMELINE.md` — the inherited v4.0.0 baseline and its revocation
- `docs/governance/STAGE_CONFIG.yaml` — `stages.ALPHA.doc_artifacts`
  (the criterion this file satisfies) and the per-stage thresholds
- `docs/governance/DOCUMENT_COMPLETENESS_CHECK.md` — the second mandatory
  document list referenced in section 4

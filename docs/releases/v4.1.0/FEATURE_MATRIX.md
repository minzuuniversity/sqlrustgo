# v4.1.0 — FEATURE_MATRIX

> **Date**: 2026-10-21
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
`VERSION_PLAN.md` §2 (deltas over v4.0.0) or a bullet of `CHANGELOG.md`
§"[Unreleased]" (the P0 fix round of 2026-10-08). Section 2 is `ROADMAP.md`
§1 verbatim in status. Section 3 is `STAGE.yaml` → `known_gaps` +
`open_issues`, plus the artefact gaps that `check_release_artifacts.sh`
reports.

---

## 1. Delta matrix (v4.1.0 over v4.0.0)

### 1.1 Carried-forward deltas — source: `VERSION_PLAN.md` §2

| ID | Area | Delta | Status | Evidence |
|---|---|---|---|---|
| D-01 | zombie-fix core | Cherry-pick `67b624cbd2` + merge `9c6767a512`; `gap_lock_manager` + `read_write_splitter` updated | DONE | `VERSION_PLAN.md` §2; `STAGE.yaml` `last_transition` (WP-C/D/F/G merged via #5041/#5043/#5045/#5047) |
| D-02 | `workers.push` | Commit `b4d46e6e18` + PR #4906 + PR #3793 merges | DONE | `VERSION_PLAN.md` §2 |
| D-03 | DML/storage regressions | `f3595e7361` + PR #4905 + PR #3792 merges | DONE | `VERSION_PLAN.md` §2 |
| D-04 | Review queue closure | 3 cherry-picks + zombie merge + `0bbb044da3` (CLOSED status doc) | DONE | `VERSION_PLAN.md` §2 |
| D-05 | 5-remote sync tooling | `scripts/sync/5remotes_sync.sh`, `5remotes_drift_check.sh`, `scripts/sync/README.md` | DONE | `VERSION_PLAN.md` §2; `ROADMAP.md` M-4 |

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
   PASS)" wording conflict, `check_anti_ignore_gate.sh` being a no-op gate
   (P0-01), the parser block-comment gap masked by an unregistered
   `#[ignore]` (P0-02), the literal `XYZ` placeholder in
   `debt-registry.yaml` SEM-4 (P0-05), the parser coverage gap
   (60.58 % measured vs the 80 % ALPHA threshold — P1-01), the missing Q20
   oracle (P1-02), and the O(N²) WAL recovery rewrite (P0-08).
4. **No gate verdict is asserted.** Nothing in this file says any gate
   passes. The gate verdicts live in `docs/releases/v4.1.0/logs/` (run
   logs) and in the evidence directory, not here.
5. **No coverage, throughput or latency number is asserted** beyond the
   figures quoted in section 1, each of which points at the measurement
   document it came from.

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

---

## 5. Sources

- `docs/releases/v4.1.0/VERSION_PLAN.md` — §2 delta table
- `docs/releases/v4.1.0/CHANGELOG.md` — "[Unreleased] / P0 修复 — 2026-10-08"
- `docs/releases/v4.1.0/ROADMAP.md` — §1 milestones
- `docs/releases/v4.1.0/STAGE.yaml` — `current_stage`, `last_transition`,
  `known_gaps`, `open_issues`
- `docs/releases/v4.1.0/evidence/` — the measurement documents named above
- `docs/governance/STAGE_CONFIG.yaml` — `stages.ALPHA.doc_artifacts`
  (the criterion this file satisfies)

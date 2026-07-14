# SQLRustGo v3.10.0 Release Documentation

**One-line description**: A MySQL 5.7-compatible relational database in Rust, designed as a lightweight embedded/developer替代方案。

**Status**: GA (2026-07-13)

---

## Reading Paths by Role

### Decision-maker / Project Manager
1. [COMPREHENSIVE_ASSESSMENT_REPORT.md](./COMPREHENSIVE_ASSESSMENT_REPORT.md) — Overall verdict, all gates PASS
2. [MYSQL57_COMPARISON_REPORT.md](./MYSQL57_COMPARISON_REPORT.md) — §8.3 适用场景速查表 for quick fit/no-fit decision
3. [POST_GA_PLAN.md](./POST_GA_PLAN.md) — v3.11.0 roadmap and milestone tracking

### Developer / Operations
1. [GA_GATE_REPORT.md](./GA_GATE_REPORT.md) — R1-R8 gate evidence, all PASS
2. [MYSQL57_COMPARISON_REPORT.md](./MYSQL57_COMPARISON_REPORT.md) — Full SQL syntax compatibility table (§2)
3. [RELEASE_NOTES.md](./RELEASE_NOTES.md) — What changed since v3.9.0
4. `scripts/gate/` — All gate scripts (executable)

### Performance Engineer
1. [perf/PERFORMANCE_BASELINE.md](./perf/PERFORMANCE_BASELINE.md) — ⚠️ PARTIAL: SF=0.1 baseline exists; SF=1 blocked (hardware)
2. [perf/SF01_BASELINE_REPORT.md](./perf/SF01_BASELINE_REPORT.md) — TPC-H SF=0.1 22-query results (~157s total)
3. [perf/SERIAL_VS_PARALLEL_REPORT.md](./perf/SERIAL_VS_PARALLEL_REPORT.md) — Parallel executor analysis

---

## Document Map

```
v3.10.0/
├── README.md                          ← You are here
├── COMPREHENSIVE_ASSESSMENT_REPORT.md ← Highest-level summary (start here)
├── MYSQL57_COMPARISON_REPORT.md       ← MySQL 5.7 feature comparison + 适用场景速查表
├── GA_GATE_REPORT.md                  ← R1-R8 gate evidence (PASS/WARN status)
├── STAGE.yaml                         ← Release stage state machine
├── GA_RELEASE_TIMELINE.md             ← RC→GA timeline with milestones
├── CA_SIGNING_LOG.md                  ← CA authorization chain
├── POST_GA_PLAN.md                    ← v3.11.0 planning + GitHub Milestone tracking
├── EVIDENCE_STATUS.md                 ← D1-D5 evidence tracking
├── CHANGELOG.md                       ← Per-version changelog
├── RELEASE_NOTES.md                   ← User-facing release notes
│
├── perf/
│   ├── PERFORMANCE_BASELINE.md        ← ⚠️ PARTIAL: v3.10.0 vs v3.9.0 (SF=0.1)
│   ├── SF01_BASELINE_REPORT.md        ← TPC-H SF=0.1 (10% of SF=1) 22/22 query results
│   ├── V310_BASELINE.md               ← Extracted v3.10.0 numbers from SF01 report
│   ├── SERIAL_VS_PARALLEL_REPORT.md   ← Parallel executor performance analysis
│   ├── comparison.md                  ← v3.10.0 vs v3.9.0 (PENDING: needs v3.9.0 binary)
## Document Relationship Map

---

## Quick Status

| Gate | Status |
|------|--------|
| R1 Files | ✅ PASS (5/5) |
| R2 Functional | ✅ PASS (6/6) |
| R3 Compilation | ✅ PASS |
| R4 E2E | ✅ PASS (8/8) |
| R5 Ignore Debt | ✅ PASS (8 ≤ 10) |
| R6 Coverage | ⚠️ WARN (14.71% lib → 80% target v3.11.0) |
| R7 OPEN Debt | ✅ PASS (0 OPEN, 6 DEFERRED→v3.11.0) |
| R8 Perf Baseline | ⚠️ HARDWARE-BLOCKED (SF=1 needs 75GB+ disk) |

**Overall**: RC GATE PASS with WARN on R6, R8.

---

## Known Limitations (hardware-blocked)

- TPC-H SF=1 (75GB+ disk required) — blocked; SF=0.1 baseline established
- sysbench OLTP QPS — PENDING
- Gap-lock P99 — PENDING
- v3.9.0 regression comparison — PENDING (needs v3.9.0 binary)

---

## v3.11.0 Commitments (tracked via GitHub Milestone)

8 Issues to be created post-GA on 250 Gitea. See [POST_GA_PLAN.md](./POST_GA_PLAN.md) §v3.11.0 GitHub Milestone Creation.

# Stability Test Ladder — Choose the Right Test for the Right Need

> **Status**: Reference document for v3.9.0 GA team
> **Date**: 2026-06-14
> **Scope**: All 4 time tiers of wired stability testing, when to use each

## Why this document

After this PR (PR #3268) lands, there are **4 distinct wired stability scripts** that
look superficially similar. This doc maps them to the right situation so we don't
waste 24h running a 5-min test, or miss a regression by running only the 5-min.

## The Ladder (4 tiers, ascending time × depth)

```
   TIME  TIER                  SCRIPT                                  PURPOSE
   ─────────────────────────────────────────────────────────────────────────────
   60s   TIER 0: Smoke         test_integration_5min.sh                Architecture proof
         (PR #3380)            60s real, ulimit 2GB/512/500MB            (does server + sysbench
                                Resource-bounded                        wire path WORK?)
                               
   30m   TIER 1: Short         run_tpch_30min.sh                       First real soak signal
         (PR #3268)            30min real, no resource limits           (server crash? RSS leak?
                                TPC-H 22 via mysql CLI, 30 rounds       growth trend?)
                                sysbench skipped if unavailable
                               
   4-12h TIER 2: Medium        run_wired_soak.sh HOURS=4..12           Build confidence
         (PR #3268)            4-12h real, no resource limits            (resource growth
                                sysbench + TPC-H 22 in parallel          trend, mid-range
                                                                         query stability)
                               
   24h+ TIER 3: Long          run_24h_soak_v2.sh / run_72h_soak.sh /  GA gate, post-GA
         (existing)            run_168h_soak.sh                          (TPC-H 22 24h stress,
                                24/72/168h real, no resource limits        crash detection, RSS
                                                                         ceiling, FD ceiling)
```

## When to use each tier

| Situation | Right tier | Why |
|-----------|-----------|-----|
| Just built a new binary, want fast sanity | **Tier 0** (5min) | Catches architecture break, ulimit prevents OOM |
| About to claim "this PR doesn't break wired path" | **Tier 1** (30min) | Real workload, fast feedback (~30 min) |
| Mid-development iteration on engine changes | **Tier 1** or **Tier 2** | Tier 1 for fast cycle, Tier 2 when claim "stable" |
| Pre-GA gate | **Tier 3** (24h) | Hard requirement for #3225 closure |
| Post-GA weekly check | **Tier 3** (72h / 168h) | Long-tail regression detection |
| Cross-version upgrade test | **Tier 1** (30min) per version | Faster than 24h, still real workload |

## Decision flowchart

```
START
  │
  ├── "Is the server binary new and we don't know if it works at all?"
  │     └─→ Tier 0 (5min) → fix breakages
  │
  ├── "Did I just change executor/storage code?"
  │     └─→ Tier 1 (30min) → if clean, proceed; if growth/warns, debug
  │
  ├── "Did I just change server startup / connection handling?"
  │     └─→ Tier 1 (30min) first, then Tier 2 (4-12h) if clean
  │
  ├── "Is this a pre-GA claim?"
  │     └─→ Tier 3 (24h) → required for GA
  │
  ├── "Is this a weekly maintenance cycle?"
  │     └─→ Tier 3 (72h or 168h)
  │
  └── "Is this for a hot fix backport?"
        └─→ Tier 1 (30min) → if clean, deploy
```

## What each tier catches (and what it doesn't)

| Failure type | Tier 0 (5min) | Tier 1 (30min) | Tier 2 (4-12h) | Tier 3 (24h+) |
|--------------|:-:|:-:|:-:|:-:|
| Server boot crash | ✅ | ✅ | ✅ | ✅ |
| sysbench wire path | ✅ | (skipped) | ✅ | ✅ |
| TPC-H 22 query engine | (only Q1) | ✅ | ✅ | ✅ |
| RSS leak (gradual) | ❌ | ⚠️ (truncated) | ✅ | ✅ |
| FD leak (gradual) | ❌ | ⚠️ (truncated) | ✅ | ✅ |
| WAL growth (long-term) | ❌ | ⚠️ | ✅ | ✅ |
| Intermittent crash | ❌ | ❌ | ⚠️ | ✅ |
| 24h endurance | ❌ | ❌ | ❌ | ✅ |
| Lock contention long-tail | ❌ | ❌ | ⚠️ | ✅ |

**Legend**: ✅ = catches | ⚠️ = partial signal | ❌ = cannot catch in this duration

## Resource expectations (z440 reference, 2026-06-14)

| Tier | RSS peak | FD count | CPU avg | Notes |
|------|----------|----------|---------|-------|
| Tier 0 (5min) | 6 MB | 5 | 0% | ulimit-bounded, idle |
| Tier 1 (30min) | 13 MB | 12 | 12% | TPC-H 22, observed |
| Tier 2 (4-12h) | TBD | TBD | TBD | not yet run |
| Tier 3 (24h+) | TBD | TBD | TBD | in progress on 250 |

## Acceptance criteria (each tier)

### Tier 0 (5min)
- Server stays alive 60s
- RSS < 2GB (ulimit)
- FD < 512 (ulimit)
- 0 sysbench errors (when sysbench available)
- REGRESSION: 0 OOM/FD exhaustion

### Tier 1 (30min)
- 0 server crashes
- RSS growth < 50 MB (24h-equiv) / < 5 MB (30min actual)
- FD growth < 5
- TPC-H rounds ≥ 25 (out of 30)
- TPC-H per-query errors documented (Q2/Q9/Q13/Q16/Q17/Q20/Q21/Q22 known)

### Tier 2 (4-12h)
- 0 server crashes
- RSS growth < 50 MB (24h-equiv), scaled to elapsed
- FD growth < 50
- TPC-H rounds ≥ H * 0.8
- sysbench 0 errors

### Tier 3 (24h+)
- 0 server crashes
- RSS growth < 50 MB
- FD growth < 50
- Final RSS < 4096 MB
- Final WAL < 10240 MB
- sysbench 0 errors
- TPC-H ≥ H * 0.5 rounds

## Scripts in this ladder

| Tier | Script | PR | Status |
|------|--------|----|----|
| 0 | `scripts/stability/test_integration_5min.sh` | #3380 | merged in develop/v3.9.0 |
| 1 | `scripts/stability/run_tpch_30min.sh` (this PR) | #3268 | open |
| 1-2 | `scripts/stability/run_wired_soak.sh` (this PR) | #3268 | open |
| 1-2 | `scripts/stability/load_tpch_fixture_insert.sh` (this PR) | #3268 | open |
| 1-2 | `scripts/stability/load_tpch_fixture.sh` (this PR, LOAD DATA fallback) | #3268 | open |
| 1-2 | `scripts/stability/tpch_22_rotate.sh` (this PR) | #3268 | open |
| 1-2 | `scripts/stability/launch_parallel_soak.sh` (this PR) | #3268 | open |
| 3 | `scripts/stability/run_24h_soak_v2.sh` | (existing) | existing |
| 3 | `scripts/stability/run_72h_soak.sh` | (existing) | existing |
| 3 | `scripts/stability/run_168h_soak.sh` | (existing) | existing |

## Companion documentation

- `docs/openspec/3185-exchange-operator.md` — future v3.10+ work, will make Tier 1/2 actually parallel-accelerated
- `docs/audit/status/SOAK_WIRED_SHORT_DURATIONS.md` — Tier 1/2 detail (this PR)
- `docs/audit/status/2026-06-14-30min-tpch-result.md` — Tier 1 first run (this PR)
- `V390_TEST_PLAN.md §G7/G13` — Tier 3 original spec
- `V390_DEVELOPMENT_PLAN.md §Phase 4` — Tier 3 priority

## Maintenance

This document should be updated whenever:
- A new tier is added (e.g., 7-day endurance tier above 168h)
- A tier's resource expectations change
- A new acceptance criterion is added to a tier
- A new script is added to the ladder

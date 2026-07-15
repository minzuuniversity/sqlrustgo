# SQLRustGo v3.11.0 Release Notes — DRAFT

> **Status**: DRAFT (2026-07-15)
> **Branch**: `develop/v3.11.0`
> **Based on**: v3.10.0 GA (develop/v3.10.0 @ 14979a5f16)
> **Target GA**: 2026-10-01

## Overview

v3.11.0 = **Debt Clearance + Feature Island Integration + Performance Breakthrough**

This release completes the debt clearance cycle from v3.6.0 through v3.10.0:
- 23 legacy debt tasks inherited from v3.10.0
- 9 F-XX ISOLATED features → main-path integration
- 3 F-XX NOT IMPLEMENTED features → full implementation
- 11 extension crate product decisions
- Q4 correlated subquery performance (96% runtime bottleneck)

## v3.10.0 → v3.11.0 Changes

### Phase 0: Foundation (5 issues)

| Issue | Description | Status |
|-------|-------------|--------|
| #3421 | Re-enable disabled integration tests | ✅ CLOSED |
| #3422 | Fix compiler warnings (-D warnings clean) | ✅ CLOSED |
| #3423 | Fix examples compilation | ✅ CLOSED |
| #3424 | F-03 restore_cursor_backup | ✅ CLOSED |
| #3420 | SEM-4 Coverage ≥ 80% (main crate) | 🔄 IN PROGRESS |

### Phase 1: Storage Engine (5 issues)

| Issue | Description | Status |
|-------|-------------|--------|
| #3425 | restore_filespace_resync | ✅ CLOSED |
| #3426 | restore_filespace_cleanup | ✅ CLOSED |
| #3427 | Remove MOCK storage backend | ✅ CLOSED |
| #3428 | SEM-3: ALTER TABLE RENAME/MODIFY | ✅ CLOSED |
| #3440 | F-36 Column-level privilege | ✅ CLOSED |

### Phase 2: SQL / Protocol (2 issues)

| Issue | Description | Status |
|-------|-------------|--------|
| #3429 | MySQL wire protocol DDL fix | ✅ CLOSED |
| #3430 | TPC-H Q22 cell-level mismatch | ✅ CLOSED |

### Feature Tasks Completed (V311-XX)

| ID | Task | F-XX | Status |
|----|------|------|--------|
| V311-01 | Clustered Index main-path integration | F-23 | ✅ DONE (PR #3461) |
| V311-02 | Adaptive Hash Index main-path integration | F-24 | ✅ DONE (PR #3465/#3476/#3478) |
| V311-06 | Performance Schema instrumentation hooks | F-31 | ✅ DONE (trait + Noop + Counting) |
| V311-07 | MySQL Admin ↔ mysql-server integration | F-32 | ✅ DONE (PR feature branch) |
| V311-09 | Column-level privilege implementation | F-36 | ✅ DONE (PR #3457) |
| V311-13 | ALTER TABLE RENAME/MODIFY complete | SEM-3 | ✅ DONE (PR #3444/#3449) |
| V311-15 | Hash Semi Join operator | PERF-1 | ✅ DONE (PR #3455) |
| V311-16 | Decorrelation optimizer pass | PERF-4 | ✅ DONE (rewrite v2) |
| V311-17 | Hash Anti Join operator | PERF-2 | ✅ DONE |
| V311-19 | Extension Crate decision | — | ✅ DONE (5 delete + 3 archive + 1 integrate) |
| V311-22 | Docs restructure (5 plans → 3 plans) | — | ✅ DONE |
| V311-23 | High-concurrency INSERT fix | PERF-5 | ✅ DONE (from v3.10.0 SOAK) |

### Remaining Tasks

| ID | Task | Effort | Priority | Stage |
|----|------|--------|----------|-------|
| V311-03 | F-25 Change Buffer main-path integration | 40h | P0 | ALPHA-BETA |
| V311-04 | F-26 Double-Write Buffer main-path integration | 50h | P0 | ALPHA-BETA |
| V311-05 | F-29 Row-Level Security main-path integration | 40h | P1 | BETA |
| V311-08 | F-35 Password Rotation main-path integration | 20h | P1 | BETA |
| V311-10 | F-30 CREATE SEQUENCE implementation | 20h | P1 | BETA |
| V311-11 | F-03 GIS spatial types (POINT + WITHIN) | 80h | P1 | BETA |
| V311-12 | F-27 Table Compression (LZ4/zstd) | 50h | P1 | BETA |
| V311-14 | SEM-4 Coverage ≥85% | 60h | P0 | BETA-RC |
| V311-18 | CTE materialization | 30h | P1 | BETA |
| V311-20 | TPC-H SF=1.0 baseline | 80h | P0 | BETA-RC |
| V311-21 | 168h SOAK v3.11.0 | — | P1 | RC |

## Known Issues

- **LFS corruption**: SF=0.1 data files are LFS pointers (not actual data). Need `git lfs fetch` from backup250 to restore.
- **Network**: Remote origin (192.168.0.252:3000) unreachable; pushes via backup250 Gitea.
- **TPC-H SF=1 baseline**: Requires 75GB+ dedicated hardware (deferred from v3.10.0).
- **Benchmark tests**: 13 pre-existing benchmark targets fail (require `unstable` feature gate).

## Branch Protection

| Branch | Push | Approvals | Status Checks |
|--------|------|-----------|---------------|
| `develop/v3.11.0` | ❌ Disabled | ✅ 2 required | lint, build, docs-links, cargo-build |

## Quality Gates (DRAFT → ALPHA)

- ✅ build: `cargo build --all-features` 0 errors
- ✅ clippy: `cargo clippy --all-features -- -D warnings` 0 errors (post-#3422)
- ✅ fmt: `cargo fmt --check --all` 0 diffs
- ✅ test: 344 targets compile, 615+ lib tests pass
- ✅ branch protection: develop/v3.11.0 with required approvals
- ✅ STAGE.yaml with ALPHA/BETA/RC promotion criteria
- ❌ V311-03 through V311-05, V311-08, V311-10..12, V311-14, V311-18, V311-20, V311-21 open
- ❌ Coverage ≥80% per crate (in progress via #3420)

---

*Created: 2026-07-15 (DRAFT init)*
*Maintainer: openclaw*

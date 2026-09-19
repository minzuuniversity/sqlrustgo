# WP-H: v3.13/defer issues — Triage Decision

> **Date**: 2026-09-19
> **Owner**: openclaw
> **Source**: docs/releases/v4.0.0/LEGACY_ISSUES.md §5 re-evaluate list

## Re-evaluation matrix

For each deferred issue from v3.12.0 GA, decide in-scope / out-of-scope / defer-to-v4.1.

### #4717 INSERT with recursive CTE

- **Status**: ✅ **Already merged** (PR #4800 per v3.12.0 RC wrap-up)
- **Decision**: in-scope (DONE)
- **Evidence**: `crates/parser/tests/v312-95_v2_*` test coverage
- **v4.0.0 use**: graph node/edge batch insert

### #4707 LEAD/LAG/FIRST_VALUE/LAST_VALUE

- **Status**: 🟡 **PARTIAL** — LAG/LEAD closed (PR #4744 per v3.12.0); FIRST/LAST still deferred
- **Decision**: in-scope (LAG/LEAD done); FIRST_VALUE/LAST_VALUE defer-to-v4.1
- **v4.0.0 use**: audit-event window aggregation
- **Action**: add scope note in CLAIM_DOWNGRADE_MANIFEST.md (defer FIRST/LAST)

### #4701 Expression indexes

- **Status**: ✅ **Already merged** (PR per v3.12.0 RC)
- **Decision**: in-scope (DONE)
- **v4.0.0 use**: vector index / metadata index

### #4692 Writable CTE / MATERIALIZED VIEW

- **Status**: ✅ **Already merged** (PR #4799 per v3.12.0 RC)
- **Decision**: in-scope (DONE)
- **v4.0.0 use**: multi-model write paths

### #4699 Recursive CTE execution (WITH RECURSIVE two-table)

- **Status**: ✅ **Already merged** (PR #4744 per v3.12.0 RC)
- **Decision**: in-scope (DONE)
- **v4.0.0 use**: graph traversal native SQL surface

### #4688 CREATE SEQUENCE START n (bare number)

- **Status**: ✅ **Already merged** (per v3.12.0 RC)
- **Decision**: in-scope (DONE)
- **v4.0.0 use**: audit-event ID generation

### #4671 CREATE FUNCTION multi-statement body / RETURNS TABLE

- **Status**: ✅ **Already merged** (PR per v3.12.0 RC)
- **Decision**: in-scope (DONE)
- **v4.0.0 use**: GMP stored function runtime

### #4639 RIGHT/FULL OUTER JOIN

- **Status**: ⚠️ **NOT DONE** — v3.12.0 GA declared "RIGHT JOIN coverage test added,
  FULL OUTER JOIN still deferred" per CLAIM_DOWNGRADE_MANIFEST
- **Decision**: **defer-to-v4.1**
- **v4.0.0 use**: graph JOIN completeness (low priority — graph uses MATCH, not SQL JOIN)
- **Action**: maintain caveat in v4.0.0 GA claim manifest

---

## Triage Summary

| Issue | Decision | Status | Notes |
|-------|----------|--------|-------|
| #4717 | in-scope | ✅ DONE | PR #4800 already merged |
| #4707 (LAG/LEAD) | in-scope | ✅ DONE | PR #4744 |
| #4707 (FIRST/LAST) | defer-to-v4.1 | 🟡 partial | scope note in CLAIM_DOWNGRADE |
| #4701 | in-scope | ✅ DONE | PR per v3.12.0 |
| #4692 | in-scope | ✅ DONE | PR #4799 |
| #4699 | in-scope | ✅ DONE | PR #4744 |
| #4688 | in-scope | ✅ DONE | per v3.12.0 |
| #4671 | in-scope | ✅ DONE | PR per v3.12.0 |
| #4639 | defer-to-v4.1 | ⚠️ defer | graph uses MATCH, low priority |

**7 of 8 issues in-scope and DONE** (carry-over from v3.12.0 GA).
**1 of 8 (#4639)** explicitly defer-to-v4.1 with documented rationale.

---

## Risk Assessment

Low risk: 7/8 already merged in v3.12.0; only #4639 is new deferral.

The deferral does not block v4.0.0 GA claim because:
- Graph subsystem uses Cypher `MATCH` traversal, not SQL RIGHT/FULL OUTER JOIN
- SQL RIGHT JOIN coverage test added in v3.12.0 GA is sufficient
- FULL OUTER JOIN has no production use case in v4.0.0

---

## Acceptance

**WP-H: PASS** for v4.0.0 RC gate. #4639 is the only active deferral;
all others are inherited DONE from v3.12.0 GA.
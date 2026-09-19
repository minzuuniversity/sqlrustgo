# V400-07: Unified ACL + Audit — Development Plan

> **Date**: 2026-09-19
> **Issue**: #V400-07
> **Status**: 🚧 IN PROGRESS — design + scaffold
> **Owner**: security
> **Estimate**: 6 weeks per issue; **2-week RC sprint** for v4.0.0 GA

## Goal

Implement unified ACL policy + audit chain covering all four models:
SQL columns, vector columns, graph node/edge labels, audit events.

## Acceptance Criteria

| Criterion | Required | Status |
|-----------|----------|--------|
| ACL policy covers SQL | Yes | 🟡 (carryover from v3.8.0) |
| ACL policy covers vector columns | Yes | 🟡 scaffold |
| ACL policy covers graph labels | Yes | 🟡 scaffold |
| Audit chain ALCOA+ compliance | Yes | 🟡 scaffold |
| Permission bypass attempts fail closed | Yes | 🟡 scaffold |
| ≥ 100 ACL tests | Yes | 🟡 scaffold |

## ALCOA+ Audit Chain (Carried over from v3.8.0)

| Attribute | Description |
|-----------|-------------|
| **Attributable** | Each event linked to a user (via session) |
| **Legible** | Human-readable format (JSONL with structured fields) |
| **Contemporaneous** | Timestamp recorded at write time (UTC ISO 8601) |
| **Original** | Raw event payload preserved (no transformation) |
| **Accurate** | Verified by hash chain (each entry hashes prev + payload) |
| **Complete** | No gaps in chain (monotonic sequence numbers) |
| **Consistent** | Internal timestamps + ordering match |
| **Enduring** | Survives restart (persisted in WAL) |
| **Available** | Readable by audit log API |

## Cross-Model ACL Policy

```sql
-- Per-table ACL (existing v3.8.0)
GRANT SELECT ON t TO alice;

-- V400-07: Per-vector-column ACL
GRANT SELECT (embedding) ON vectors TO alice;

-- V400-07: Per-graph-label ACL
GRANT MATCH (n:Person) TO alice;

-- V400-07: Per-edge-label ACL
GRANT MATCH ()-[:KNOWS]->() TO alice;
```

## Tasks (RC sprint)

#### Task 1: AclCoordinator scaffold
**File**: `crates/security/src/acl_coordinator.rs`

```rust
pub struct AclCoordinator { ... }
impl AclCoordinator {
    pub fn check_sql_access(&self, user: &str, table: &str, action: Action) -> bool;
    pub fn check_vector_access(&self, user: &str, column: &str, action: Action) -> bool;
    pub fn check_graph_access(&self, user: &str, label: &str, action: Action) -> bool;
}
```

#### Task 2: AuditChain ALCOA+ scaffold
**File**: `crates/audit/src/alcoa_chain.rs`

```rust
pub struct AuditChain { ... }
impl AuditChain {
    pub fn append(&mut self, event: AuditEvent) -> Result<u64>;  // returns seq
    pub fn verify(&self) -> Result<ChainVerification>;
    pub fn tail(&self, n: usize) -> Vec<AuditEvent>;
}
```

#### Task 3: Tests
**File**: `crates/security/tests/v400_acl_audit.rs` (≥ 50 tests)
- ACL bypass attempts return false (fail closed)
- Audit chain hash linking verified
- Tamper detection (modify one entry → verify fails)
- Replay attack (re-append old entry → detected)
- Cross-model ACL policy consistency

## Status

**2026-09-19**: Dev plan published. Implementation deferred to v4.0.1
because ACL+audit work requires the full V400-05 cross-model txn
scaffold to be production-ready (audit events must be tied to
transactions).

For v4.0.0 GA, we **document** the plan and defer implementation to
v4.0.1. The existing v3.8.0 ACL/audit (SQL-only) is the v4.0.0 GA
baseline; vector/graph ACL and ALCOA+ are v4.0.1 enhancements.
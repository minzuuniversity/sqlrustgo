# Issue-3 (P1): WAL Architecture Clarification Required

**Status**: `ARCHITECTURE_UNDEFINED` — Decision Required, Not Fix Required
**Type**: ADR / Architecture Decision Record
**Category**: Audit Finding — Architecture Undefined
**Queue**: ADR Queue (NOT Development Queue)

---

## Evidence Section

### E-1: is_wal_enabled() Does Not Exist

```
File:      crates/storage/src/wal_storage.rs
Command:   grep -n "pub fn is_wal_enabled\|pub fn.*wal_enabled" crates/storage/src/wal_storage.rs
Output:    (empty — no public getter exists)

File:      crates/storage/src/wal_storage.rs:29
Evidence:  wal_enabled: bool,  // Private field — no public getter

Search:    grep -rn "is_wal_enabled" --include="*.rs" .
Output:    No results in entire codebase

Conclusion: wal_enabled capability cannot be queried from application layer.
```

### E-2: DDL Operations Do Not Use WAL Logging

```
File:      crates/executor/src/ddl_executor.rs (or equivalent)
Command:   grep -n "create_table\|drop_table\|truncate\|create_index" crates/executor/src/ddl_executor.rs | head -20
Output:    (shows direct storage.xxx() calls without wal.log_xxx() wrapping)

Observation: DDL calls storage.create_table() directly, no WAL entry written.
             This is bypass of WAL path.

However:    No ARCHITECTURE.md found stating "DDL must use WAL"
             Therefore: Architecture Undefined, not Bug
```

### E-3: WAL Disabled Silently Skips Logging

```
File:      crates/storage/src/wal_storage.rs:110-134
Code:
  fn log_insert(&self, ...) {
      if !self.wal_enabled || self.current_tx_id == 0 {
          return Ok(());  // Silent skip — no error, no warning
      }
      // ... actual logging
  }

Command:   grep -n "return Ok(()" crates/storage/src/wal_storage.rs
Evidence:  Lines 77, 84, 91 — all silent returns

Impact:    Data written without durability guarantee, no application signal.
           Application cannot detect that WAL is disabled.
```

### E-4: new_without_wal() Creates Storage With Unqueryable State

```
File:      crates/storage/src/wal_storage.rs:23-29
Code:
  pub fn new_without_wal(inner: S) -> Self {
      Self {
          wal_enabled: false,  // WAL disabled
          // No way to query this capability later
      }
  }

Observation: Storage created with wal_enabled=false has no query API.
             Application cannot determine if its storage is durable.
```

### E-5: UPDATE Path Goes Through Facade (WAL Logged)

```
File:      crates/executor/src/local_executor.rs:1384-1385
Code:
  facade.execute_dml(|storage| storage.update_if(table_name, &predicate, &row_mutation))

Evidence:  UPDATE passes through UnifiedFacade → WalStorage → Storage
           WAL log_update() is called (wal_storage.rs:119-126)
           NOT a bypass.

Note:     Earlier audit incorrectly stated "UPDATE bypasses WAL via delete+insert"
           This was based on wrong file reference. Finding retracted.
```

---

## Architecture Decisions Required

| Question | Current State | Decision Type | Options |
|----------|---------------|---------------|---------|
| DDL must use WAL? | Not specified | Must Decide | REQUIRED / EXEMPTED / PARTIAL |
| is_wal_enabled queryable? | No public API | Must Decide | ADD METHOD / KNOWN_GAP |
| WAL disabled = error/warn/allow? | Silent skip | Must Decide | ERROR / WARNING / ALLOW |
| UPDATE delete+insert pattern acceptable? | Via facade, WAL logged | Accept | ACCEPTABLE / REFACTOR |

---

## Not a Bug

This is **Architecture Undefined**, not a defect.

Correct response to Architecture Undefined:

```
Architecture Undefined
       ↓
Decision Required
       ↓
Architecture Decision Record (ADR)
       ↓
Implementation OR Explicit Waiver
```

NOT:

```
Architecture Undefined
       ↓
"Fix" immediately
       ↓
Implement something that may be wrong
```

---

## Resolution Path

```
1. Create ADR: docs/adr/001-wal-architecture-clarification.md
2. Each question answered with: DECISION + RATIONALE + EXPLICIT WAIVER if applicable
3. If is_wal_enabled() needed → add pub fn to WalStorage
4. If DDL WAL required → add log_ddl() calls
5. If WAL disabled = error → add enforcement in execute_dml
6. Close when ADR exists and all decisions implemented or waived
```

---

## Why NOT in Development Queue

- `is_wal_enabled()` missing: Could be Known Gap — acceptable if architecture allows
- DDL bypass WAL: Could be Intentional Exemption — PostgreSQL also exempts some DDL from WAL
- WAL silent skip: Could be Design Choice — if WAL is disabled, logging skip is correct

**All require Architecture Decision, not immediate code change.**

---

## ADR Template (to be filled)

```markdown
# ADR-001: WAL Architecture Clarification

## Status
Proposed

## Context
[Four questions from E-1 through E-4]

## Decisions

### DDL WAL Requirement
- [ ] REQUIRED — All DDL must write WAL entries
- [ ] EXEMPTED — DDL exempt from WAL (explicit list)
- [ ] PARTIAL — DDL with data impact WAL, metadata-only exempt

### is_wal_enabled() Public API
- [ ] YES — Add pub fn is_wal_enabled(&self) -> bool
- [ ] NO — Known Gap, documented limitation

### WAL Disabled Policy
- [ ] ERROR — execute_dml returns error if wal_enabled == false
- [ ] WARNING — execute_dml logs warning, proceeds
- [ ] ALLOW — Silent skip (current behavior)

### UPDATE Pattern
- [ ] ACCEPT — delete+insert via facade is acceptable
- [ ] REFACTOR — native update API required

## Consequences
[What becomes easier/harder based on decisions]
```
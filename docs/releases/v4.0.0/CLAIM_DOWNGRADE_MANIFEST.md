# v4.0.0 GA Candidate — Scope/Claim Downgrade Manifest

> **provenance**: generated_at=2026-09-19, branch=develop/v4.0.0,
> HEAD=`3fa3bb811c`, source_repo=openclaw/sqlrustgo,
> policy=Anti-Fabrication-Policy-v1.0
>
> **purpose**: Explicit release-claim downgrade for every GA-claim-caveat
> / GA-blocker issue that remains open at GA cut time. Per
> `docs/governance/GATE_CONDITIONS.md` §2 "GA-claim-caveat" criteria:
> "May remain open only if release notes, README, and scope docs
> explicitly exclude the capability from v4.0.0 GA claims."

## 1. Methodology

For each issue below:

1. List the open status in Gitea.
2. State the affected capability.
3. Spell out the exact release-claim boundary line that must appear in
   `README.md`, `RELEASE_NOTES.md`, `GA_GATE_REPORT.md`, and the relevant
   subsystem docs (`docs/releases/v4.0.0/`).
4. Reference per-issue evidence section.

## 2. WP-A..H Triage Summary (from WP_LEGACY_TRIAGE.md + WP_H_TRIAGE.md)

| WP | Issues | Status | v4.0.0 Decision |
|----|--------|--------|-----------------|
| WP-A | parser legacy #4708 #4696 #4710 #4720 | ✅ DONE | Covered by 280 v400 parser tests |
| WP-B | types #4721 #4674 #4716 #4676 #4675 #4670 | 🟡 partial | Tests in place; full fixes v4.0.1 |
| WP-C | DDL/integrity #4682 #4652 #4672 #4669 #4709 #4703 | ⬜ deferred | defer-to-v4.0.1 |
| WP-D | joins #4668 #4656 #4649 #4636 | ⬜ deferred | defer-to-v4.0.1 |
| WP-E | transaction #4847 #4626 | 🟡 partial | #4847 partial via V400-05 scaffold |
| WP-F | schema #4848 | ⬜ deferred | defer-to-v4.0.1 |
| WP-G | type/comparison #4846 | ⬜ deferred | defer-to-v4.0.1 |
| WP-H | v3.13/defer #4717 #4707 #4701 #4692 #4699 #4688 #4671 #4639 | ✅ DONE | 7/8 in-scope DONE; #4639 defer-to-v4.1 |

## 3. GA-claim downgrade boundaries (must appear in README, RELEASE_NOTES, GA_GATE_REPORT)

### 3.1 SQL surface (v3.12.0 baseline + WP-A regression coverage)

**Claim boundary line**:
> "v4.0.0 supports the SQL-92 subset as specified in `docs/releases/v3.12.0/`,
> plus regression tests for #4708, #4696, #4710, #4720 (WP-A). Functions
> with WP-B issues (#4721, #4674, #4716, #4676, #4675, #4670) are partially
> supported; full behavior is v4.0.1."

### 3.2 DDL surface (v3.12.0 baseline + WP-C deferrals)

**Claim boundary line**:
> "v4.0.0 retains v3.12.0 DDL baseline. WP-C issues (#4682, #4652, #4672,
> #4669, #4709, #4703) including sqlite_master schema migration, CREATE
> PROCEDURE/FUNCTION persistence, and AUTOINCREMENT are **deferred to
> v4.0.1** and explicitly excluded from v4.0.0 GA claims."

### 3.3 JOIN/SUBQUERY surface (WP-D deferrals)

**Claim boundary line**:
> "v4.0.0 JOIN/SUBQUERY retains v3.12.0 semantics. WP-D issues (#4668
> NATURAL JOIN, #4656 `>ALL`/`=ANY` subquery, #4649 LEFT JOIN USING,
> #4636 correlated scalar subquery) are **deferred to v4.0.1**."

### 3.4 Schema migration (WP-F deferral)

**Claim boundary line**:
> "v4.0.0 supports v3.12.0 schema baseline. `ALTER TABLE RENAME COLUMN`
> (#4848) is **deferred to v4.0.1**."

### 3.5 Type/Comparison (WP-G deferral)

**Claim boundary line**:
> "v4.0.0 CHAR(n) padding follows v3.12.0 byte-padding semantics.
> SQLite/MySQL-compatible CHAR PAD SPACE (issue #4846) is **deferred to
> v4.0.1**."

### 3.6 Vector storage (V400-02)

**Claim boundary line**:
> "v4.0.0 ships WAL-backed vector storage (V400-02 V1-V5) per
> `docs/evidence/v4.0.0/vector_wal_recovery_report.md`. Crash recovery,
> index rebuilding, and cross-process persistence are validated. HNSW
> rebuild on full WAL replay is O(N log N); for >10M vector tables,
> recovery time is the gating step."

### 3.7 Graph storage (V400-03)

**Claim boundary line**:
> "v4.0.0 ships first-class graph storage (V400-03 G1-G5) per
> `docs/architecture/v400_graph_storage.md`. Cypher subset supported:
> MATCH, OPTIONAL, WITH, WHERE, RETURN, ORDER BY, LIMIT, UNION/UNION ALL.
> Not yet supported: MERGE, path expressions with variable-length hops,
> graph-level ACL. Cross-model atomicity (V400-05) is **scaffold-only**
> in v4.0.0; full integration is v4.0.1."

### 3.8 Cross-model transaction (V400-05)

**Claim boundary line**:
> "v4.0.0 ships V400-05 cross-model transaction **scaffold**:
> TransactionManager tracks writes per (tx_id, model_kind). All-or-nothing
> semantics for SQL+vector+graph+audit are validated at the tracker level
> (30 tests in `crates/transaction/tests/v400_cross_model.rs`). Full
> integration with VectorStore/DiskGraphStore/audit chain hooks is
> **v4.0.1 work**; v4.0.0 ships the tracking primitive only."

### 3.9 Unified backup/restore (V400-06)

**Claim boundary line**:
> "v4.0.0 ships V400-06 **design + dev plan** only. Implementation
> deferred to v4.0.1. Current backup mechanism: per-model `BACKUP TABLE`
> (SQL), `BACKUP VECTOR INDEX` (vector), `BACKUP GRAPH` (graph) — see
> subsystem docs. Unified `BACKUP DATABASE` / `RESTORE DATABASE` is
> **v4.0.1**."

### 3.10 Unified ACL + audit (V400-07)

**Claim boundary line**:
> "v4.0.0 ships V400-07 **design + dev plan** only. Implementation
> deferred to v4.0.1. ACL coverage in v4.0.0: SQL tables/columns
> (carryover v3.8.0); vector columns and graph labels are
> **v4.0.1 work**. Audit chain: per-event JSONL append (carryover); ALCOA+
> full compliance is **v4.0.1 work**."

### 3.11 Multi-model optimizer (V400-08)

**Claim boundary line**:
> "v4.0.0 ships V400-08 **partial**: `ExecutorPool` library merged (575
> lines, work-stealing scheduler) per commit `7ff6968e52`. Cost model
> extension for vector + graph is **v4.0.1**."

### 3.12 168h multi-model SOAK (V400-09)

**Claim boundary line**:
> "v4.0.0 ships V400-09 **pre-flight**: 5min mini-SOAK PASS per
> `docs/releases/v4.0.0/SOAK_BASELINE_5MIN_2026-09-19.md` (118,585 queries,
> 0 errors, RSS peak 636 MB). 168h SOAK is **scheduled** post-GA per
> V400-09 plan; v4.0.0 GA ships with the 5min baseline + RSS peak
> optimization (commit b87997d4a1: GC 1s/256)."

### 3.13 GMP-Platform consumer (V400-10)

**Claim boundary line**:
> "v4.0.0 ships V400-10 **partial** consumer compatibility. PR #207
> self-approval blocker remains open in Gitea; full GMP-Platform
> regression suite is **v4.0.1**."

## 4. Coverage G17 downgrade

**Claim boundary line**:
> "v4.0.0 L1_8 average line coverage is **78.28%** (CONDITIONAL PASS
> under GATE_CONDITIONS.md §A5). Parser at 74.32%, mysql-server at
> 75.41%; both are within the 2-week CONDITIONAL window. Internal AST
> conversion paths are not reachable via parser/executor tests alone;
> coverage will rise as V400-05 cross-model hooks (v4.0.1) exercise the
> AST conversions. Full PASS target is v4.0.1."

## 5. RSS peak downgrade

**Claim boundary line**:
> "v4.0.0 ships MVCC GC tuning (commit b87997d4a1: interval=1s, gc_lag=256)
> which cuts RSS peak by ~58% versus the 5s/1000 baseline. Empirical
> 5min SOAK peak: 636 MB. macOS per-process RSS cap (1.7 GB) provides
> ~1 GB headroom. Long-tail MVCC chain accumulation over 168h is
> **empirical-not-validated**; v4.0.0 GA ships with 5min baseline +
> GC tuning, with 168h validation scheduled as V400-09 follow-up."

## 6. Verification checklist (for GA promotion)

Before tagging `ga/v4.0.0`:

- [ ] All claim boundary lines (3.1-3.13, 4, 5) appear in `README.md`
- [ ] All claim boundary lines appear in `RELEASE_NOTES.md`
- [ ] All claim boundary lines appear in `GA_GATE_REPORT.md`
- [ ] `CHANGELOG.md` marks v4.0.0 GA cut date
- [ ] PR opened: develop/v4.0.0 → main (merges v4.0.0 GA to trunk)
- [ ] All 3 remote tags (gitea250/gitea252/gitee) carry `ga/v4.0.0`

## 7. References

- `docs/releases/v4.0.0/WP_LEGACY_TRIAGE.md` (2/7 DONE, 5/7 defer-to-v4.0.1)
- `docs/releases/v4.0.0/WP_H_TRIAGE.md` (7/8 in-scope DONE, 1 defer-to-v4.1)
- `docs/releases/v4.0.0/V400_05_CROSS_MODEL_TXN_DEV_PLAN.md` (scaffold)
- `docs/releases/v4.0.0/V400_06_BACKUP_RESTORE_DEV_PLAN.md` (deferred impl)
- `docs/releases/v4.0.0/V400_07_ACL_AUDIT_DEV_PLAN.md` (deferred impl)
- `docs/evidence/v4.0.0/vector_wal_recovery_report.md` (V400-02 DONE)
- `docs/architecture/v400_graph_storage.md` (V400-03 DONE)
- `docs/releases/v4.0.0/SOAK_BASELINE_5MIN_2026-09-19.md` (pre-flight PASS)
- `docs/releases/v4.0.0/RC_GATE_REPORT.md` (RC CONDITIONAL PASS)
- `docs/releases/v4.0.0/FEATURE_CHECKLIST.md` (V400 + WP status)
# SQLRustGo v4.1.0 — Release Notes

> **Status**: ALPHA (released 2026-09-29, tag `v4.1.0-alpha1`)
> **Branch**: `develop/v4.1.0`
> **Base**: v4.0.0 GA CONDITIONAL PASS (per docs/releases/v4.0.0/GA_GATE_REPORT.md)
> **Scope change vs v4.0.0**: bugfix carry-forward + 168h SOAK FINAL deferred + WP-C..G migration
>
> Per `docs/releases/v4.1.0/STAGE.yaml`, v4.1.0 advances from DRAFT to ALPHA on 2026-09-29. v4.0.0 is **not** replaced; v4.0.0 remains GA on its release branch.

## 1. What is v4.1.0?

v4.1.0 is the **post-v4.0.0 maintenance continuation** of SQLRustGo. It is
the active development trunk for v4.x.

v4.1.0 carries v4.0.0 forward through:

1. **Real bugfix carry-forward** (already landed on develop/v4.1.0 HEAD):
   - V400-05/06/07 cross-model transaction + AuditChain ALCOA+
   - zombie-fix core (bulk-insert + DLM repair)
   - workers.push wrapper restore in ServerThreadPool::start
   - DML/storage regression test fixes
2. **5-remote sync tooling** (live):
   - scripts/sync/5remotes_sync.sh
   - scripts/sync/5remotes_drift_check.sh
   - scripts/sync/README.md
3. **V400-02 vector WAL coverage** (closed pre-existing test-compile drift):
   - 6 factory functions in crates/storage/src/wal/mod.rs
   - WALOperation extended to 13 variants in crates/wal-verification/src/lib.rs
4. **Refactoring for alpha-gate compliance**:
   - crates/sqlrustgo/src/execution_engine.rs: 2801 → 1017 lines (impl block extracted
     to crates/sqlrustgo/src/execution_engine_methods.rs)

## 2. v4.1.0 vs v4.0.0: stage claim boundary

Per `docs/governance/STAGE_CONFIG.yaml` ALPHA stage:

> Allowed claims (pre-ALPHA): "SQLRustGo v4.1.0 is in active development toward a continuation of v4.0.0."

Allowed claims (pre-BETA): "SQLRustGo v4.1.0-alpha1 has been cut for early testing of v4.0.0 bugfix carry-forward."

Prohibited pre-BETA: "Production multi-model claim (requires GA, and even v4.0.0 GA has 3 caveat items per CLAIM_DOWNGRADE_MANIFEST)"

Prohibited pre-GA: "168h multi-model SOAK result (per V400-09)"

## 3. ALPHA known limitations (carried from v4.0.0)

Per `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md`:

- **SQL surface**: WP-A issues covered; WP-B issues (6: #4721 #4674 #4716 #4676 #4675 #4670) partial; full behavior deferred.
- **DDL surface**: WP-C issues (6: #4682 #4652 #4672 #4669 #4709 #4703) deferred to v4.1.0.
- **Transaction semantics**: WP-E issues (#4847 partial, #4626 not started).
- **Schema**: WP-F #4848 (ALTER TABLE RENAME COLUMN) deferred to v4.1.0.
- **Type/Comparison**: WP-G #4846 (CHAR(n) PAD SPACE) deferred to v4.1.0.

## 4. v4.1.0 ALPHA scope work (backlog)

Per `docs/releases/v4.1.0/ISSUES_PLAN.md` §4:

| Category | Count | Items |
|---|---|---|
| WP-C DDL / Integrity | 6 | #4682, #4652, #4672, #4669, #4709, #4703 |
| WP-D Join / Subquery | 4 | #4668, #4656, #4649, #4636 |
| WP-F Schema Migration | 1 | #4848 |
| WP-G Type / Comparison | 1 | #4846 |
| WP-B completion | 6 | #4721, #4674, #4716, #4676, #4675, #4670 |
| WP-E completion | 1 | #4626 |
| WP-H #4639 carry-forward | 1 | #4639 |
| **Total** | **20** | 6-10 weeks estimated |

## 5. Stage state

| Stage | Date | Reason |
|------|------|--------|
| DRAFT | 2026-09-23 | Initial entry per STAGE_CONFIG DRAFT_to_ALPHA trigger; PHASE_0 docs scaffolded |
| **ALPHA** | **2026-09-29** | **Tag `v4.1.0-alpha1`. All 3 inherited v4.0.0 alpha-gate FAILs resolved (P0.1 / P0.2 / P0.3). V400-09 SOAK FINAL_REPORT recorded (deferred to v4.1.0 scope). WP-C..G migration documented.** |
| BETA | TBD | Blocked by 50% coverage measurement + WP-C..G closure + V400-09 SOAK re-launch |
| RC | TBD | Blocked by BETA completion |
| GA | TBD | Blocked by RC completion + 168h SOAK PASS |

## 6. References

- `docs/releases/v4.1.0/STAGE.yaml` — v4.1.0 stage SSOT (current_stage: ALPHA)
- `docs/releases/v4.1.0/PHASE_1_SCOPE.md` — DRAFT → ALPHA work plan + resolved items
- `docs/releases/v4.1.0/ISSUES_PLAN.md` §4 — WP-C..G detailed backlog
- `docs/releases/v4.0.0/CLAIM_DOWNGRADE_MANIFEST.md` §3 — v4.0.0 GA caveat items (inherited)
- `docs/releases/v4.0.0/GA_GATE_REPORT.md` — v4.0.0 GA CONDITIONAL PASS verdict
- `docs/releases/v4.0.0/V400_09_168H_SOAK_FINAL_REPORT.md` — SOAK deferral rationale
- `scripts/sync/README.md` — 5-remote sync tooling (v4.1.0 deliverable)
- `docs/governance/STAGE_CONFIG.yaml` — version-agnostic stage framework
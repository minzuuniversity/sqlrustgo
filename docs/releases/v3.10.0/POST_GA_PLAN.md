# v3.10.0 Post-GA Plan

**Status**: Draft
**Last Updated**: 2026-07-13
**Maintainer**: openclaw

---

## Purpose

Define the post-GA maintenance, support, and development roadmap for v3.10.0, including hotfix criteria for v3.10.1+ and transition planning for v3.11.0.

---

## Phase 1: GA Stabilization Window (GA + 14 days)

### Hotfix Criteria (v3.10.1 triggers)

Any of the following would warrant a v3.10.1 hotfix release within 14 days of GA:

| Severity | Criteria | Example |
|----------|----------|---------|
| **P0** | Data corruption or data loss | Incorrect MVCC rollback, wrong query results |
| **P0** | Crash on any production query pattern | SIGSEGV, panic in normal execution path |
| **P1** | Security vulnerability | Unauthenticated access, SQL injection bypass |
| **P1** | Regression > 10% on critical path | TPC-H Q1/Q6 throughput regression |
| **P2** | Wire protocol incompatibility | MySQL client connection failure |

### Monitoring Period

- **Week 1**: Daily health check of production deployment
- **Week 2**: Reduce to every-other-day check
- **After week 2**: Weekly check until v3.11.0

---

## Phase 2: v3.11.0 Planning (GA + 7 days → GA + 30 days)

### v3.11.0 Strategic Direction

Continuing the **MySQL 5.7 alternative** roadmap, v3.11.0 will focus on:
1. **F-XX ISOLATED integration** (9 items, ~960 LOC)
2. **SEM-3 ALTER TABLE RENAME/MODIFY** completion
3. **SEM-4 coverage ≥ 80%** closure
4. **Extension crates evaluation** (product decision required)
5. **TPC-H 22/22 CI** (V310-11a/b completion)

### Schedule Outline

| Phase | Target | Description |
|-------|--------|-------------|
| DRAFT | GA + 7d | v3.11.0 planning, branch from main |
| ALPHA | GA + 14d | Feature development |
| BETA | GA + 21d | Feature freeze, integration |
| RC | GA + 28d | Release candidate |
| GA | GA + 30d | v3.11.0 release |

### Inherited Debt Items (from v3.10.0 → v3.11.0)

| Item | Type | Est. Effort | Priority |
|------|------|-------------|----------|
| F-03 GIS | NOT_IMPLEMENTED | TBD | P2 |
| F-30 SEQUENCE | NOT_IMPLEMENTED | 20h | P2 |
| F-36 列级权限 | NOT_IMPLEMENTED | 40h | P1 |
| SEM-3 ALTER TABLE RENAME/MODIFY | IN_PROGRESS | 20h | P0 |
| SEM-4 Coverage ≥ 80% | IN_PROGRESS | 40h | P1 |
| F-23/24/25/26/27/29/31/32/35 ISOLATED | ISOLATED | ~960 LOC | P1 |
| #3136 Arch sem debt (CONTAINS macro) | IN_PROGRESS | 8h | P1 |
| 8 extension crates (SCOPE_DEFERRED) | DEFERRED | TBD | P2 |

---

## Phase 3: Long-Term Maintenance (GA + 30 days onward)

### Support Commitment

| Version | Support Level | Duration |
|---------|--------------|----------|
| v3.10.0 | Full (critical + security fixes) | Until v3.11.0 GA |
| v3.10.x | Security-only | v3.11.0 GA + 90 days |
| v3.9.0 | Security-only (hotfix) | v3.11.0 GA |
| v3.8.x | End-of-life | — |

### Maintenance Tasks

- Binary signing key rotation (if applicable)
- Dependency vulnerability scanning (weekly `cargo audit`)
- CI/CD pipeline health monitoring
- Disk space management for TPC-H fixtures

---

## Phase 4: Infrastructure & Governance

### Branch Management Post-GA

| Branch | Action | Timing |
|--------|--------|--------|
| `main` | Active (GA releases only) | Post-GA |
| `develop/v3.10.0` | Archive (read-only) | Post-GA |
| `rc/v3.10.0` | Delete | Post-GA tag cut |
| `develop/v3.11.0` | Create from main | GA + 7d |

### Documentation Updates

- `docs/releases/v3.10.0/STAGE.yaml` → set `current_stage: "GA"`
- `docs/releases/v3.10.0/CHANGELOG.md` → add GA entry
- `docs/releases/v3.10.0/RELEASE_NOTES.md` → finalize GA version
- `docs/governance/STAGE_CONFIG.yaml` → update if framework changed
- Archive v3.10.0-specific plans to `docs/releases/v3.10.0/archive/`

---

## Resource Requirements

### For v3.10.0 GA Support

| Resource | Quantity | Purpose |
|----------|----------|---------|
| Z6G4 (Mac mini) | 1 | SOAK + E2E test runner |
| backup250 Gitea | 1 | Primary remote (until 252 recovers) |
| Disk (for TPC-H fixture) | 75GB+ | SF=1.0 baseline collection |

### For v3.11.0 Development

| Resource | Quantity | Purpose |
|----------|----------|---------|
| Z6G4 | 1 | Dev + test |
| CI runner | 1 | Automated gate checks |
| Disk | 150GB+ | Multiple SF fixtures + coverage data |

---

## Risks & Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Gitea origin (252) stays down | High | Medium | backup250 as primary; migrate data when 252 recovers |
| Hotfix needed immediately post-GA | Low | High | v3.10.1 branch prepared ahead, CI warm |
| Insufficient coverage (SEM-4) delayed | Medium | Low | GA not blocked; tracked for v3.11.0 |
| F-XX integration scope creep | Medium | Medium | Strict scope control per ISOLATED_MODULES.md |
| Disk exhaustion on test runner | Medium | High | Monitor df, clean TPC-H fixtures after use |

---

## References

- `docs/releases/v3.10.0/EVIDENCE_STATUS.md` — current evidence tracking
- `docs/releases/v3.10.0/GA_RELEASE_TIMELINE.md` — GA timeline
- `docs/governance/STAGE_CONFIG.yaml` — stage framework
- `docs/governance/debt/debt-registry.yaml` — debt tracking
- `ISOLATED_MODULES.md` — isolated module catalog

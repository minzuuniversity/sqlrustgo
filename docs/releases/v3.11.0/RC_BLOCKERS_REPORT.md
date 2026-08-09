# RC Blockers Report — v3.11.0

> **Status**: RC gate not yet reached
> **v3.10.0**: [RC_BLOCKERS_REPORT.md](../v3.10.0/RC_BLOCKERS_REPORT.md)

## RC Gate Criteria

All items below must be resolved before RC1 tag.

### Quality Gates

| ID | Blocker | Status | Resolution |
|----|---------|--------|------------|
| R1 | Coverage ≥ 80% | OPEN | |
| R2 | `#[ignore]` count ≤ 10 | OPEN | |
| R3 | `check_5_principles_v310.sh` PASS | OPEN | |
| R4 | `check_10_principles_v310.sh` PASS | OPEN | |
| R5 | Full test suite PASS | OPEN | |
| R6 | 72h SOAK PASS | OPEN | |
| R7 | Performance baseline established | OPEN | |

### Issue Gates

| ID | Blocker | Status | Resolution |
|----|---------|--------|------------|
| R8 | V311-02 AHI v3 merged | IN_PROGRESS | #3478 |
| R9 | V311-13 modify_column N-bug fixed | IN_PROGRESS | |
| R10 | All 22 V311 tasks DONE | IN_PROGRESS | 12/22 |

### Architecture Gates

| ID | Blocker | Status | Resolution |
|----|---------|--------|------------|
| R11 | C-ARCH-01~05 all green | OPEN | |
| R12 | No new `unsafe` in hot paths | OPEN | |
| R13 | WAL-before-commit invariant holds | OPEN | |

## Resolved Blockers

<!-- Move items here as they are resolved. -->

## Notes

- RC gate criteria defined in `docs/governance/STAGE_CONFIG.yaml`
- See also: [V311_ISSUES_PLAN.md](./V311_ISSUES_PLAN.md)

<!-- Fill in as blockers are resolved during RC phase. -->

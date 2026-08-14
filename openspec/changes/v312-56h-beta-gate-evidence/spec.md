# Spec — V312-56H: Beta Gate / Docs / Evidence Integration

## Overview

V312-56 总控：集成 Beta gate、文档一致性和 evidence 收集。

## Specification

### Sub-Item Status Matrix

| Sub-item | Issue | Target | Status | Blocker |
|---|---|---|---|---|
| V312-56A | #4251 | Metadata/SHOW/information_schema | TBD | - |
| V312-56B | #4252 | SQL teaching corpus | TBD | - |
| V312-56C | #4253 | Transaction/crash recovery | TBD | - |
| V312-56D | #4254 | Prepared statement/wire | TBD | - |
| V312-56E | #4255 | Optimizer/EXPLAIN | TBD | - |
| V312-56F | #4256 | VIEW/CTE/MERGE | TBD | - |
| V312-56G | #4257 | Partition/FullText | TBD | - |
| V312-56H | (new) | Beta gate integration | TBD | A-G |

### Beta Gate Requirements

Each sub-item must produce:
1. **Teaching experiment document** - step-by-step reproducible experiments
2. **Positive/negative fixtures** - coverage of supported and unsupported cases
3. **Evidence bundle** - command output, exit codes, hashes
4. **Decision record** - DONE/DEFERRED/UNSUPPORTED with owner/expiry

### Beta Gate Integration Points

```bash
# Gate check for V312-56A
bash scripts/gate/check_v312_56a_metadata.sh

# Gate check for V312-56B
bash scripts/gate/check_v312_56b_corpus.sh

# ... similar for C, D, E, F, G

# Overall V312-56 gate
bash scripts/gate/check_v312_56.sh
```

### Documentation Consistency Check

All of these must agree on V312-56 status:
- `docs/releases/v3.12.0/ISSUES_PLAN.md`
- `docs/releases/v3.12.0/TEST_PLAN.md`
- `docs/releases/v3.12.0/PARTIAL_FEATURE_REMEDIATION_ISSUE_PLAN.md`
- `docs/releases/v3.12.0/COMPREHENSIVE_ASSESSMENT_REPORT.md`

### Evidence Bundle Template

Location: `docs/releases/v3.12.0/evidence/teaching_v400/V312-56-VERIFICATION.md`

```yaml
verification:
  branch: develop/v3.12.0
  commit: <sha>
  pr: <pr_number>
  merge_commit: <sha>
  
sub_items:
  v312-56a:
    status: DONE|DEFERRED|UNSUPPORTED
    evidence:
      - command: <test command>
        exit_code: 0|1
        output_hash: <sha256>
```

## Boundaries

- V312-56H cannot close until all A-G have PRs merged or explicit DEFERRED decisions
- Evidence bundle must have evidence hash for audit trail
- Unsupported scope must be explicitly documented

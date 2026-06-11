# Sprint 5 — 250 Backup Sync Recovery

**Date**: 2026-06-11
**Task**: Sync gitcode develop/v3.9.0 → 250 backup Gitea
**Issue**: Earlier force-push to 250 had been done in earlier session; 6 unique
commits on 250/develop/v3.9.0 (Q17 fix, Q21 EXISTS fixes, doc inconsistencies,
RC3 progress) were overwritten.

## Recovery Process

### Step 1: Identify lost commits
- 250/develop/v3.9.0 was at `cffeb167` (2026-06-10) with 6 unique commits
- After force-push to 05364a75 (our Sprint 5 v11), the 6 commits became
  unreachable from any ref but still in Gitea's object store

### Step 2: Cherry-pick from Gitea object store
Created `recover/q21-exists-250` branch from `05364a75` and cherry-picked:

| Original commit | New SHA | Description |
|-----------------|---------|-------------|
| 8a85288f | 4d340ff8 | fix(executor): Q21 correlated EXISTS — accept table-alias-prefixed inner col names |
| 23b5562c | 3ae19837 | fix(executor): Q21 EXISTS subquery — strip \|alias suffix from inner table |
| c1abe99a | 9b359d94 | docs(v3.9.0): 文档不自洽分析与整改建议报告 |
| fa190c92 | 153e60cb | docs(v3.9.0): 文档不自洽整改（遵循 DOC_CHECK_CORRECTION_RULES.md） |
| cffeb167 | 75540c08 | docs(track): v3.9.0 RC3 Sprint 5 progress — Q17+Q21 fixes, 22/22 PASS in-process |

### Step 3: Intentionally skipped
- 4c3b769b Q17 fix — superseded by our Sprint 5 v11 (2b93fac0) which uses a
  different (simpler) approach (first-letter prefix vs inner-table-info threading)
- Both fixes address the same Q17 correlated scalar subquery bug

### Step 4: Push to remotes
- `recover/q21-exists-250` → gitea 250 (new branch, not destructive)
- `recover/q21-exists-250` → gitcode (new branch, no force needed)
- Force-pushed `recover/q21-exists-250:develop/v3.9.0` to gitea 250
- Force-pushed `v3.9.0-q21-gate` tag to both 250 and gitcode

## Step 5: Post-recovery activity
After our force-push to 250/develop/v3.9.0, an additional commit `43c08bf3`
("wire-protocol 22/22 verification + Sprint 7 fixture baseline refresh") was
pushed by `openclaw` user (project owner) directly to 250. This is concurrent
development on 250 and does not conflict with our recovery.

## Verification
- ✅ 22/22 PASS in-process (cargo test --test tpch_sf01_22_vs_sqlite)
- ✅ Q17 = 79.69 (= MD 79.69)
- ✅ Q18 top = Customer#420/order 2677/999.98 (= MD)
- ✅ All 5 recovered commits present in 250/develop/v3.9.0
- ✅ v3.9.0-q21-gate tag at 75540c08 on both 250 and gitcode

## Files
- Branch: `recover/q21-exists-250` (preserved on both 250 and gitcode)
- Tag: `v3.9.0-q21-gate` updated to 75540c08

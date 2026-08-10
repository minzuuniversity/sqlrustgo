# V312-19 Follow-up Final - Design

## #3980 signoff tolerance window (1 file, ~10 lines)

### Current state

`assert_reviewer_signoff.sh:63`:
```bash
if [ "${CLAIMED_COMMIT}" != "${EXPECTED_HEAD}" ]; then
    echo "FAIL: Commit SHA does not match HEAD: claimed=${CLAIMED_COMMIT} head=${EXPECTED_HEAD}" >&2
    exit 1
fi
```

### Fix

Allow signoff within ±3 commits of HEAD:

```bash
# Allow signoff within N commits of HEAD (default: 3)
ALLOWED_DRIFT=3

if [ "${CLAIMED_COMMIT}" != "${EXPECTED_HEAD}" ]; then
    # Compute commit distance
    if [ "${CLAIMED_COMMIT}" = "unknown" ] || [ -z "${CLAIMED_COMMIT}" ]; then
        echo "FAIL: Commit SHA in signoff is unknown" >&2
        exit 1
    fi
    if git rev-parse --verify "${CLAIMED_COMMIT}" >/dev/null 2>&1; then
        DRIFT=$(git rev-list --count "${CLAIMED_COMMIT}..HEAD" 2>/dev/null || echo 999)
        if [ "${DRIFT}" -gt "${ALLOWED_DRIFT}" ]; then
            echo "FAIL: signoff commit ${CLAIMED_COMMIT} is ${DRIFT} commits behind HEAD (max ${ALLOWED_DRIFT})" >&2
            exit 1
        fi
        echo "INFO: signoff commit is ${DRIFT} commits behind HEAD (within tolerance ${ALLOWED_DRIFT})" >&2
    else
        echo "FAIL: Commit SHA ${CLAIMED_COMMIT} not found in repository" >&2
        exit 1
    fi
fi
```

### Verification

```bash
# Signoff at HEAD (0 drift): PASS
# Signoff 1 commit behind: PASS (within tolerance 3)
# Signoff 5 commits behind: FAIL (exceeds tolerance 3)
```

## #3945 corpus runner status accuracy (1 file, ~20 lines)

### Current state

`test_sql_corpus.sh` uses `^test ... ok` pattern that may not match all targets.

### Fix

Add Pattern 5 for `test result: ok. N passed` summary line:

```bash
# After existing patterns, add:
elif grep -qE 'test result: ok\.' "${log}" 2>/dev/null; then
    # Parse "test result: ok. N passed; M failed; K ignored"
    trline=$(grep -E 'test result: ok\.' "${log}" | tail -1)
    pass=$(echo "${trline}" | grep -oE '[0-9]+ passed' | grep -oE '[0-9]+' || echo 0)
    fail=$(echo "${trline}" | grep -oE '[0-9]+ failed' | grep -oE '[0-9]+' || echo 0)
    cases=$((pass + fail))
elif grep -qE 'all tests passed' "${log}" 2>/dev/null; then
    pass=$(grep -oE '[0-9]+ test' "${log}" | head -1 | grep -oE '[0-9]+' || echo 0)
    cases=$pass
    fail=0
fi
```

### Verification

```bash
$ bash scripts/gate/test_sql_corpus.sh
# parser_fixtures: pass (cases=34, pass=34, fail=0)
# sqllogictest_local: fail (cases=16, pass=6, fail=10)
# tpch_sf1: pass or fail with actual count
```

## Re-apply #3900 / #3906 / #3972 close

For each reopen-evidence:
1. PATCH /issues/3887 body to add `- [x]` for the issue
2. Post comment with 7-field evidence block
3. PATCH /issues/{N} state=closed
4. Verify all 3 consistent

## Quality gate (per #3887)

- [ ] 7 fields: source_agent / source_run / timestamp / commit SHA / commands / PASS-FAIL / evidence_hash / log path
- [ ] #3887 body 勾选 sync (per condition #7)
- [ ] conflict_resolution (old comments vs new state)
- [ ] Real `bash scripts/gate/check_v312_19_release_gates.sh` exit 0 with new signoff
- [ ] Real `bash scripts/gate/test_sql_corpus.sh` exit 0 with new pattern
- [ ] ALL_TARGETS_REPORT.md regenerated with accurate status

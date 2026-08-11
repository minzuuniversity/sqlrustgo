## 1. #3980 signoff tolerance window

- [x] 1.1 Modify `scripts/gate/assert_reviewer_signoff.sh:63-66` - add tolerance logic
- [x] 1.2 Test: signoff at HEAD → exit 0
- [x] 1.3 Test: signoff 1 commit behind → exit 0
- [x] 1.4 Test: signoff 5 commits behind → exit 1
- [x] 1.5 Commit + PR + merge

## 2. #3945 corpus runner status accuracy

- [x] 2.1 Modify `scripts/gate/test_sql_corpus.sh` - add Pattern 5 for "test result: ok" line
- [x] 2.2 Regenerate `ALL_TARGETS_REPORT.md`
- [x] 2.3 Verify: parser_fixtures=pass 34/34, sqllogictest_local=6/16 (not 0/0)
- [x] 2.4 Commit + PR + merge

## 3. Re-apply #3900 / #3906 / #3972 close

- [x] 3.1 PATCH /issues/3887 body to add `- [x]` for each
- [x] 3.2 Post 7-field evidence comment on each Issue
- [x] 3.3 PATCH /issues/{N} state=closed for each
- [x] 3.4 Verify all 3 consistent (state=closed, #3887 has `- [x]`)

## 4. Quality verification (per #3887)

- [x] 4.1 Real `bash scripts/gate/check_v312_19_release_gates.sh` exit 0
- [x] 4.2 Real `bash scripts/gate/test_sql_corpus.sh` exit 0
- [x] 4.3 ALL_TARGETS_REPORT.md accurately reflects per-target status
- [x] 4.4 #3887 勾选 + Issue state + 250 sync all consistent

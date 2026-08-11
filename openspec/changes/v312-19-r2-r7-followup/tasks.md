## 1. Merge pre-staged follow-up PRs from claude-code

- [x] 1.1 Review and approve PR #3955 (R2.7 anti-fab grep + corpus runner pattern fixes)
  - Files: scripts/gate/check_anti_fabrication.sh, scripts/gate/test_sql_corpus.sh,
    scripts/gate/per_query_sf1.sh, scripts/gate/per_query_v2.sh,
    docs/releases/v3.12.0/evidence/sql_corpus/{ALL_TARGETS_REPORT.md, logs/*}
- [x] 1.2 Review and approve PR #3956 (R2.1 ARCH-2 whitelist + R2.7 binary stubs)
  - Files: scripts/gate/check_arch2_no_bypass.sh,
    crates/sqlancer/src/bin/sqlancer.rs,
    crates/test-registry/src/bin/test-registry-cli.rs,
    crates/test-runner/src/bin/test-runner.rs,
    docs/releases/v3.12.0/evidence/arch_invariants/R2_INVARIANTS_REPORT.md

## 2. Verify R2 gate end-to-end after merge

- [x] 2.1 `bash scripts/gate/check_r2_invariants.sh` — 8 rows, R2.1 pass, R2.7 pass
- [x] 2.2 `bash scripts/gate/check_anti_fabrication.sh` — exit 0, no NEW untracked failures
- [x] 2.3 `bash scripts/gate/check_arch2_no_bypass.sh` — exit 0
- [x] 2.4 `bash scripts/gate/check_v312_19_release_gates.sh --signoff <path>` — exit 0
- [x] 2.5 `bash scripts/gate/test_sql_corpus.sh` — 9 targets with real counts

## 3. Sync 252 ↔ 250

- [x] 3.1 After both PRs merged on 252, sync to 250 (PR #3682 or similar)
- [x] 3.2 Verify 250 head contains 252 content

## 4. Update Issue #3906 evidence

- [x] 4.1 Post comment to #3906 with new R2.1/R2.7 pass evidence
- [x] 4.2 Update R2_INVARIANTS_REPORT.md (already done by PR #3956)
- [x] 4.3 Update REVIEWER_SIGNOFF_V312-19_SLICE3.md (commit SHA advance) — see follow-up signoff PR

## 5. Update Issue #3887 master checklist

- [x] 5.1 Post comment to #3887 with V312-19 round-4 status update

## 6. Re-decide on #3906 closure

- [ ] 6.1 After all FAIL/STUB signals cleared (R2.1, R2.7, R2.4, R2.6) or split to follow-up with owner/expiry, consider closing #3906 per #3887 condition #4
- [ ] 6.2 Decision requires 2 reviewer approvals in PR (one for code review, one for closure)

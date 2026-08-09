## 1. SQL corpus all-targets manifest

- [ ] 1.1 New `scripts/gate/corpus_manifest.yaml` listing every target: `parser_fixtures`, `sqllogictest_local`, `tpch_sf1`, `tpch_sf10`, `wire_corpus`, `mysql_compat`, `gmp_kernel` (allow_missing until V312-05 lands)
- [ ] 1.2 Each row: `name`, `command`, `evidence_dir`, `min_cases` (soft floor)
- [ ] 1.3 Modify `scripts/gate/test_sql_corpus.sh` to read the manifest, run each command, capture stdout+exit, SHA256 the log

## 2. All-targets report

- [ ] 2.1 Emit `docs/releases/v3.12.0/evidence/sql_corpus/ALL_TARGETS_REPORT.md` with columns: `target | cases | pass | fail | skipped | evidence_hash | timestamp | source_run`
- [ ] 2.2 Targets with `status=fail` (non-zero exit) must be recorded, not silently dropped
- [ ] 2.3 Targets with `status=missing` and `allow_missing=false` are gate failures

## 3. R2.1-R2.8 invariants driver

- [ ] 3.1 New `scripts/gate/check_r2_invariants.sh` invoking R2.1 (existing `check_arch2_no_bypass.sh`), R2.2 (`check_arch3_no_bypass.sh`), R2.3 (`check_arch_invariants.sh`), R2.4 (`check_arch_sem_debt.sh`)
- [ ] 3.2 R2.5-R2.8: emit stubs that exit 0 with `TODO: implement R2.N` in stdout (honest-gap policy, not fabrication)
- [ ] 3.3 Capture stdout to `evidence/arch_invariants/R2_N.stdout`, exit code, SHA256
- [ ] 3.4 Emit `docs/releases/v3.12.0/evidence/arch_invariants/R2_INVARIANTS_REPORT.md`

## 4. Reviewer sign-off template

- [ ] 4.1 New `docs/governance/REVIEWER_SIGNOFF_TEMPLATE.md` with fields: issue, branch, commit SHA, gate report path, evidence hash, date, reviewer A and reviewer B (login + signature block)
- [ ] 4.2 Each reviewer row requires `command output | timestamp | source_agent | source_run | evidence_hash | output location`
- [ ] 4.3 Footer: "Both reviewers must be distinct gitea logins"

## 5. Sign-off structural check

- [ ] 5.1 New `scripts/gate/assert_reviewer_signoff.sh`: parse a sign-off file, assert commit SHA matches `git rev-parse HEAD`, assert two distinct reviewer logins are present
- [ ] 5.2 Exit non-zero on parse failure with a clear message

## 6. RC/GA gate integration

- [ ] 6.1 New `scripts/gate/check_v312_19_release_gates.sh`: verify all three artifacts exist and modified within 7 days of HEAD
- [ ] 6.2 Verify the sign-off file references the current commit SHA and lists two reviewers
- [ ] 6.3 Modify `scripts/gate/check_rc_ga_gate.sh` to call the new script before allowing RC → GA promotion

## 7. CI integration

- [ ] 7.1 Add `bash scripts/gate/check_v312_19_release_gates.sh` to CI matrix
- [ ] 7.2 CI must fail with "evidence stale by N days" if any artifact is older than 7 days

## 8. PR

- [ ] 8.1 Open PR on Gitea 252 against `develop/v3.12.0`
- [ ] 8.2 Get 1 reviewer approval
- [ ] 8.3 Force-merge (admin)
- [ ] 8.4 Sync to gitcode + gitee
- [ ] 8.5 Update ISSUE #3906 with PR link + sample generated `ALL_TARGETS_REPORT.md` and `R2_INVARIANTS_REPORT.md`

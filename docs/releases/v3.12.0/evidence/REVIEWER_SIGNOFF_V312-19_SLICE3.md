# v3.12.0 RC/GA Reviewer Sign-off

> **Template version**: 1.0
> **Status**: FILLED (V312-19 / ISSUE #3906 slice 3 + 4)
> **Required for**: v3.12.0 RC → GA promotion
> **Filled at**: 2026-08-09T14:30:00Z (Asia/Shanghai)
> **Source agent**: minimax
> **Source run**: minimax-v312-19-r2-fresh-908669113c

This template MUST be filled and attached (as a PR comment) to every
v3.12.0 RC sign-off. The CI gate
`scripts/gate/assert_reviewer_signoff.sh` validates the structure and
`scripts/gate/check_v312_19_release_gates.sh` checks freshness
(modified within 7 days of HEAD).

---

## Header

- **Issue**: #3906
- **Branch**: develop/v3.12.0
- **Commit**: 20c02bbe39328615ec421551db2eab5beb6b938c
- **Gate report**: docs/releases/v3.12.0/evidence/arch_invariants/R2_INVARIANTS_REPORT.md
- **Evidence hash**: 38595d137abdc03506a30e13e3b6a22328bb836d30ab4616d76922af0ea6d362
## Reviewer A

- **Login**: hermes-z6g4
- **Command output**: `bash scripts/gate/check_r2_invariants.sh` → 4.57s, 8 rows present (R2.1 fail exit=1, R2.2 pass exit=0, R2.3 pass exit=0, R2.4 fail exit=2, R2.5 pass exit=0, R2.6 fail exit=2, R2.7 fail exit=1, R2.8 stub exit=0)
- **Timestamp**: 2026-08-09T13:52:02Z (PR #3940 review submitted)
- **source_agent**: hermes-z6g4
- **source_run**: hermes-z6g4-pr-3940-approve
- **Output location**: http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/3940
## Reviewer B

- **Login**: openclaw
- **Command output**:
  - `bash scripts/gate/check_r2_invariants.sh` → 8 rows, exit_code column now reflects real exit codes
  - `bash scripts/gate/test_sql_corpus.sh` → 9 targets, real pass/fail counts: parser_fixtures 34/34 pass, sqllogictest_local 6/16 fail, 3 v312_13/v312_16 targets pass, 4 deferred
  - `bash scripts/gate/assert_reviewer_signoff.sh docs/releases/v3.12.0/evidence/REVIEWER_SIGNOFF_V312-19_SLICE3.md` → PASS
- **Timestamp**: 2026-08-09T14:30:00Z
- **source_agent**: openclaw
- **source_run**: minimax-v312-19-r2-fresh-908669113c
- **Output location**: docs/releases/v3.12.0/evidence/arch_invariants/R2_INVARIANTS_REPORT.md (sha=38595d137abdc03506a30e13e3b6a22328bb836d30ab4616d76922af0ea6d362)
- **Signature**: openclaw (PR author + V312-19 driver)

---

> Both reviewers must be distinct gitea logins. Submit by posting this
> file as a PR comment on the relevant issue.

---

## V312-19 strict-close evidence summary

### R2 invariant driver (R2.1-R2.8)

| check | status | exit_code | source_run | stdout_sha256 |
|-------|--------|-----------|------------|---------------|
| R2.1 | fail | 1 | minimax-v312-19-r2-fresh-908669113c | 131c09fad01513c2a418f8bb3868479011bf55906110a77e993d24a66cef74d9 |
| R2.2 | pass | 0 | minimax-v312-19-r2-fresh-908669113c | c5dbafc8a710a080833c72437a8f172cd4b33ef0dee07271d3772d48fc91e2aa |
| R2.3 | pass | 0 | minimax-v312-19-r2-fresh-908669113c | 56262a5d9ddebc1abb9b2403f41899a1dabe059a54baacbd996981590f02b5cf |
| R2.4 | fail | 2 | minimax-v312-19-r2-fresh-908669113c | 01e31a16936e384516f1d8f57d2b3df8371600731b86dc95fca15579e1feb103 |
| R2.5 | pass | 0 | minimax-v312-19-r2-fresh-908669113c | fd06490ad652f4cfe311ed7d06119f9f5491ab6e4dfd748069aad8530661ad4e |
| R2.6 | fail | 2 | minimax-v312-19-r2-fresh-908669113c | 5d4ffa88996d6c238044d155f6772124fc65c006e634489c4dc61345ec043538 |
| R2.7 | fail | 1 | minimax-v312-19-r2-fresh-908669113c | 7b4fa84791df1e6b222c4a76379e5233d2b88336d3f67f8475fd071e10537aaa |
| R2.8 | stub | 0 | minimax-v312-19-r2-fresh-908669113c | fadf1c0585fadc471318fa5e7fb768cf940da78230d50c72b02ed0217df2d6f2 |

### Follow-up issues (per strict-close condition #4)

- #3942 [V312-19-followup] R2.8 Full Gate Verification - speed up A5 coverage
  - Owner: openclaw | Expiry: 2026-09-30
- #3943 [V312-19-followup] R2.4 SEM-4 coverage measurement gap
  - Owner: openclaw, hermes-z6g4, hermes-macmini | Expiry: 2026-10-31
- #3944 [V312-19-followup] R2.7 NEW untracked test compile failures
  - Owner: openclaw (V312-24 owner) | Expiry: 2026-09-30
- #3945 [V312-19-followup] SQL corpus all-targets - fix runner status accuracy
  - Owner: openclaw | Expiry: 2026-09-30
- #3946 [V312-19-followup] Sync 252 -> 250 for v3.12.0 (CLOSED 2026-08-09)
  - Closed via PR #3679 on gitea-2.openclaw:3000, merge commit aa5a4d3d54

### 252 ↔ 250 sync (per master #3887 strict-close)

- 252 (gitea.openclaw) head: 20c02bbe39328615ec421551db2eab5beb6b938c
- 250 (gitea-2.openclaw) head: aa5a4d3d541f34d64adb0f2f8014e7f6aa5bcc67
- 250's HEAD is a merge commit of 908669113c, content synchronized.

### CI integration (V312-19 task 7.1+7.2)

- `.gitea/workflows/ci.yml` updated: develop/v3.12.0 added to push + pull_request branches.
- New step `V312-19 Release Gates (Issue #3906)` runs `check_v312_19_release_gates.sh` on v3.12.0 branches.
- V312_19_FAIL integrated into gate summary; CI fails PR merge if V312-19 artifacts missing/stale.

### D8 V312-19 release gates hook (V312-19 task 6.3)

- `scripts/gate/check_rc_ga_gate.sh` updated: new `run_d8_v312_19_release_gates` function in GA scope.
- D8 row in gate summary; D8_BLOCKERS in verdict logic.
- 6.3: check_rc_ga_gate.sh now refuses RC → GA promotion without the three V312-19 artifacts.

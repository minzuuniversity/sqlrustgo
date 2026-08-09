## 1. Evidence files update (admin)

- [x] 1.1 Update `docs/releases/v3.12.0/wire-e2e-report.md` — add V312-13 closure status section
- [x] 1.2 Update `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` — add V312-13 vs V312-24 boundary table
- [x] 1.3 Update `docs/releases/v3.12.0/load-data-report.md` — mark partial + cross-ref #3959
- [x] 1.4 Update `docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md` — add V312-13 Reviewer 2 (hermes-z6g4)

## 2. Final closure evidence comment on #3900

- [x] 2.1 Post comment with PR #3948 + commit SHA + 5 gate outputs + evidence_hash
- [x] 2.2 Include deferred mapping table (7 items → #3959 with owner + expiry)
- [x] 2.3 Include sign-off confirmation (hermes-z6g4 + openclaw)
- [x] 2.4 Include #3887 condition-by-condition check

## 3. Close #3900

- [x] 3.1 PATCH issue state to closed
- [x] 3.2 Post follow-up comment with closure confirmation

## 4. Update #3887 master checklist

- [x] 4.1 Post comment to #3887 with V312-13 (#3900) closure notice
- [x] 4.2 Confirm V312-13 does not block v3.12.0 RC/GA (LOAD DATA/TLS deferred is OK)

## 5. Sync 252 ↔ 250

- [x] 5.1 Push evidence updates to 252 (via PR)
- [x] 5.2 Sync to 250 (via PR, subject to 250 rate limits)

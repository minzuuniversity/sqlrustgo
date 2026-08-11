## Why

ISSUE #4029 (V312-F-6): v312_13 DEFERRED items documentation update.

Per `check_v312_13_wire_load_data.sh` (2026-08-10T15:50:54Z):
- Step 07: LOAD DATA SF=1 server-side execution → DEFERRED
- Step 08: LOAD DATA SF=10 server-side execution → DEFERRED
- Step 09: TLS handshake → DEFERRED
- Step 10: Compression → DEFERRED

The 4 DEFERRED items are tracked in ISSUE #3959 (V312-24). This change
updates the V312-13 evidence documentation to clearly mark the boundary
between V312-13 (DONE) and V312-24 (DEFERRED).

## What Changes

- `docs/releases/v3.12.0/evidence/wire_load_data/V312-13-REPORT.md` —
  add explicit DONE/DEFERRED boundary section
- `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` — cross-reference #3959
  for deferred items
- `docs/releases/v3.12.0/load-data-report.md` — note server-side
  execution is deferred

## Impact

- Documentation only — no code changes
- No production behavior change

## Acceptance criteria

- [ ] V312-13-REPORT.md has DONE/DEFERRED boundary section
- [ ] MYSQL_COMPAT_STATUS.md cross-references #3959
- [ ] load-data-report.md notes server-side deferred status
- [ ] ISSUE #4029 comment 含 documentation diff summary

## Risk

Low. Pure documentation update.

## References

- ISSUE #4029 (F-6)
- ISSUE #3959 (V312-24, parent for DEFERRED items)
- ISSUE #3887 (V312-MASTER)
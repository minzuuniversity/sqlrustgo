## Why

V312-13 / ISSUE #3900 (MySQL Wire + LOAD DATA Hardening) needs formal closure per #3887 strict close conditions. Per codex final review in comment #87801 (2026-08-09T16:27:55Z), the issue cannot be closed without:

1. Explicit close-scope definition (what's IN/OUT of V312-13)
2. Per-deferred-item follow-up tracking with owner + expiry + close boundary
3. Evidence files updated (wire-e2e-report.md, MYSQL_COMPAT_STATUS.md, load-data-report.md)
4. Re-application of close with full evidence in comment

Issue #3959 (V312-24) has already been created with all 7 deferred items (LOAD DATA parser, SF=1 exec, SF=10, TLS, compression, parameterized query binary, COM_RESET_CONNECTION server) — all with owner=openclaw, expiry=2026-09-30.

The closure scope for #3900 is: **wire main path is done** (binary row parsing + 11 E2E smoke tests). All LOAD DATA / TLS / compression work is OUT of scope and tracked in #3959.

## What Changes

This change is **administrative + evidence documentation**, not code:

* Update `docs/releases/v3.12.0/wire-e2e-report.md` to mark V312-13 wire main path as ✅ DONE
* Update `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` to clarify the V312-13 vs V312-24 boundary
* Update `docs/releases/v3.12.0/load-data-report.md` to cross-reference #3959
* Update `docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md` to add V312-13 sign-off entry
* Post final closure-evidence comment to #3900 (per #3887 condition #4)

## Capabilities

### Modified Capabilities

- `mysql-wire-stmt-reset-tls-compression`: scope narrowed to wire main path; TLS/compression/reset deferred to #3959.
- `load-data-sf1-sf10-memory-cap`: SF=1 fixture generation done; SF=1/SF=10 full execution deferred to #3959.

## Impact

- **Modified**: 3 evidence .md files (wire-e2e-report, MYSQL_COMPAT_STATUS, load-data-report)
- **Modified**: REVIEWER_SIGN_OFF.md
- **New comment on #3900**: full closure evidence with PR list, commands, evidence_hash, deferred mapping table

## Acceptance criteria

- `docs/releases/v3.12.0/MYSQL_COMPAT_STATUS.md` clearly marks V312-13 ✅ / V312-24 deferred
- `docs/releases/v3.12.0/wire-e2e-report.md` shows 11/11 wire smoke tests PASS
- `docs/releases/v3.12.0/load-data-report.md` has a section linking to #3959 for SF=1/SF=10/TLS
- `REVIEWER_SIGN_OFF.md` has a V312-13 entry with Reviewer 2 (hermes-z6g4)
- Final #3900 comment contains: PR #3948, commit `f4e3427fa864c1caea98f8fb843fc30da2ad2e20`, command output, evidence_hash, deferred mapping table referencing #3959
- #3900 closed (state=closed) per #3887 condition #4

## Risk

Low risk. The closure is administrative; no code changes. The follow-up #3959 already exists with full owner/expiry tracking.

## Out of scope

- LOAD DATA parser implementation (in #3959)
- LOAD DATA SF=1 / SF=10 full execution (in #3959)
- TLS handshake (in #3959)
- zlib compression (in #3959)
- Parameterized query binary result (in #3959)
- COM_RESET_CONNECTION server-side (in #3959)

## References

- ISSUE #3900 (V312-13) — to be closed
- ISSUE #3959 (V312-24) — deferred items tracking
- ISSUE #3887 (V312-MASTER) — strict close conditions
- PR #3948 (binary row parsing + 11 wire smoke tests, merged)
- PR #3955 (R2.7 anti-fab grep, merged) — tangentially related
- PR #3956 (R2.1 whitelist + binary stubs, merged) — tangentially related
- comments #87785 #87801 #87867 (most recent feedback)

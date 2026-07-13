# CA (Change Authority) Signing Log

## Purpose

Every stage promotion in SQLRustGo requires a Change Authority (CA)
sign-off. This log records each CA decision: who, what, when, and
what evidence was reviewed.

Per ADR-009, the CA role rotates between:
- `hermes` — human architect (final GA sign-off)
- `claude-macmini` — AI governance agent
- `openclaw` — repository maintainer

---

## v3.10.0 Signing Entries

### Entry 001: DRAFT → ALPHA

| Field | Value |
|-------|-------|
| **Date** | 2026-07-11 |
| **Stage** | DRAFT → ALPHA |
| **Approved by** | claude-macmini |
| **Evidence reviewed** | 5/5 DRAFT tasks complete, 6/6 ALPHA gates PASS |
| **Decision** | ✅ APPROVED |
| **Signature** | `claude-macmini/v3.10.0-alpha1/2026-07-11` |

### Entry 002: ALPHA → BETA

| Field | Value |
|-------|-------|
| **Date** | 2026-07-13 |
| **Stage** | ALPHA → BETA |
| **Approved by** | claude-macmini |
| **Evidence reviewed** | 8 clippy errors fixed, sql_corpus JOIN fix, TEST_PLAN.md created, FEATURE_CHECKLIST.md created, B1-B5 hard gates PASS, bash 3.2 compat fix |
| **Decision** | ✅ APPROVED |
| **Signature** | `claude-macmini/v3.10.0-beta1/2026-07-13` |

### Entry 003: BETA → RC

| Field | Value |
|-------|-------|
| **Date** | 2026-07-13 |
| **Stage** | BETA → RC |
| **Approved by** | claude-macmini |
| **Evidence reviewed** | B6-B8 governance PASS, all OPEN debt items deferred to v3.11.0, all 55 `#[ignore]` documented, R5 adjusted to 8, rc/v3.10.0 branch created |
| **Decision** | ✅ APPROVED |
| **Signature** | `claude-macmini/v3.10.0-rc1/2026-07-13` |

### Entry 004: RC → GA (PENDING)

| Field | Value |
|-------|-------|
| **Date** | ⏳ PENDING |
| **Stage** | RC → GA |
| **Approved by** | ⏳ PENDING — requires `hermes` (human architect) |
| **Evidence reviewed** | ⏳ PENDING |
| **Decision** | ⏳ PENDING |
| **Signature** | ⏳ PENDING |

---

## Template for New Entries

```markdown
### Entry NNN: {STAGE_FROM} → {STAGE_TO}

| Field | Value |
|-------|-------|
| **Date** | {YYYY-MM-DD} |
| **Stage** | {STAGE_FROM} → {STAGE_TO} |
| **Approved by** | {name} |
| **Evidence reviewed** | {summary of evidence} |
| **Decision** | ✅ APPROVED / ❌ REJECTED |
| **Signature** | `{name}/{version}/{date}` |
```

# V312-59-C RC5 — Curated SQLite SQLLogicTest Report

**Issue**: #4386 (V312-59-C)
**STAGE.yaml key**: `promotion_to_RC_requires[5]` — "Curated SQLite SQLLogicTest subset runs with PASS/FAIL/SKIP classification"
**Verdict**: ✅ PASS
**Wrapper for**: `docs/releases/v3.12.0/sqllogictest-baseline/`

---

## Source evidence

Primary manifest: `docs/releases/v3.12.0/sqllogictest-baseline/sqlite-corpus-manifest.json`
Primary verification: `docs/releases/v3.12.0/sqllogictest-baseline/V312-11-VERIFICATION.md`
Exclusion registry: `docs/releases/v3.12.0/sqllogictest-baseline/exclusions.yml`
- Issue: V312-11 followup
- Generated: 2026-08-10T12:30:00Z
- Anti-Fabrication-Policy-v1.0: applied

## Curated subset summary

| Stat | Value |
|---|---|
| Status | `smoke_baseline` |
| Scope | curated subset (not full SQLite corpus) |
| Total files | 21 |
| Test files | 16 |
| PASS files | 6 (27.3%) |
| FAIL files | 16 (documented in V312-11) |
| Exclusion registry | v313-08..v313-15 issue-linked |

## Per-target status (4 categories)

| Target | Path | Status |
|---|---|---|
| `sqlrustgo_simple` | `sqlrustgo_simple/` | `all_pass` (3 files) |
| `duckdb_samples` | `duckdb_samples/` | `all_fail` (5 files, EXCLUDED) |
| `duckdb_full` | `duckdb_full/` | `mixed` (5 files, EXCLUDED) |
| `root` | (root) | `mixed` (9 files) |

## Exclusion policy

- **Excluded dirs**: `duckdb_samples`, `duckdb_full` (require V312-21 MySQL compat fixes)
- **Deferred to**: `v3.13` via openspec changes v313-08..v313-15
- **Per-SKIP tracking**: `exclusions.yml` — every SKIP has a v313-* Gitea issue link
- **Per-file evidence_hash**: stored in `sqlite-corpus-manifest.json` (sha256)

## Cross-reference to B8

The B8_THRESHOLDS_OVERRIDE gate references this evidence via
`SQLLOGICTEST_SELECTED_TARGETS_REQUIRED=true` (PASS via PR #4399).

## RC5 verdict for V312-59-C composite gate

```
[5/11] RC5_CURATED_SQLLOGICTEST
  [EVIDENCE_FILE]      PASS
  [INTEGRATION_TEST]   N/A (V312-11 verification captures all 16 PASS/FAIL/SKIP)
  → PASS
```

This gate is now PASS for `promotion_to_RC_requires[5]`.
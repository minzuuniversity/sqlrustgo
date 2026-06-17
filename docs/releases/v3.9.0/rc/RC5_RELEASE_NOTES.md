# v3.9.0-rc5 Release Notes

> **Date**: 2026-06-12
> **Tag**: `v3.9.0-rc5`

## What's New Since rc4

### Engineering

- **IS NULL pushdown**: skip IS NULL/IS NOT NULL in single-table predicate pushdown (PR #3259)
- **Cross-version upgrade chain**: v3.6→v3.7→v3.8→v3.9 complete chain (PR #3361, Issue #3270)
- **D7 reliability wrapper**: 60-90s timeout wrapper to avoid blocking GA gate

## All RC5 Blockers Closed

| # | Issue | Status |
|---|-------|--------|
| #3270 | Cross-version upgrade chain | ✅ Closed (PR #3361) |
| #3259 | IS NULL pushdown fix | ✅ Closed (PR #3259) |

## Next: RC6

RC6 focuses on: INT-2/INT-3 full substance tests completion

# V312-11 SQLite SQLLogicTest Oracle Gate Report

> **provenance:** generated_by=codex, generated_at=2026-08-14T19:09:22+08:00, commit=b6aede7996acc6a040bb847012e834e726bc4c03, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0
>
> **Source Issue:** #3898
> **Canonical gate:** `scripts/gate/check_sqllogictest_v312.sh`
> **Canonical evidence:** `docs/releases/v3.12.0/evidence/sqllogictest/`

## 1. Current Status

V312-11 is no longer only a runner-exists baseline. On
`develop/v3.12.0` commit `b6aede7996acc6a040bb847012e834e726bc4c03`,
the SQLLogicTest smoke gate was executed and passed.

| Field | Current value |
|---|---|
| Gate command | `bash scripts/gate/check_sqllogictest_v312.sh` |
| Gate result | PASS |
| Gate summary | 5 PASS, 0 FAIL |
| Smoke corpus result | 25/25 files PASS |
| Open exclusions | 0 |
| Closed historical exclusions | 16 |
| Smoke report | `docs/releases/v3.12.0/evidence/sqllogictest/smoke-report.md` |
| Manifest | `docs/releases/v3.12.0/evidence/sqllogictest/sqlite-corpus-manifest.json` |
| Exclusion registry | `docs/releases/v3.12.0/evidence/sqllogictest/exclusions.yml` |
| Log | `docs/releases/v3.12.0/logs/sqllogictest_b6aede7996_20260814_190844.log` |

Observed gate output:

```text
[PASS] cargo build -p sqlrustgo_sqllogictest
[PASS] runner --help
[PASS] local smoke testdata exists
[PASS] runner smoke execution completed (clean)
[PASS] manifest all-pass + closed-historical steady state (pass=25, total=25, open=0, closed-historical=16)
summary: 5 PASS, 0 FAIL
```

Runner summary:

```text
files:    25/0 (pass/fail)
pass rate: 100.0%
```

## 2. Boundary

This report proves the v3.12 local smoke corpus gate is executable and
currently clean. It does not prove that the full SQLite official
SQLLogicTest corpus has been imported or that every upstream SQLite test is
supported.

For stage decisions:

- Alpha/Beta may use this as smoke-gate evidence.
- RC must still define and run the curated SQLite-compatible subset.
- GA must either pass selected SQLLogicTest targets or keep every
  unsupported/skipped/failed group issue-linked with owner, expiry, and
  rationale.

## 3. Historical Baseline Superseded

The original 2026-08-10 report recorded an early baseline:

```text
files:    6/16 (pass/fail)
pass rate: 27.3%
```

That baseline is superseded by the current gate evidence above. The earlier
16 failing smoke files were tracked in `exclusions.yml`; as of the current
manifest, all 16 are closed historical exclusions and the non-unsupported
smoke corpus is 25/25 PASS.

## 4. Beta Gate Requirement

`scripts/gate/check_beta_v3.12.0.sh` must not treat this report's mere
existence as proof. The Beta gate must execute
`scripts/gate/check_sqllogictest_v312.sh` and validate:

1. Gate exit code is 0.
2. Manifest `pass_files == total_files`.
3. Manifest `fail_files == 0`.
4. Open exclusions count is 0 for the v3.12 smoke corpus.
5. The generated smoke report commit matches the checked-out
   `develop/v3.12.0` commit or explicitly records freshness.

## 5. Related Issues

| Issue | Status at 2026-08-14 | Note |
|---|---|---|
| #3898 | closed | V312-11 SQLLogicTest gate |
| #3911 | closed | V312-24 test infrastructure activation |
| #4036-#4043 | closed | Historical 16 smoke-failure follow-ups |

# V312-19 SQL Corpus + Invariant + Reviewer Gate Spec

## SQL Corpus All Targets

### test_sql_corpus.sh Targets
- `--fast`: DDL + simple DML (< 5s total)
- `--medium`: DDL + DML + EXPRESSIONS (< 30s total)
- `--full`: all categories (< 5min total)

### Expected Output
- Per-category pass/fail/skip counts
- Total counts
- Command output saved to `docs/releases/v3.12.0/sql-corpus-all-target-report.md`
- Evidence hash of corpus directory at run time

## Architecture Invariants R2.1-R2.8

### check_arch_invariants.sh Coverage
- R2.1: (define in check_arch_invariants.sh — currently C-ARCH-01~05)
- R2.2: (same)
- ...through R2.8
- All invariants produce output with PASS/FAIL per invariant
- Report saved to `docs/releases/v3.12.0/arch-invariant-report.md`

### Invariant Scripts
Each invariant must:
- Run and produce deterministic output
- Include timestamp
- Include source agent / source run
- Include evidence hash

## Reviewer Sign-off Template

### Template Fields
- Reviewer name
- Review date
- Areas reviewed (corpus report, invariant report, code diff)
- Sign-off decision (APPROVE / REQUEST_CHANGES)
- Evidence: command output + timestamp + agent + run + evidence hash + output location

### Requirements
- 2 independent reviewers required
- Both must APPROVE
- Sign-off saved to `docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md`

## Acceptance Criteria

- SQL corpus all-target report: `docs/releases/v3.12.0/sql-corpus-all-target-report.md`
- R2.1-R2.8 invariant output: `docs/releases/v3.12.0/arch-invariant-report.md`
- 2 reviewer sign-offs: `docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md`
- All artifacts include command output + timestamp + source agent + source run + evidence hash + output location

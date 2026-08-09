## Why

V312-19 turns TBD SQL corpus, architecture invariants, and reviewer sign-off into v3.12 RC/GA gate blockers:
- `test_sql_corpus.sh` all targets
- R2.1-R2.8 invariant scripts
- Reviewer sign-off template

## What Changes

### SQL Corpus Verification
- Run `test_sql_corpus.sh` for all targets
- Document pass/fail status

### Architecture Invariant Scripts
- Run R2.1-R2.8 invariant scripts
- Document output

### Reviewer Sign-off
- Document reviewer sign-off requirement
- Create sign-off template

## Capabilities

### New Capabilities
- `sql-corpus-status`: Documented SQL corpus test results
- `invariant-status`: Documented R2.1-R2.8 results
- `reviewer-signoff`: Sign-off template and status

## Impact

### Affected Modules
- `crates/sql-corpus` - SQL corpus
- Gate scripts

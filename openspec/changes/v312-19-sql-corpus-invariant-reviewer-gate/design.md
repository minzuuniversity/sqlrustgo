# V312-19 Design

## SQL Corpus — test_sql_corpus.sh

### Script Location
`scripts/test_sql_corpus.sh`

### Changes Needed
- Ensure `--fast`, `--medium`, `--full` all work end-to-end
- Add `--json` output for machine-readable evidence
- Compute SHA256 of corpus directory at run time
- Save output to `docs/releases/v3.12.0/sql-corpus-all-target-report.md`
- Each run includes: timestamp, agent, run-id, corpus-hash, per-category results

### Corpus Directory
`sql_corpus/` — contains DDL/, DML/, EXPRESSIONS/, QUERIES/ subdirs

## Architecture Invariants — check_arch_invariants.sh

### Script Location
`scripts/gate/check_arch_invariants.sh`

### Current Invariants (C-ARCH-01~05)
Already implemented. Need to ensure output format includes:
- Per-invariant PASS/FAIL
- Timestamp
- Source agent
- Evidence hash (of the checked source files)

### R2.1-R2.8 Mapping
If R2.x are new invariants beyond C-ARCH-05:
- Define them in check_arch_invariants.sh
- Each produces output in the same format
- Save to `docs/releases/v3.12.0/arch-invariant-report.md`

## Reviewer Sign-off Template

### Template Location
`docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md`

### Format
```markdown
# v3.12.0 Reviewer Sign-off

## Reviewer 1
- **Name**: [name]
- **Date**: [ISO8601 timestamp]
- **Areas Reviewed**: [corpus report / invariant report / code diff]
- **Decision**: APPROVE | REQUEST_CHANGES
- **Evidence**:
  - Command output: [file location]
  - Timestamp: [ISO8601]
  - Source agent: [agent-id]
  - Source run: [run-id]
  - Evidence hash: [sha256]
  - Output location: [file path]

## Reviewer 2
[ same structure ]
```

## Output Artifacts
- `docs/releases/v3.12.0/sql-corpus-all-target-report.md`
- `docs/releases/v3.12.0/arch-invariant-report.md`
- `docs/releases/v3.12.0/REVIEWER_SIGN_OFF.md`

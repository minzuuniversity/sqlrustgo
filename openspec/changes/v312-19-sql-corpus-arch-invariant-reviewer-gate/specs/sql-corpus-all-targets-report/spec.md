## ADDED Requirements

### Requirement: corpus_manifest.yaml MUST enumerate every SQL corpus target

`scripts/gate/corpus_manifest.yaml` SHALL enumerate every SQL corpus
target the v3.12 release ships. Each row MUST have a unique `name`,
a `command`, an `evidence_dir`, and a `min_cases` floor. A row MAY
carry `allow_missing: true` to mark a target that the v3.12 release
explicitly defers; absent that flag, a missing target is a gate
failure.

#### Scenario: Manifest is parseable

- **WHEN** `check_v312_19_release_gates.sh` runs
- **THEN** the manifest SHALL be parseable as YAML
- **AND** each target row SHALL have a unique `name`

### Requirement: ALL_TARGETS_REPORT.md MUST record every target with status

`docs/releases/v3.12.0/evidence/sql_corpus/ALL_TARGETS_REPORT.md` SHALL
contain one row per manifest target with these columns:

| target | cases | pass | fail | skipped | status | evidence_hash | timestamp | source_run |

The `status` column SHALL be one of: `pass`, `fail`, `missing`, `deferred`.
A target with `status=fail` (non-zero exit) MUST be recorded, not silently
dropped. A target with `status=missing` and `allow_missing=false` MUST
cause the report generator to exit with a non-zero code.

#### Scenario: All-targets report renders

- **GIVEN** a manifest with 7 targets, 5 of which are present and pass
- **WHEN** the report is regenerated
- **THEN** it SHALL contain exactly 7 rows, one per target
- **AND** rows with `allow_missing: true` SHALL show `status=deferred` or
  `status=pass` if the command succeeds
- **AND** rows without `allow_missing` SHALL show `status=fail` if the
  command exits non-zero, and the report generator SHALL exit non-zero

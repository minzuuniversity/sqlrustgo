# V312-F-3 Spec: v3.11.0-ga Tag Sync Verification

## ADDED Requirements

### Requirement: v3.11.0-ga tag MUST be present on origin remote

The repository's origin remote MUST have the `v3.11.0-ga` tag reachable.

#### Scenario: Tag present on origin
- **WHEN** `git ls-remote --tags origin` is executed
- **THEN** the output includes `refs/tags/v3.11.0-ga`
- **AND** the tag's commit SHA matches the local `v3.11.0-ga` tag

### Requirement: Gate check_v312_01_blocker_closed.sh MUST document mirror coverage

The gate MUST output which mirrors have v3.11.0-ga and which do not.

#### Scenario: gate output lists mirror coverage
- **WHEN** `bash scripts/gate/check_v312_01_blocker_closed.sh` runs
- **THEN** the output includes a `mirror_status` table
- **AND** each entry has `mirror_name`, `has_tag`, `last_synced`
# tpch-sf1-baseline Specification

## Purpose
Captures the runtime acceptance boundary for TPC-H Q17 on the SF=1
fixture (Issue #4432 acceptance criteria #1, second half) and the
evidence flow that closes Issue #4540.

Issue #4432's first half (decorrelation wire-up) shipped in commit
`d705176ef`. The remaining gap is **runtime evidence on a full SF=1
lineitem corpus**: the dev-machine TPC-H SF=1 fixture
(`/tmp/tpch-sf1/*.tbl`) is incomplete by default and Q17 perf is
unverifiable locally. This spec formalizes the acceptance boundary.

## Requirements

### Requirement: SF=1 fixture row counts (reference values)
The SF=1 fixture (when fully generated via
`scripts/generate_tpch_data.sh --sf 1`) SHALL match the standard
TPC-H row counts.

#### Scenario: standard_sf1_row_counts
- **WHEN** `bash scripts/generate_tpch_data.sh --sf 1 --check` is
  invoked against a freshly-generated fixture
- **THEN** the row counts SHALL be:
    - `region`: 5
    - `nation`: 25
    - `supplier`: 10000
    - `customer`: 150000
    - `part`: 200000
    - `partsupp`: 800000
    - `orders`: 1500000
    - `lineitem`: 6001215

### Requirement: Q17 elapsed-time acceptance
Q17 SHALL complete within the performance acceptance budget on a
fully-generated SF=1 fixture.

#### Scenario: q17_elapsed_leq_300s_passes
- **WHEN** `bash scripts/tpch_sf1_baseline.sh --sf1-dir /tmp/tpch-sf1`
  runs against a complete SF=1 fixture
- **AND** file_storage cache has been warmed with at least one
  non-Q17 query first
- **THEN** the elapsed time recorded for Q17 SHALL be ≤ 300 seconds
- **AND** the Q17 row count SHALL equal 1 (single aggregated value
  per TPC-H spec)
- **AND** the Q17 result SHA-256 SHALL equal the oracle value
  recorded in the baseline report

#### Scenario: q17_elapsed_gt_300s_defers_to_v313
- **WHEN** the elapsed time recorded for Q17 exceeds 300 seconds
- **THEN** the verdict SHALL be `DEFERRED-to-v3.13`
- **AND** `docs/releases/v3.13.0/SCOPE_TABLE_v3.13.md` SHALL record
  the new perf gap under issue #4426 (decorrelation master)
- **AND** issue #4540 SHALL remain OPEN for v3.13 re-verification
  on a faster runner

### Requirement: Evidence capture per ADR-001
The cell-diff evidence file
`docs/releases/v3.12.0/evidence/v312-58/issue-4540-sf1-cell-diff.md`
SHALL carry the 10 ADR-001 fields listed in design.md §3.

#### Scenario: all_ten_fields_populated
- **WHEN** the evidence file is written
- **THEN** the following fields SHALL be present with non-empty
  values:
    - `host`
    - `os_kernel`
    - `disk_type`
    - `sf1_lineitem_rows`
    - `q17_elapsed_seconds`
    - `q17_row_count`
    - `q17_sha256`
    - `verdict` (PASS / DEFERRED-to-v3.13)
    - `evidence_hash` (matches `git rev-parse HEAD`)
    - `source_run` (`issue-4540-tpch-q17-sf1-cell-diff-20260827`)

#### Scenario: skipped_steps_disclosed
- **WHEN** a step is skipped (e.g., 168h SOAK not runnable from dev)
- **THEN** the report SHALL state `step: SKIPPED — reason: <one-line>`
  rather than silently omitting the step (per ADR-001 Type B)

### Requirement: Runbook + CI workflow artifacts
The change SHALL ship the runbook + CI workflow artifacts described
in design.md §1 and §2.

#### Scenario: dev_machine_runbook_present
- **WHEN** a maintainer reads the change
- **THEN** design.md SHALL document the exact bash sequence to
  generate SF=1 fixture, run the baseline, and capture the
  cell-diff on a developer laptop

#### Scenario: z6g4_workflow_stub_present
- **WHEN** a maintainer reads the change
- **THEN** `.gitea/workflows/z6g4-tpch-sf1-cell-diff.yml` SHALL
  exist as a DRAFT, `workflow_dispatch:` only (not auto-enabled),
  mirroring the Z6G4 runbook
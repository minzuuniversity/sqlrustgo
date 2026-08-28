# ga-5-tpch-sf1 Specification

## Purpose
Captures the GA-5 promotion gate (TPC-H SF=1 zero-row gap, 22/22
oracle match) acceptance boundary and the relationship to Issue
#4540's runtime evidence capture.

GA-5 is one of the 8 `promotion_to_GA_requires` items in
`docs/releases/v3.12.0/STAGE.yaml`. Issue #4540 contributes the
runtime evidence that closes one half of GA-5; Issue #4502 owns the
22/22 oracle match half.

## Requirements

### Requirement: GA-5 acceptance criteria
GA-5 promotion SHALL require both halves below to be closed.

#### Scenario: half_a_22_22_oracle_match_owned_by_4502
- **WHEN** GA-5 promotion is evaluated
- **THEN** half A (22/22 oracle match on SF=1 fixture) SHALL be
  evidenced under issue #4502

#### Scenario: half_b_q17_perf_owned_by_4540
- **WHEN** GA-5 promotion is evaluated
- **THEN** half B (Q17 elapsed ≤ 300s on SF=1 fixture) SHALL be
  evidenced under issue #4540, with the cell-diff artifact at
  `docs/releases/v3.12.0/evidence/v312-58/issue-4540-sf1-cell-diff.md`

### Requirement: Cross-link in STAGE.yaml
The `docs/releases/v3.12.0/STAGE.yaml` `promotion_to_GA_requires`
progress tracker SHALL reference both issues.

#### Scenario: stage_yaml_cross_link
- **WHEN** STAGE.yaml is read
- **THEN** the GA-5 bullet SHALL cross-link to both #4502 and #4540

### Requirement: STAGE.yaml deferral flow on failure
If Q17 elapsed exceeds 300s on dev, the change SHALL NOT block GA-5
promotion but SHALL be deferred to v3.13.

#### Scenario: q17_deferred_does_not_block_ga5
- **WHEN** issue #4540 verdict is `DEFERRED-to-v3.13`
- **THEN** GA-5 promotion MAY proceed on half A alone (22/22 oracle
  match) with half B explicitly marked as a v3.13 follow-up
- **AND** `docs/releases/v3.13.0/SCOPE_TABLE_v3.13.md` SHALL record
  the deferral with the captured `q17_elapsed_seconds` value
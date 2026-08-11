# V312-F-5 Spec: cargo fmt --all Compliance

## ADDED Requirements

### Requirement: cargo fmt --check MUST exit 0

`cargo fmt --all -- --check` MUST exit 0 on HEAD with no further
mutations to the working tree.

#### Scenario: fmt check is read-only
- **WHEN** `cargo fmt --all -- --check` runs
- **THEN** exit code is 0
- **AND** `git status --short` is unchanged before and after the check

### Requirement: Gate SGL-001 MUST PASS

The semantic_gate_check.py SGL-001 contract MUST pass.

#### Scenario: SGL-001 status=pass
- **WHEN** `bash scripts/gate/check_integration_gate.sh` runs
- **THEN** SGL-001 reports status=pass
- **AND** `cargo build --all-features` exits 0
- **AND** `cargo test --lib` exits 0
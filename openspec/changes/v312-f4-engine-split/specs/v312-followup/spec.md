# V312-F-4 Spec: execution_engine.rs Split

## MODIFIED Requirements

### Requirement: execution_engine.rs MUST be ≤ 1500 lines

The file `src/execution_engine.rs` MUST contain ≤ 1500 lines per
C-ARCH-05 AD-001 target (SSOT: check_rc_ga_gate.sh).

#### Scenario: file under limit
- **WHEN** `wc -l src/execution_engine.rs` is executed after the split
- **THEN** the line count MUST be ≤ 1500
- **AND** the file MUST continue to compile via `cargo build --all-features`

### Requirement: Sub-modules MUST preserve public API

Any split into sub-modules MUST preserve the existing public API of
`execution_engine` so external callers (mod.rs, main.rs, integration
tests) continue to compile without changes.

#### Scenario: external API unchanged
- **WHEN** external code references `execution_engine::*` symbols
- **THEN** all references resolve via re-exports
- **AND** no external file requires modification

### Requirement: Gate check_integration_gate.sh C-ARCH-05 MUST PASS

The integration gate C-ARCH-05 check MUST report PASS after the split.

#### Scenario: gate C-ARCH-05 status=pass
- **WHEN** `bash scripts/gate/check_integration_gate.sh` runs
- **THEN** C-ARCH-05 reports status=pass
- **AND** cargo test --lib continues to pass
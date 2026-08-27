# ga-aggregator Specification

## Purpose
Captures the two-mode execution contract for the v3.12.0 GA aggregator
(`scripts/gate/check_ga_v3.12.0.sh`) introduced in Issue #4536.

The aggregator historically claimed in its header doc to be a fast-path
verification tool, but `run_beta_gate()` unconditionally invoked
`bash $beta_script`, which on a developer laptop blocked for >3 min on the
cargo build + clippy + fmt cascade inside `check_beta_v3.12.0.sh`. This
spec formalizes a `--fast-path` opt-in (local-dev ergonomics, <30s) and a
`--full` mode that preserves the legacy heavy behavior for CI parity.

## Requirements

### Requirement: Two-mode invocation contract
The script SHALL accept three CLI flag forms:

#### Scenario: default_invocation_runs_heavy_beta
- **WHEN** the script is invoked with no arguments
- **THEN** `MODE="full"` (backward-compatible default)
- **AND** `run_beta_gate()` invokes `bash $beta_script` and captures the
  output log for PASS/WARN/FAIL counting

#### Scenario: full_flag_runs_heavy_beta
- **WHEN** the script is invoked with `--full`
- **THEN** behavior SHALL be identical to the default no-flag invocation

#### Scenario: fast_path_flag_skips_heavy_beta
- **WHEN** the script is invoked with `--fast-path`
- **THEN** `MODE="fast"`
- **AND** `run_beta_gate()` SHALL perform only `bash -n` syntax check on
  `$beta_script`
- **AND** on successful syntax check, `BETA_PASS=40` SHALL be reported
- **AND** `BETA_EVIDENCE_HASH="fast-path-no-evidence"` SHALL be recorded

#### Scenario: help_flag_prints_usage_and_exits_zero
- **WHEN** the script is invoked with `--help` or `-h`
- **THEN** the usage block (modes + JSON report contract) SHALL be
  printed to stdout
- **AND** the script SHALL exit with code 0

#### Scenario: unknown_arg_exits_two
- **WHEN** the script is invoked with any other argument
- **THEN** `[ERROR] unknown arg: <arg>` SHALL be printed to stderr
- **AND** `try --help for usage` SHALL be printed to stderr
- **AND** the script SHALL exit with code 2

### Requirement: JSON report carries mode field
The output JSON report
(`docs/releases/v3.12.0/evidence/v312-59/ga_gate_report.json`)
SHALL carry a top-level `mode` field so reviewers can distinguish
fast-path verdicts from heavy-run verdicts.

#### Scenario: mode_full_for_heavy_runs
- **WHEN** the aggregator ran in heavy mode (default or `--full`)
- **THEN** the JSON top-level `mode` field SHALL be `"full"`

#### Scenario: mode_fast_path_for_fast_runs
- **WHEN** the aggregator ran in fast-path mode (`--fast-path`)
- **THEN** the JSON top-level `mode` field SHALL be `"fast-path"`

### Requirement: Backward compatibility for CI
Default behavior MUST remain unchanged for existing CI invocations.

#### Scenario: ci_workflow_invocation_unaffected
- **WHEN** a `.gitea/workflows/*.yml` entry invokes
  `bash check_ga_v3.12.0.sh` without flags
- **THEN** the heavy BETA stage SHALL execute as before
- **AND** the workflow files SHALL NOT need modification
- **AND** the JSON `mode` field SHALL be `"full"`

#### Scenario: ci_must_not_pass_fast_path
- **WHEN** the CI gate contract is enforced
- **THEN** `.gitea/workflows/*.yml` aggregator calls SHALL NOT pass
  `--fast-path` (CI parity required)

### Requirement: Header documentation reflects two modes
The script header SHALL document both modes explicitly.

#### Scenario: header_doc_lists_both_modes
- **WHEN** the script is read top-to-bottom
- **THEN** the header SHALL list `(default | --full)` and `--fast-path`
  modes with their behavior, wall-clock budget, and use case

### Requirement: Heavy-mode startup banner
The aggregator SHALL print a hint banner when the heavy BETA path is
about to run, so users can opt into fast-path instead.

#### Scenario: heavy_mode_banner_with_hint
- **WHEN** `MODE="full"`
- **THEN** the main() banner SHALL include a hint pointing at
  `--fast-path` for local-dev usage
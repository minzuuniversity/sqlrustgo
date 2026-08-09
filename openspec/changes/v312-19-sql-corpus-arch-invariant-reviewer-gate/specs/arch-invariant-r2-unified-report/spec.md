## ADDED Requirements

### Requirement: R2.1-R2.8 driver MUST run or stub all 8 checks

`scripts/gate/check_r2_invariants.sh` SHALL invoke or stub 8 architectural
invariant checks (R2.1..R2.8) and emit a single report at
`docs/releases/v3.12.0/evidence/arch_invariants/R2_INVARIANTS_REPORT.md`.

For each R2.N, the driver:

1. If `scripts/gate/check_arch_<N>.sh` exists, invoke it.
2. Else, emit a stub script that exits 0 with `TODO: implement R2.N` in
   stdout (honest-gap policy; not a fabrication of a passing check).
3. Capture stdout to `R2_N.stdout`.
4. SHA256 the stdout.
5. Record exit code, status, and stdout hash.

#### Scenario: Existing R2 checks pass

- **GIVEN** R2.1 (`check_arch2_no_bypass.sh`), R2.2 (`check_arch3_no_bypass.sh`),
  R2.3 (`check_arch_invariants.sh`), R2.4 (`check_arch_sem_debt.sh`) all exit 0
- **WHEN** the driver runs
- **THEN** the report SHALL show `status=pass` for R2.1..R2.4
- **AND** `status=stub` for R2.5..R2.8

#### Scenario: R2 check fails

- **WHEN** an existing R2.N check exits non-zero
- **THEN** the report SHALL show `status=fail` for that R2.N
- **AND** the driver SHALL exit with a non-zero code

### Requirement: R2_INVARIANTS_REPORT.md MUST be self-stamped

The report SHALL have one row per R2.N with columns:

| check | status | stdout_sha256 | exit_code |

The `status` column SHALL be one of: `pass`, `fail`, `stub`. The report
file itself SHALL be SHA256-stamped in its footer.

#### Scenario: Report footer contains its own hash

- **WHEN** the report is written
- **THEN** the final line of the file SHALL be
  `<!-- report_sha256: <hex> -->`

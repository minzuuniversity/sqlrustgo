# v3.10.0 Coverage Baseline

Coverage baseline measurement pending. Required by RC Gate R6 (≥ 80% per crate).

## Generate

```bash
cargo llvm-cov --lib --json > docs/releases/v3.10.0/coverage-baseline/coverage.json
```

## Format

Each `-lib.json` file (e.g. `sqlrustgo-lib.json`) is read by the RC gate script
(`check_rc_gate_v3.10.0.sh §R6`) using this structure:

```json
{
  "data": [{"summary": {"percent_covered": 82.3}}]
}
```

## Notes

- Only `--lib` targets are measured (not integration tests)
- RC requirement: ≥ 80% per crate
- Tracking issue: V310-10

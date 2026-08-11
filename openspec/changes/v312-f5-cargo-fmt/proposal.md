## Why

ISSUE #4028 (V312-F-5): cargo fmt 183 文件 237 处违规 (SGL-001).

Per `check_integration_gate.sh` (2026-08-10T23:55):
```
SGL-001: B4 Format check exits 1 — 183 files need reformat (237 violations)
```

The `semantic_gate_check.py` SGL-001 contract:
> "B4 Format must pass 'cargo fmt --all -- --check' without auto-fix"

This means we MUST run `cargo fmt --all` to apply the canonical formatting,
then commit the changes. The gate ensures `cargo fmt --check` is truly
read-only (no further mutations) after our fix.

## What Changes

- Run `cargo fmt --all` to apply canonical Rust formatting
- Commit the formatting changes (no semantic change)
- Verify `cargo fmt --all -- --check` exits 0 (no mutations)
- Verify `cargo build --all-features` still passes
- Verify `cargo test --lib` still passes

## Impact

- Modified: 183 files across the workspace (formatting only, no behavior change)
- No code semantics change

## Acceptance criteria

- [ ] `cargo fmt --all -- --check` exit 0
- [ ] `bash scripts/gate/check_integration_gate.sh` SGL-001 PASS
- [ ] `cargo build --all-features` exit 0
- [ ] `cargo test --lib` exit 0
- [ ] ISSUE #4028 comment 含 commit SHA + line change summary

## Risk

Low. `cargo fmt` is a mechanical, deterministic formatter. Risk is
limited to unusual formatting edge cases (e.g., macro-generated code)
which we can address with `#[rustfmt::skip]` if needed.

## References

- ISSUE #4028 (F-5)
- ISSUE #3887 (V312-MASTER)
- semantic_gate_check.py SGL-001
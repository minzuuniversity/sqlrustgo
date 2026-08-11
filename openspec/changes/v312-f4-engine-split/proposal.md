## Why

ISSUE #4027 (V312-F-4): execution_engine.rs 1762 lines exceeds 1500 limit
(C-ARCH-05 in check_integration_gate.sh).

Per `check_integration_gate.sh` (2026-08-10T23:55):
```
C-ARCH-05: execution_engine.rs has 1762 lines (limit: 1500, SSOT: check_rc_ga_gate.sh)
```

The file has grown from 1471 (post-SPEC-012 CBO split baseline) → 1762
due to v3.12.0 additions (GIS Phase 2, Sequence executor enhancements,
window function extensions, etc.).

## What Changes

- Split `src/execution_engine.rs` (1762 lines) into multiple sub-files:
  - `src/execution_engine.rs` — core dispatch + common helpers
  - `src/execution_engine/select.rs` — SELECT execution paths
  - `src/execution_engine/dml.rs` — INSERT/UPDATE/DELETE
  - `src/execution_engine/ddl.rs` — CREATE/DROP/ALTER
  - `src/execution_engine/tx.rs` — transaction control
- Each sub-file < 500 lines
- Core `execution_engine.rs` < 1500 lines

## Impact

- Modified: `src/execution_engine.rs` (split into modules)
- New files: `src/execution_engine/{select,dml,ddl,tx}.rs`
- All call sites unchanged (re-exports preserve API)
- Risk: medium — refactoring touches the largest file in the codebase

## Acceptance criteria

- [ ] `wc -l src/execution_engine.rs` shows ≤ 1500
- [ ] `bash scripts/gate/check_integration_gate.sh` C-ARCH-05 PASS
- [ ] `cargo build --all-features` exit 0
- [ ] `cargo test --lib` PASS
- [ ] `cargo test --test v312_13_*` PASS
- [ ] ISSUE #4027 comment 含 PR#, SHA, line count evidence

## Risk

Medium. The refactor must maintain all internal API surface. The existing
`check_arch_invariants.sh` already PASSES (1600 limit) but
`check_integration_gate.sh` requires the stricter 1500 AD-001 target.

## References

- ISSUE #4027 (F-4)
- ISSUE #3887 (V312-MASTER)
- AD-001 long-term target
- C-ARCH-05 gate (SSOT: check_rc_ga_gate.sh)
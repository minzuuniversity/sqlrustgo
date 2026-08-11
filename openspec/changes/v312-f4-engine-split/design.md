# V312-F-4 Design: execution_engine.rs Split

## Current State

```
$ wc -l src/execution_engine.rs
1762 src/execution_engine.rs
```

Limit per `check_integration_gate.sh` (SSOT: check_rc_ga_gate.sh): 1500.
Per `check_arch_invariants.sh` (transitional): 1600.

## Split Strategy

The file contains ~30 top-level functions. Group by responsibility:

| Sub-module | Functions (estimated) | Target lines |
|-----------|----------------------|--------------|
| `execution_engine.rs` (core) | dispatch + state + helpers | ~600 |
| `execution_engine/select.rs` | execute_with_select + subqueries | ~400 |
| `execution_engine/dml.rs` | INSERT/UPDATE/DELETE/REPLACE | ~350 |
| `execution_engine/ddl.rs` | CREATE/DROP/ALTER/TRUNCATE | ~250 |
| `execution_engine/tx.rs` | BEGIN/COMMIT/ROLLBACK/SAVEPOINT | ~150 |

Total stays roughly the same (function bodies unchanged), but each
file is well under 500 lines.

## API Preservation

The `pub use` re-exports in `execution_engine.rs` ensure external code
(`mod.rs`, `main.rs`, integration tests) sees the same surface:

```rust
pub use self::select::*;
pub use self::dml::*;
pub use self::ddl::*;
pub use self::tx::*;
```

## Verification

After split: `wc -l src/execution_engine.rs` < 1500 (likely ~600).
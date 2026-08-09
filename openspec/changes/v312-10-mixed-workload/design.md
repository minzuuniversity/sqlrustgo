# Design: V312-10 Mixed Workload SOAK

## Decisions

### Decision: Deterministic round-robin scheduling

Instead of random, use `WorkloadOp::from_index(i % 9)` for reproducible test runs.

### Decision: crash_safe = true by default

Halt on first error, recording crash count. Set to false for lenient runs.

### Decision: Periodic audit chain verification

Every N operations (default 100), call `verify_audit_chain`. Fail if chain is broken.

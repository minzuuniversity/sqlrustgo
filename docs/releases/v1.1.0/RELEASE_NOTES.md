# Release Notes v1.1.0

**Release Date**: 2026-03-XX
**Version**: v1.1.0
**Type**: Draft → GA

---

## Overview

v1.1.0 is a performance-focused release that establishes a comprehensive benchmark framework and lays the foundation for the LogicalPlan/PhysicalPlan architecture. This release targets L3 product-level maturity.

---

## What's New

### Performance Benchmark Framework (B-001~B-010)

Added comprehensive performance benchmarking infrastructure using Criterion.rs:

#### Lexer & Parser Benchmarks (B-001~B-003)
- Tokenization performance for simple and complex SQL
- Parsing performance for various SQL statement types

#### Executor Benchmarks (B-004)
- 14 benchmarks covering SELECT, INSERT, UPDATE, DELETE operations
- Single row and batch operation performance

#### Storage Benchmarks (B-005)
- 22 benchmarks for BufferPool, B+Tree, and Page operations
- CRUD operations and range queries

#### Network Benchmarks (B-006)
- 16 benchmarks for Packet serialization, RowData encoding, and Value conversion

#### Integration Benchmarks (B-008)
- 11 end-to-end pipeline benchmarks
- Full query execution from lexer to executor

#### Benchmark Tools
- `scripts/run_benchmarks.sh` - Automated benchmark runner
- `scripts/compare_benchmarks.py` - Performance regression detection

#### Performance Results

| Component | Result | Target | Status |
|-----------|--------|--------|--------|
| Executor SELECT (100 rows) | ~12µs | < 1ms | ✅ |
| Storage B+Tree insert 1K | <1ms | < 10ms | ✅ |
| Integration 1K rows insert | ~290ms | < 500ms | ✅ |
| Integration 1K rows select | ~130µs | < 1ms | ✅ |

### Query Analyzer Phase 1 (Issue #89)

Added the foundation for LogicalPlan/PhysicalPlan architecture:

- **Analyzer struct** for SQL statement analysis
- **SELECT/INSERT/UPDATE/DELETE** analysis support
- **Table schema binding** and type inference
- **Column binding** and expression binding
- **Aggregate function support** (COUNT, SUM, AVG, MIN, MAX)

### Security Audit (F-01)

- All dependencies verified safe
- No known vulnerabilities in current dependency versions
- Historical vulnerabilities patched

---

## Dependencies Updated

| Package | Old Version | New Version |
|---------|-------------|-------------|
| tokio | 1.x | 1.49.0 |
| bytes | 1.x | 1.11.1 |
| regex | 1.x | 1.12.3 |

---

## Breaking Changes

None. This release is fully backward compatible with v1.0.0.

---

## Migration Guide

For users upgrading from v1.0.0:

1. **No API changes** - All existing code continues to work
2. **New benchmarks** - Run `cargo bench` to measure performance
3. **Optional: Query Analyzer** - New analyzer module available at `sqlrustgo::planner`

```rust
// Optional: Use the new Query Analyzer
use sqlrustgo::planner::analyzer::Analyzer;

let analyzer = Analyzer::new();
let plan = analyzer.analyze(&statement);
```

---

## Known Limitations

- HashJoin not yet implemented (planned for v1.2.0)
- Test coverage at 74.83% (target: 90%)
- Function coverage at ~80% (target: 85%)

---

## Contributors

Thanks to all contributors who made this release possible.

---

## What's Next (v1.2.0)

- HashJoin implementation
- 100万行级 data processing support
- Complete Client-Server architecture
- Plugin-based execution engine

---

## Links

- [Release Gate Checklist](./RELEASE_GATE_CHECKLIST.md)
- [Security Audit Report](./SECURITY_AUDIT.md)
- [v1.0.0 Release Notes](../v1.0.0/RELEASE_NOTES.md)

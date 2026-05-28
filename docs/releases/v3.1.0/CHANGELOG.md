# v3.1.0 Changelog

> **Version**: 3.1.0
> **Date**: 2026-05-11
> **Status**: GA (General Availability)
> **Branch**: develop/v3.1.0
> **Truthfulness Score**: 70% — Retrospective reconstruction
> **Note**: 本文档为事后补充（Retrospective），基于 git log 和门禁报告重建

---

## v3.1.0 GA (2026-05-11)

### Added

#### Spill Framework Integration (P1)

**Spill Operators**
- `SpillOperator` — Disk-based spilling for large result sets
- `SpillHashJoin` — Hash join with spill support
- `SpillAggregate` — Aggregate with spill support
- Spill framework integration into LocalExecutor
- PR: #1064, #1049

**TPC-H SF=10 Support**
- Spill framework enables TPC-H SF=10 execution
- Disk-based sorting for large result sets
- Memory management for multi-gigabyte queries
- PR: #1064, #1069

#### Parser Improvements

**Aggregate Expressions**
- `SUM(col1 * col2)` — aggregate with expressions
- `AVG(column / divisor)` — arithmetic in aggregates
- Complex aggregate argument support
- PR: #1048

#### MySQL Protocol Enhancement

**Connection Pool**
- Thread pool implementation
- `MAX_CONNECTIONS` configuration
- Connection multiplexing
- PR: #1013

**Performance Optimization**
- MySQL protocol flush optimization
- Reduced latency for client responses
- PR: #1059, #1060

#### Beta Gate System Enhancement

**v3.1.0 Beta Gate Scripts**
- `check_beta_v310.sh` — v3.1.0 Beta Gate script
- Enhanced stability checks (300s timeout for R9/R-S1)
- Parallel coverage execution to avoid timeout
- PR: #969, #970

**Gate Improvements over v3.0.0**
- B1 Build: Enhanced build validation
- B2 Test ≥90%: L1 core crate pass rate ≥90%
- B3-B4: Clippy/Format checks
- B5 Coverage ≥75%: Coverage threshold
- B6 Security: Vulnerability scan
- B7 SQL Compat: SQL Corpus ≥80%
- B8 TPC-H SF=1: TPC-H benchmark
- B9 Proof: Formal verification files

---

## v3.0.0-ga (Previous Release)

Previous stable release — SQL Query Engine MVP with complete Beta/RC/GA gate system.

---

**Maintenance**: hermes-z6g4
**Generated**: 2026-05-29 (Retrospective)
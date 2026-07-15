## 1. AST changes

- [x] 1.1 Add `ExistsSubquery` variant to `Expression` enum in `crates/parser/src/parser.rs:570`
- [x] 1.2 Add `parse_exists_subquery()` function in parser
- [x] 1.3 Update WHERE clause parsing to produce `ExistsSubquery` for EXISTS syntax
- [x] 1.4 Re-export `ExistsSubquery` in `crates/parser/src/lib.rs`

## 2. Expression evaluator

- [x] 2.1 Add `Expression::ExistsSubquery` handler in `crates/executor/src/expr/mod.rs`
- [x] 2.2 Implement `HashSet`-based inner query evaluation
- [x] 2.3 Add bloom filter (FNV-1a + DJB2, 1024 bits) for probe optimization
- [x] 2.4 Implement ExistsSubquery evaluation as boolean True/False

## 3. Row context plumbing

- [x] 3.1 Add row context to expression evaluator (for outer row)
- [x] 3.2 Update Filter operator to handle ExistsSubquery specially
- [x] 3.3 Test integration with FilterExec

## 4. Bloom filter optimization

- [x] 4.1 Implement `BloomFilter` struct (128 bytes, 2 hash functions)
- [x] 4.2 Add `add()` and `might_contain()` methods
- [x] 4.3 Integrate with ExistsSubquery evaluation

## 5. Tests

- [x] 5.1 Unit test: ExistsSubquery with empty inner → False
- [x] 5.2 Unit test: ExistsSubquery with matching inner → True
- [x] 5.3 Integration test: TPC-H Q4 with SF=0.1
- [x] 5.4 Performance test: TPC-H Q4 with SF=3 (< 5 minutes)
- [x] 5.5 Regression test: Q1/Q3/Q5/Q6/Q12/Q14 unchanged

## 6. Performance test

- [x] 6.1 Generate SF=3 TPC-H fixture (450K orders, 3M lineitem)
- [x] 6.2 Measure Q4 execution time before/after fix
- [x] 6.3 Verify ≥2.9x speedup (v3.10.0 = 14.5min → ≤5min)
- [x] 6.4 Document in `docs/releases/v3.11.0/perf/SEMI_JOIN_PERF.md`

## 7. PR and merge

- [x] 7.1 Create PR on Gitea 250
- [x] 7.2 Get 2 approvals
- [x] 7.3 Force-merge (admin)
- [x] 7.4 Sync to gitcode + gitee
- [x] 7.5 Update Issue #3434 with PR link

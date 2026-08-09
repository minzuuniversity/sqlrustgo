## Why

V312-11 integrates the SQLite SQLLogicTest oracle as a v3.12 release gate blocker.

## What Changes

### Current State
- `crates/sqlrustgo_sqllogictest` EXISTS and BUILDS ✅
- `sqllogictest` crate (risinglightdb/sqllogictest-rs) used as parser/runner
- 16 test files in `testdata/` directory
- Local smoke corpus produces results

### Test Results (Baseline)
- Files: 6/16 pass, 10/16 fail
- Pass rate: 27.3%
- Known failures: ALTER TABLE, VALUES constructor, EXCEPT/INTERSECT ALL

### Gate Requirements
1. `cargo build -p sqlrustgo_sqllogictest` succeeds ✅
2. Local smoke corpus produces output ✅
3. Document pass/fail criteria for v3.12 gate

## Capabilities

### New Capabilities
- `sqllogictest-gate`: SQLLogicTest oracle as correctness gate

## Impact

### Affected Modules
- `crates/sqlrustgo_sqllogictest` - runner binary
- `crates/sqlrustgo-parser` - PARSE errors are main failure source

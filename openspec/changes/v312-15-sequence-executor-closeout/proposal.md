## Why

V312-15 closes out the CREATE SEQUENCE executor gap identified in v3.11.0 assessment.

## What Changes

### Current State Analysis
- Parser: IMPLEMENTED (CREATE SEQUENCE, DROP SEQUENCE, ALTER SEQUENCE parse correctly)
- Storage trait: DEFINED (SequenceInfo struct + trait methods)
- MemoryStorage: IMPLEMENTED (all sequence CRUD methods work)
- Expression evaluation: IMPLEMENTED (SequenceNextVal/SequenceCurrval resolve)
- DDL execution path: NOT IMPLEMENTED (no handler for Statement::CreateSequence)

### Required Changes
1. Implement DDL execution for CREATE SEQUENCE, DROP SEQUENCE, ALTER SEQUENCE
2. Integrate with WAL replay for sequence state
3. Backup/restore sequence state

## Capabilities

### New Capabilities
- `sequence-ddl-execution`: CREATE/DROP/ALTER SEQUENCE via SQL

## Impact

### Affected Modules
- `crates/executor` - DDL execution
- `crates/storage` - Already has implementation

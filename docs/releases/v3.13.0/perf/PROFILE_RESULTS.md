# V313.3 BINT v3 6M Profile Results

> **Status (2026-08-24):** Experiments in progress. This document is the
> single source of truth for V313.3 profiling measurements.

## Audit findings

### `StorageEngine::scan` caller audit

`BinaryTableStorageV2::scan` is a stub returning `Ok(vec![])` (file
`crates/storage/src/binary_storage_v2.rs:282`). It is not called from
executor/planner code paths today. The only mutators of
`tables.rows` are the two streaming insert paths at lines 98 and 163
of `binary_storage_v2.rs`. Therefore removing `tables.rows.push` from
streaming inserts does NOT break any existing read path.

(Filled by Task 1)

## Baseline (current HEAD)

(Filled by Task 2)

## Experiment results table

(Filled by Task 3)

## Decision: optimize <top suspect>

(Filled by Task 3)

## Optimized measurement

(Filled by Task 5)

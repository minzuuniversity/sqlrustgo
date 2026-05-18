# sqlrustgo-crash-sim

Crash Simulation Framework for SQLRustGo recovery correctness testing.

## Overview

Provides deterministic and randomized crash injection for testing database recovery correctness. Tracks 11 crash injection points across the storage pipeline.

## Crash Points

- `wal_write` - WAL write operation (before fsync)
- `wal_flush` - WAL flush/sync operation
- `transaction_commit` - Transaction commit acknowledgment
- `page_split` - B-tree page split operation
- `checkpoint` - Checkpoint creation
- `buffer_evict` - Buffer pool page eviction
- `index_update` - Index update operation
- `lock_acquire` - Lock acquisition
- `audit_write` - Audit log write
- `signature_compute` - Cryptographic signature computation
- `post_commit` - After commit, before client response

## Usage

```rust
use sqlrustgo_crash_sim::*;

let mut runner = CrashTestRunner::new(CrashCampaignConfig::default());

// Run a single deterministic scenario
let scenario = CrashScenario::at(CrashPoint::WalFlush);
let result = runner.run_scenario(&scenario);

// Run a full campaign
let results = runner.run_campaign();
runner.print_report();
```

## Verification

After recovery, the framework verifies:
- Data integrity (row counts, checksums)
- WAL replay correctness
- Audit chain continuity
- Digital signature validity
- No phantom commits
- No lost writes

## Integration with Gate

Add to RC/GA gate scripts for recovery verification:

```bash
cargo test -p sqlrustgo-crash-sim
```

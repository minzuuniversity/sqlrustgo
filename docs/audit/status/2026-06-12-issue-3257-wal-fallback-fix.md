# Issue #3257 Resolution: WAL data_dir Fallback (v3.9.0)

> **Date**: 2026-06-12
> **Sprint**: 5 v18+ (post-cleanup, pre-RC3 stabilization)
> **Severity**: P0 (stability blocker for 24h soak + Z6G4)
> **PR**: [#3261](http://192.168.0.250:3000/openclaw/sqlrustgo/pulls/3261) (merged)
> **Commit**: `7a3e9d84e`
> **Issue**: [Issue #3257](http://192.168.0.250:3000/openclaw/sqlrustgo/issues/3257) (closed)

## Problem

The production MySQL server (`run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql`)
fell back to a port-keyed `/tmp` path when no `data_dir` was supplied:

```rust
let wal_data_dir = match data_dir {
    Some(p) => p,
    None => {
        std::env::temp_dir().join(format!("sqlrustgo_wal_{}", listener.local_addr()?.port()))
    }
};
```

### Impact

1. **Stale WAL persistence**: The WAL file persisted forever across server
   restarts on the same port, accumulating writes from prior runs.
2. **20+ minute startup**: The recovery engine replayed the entire WAL on
   startup. For a 9.9 GB stale WAL (from a previous sysbench run), this
   took 20+ minutes.
3. **24h soak driver timeout**: The 24h soak driver (PR #3370) silently
   timed out at sysbench prepare because the server couldn't start fast
   enough between restarts.
4. **Blocks all devs**: Any developer running the server on a previously-
   used port hit long recovery times.

## Fix (PR #3261)

### Resolution order (new)

When `data_dir` is `None`:

1. `$SQLRUSTGO_DATA_DIR` env var (operator override)
2. `<cwd>/.sqlrustgo/data/` (stable, predictable default)

### Additional improvement

The server now emits a startup warning when the existing WAL file is
>= 100 MB, catching stale-WAL regressions early.

### Code (before/after)

```rust
// Before: port-keyed /tmp
let wal_data_dir = match data_dir {
    Some(p) => p,
    None => std::env::temp_dir().join(format!("sqlrustgo_wal_{}", port)),
};

// After: stable cwd-based fallback with env override
let wal_data_dir = match data_dir {
    Some(p) => p,
    None => match std::env::var("SQLRUSTGO_DATA_DIR") {
        Ok(s) if !s.is_empty() => PathBuf::from(s),
        _ => std::env::current_dir().unwrap().join(".sqlrustgo").join("data"),
    },
};
```

## Tests (3/3 PASS)

`tests/integration/issue_3257_wal_fallback_test.rs`:

| Test | Assertion | Time |
|------|-----------|------|
| `issue_3257_wal_default_survives_restart` | Data written through wire protocol survives process restart on the resolved data_dir | 0.14s |
| `issue_3257_no_port_keyed_tmp` | Resolved path never contains port-keyed /tmp segment | 0.00s |
| `issue_3257_wal_recovery_does_not_hang_on_existing_wal` | Server starts in < 10s on a small existing WAL | 0.06s |

## Verification

| Check | Result |
|-------|--------|
| `cargo test --all-features --test issue_3257_wal_fallback_test` | 3/3 PASS |
| `cargo clippy --all-features -- -D warnings` | clean |
| `cargo fmt --check` (changed files) | clean |
| `cargo build --all-features` | clean |

## Impact on Roadmap

- **Closes** the only remaining GA-P0 stability blocker identified by
  Hermes 2026-06-12 audit (#3252).
- **Unblocks** the 24h soak driver (PR #3370) for repeatable runs on
  the same port.
- **Reduces** the v3.9.0-ga critical-path items by 1 (of 13).

## Refs

- Issue #3257 (this issue, closed)
- Issue #3252 (Hermes 2026-06-12 audit, GA rollback to RC3)
- PR #3370 (24h soak driver that surfaced the bug)
- PR #3348 (introduced WAL replay on startup)
- `docs/audit/status/2026-06-12-2bdeleted-namespace-migration.md`
  (sister audit doc)

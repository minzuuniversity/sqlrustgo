## Design

### Lint fixes

All L1 clippy errors are mechanical:

1. **Dead imports** (`binary_storage.rs` lines 11, 13): removed.
2. **Dead method** (`binary_storage.rs` line 71 `ensure_loaded`): deleted.
   Verified no callers via `grep` over the workspace.
3. **`sort_by` → `sort_by_key`**: same comparator, key-only form.
   `b.timestamp.cmp(&a.timestamp)` (descending) → `sort_by_key(|b| Reverse(b.timestamp))`.
   `a.archive_id.cmp(&b.archive_id)` (ascending) → `sort_by_key(|a| a.archive_id)`.
4. **Collapsible match** (`engine.rs:151`): the `if upper[i..].starts_with(&op_upper)`
   inside the `_ if !in_string && depth == 0 =>` arm becomes part of the guard:
   `_ if !in_string && depth == 0 && upper[i..].starts_with(&op_upper) =>`.

For `optimizer/src/stats.rs:401` the two `Some(Value::Text(_)|Value::Blob(_))` arms
also have a collapsible match, but the guard cannot reference the outer `value`
(because the match expression is `&min_value`). The chosen fix is to **extract
two closures `update_min` / `update_max`** that capture `value` by reference and
return `bool`; the match arms then become `cur if update_min(cur) => …`. This
keeps the original control flow and lets the lint pass without restructuring
the data flow.

### Manual checked division

`telemetry/src/lib.rs:153` and `executor/src/executor_metrics.rs:66` both
compute `dividend / divisor` after a `if total == 0` check. The clippy lint
`manual_checked_div` suggests `dividend.checked_div(total).unwrap_or(0)`.
Adopted. The return type stays `u64` and the zero-division behavior is
unchanged.

### Targeted allows

- `is_select_stmt` is referenced only by `test_is_select_stmt` in the same
  file. Adding `#[cfg(test)]` is the minimal change that satisfies the
  `dead_code` lint and matches the test-only usage.
- `run_server_with_listener_and_shutdown_with_bootstrap_tables_and_sql`
  has 8 args (listener, shutdown, bootstrap, bootstrap_tables, bootstrap_sql,
  data_dir, server_threads, storage). This is a single integration test
  harness entry point consolidating many pre-existing args. Refactoring
  to a config struct is out of scope for a lint-drift fix; the chosen
  solution is a function-level `#[allow(clippy::too_many_arguments)]`.

### Verification

```bash
cd ~/dev/sqlrustgo
cargo clippy --all-features -- -D warnings    # exit 0
cargo fmt --check --all                       # exit 0
cargo test --lib --quiet                      # 25/25 PASS
bash gate/gate.sh v3.9.0                      # Gate Result: PASSED
```

### Rollout

Single feature branch `fix/gate-clippy-fmt-drift` → PR to
`develop/v3.9.0`. After merge, `gate.sh` is back to PASS and the
release remains on the RC7 baseline. No new tests, no behavior change.

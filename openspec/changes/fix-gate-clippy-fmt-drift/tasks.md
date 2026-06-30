## Tasks

- [x] **1. Fix `binary_storage.rs`**: drop `Read` / `Arc` imports; delete `ensure_loaded` method.
- [x] **2. Fix `checkpoint.rs:185`**: `sort_by` → `sort_by_key`.
- [x] **3. Fix `engine.rs:150`**: collapse nested `if` into match arm guard.
- [x] **4. Fix `wal_legacy.rs:913`**: `sort_by` → `sort_by_key`.
- [x] **5. Fix `optimizer/src/stats.rs:401`**: extract `update_min` / `update_max` closures; rewrite match arms with guards.
- [x] **6. Fix `executor_metrics.rs:66`**: `checked_div(...).unwrap_or(0)`.
- [x] **7. Fix `telemetry/src/lib.rs:153`**: same `checked_div` pattern.
- [x] **8. Fix `ivfpq.rs:144`**: drop redundant `.into_iter()`.
- [x] **9. `mysql-server/src/lib.rs:2115`**: add `#[cfg(test)]` to `is_select_stmt`.
- [x] **10. `mysql-server/src/lib.rs:3276`**: add `#[allow(clippy::too_many_arguments)]` to `run_server_*`.
- [x] **11. `cargo fmt --all`**.
- [x] **12. Update `CHANGELOG.md`** (root) and `docs/releases/v3.9.0/CHANGELOG.md`.
- [x] **13. Verify**: `cargo clippy --all-features -- -D warnings` exit 0; `cargo fmt --check --all` exit 0; `cargo test --lib` PASS; `bash gate/gate.sh v3.9.0` PASS.

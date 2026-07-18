## 1. Upgrade Test Script

- [x] 1.1 Create `scripts/test_upgrade_v310_to_v311.sh`
- [x] 1.2 Implement `setup_v310_data()` function — start v3.10.0, create tables, insert sample data
- [x] 1.3 Implement `run_upgrade()` function — stop v3.10.0, swap binary, start v3.11.0
- [x] 1.4 Implement `verify_data()` function — verify row counts, checksums, indexes
- [x] 1.5 Implement `cleanup()` function — remove temp directories
- [x] 1.6 Add `--rollback` flag to test downgrade path

## 2. Rust Upgrade Test

- [x] 2.1 Create `tests/integration/migration/upgrade_v310_v311_test.rs`
- [x] 2.2 Implement `test_upgrade_v310_to_v311_catalog_migration`
- [x] 2.3 Implement `test_upgrade_v310_to_v311_data_integrity`
- [x] 2.4 Implement `test_upgrade_v310_to_v311_index_rebuild`
- [x] 2.5 Register test in Cargo.toml

## 3. GA Gate Script

- [x] 3.1 Create `scripts/gate/check_upgrade_v310_v311.sh`
- [x] 3.2 Verify upgrade script exists and is executable
- [x] 3.3 Verify Rust test compiles
- [x] 3.4 Add to RC gate check list

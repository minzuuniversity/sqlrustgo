## Overview

v3.10.0 → v3.11.0 原地升级测试脚本，通过实际启动两个版本的二进制文件并执行升级来验证迁移平滑性。

## Architecture

### Upgrade Test Flow

```
1. Setup: Start v3.10.0, write test data
2. Shutdown v3.10.0
3. Replace binary with v3.11.0
4. Start v3.11.0 (triggers auto-migration)
5. Verify: data integrity, catalog, indexes
6. Cleanup
```

### Script Structure (`test_upgrade_v310_to_v311.sh`)

```bash
#!/bin/bash
set -euo pipefail

# Configuration
V310_DIR="/tmp/sqlrustgo-v310"
V311_DIR="/tmp/sqlrustgo-v311"
DATA_DIR="/tmp/sqlrustgo-upgrade-data"
PORT=3307

# Steps
setup_v310_data()   # Start v3.10.0, create tables, insert data
run_upgrade()       # Stop v3.10.0, swap binary, start v3.11.0
verify_data()       # Verify row counts, checksums, indexes
cleanup()           # Remove temp directories
```

### Integration with Existing Harness

The existing `upgrade_test_harness.rs` and `upgrade_test.rs` provide a framework for upgrade scenarios. This change adds v3.10→v3.11 specific scenarios to that framework.

## Implementation Details

### 1. Shell Script (`test_upgrade_v310_to_v311.sh`)

- Downloads/builds v3.10.0 and v3.11.0 binaries
- Uses separate data directories to avoid port conflicts
- Supports `--rollback` flag to test downgrade path
- Returns exit code 0 on success, non-zero on failure

### 2. Rust Test (`upgrade_v310_v311_test.rs`)

Extends existing upgrade_test harness with v3.10→v3.11 specific tests:
- `test_upgrade_v310_to_v311_catalog_migration`
- `test_upgrade_v310_to_v311_data_integrity`
- `test_upgrade_v310_to_v311_index_rebuild`

### 3. Gate Script (`check_upgrade_v310_v311.sh`)

- Verifies upgrade script exists and is executable
- Verifies Rust test compiles
- Optionally runs the upgrade test (may be slow)

## Dependencies

- Both v3.10.0 and v3.11.0 binaries available
- Separate data directories for each version
- Network port availability for both instances

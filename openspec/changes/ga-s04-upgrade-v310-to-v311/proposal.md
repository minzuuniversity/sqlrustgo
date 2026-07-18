## Why

当前所有测试均基于全新初始化，v3.11.0 GA 发布前必须验证从 v3.10.0 到 v3.11.0 的原地升级路径是否平滑。升级失败会导致用户数据丢失，是 GA 发布 P0 阻塞项。

## What Changes

- 新增 `scripts/test_upgrade_v310_to_v311.sh` — 升级测试脚本
- 新增 `tests/integration/migration/upgrade_v310_v311_test.rs` — 升级测试用例
- 新增 `scripts/gate/check_upgrade_v310_v311.sh` — GA-P0 门禁脚本
- 扩展 `upgrade_test_harness.rs` — 支持 v3.10.0 → v3.11.0 特定场景

### 测试场景

1. **Catalog 迁移**: v3.10.0 的 system tables 在 v3.11.0 中正确读取
2. **数据完整性**: 历史数据行数、checksum 在升级后一致
3. **索引重建**: 旧索引在 v3.11.0 中可用
4. **回滚验证**: 升级失败可回滚到 v3.10.0

## Capabilities

### New Capabilities

- `upgrade-v310-v311`: v3.10.0 → v3.11.0 原地升级测试能力

### Modified Capabilities

- `upgrade-test`: 扩展现有 upgrade test harness 支持 v3.10→v3.11 场景

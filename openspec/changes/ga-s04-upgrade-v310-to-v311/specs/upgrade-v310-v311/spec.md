# upgrade-v310-v311

v3.10.0 → v3.11.0 原地升级测试能力。

## Functionality

### Core Features

1. **Catalog 迁移验证**
   - v3.10.0 system tables 在 v3.11.0 中正确读取
   - 新增列/表结构正确扩展

2. **数据完整性验证**
   - 行数一致性
   - Checksum 校验
   - 索引可用性

3. **回滚验证**
   - 升级失败时可通过旧二进制恢复
   - 数据不丢失

### User Interactions

- 直接运行 `scripts/test_upgrade_v310_to_v311.sh`
- 可选 `--rollback` 测试降级路径
- 门禁脚本自动验证

### Edge Cases

- v3.10.0 数据损坏：升级前检测并报错
- 端口冲突：使用不同端口（3306 vs 3307）
- 二进制不存在：提示构建或下载

## Acceptance Criteria

- [ ] 升级脚本可执行
- [ ] 旧数据完整性验证通过
- [ ] catalog 迁移无错误
- [ ] 回滚路径验证通过

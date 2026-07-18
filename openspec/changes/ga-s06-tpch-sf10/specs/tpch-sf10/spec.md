# tpch-sf10

TPC-H SF=10 基准测试能力。

## Functionality

### Core Features

1. **SF=10 数据生成**
   - ~10GB 测试数据
   - 使用 dbgen 生成

2. **22 查询验证**
   - 全部 22 个查询通过
   - 记录执行时间

3. **资源监控**
   - 峰值 RSS
   - 无 OOM

## Acceptance Criteria

- [ ] SF=10 22/22 查询通过
- [ ] 无 OOM
- [ ] 性能数据记录

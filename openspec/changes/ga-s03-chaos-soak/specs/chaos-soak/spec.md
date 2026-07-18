# chaos-soak

混沌工程 SOAK 测试能力，集成到现有 SOAK 测试流程。

## Functionality

### Core Features

1. **Chaos Controller** (`chaos_inject.py`)
   - 统一的故障注入接口
   - 支持 Linux (tc, stress-ng) 和 macOS (部分)
   - 自动清理所有注入的故障

2. **Chaos Types**
   - I/O 延迟注入：通过 `tc qdisc` 模拟磁盘延迟
   - 内存压力：通过 `stress-ng --vm` 制造内存压力
   - 进程 kill：通过 `kill -9` 随机终止进程

3. **Recovery Verification**
   - 注入故障后监控服务恢复时间
   - 验证数据完整性（checksum 比较）
   - 超时自动失败

### User Interactions

- SOAK driver 调用 chaos_inject.py
- 通过命令行参数选择混沌实验类型
- 恢复阶段自动执行数据完整性验证

### Data Handling

- 混沌注入前记录数据 checksum
- 混沌注入后验证 checksum 一致性
- 临时文件在 cleanup 阶段删除

### Edge Cases

- 平台不支持：跳过该实验，记录警告
- 权限不足：需要 sudo，提示安装依赖
- 服务无法恢复：测试失败，记录日志

## Acceptance Criteria

- [ ] `chaos_inject.py` 存在且可执行
- [ ] I/O 延迟注入后 5s 内自动恢复
- [ ] 内存压力下无 OOM panic
- [ ] kill -9 后数据完整性验证通过
- [ ] Linux 平台全部通过

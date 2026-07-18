# chaos-recovery

故障恢复验证能力，验证 DWB 和 WAL 在极端条件下的恢复。

## Functionality

### Core Features

1. **Data Integrity Verification**
   - WAL 重放后数据一致性
   - Double-Write Buffer 恢复验证
   - MVCC 事务回滚验证

2. **Recovery Time Measurement**
   - 从故障到服务恢复的时间
   - 区分冷启动和热恢复

3. **Crash Recovery Scenarios**
   - 正常关闭 vs kill -9
   - WAL fsync 延迟模拟
   - Buffer Pool 部分刷写

### Data Handling

- 故障前生成数据快照（checksum）
- 故障后重放 WAL
- 验证快照一致性

### Edge Cases

- WAL 日志损坏：检测并报警
- DWB 页面不完整：回退到原始页面
- Buffer Pool 脏页丢失：按需重读

## Acceptance Criteria

- [ ] kill -9 后 WAL 完整恢复
- [ ] DWB 页面损坏回退正确
- [ ] 恢复时间 ≤ 5s（I/O 延迟场景）
- [ ] 数据零丢失验证

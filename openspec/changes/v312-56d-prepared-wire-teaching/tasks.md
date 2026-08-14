# Tasks — V312-56D: Prepared statement / wire protocol 教学实验

## Phase 1: 现状调研 ✅

### 1.1 已有 Prepared Statement 测试
- [x] `tests/compat/mysql_v3_12/prepared_stmt_roundtrip.sql` - 基本 roundtrip
- [x] `tests/integration/wire/mysql_client_prepared_test.rs` - MySQL client prepared statement 测试
- [x] `tests/integration/wire/mysql_wire_protocol_test.rs` - Wire protocol 测试

### 1.2 缺失项识别
- ❌ 没有完整的参数绑定正例/反例测试
- ❌ 没有参数数量/类型错误反例
- ❌ 没有 packet trace 输出
- ❌ LOAD DATA 没有 row-count/hash fixture
- ❌ TLS/compression 状态未明确

## Phase 2: Prepared Statement Fixtures

### 2.1 Roundtrip 正例

- [ ] 2.1.1 添加参数绑定正例 (整数、字符串) → teaching_sql_v3_12/prepared/
- [ ] 2.1.2 多次 execute 同一条 prepared statement
- [ ] 2.1.3 DEALLOCATE PREPARE 测试

### 2.2 Roundtrip 反例

- [ ] 2.2.1 参数数量不匹配 → teaching_sql_v3_12/prepared/wrong_param_count.sql
- [ ] 2.2.2 参数类型错误 → teaching_sql_v3_12/prepared/wrong_param_type.sql
- [ ] 2.2.3 execute 时参数不足
- [ ] 2.2.4 execute 时参数过多

## Phase 3: Wire Protocol 实验

- [ ] 3.1 COM_STMT_PREPARE/EXECUTE packet trace 输出 (文档)
- [ ] 3.2 error packet 格式说明 (文档)

## Phase 4: LOAD DATA Fixture

- [ ] 4.1 row-count 验证 → teaching_sql_v3_12/load_data/
- [ ] 4.2 hash 验证
- [ ] 4.3 空文件边界
- [ ] 4.4 特殊字符处理

## Phase 5: TLS/Compression 决策

- [ ] 5.1 评估 TLS/compression v3.12 Beta 完成可能性
- [ ] 5.2 若不能完成，定义为 DEFERRED

## Phase 6: 验证

- [ ] 6.1 `bash scripts/gate/check_v312_13_wire_load_data.sh` PASS
- [ ] 6.2 `bash scripts/gate/check_v312_21_mysql_compat.sh` PASS
- [ ] 6.3 `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol -- --nocapture` PASS

## Acceptance Criteria

- [ ] prepared statement roundtrip 有正例和错误参数数量/类型反例
- [ ] wire protocol 实验输出 packet trace 或结构化 summary
- [ ] LOAD DATA 教学 fixture 包含 row-count/hash
- [ ] TLS/compression 若不能在 v3.12 Beta 完成，显示为 DEFERRED

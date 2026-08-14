# Proposal — V312-56D: Prepared statement / wire protocol 教学实验

## Why

Issue #4254: COM_QUERY、COM_STMT_PREPARE/EXECUTE、text/binary result、error packet、reset、LOAD DATA 的受控路径未整理成可教学、可验证的实验。

## What Changes

### 1. Prepared Statement Roundtrip

- 正例: 完整的 prepare → execute → close 流程
- 反例: 参数数量不匹配、类型错误

### 2. Wire Protocol 实验

输出 packet trace 或结构化 summary:
- COM_QUERY packet 结构
- COM_STMT_PREPARE/EXECUTE/CLOSE 流程
- error packet 格式
- text/binary result 区别

### 3. LOAD DATA 教学 Fixture

包含:
- row-count 验证
- hash 验证
- 边界条件 (空文件、大文件、特殊字符)

### 4. TLS/Compression 决策

若不能在 v3.12 Beta 完成:
- 显示为 `DEFERRED`
- 明确 owner/expiry/关闭边界

## Capabilities

### New Capabilities

- **prepared statement 教学实验** - 完整流程正反例
- **wire protocol trace** - 结构化 packet 分析
- **LOAD DATA fixture** - 可验证的 row-count/hash

### Modified Capabilities

- 现有 wire protocol 实现 → 添加教学实验覆盖

## Non-goals

- 不实现完整的 MySQL prepared statement 协议所有角落情况
- 不实现 SSL connection setup 细节

## Acceptance Criteria

- [ ] prepared statement roundtrip 有正例和错误参数数量/类型反例
- [ ] wire protocol 实验输出 packet trace 或结构化 summary
- [ ] LOAD DATA 教学 fixture 包含 row-count/hash
- [ ] TLS/compression 若不能在 v3.12 Beta 完成，显示为 DEFERRED
- [ ] 运行 `bash scripts/gate/check_v312_13_wire_load_data.sh` PASS
- [ ] 运行 `bash scripts/gate/check_v312_21_mysql_compat.sh` PASS
- [ ] 运行 `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol -- --nocapture` PASS

## Issue Reference

Issue #4254 (V312-56D)

# openspec/3177 - P2-1 Audit Log (审计日志)

> **Issue**: #3177
> **作者**: Hermes Agent
> **日期**: 2026-06-05
> **Phase**: 5 (W9-10)
> **工作量**: 24h (按 V390_DEVELOPMENT_PLAN)
> **状态**: 借力已存在 crates/gmp/src/audit.rs (694 lines) + crates/executor/src/sql_log.rs (461 lines), 本次做主 SQL 接口整合 + G10 gate

## 一、问题分析

### 1.1 现状审计 (2026-06-05)

仓库已有**完整**审计基础设施:
- `crates/gmp/src/audit.rs` (694 lines):
  - `AuditAction` enum (Create/Update/Delete)
  - `AuditLog` struct (id, timestamp, user_id, action, table_name,
    record_id, old_value, new_value, ip_address, session_id, checksum)
  - `create_audit_log_table` (schema)
  - `record_audit_log` (insert)
  - `query_audit_logs` (filter)
  - `get_all_audit_logs`, `get_audit_log_by_id`
  - `AuditStats`, `get_audit_stats`
- `crates/security/src/audit.rs` (618 lines) - 安全审计
- `crates/executor/src/sql_log.rs` (461 lines) - SQL 执行日志

### 1.2 #3177 字段覆盖映射

| #3177 字段 | 已有覆盖 |
|------------|----------|
| who (user) | AuditLog.user_id ✅ |
| when (timestamp) | AuditLog.timestamp ✅ |
| what (action) | AuditLog.action (CREATE/UPDATE/DELETE) ✅ |
| target (table, row_id) | AuditLog.table_name, record_id ✅ |
| before_value (JSON) | AuditLog.old_value (String/JSON) ✅ |
| after_value (JSON) | AuditLog.new_value (String/JSON) ✅ |
| tx_id | AuditLog.session_id ✅ |
| source (TCP/REPL) | AuditLog.ip_address (TCP), 需扩展 REPL |
| checksum (hash chain) | AuditLog.checksum ✅ (P2-3 扩展) |

### 1.3 P2-1 任务真正需要补的 (按治理最小修改)

**A. SQL 接口整合** (新):
- `SHOW AUDIT LOG` parser + dispatcher
- `SELECT * FROM audit_events` 路由到 query_audit_logs
- `SET GLOBAL audit_log = ON/OFF` runtime 配置

**B. DML 路径挂钩** (新):
- execute_insert/update/delete 调用 record_audit_log
- 与 VtuGuard (#3169) 协调 (audit 在 VtuGuard 之后)

**C. G10 Gate** (新):
- 7 项检查

## 二、实施方案

### 2.1 范围限定

按治理 §2.1 最小修改 + 复用现有 audit.rs 基础:

**本次 PR 范围 (4 大块)**:

1. **新文件**: `tests/audit_log_harness.rs` (共享 helper, 10 self-tests)
2. **新文件**: `tests/audit_log_test.rs` (20+ tests 覆盖 5 类场景)
3. **新文件**: `scripts/gate/check_p21_audit_log.sh` (G10 gate)
4. **新文件**: `docs/openspec/3177-audit-log.md` (本文件)

**延后 (推 v3.10+)**:
- 性能优化 (审计 overhead < 5%): 当前 0 优化 baseline, v3.10+ 测试
- 多存储后端 (table/file/syslog): 当前只用 table, v3.10+ 扩展
- REPL 源标记: 当前仅 TCP, v3.10+ 扩展
- 真正的运行时 SQL 触发器集成: 需要重构 DML 主路径

### 2.2 Audit Log Harness 设计

```rust
// tests/audit_log_harness.rs (shared)
pub struct AuditEventBuilder {
    pub user: String,
    pub action: AuditAction,
    pub table: String,
    pub row_id: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

pub struct AuditReport {
    pub events_recorded: u32,
    pub events_queried: u32,
    pub checksum_verified: bool,
    pub overhead_percent: f64,
}

pub fn build_event(builder: AuditEventBuilder) -> AuditLog;
pub fn run_audit_test(events: Vec<AuditLog>) -> AuditReport;
```

### 2.3 20+ Tests (5 类)

| 类别 | Count | 示例 |
|------|-------|------|
| 1. schema | 3 | table exists, columns, indices |
| 2. record INSERT | 3 | basic, with values, multi-row |
| 3. record UPDATE | 3 | basic, before/after, row count |
| 4. record DELETE | 3 | basic, with old_value, cascade |
| 5. query | 4 | by time, by user, by table, by id |
| 6. SHOW AUDIT LOG | 2 | dispatcher, output format |
| 7. checksum | 2 | compute, verify |
| **TOTAL** | **20** | |

### 2.4 G10 Gate (7 checks)

1. `crates/gmp/src/audit.rs` exists + 编译通过
2. `tests/audit_log_test.rs` exists
3. `tests/audit_log_harness.rs` exists
4. ≥20 audit tests pass
5. AuditEvent struct 字段覆盖 (8 字段全有)
6. G10 gate scripts 7 checks pass
7. 借力 crates/gmp (no regression in 694-line audit.rs)

## 三、风险评估

| 风险 | 影响 | 缓解 |
|------|------|------|
| DML 路径挂钩破坏 TPC-H | 22/22 失败 | 单元测试 (不连真实 DML), 借力 sql_log |
| AuditLog.checksum 算法变更 | 兼容性 | 保持 sha256, 不改 hash |
| 性能 < 5% overhead | CI 慢 | 不在本次 PR 范围 |
| 借力 gmp crate | workspace deps | 已存在, 验证 |

## 四、验收标准 (G10 门禁)

```
✅ audit_log: ≥20 tests PASS
✅ 8 字段全覆盖 (who/when/what/target/before/after/tx_id/source)
✅ G10 gate: 7/7 PASS
✅ 871 L1 tests 不回归 (1555 当前)
✅ TPC-H 22/22 (G1 维持)
```

## 五、Subsumed Issues

- #3177 本身 (本任务)
- 与 P2-3 (#3179 Hash Chain) 互补 (checksum 已存在, 链式推 v3.10+)

## 六、回滚计划

如 audit_log_test 编译失败:
1. 删除 `tests/audit_log*.rs`
2. G10 gate 标记 DEFER
3. 现有 crates/gmp/src/audit.rs 保留 (不删除)

## 七、依赖

**上游**: P0-1 VtuGuard (#3169) - DML 必经 VtuGuard
**下游**: P2-2 Time Travel (#3178), P2-3 Hash Chain (#3179)

## 八、参考资料

- Issue #3177
- V390_DEVELOPMENT_PLAN.md §P2-1
- V390_TEST_PLAN.md §G10
- crates/gmp/src/audit.rs (694 lines, AuditLog + record + query + checksum)
- crates/executor/src/sql_log.rs (461 lines, global_execution_log)
- P0-1 #3169 VtuGuard (DML 路径协调)
- P0-4 #3172 Savepoint (tx_id 关联)
- P1-2 #3174 crash_test_harness (设计模型)

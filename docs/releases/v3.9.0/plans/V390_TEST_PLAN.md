# SQLRustGo v3.9.0 测试计划 — G1-G10 门禁 + 100+ 场景矩阵

<!-- env:blocked:no-ci -->

> **配套文档**: `V390_VERSION_PLAN.md` (战略) / `V390_DEVELOPMENT_PLAN.md` (任务)
> **创建日期**: 2026-06-05
> **基于**: ChatGPT 架构师 2026-06-05 G1-G10 门禁建议
> **目标**: 100% 覆盖可靠性 + 集成 + 审计

---

## 0. 测试总览 (G1-G10)

| Gate | 主题 | 测试类型 | 工作量 | 阶段 |
|------|------|----------|--------|------|
| G1 | TPC-H 保持 | 回归 (不退化) | 5h | Phase 1 |
| G2 | INT-2 ParallelExecutor 集成 | e2e + perf | 30h | Phase 2 |
| G3 | INT-3 Single Expression | 单测 + 回归 | 32h | Phase 1 |
| G4 | ARCH-3 Complete | 单元 + gate 自动化 | 40h | Phase 1 |
| G5 | SEM-1 Savepoint | 单元 + e2e | 28h | Phase 2 |
| G6 | Backup / Restore | 单元 + e2e + 100+ scenarios | 40h | Phase 3 |
| G7 | 24h Soak | 长周期 + 监控 | 48h | Phase 4 |
| G8 | Crash Matrix | 100+ scenarios | 40h | Phase 4 |
| G9 | Upgrade Test | 自动化 + 50+ scenarios | 40h | Phase 4 |
| G10 | Audit Log + 时间旅行 | 单元 + e2e | 44h | Phase 5 |
| **合计** | | | **347h** | |

**注意**: 总测试工作量 (347h) 包含在开发工作量 (451h) 内, 不重复计算.

---

## G1: TPC-H 保持 (22/22 不退化) [5h]

### 范围
- v3.8.0 已达成 22/22 PASS
- v3.9.0 必须保持不退化
- 任何新功能不能破坏 22/22

### 测试套件
- `tests/tpch_full_22_test.rs` (22/22)
- `tests/tpch_gate_test.rs` (22/22 inline)

### 验证
- [ ] `cargo test --test tpch_gate_test` → 22/22 PASS
- [ ] `cargo test --test tpch_full_22_test` → 22/22 PASS
- [ ] TPC-H hash 与 v3.8.0 完全一致

---

## G2: INT-2 ParallelExecutor 集成 [30h]

### 测试范围
- 单元测试: ParallelExecutor 6 tests
- e2e 测试: 多表 JOIN 走并行路径
- 性能测试: 4 worker ≤ sequential (不强求快, 只求不退化)

### 测试套件
```
tests/integration/parallel_executor_integration_test.rs
├── test_parallel_simple_select
├── test_parallel_two_table_join
├── test_parallel_three_table_join
├── test_parallel_with_aggregate
├── test_parallel_with_subquery
├── test_parallel_4_workers_no_regression
└── test_parallel_path_dispatch
```

### 验证
- [ ] 全部 7+ tests PASS
- [ ] `src/execution_engine.rs` grep "ParallelExecutor" ≥ 1 (实际调用, 非 capability only)
- [ ] TPC-H Q1 SF=1 4 worker ≤ sequential 1.5x

---

## G3: INT-3 Single Expression [32h]

### 测试范围
- 14 个委托分支各 1 个测试 = 14 tests
- 全量 expr regression
- TPC-H 保持 22/22

### 测试套件
```
tests/integration/expr_single_engine_test.rs
├── test_literal_delegation
├── test_binary_op_delegation
├── test_isnull_delegation
├── test_isnotnull_delegation
├── test_aggregate_delegation
├── test_like_delegation
├── test_notlike_delegation
├── test_between_delegation
├── test_casewhen_delegation
├── test_identifier_delegation
├── test_extract_delegation
├── test_functioncall_existing (1/15 已委托, 验证不退化)
└── ... (14+ tests)
```

### 验证
- [ ] 14+ tests PASS
- [ ] TPC-H 22/22 不退化
- [ ] `src/expr_utils.rs` 行数 < 533 (减少 14 个分支实现)

---

## G4: ARCH-3 Complete (VtuGuard 主路径强制) [40h]

### 测试范围
- 单元测试: 4 个 VtuGuard 新集成测试
- 静态分析: grep bypass = 0
- 自动化 gate: check_arch2_no_bypass.sh 移除白名单后仍 PASS

### 测试套件
```
tests/integration/arch3_complete_test.rs
├── test_vtu_guard_on_insert_main_path
├── test_vtu_guard_on_update_main_path
├── test_vtu_guard_on_delete_main_path
├── test_vtu_guard_assert_dml_safe_in_tx
└── test_vtu_guard_block_outside_tx

# 静态分析:
scripts/gate/check_arch2_no_bypass.sh
  期望: 移除 src/execution_engine.rs 白名单后, 0 bypass
```

### 验证
- [ ] 5+ tests PASS
- [ ] `grep -r "storage\.insert\|storage\.update\|storage\.delete" --include="*.rs" | grep -v "crates/storage/\|crates/executor/"` = **0 匹配**
- [ ] `check_arch2_no_bypass.sh` 全部 PASS
- [ ] TPC-H 22/22 不退化

---

## G5: SEM-1 Savepoint MVCC 真实还原 [28h]

### 测试范围
- 单元测试: Savepoint 创建/释放/回滚
- e2e 测试: GMP 业务场景 (10000 INSERT + SAVEPOINT + ROLLBACK 500 + COMMIT)
- MVCC 状态验证: ROLLBACK 后 SELECT 应回到 SAVEPOINT 之前

### 测试套件
```
tests/savepoint_mvcc_test.rs
├── test_savepoint_create
├── test_savepoint_release
├── test_savepoint_rollback_state_restored
├── test_savepoint_multiple_nested
├── test_savepoint_after_commit
├── test_savepoint_after_rollback
├── test_savepoint_gmp_business_scenario (10000 INSERT + SAVEPOINT + ROLLBACK 500 + COMMIT)
└── test_savepoint_with_concurrent_tx
```

### 验证
- [ ] 8+ tests PASS
- [ ] TPC-H 22/22 不退化
- [ ] ROLLBACK TO SAVEPOINT 真正还原 tuple 状态 (非仅 mark)

---

## G6: Backup / Restore (100+ scenarios) [40h]

### 测试范围
- CLI 命令: backup / restore / verify
- PITR (Point-in-Time Recovery)
- 备份文件 checksum 验证
- 100+ 场景 (db 大小, 事务状态, 压缩, 增量, 灾备等)

### 测试套件
```
tests/backup_restore_test.rs
├── 基础功能 (20 tests)
│   ├── test_backup_full
│   ├── test_backup_incremental
│   ├── test_restore_full
│   ├── test_restore_pitr
│   ├── test_verify_checksum
│   ├── test_backup_empty_db
│   ├── test_backup_with_lots_of_tables
│   ├── test_backup_with_wal
│   ├── test_restore_with_recovery
│   ├── test_restore_interrupted
│   └── ... (20+)
│
├── PITR 场景 (30 tests)
│   ├── test_pitr_to_specific_timestamp
│   ├── test_pitr_with_active_tx
│   ├── test_pitr_with_dropped_table
│   └── ... (30+)
│
├── 错误恢复 (30 tests)
│   ├── test_restore_corrupted_backup
│   ├── test_verify_invalid_checksum
│   ├── test_backup_disk_full
│   ├── test_restore_with_missing_wal
│   └── ... (30+)
│
└── GMP 业务 (20 tests)
    ├── test_backup_batch_records_10000
    ├── test_restore_after_corruption
    └── ... (20+)
```

### 验证
- [ ] 100+ tests PASS
- [ ] CLI `sqlrustgo backup/restore/verify` 全部可用
- [ ] PITR 时间精度 ≤ 1s

---

## G7: 24h Soak Test [48h]

### 测试范围
- 24h 连续运行 (CI 必跑)
- 72h + 168h 文档化 (手动跑, 季度)
- 监控: memory / fd / lock / WAL size / cache hit rate

### 监控指标
```
监控项                              阈值
─────────────────────────────────────
Memory RSS 增长                     < 5% over 24h
File descriptor 句柄数              < 1000
Active locks 数量                    < 100 (峰值)
WAL file size                        < 1GB (24h)
Cache hit rate                       > 95%
Query latency P99                    < 100ms (avg)
Crash 次数                            0
WAL 写入异常                          0
```

### 测试脚本
```
scripts/soak/soak_24h.sh
├── 启动 8 worker 并发, 混合 OLTP/OLAP 流量
├── 每 5 分钟采样 memory/fd/lock/wal
├── 24 小时后自动停止
├── 生成报告 (HTML + JSON)
└── 失败: 任一指标超阈值

scripts/soak/soak_72h.sh  (季度, Z6G4)
scripts/soak/soak_168h.sh (季度, Z6G4)
```

### 验证
- [ ] 24h Soak Test PASS (在 Z6G4 跑 1 次)
- [ ] 72h Soak Test PASS (季度)
- [ ] 168h Soak Test PASS (季度)
- [ ] 报告文档化

---

## G8: Crash Matrix (100+ scenarios) [40h]

### 测试范围
- 8 类崩溃注入 × 10+ 变体
- 每次注入后验证: 恢复后数据一致性 (commit 的存在, 未 commit 的不存在)

### 8 类 Crash 注入
```
1. SIGKILL (process_kill -9)         × 10 变体
   - INSERT 中, UPDATE 中, DELETE 中
   - BEGIN 后, COMMIT 前, COMMIT 后
   - ROLLBACK 中, ROLLBACK 后
   - 不同 WAL 状态 (空, 1 page, 100 pages)

2. SIGTERM (graceful shutdown)       × 5 变体
   - 正常优雅关闭
   - 强制立即
   - 中断在 COMMIT flush 中

3. Power loss (断电)                  × 10 变体
   - 模拟: rm -rf WAL/ 然后重启
   - 部分 WAL 文件
   - Corrupted last page

4. Disk full (磁盘满)                 × 10 变体
   - WAL append 失败
   - Snapshot 失败
   - Backup 失败

5. Network partition                  × 5 变体 (未来分布式)
   - (本版本跳过, 记入 v3.10+)

6. OOM (Out of Memory)                × 10 变体
   - 大量并发 INSERT
   - 大结果集 SELECT
   - 时序崩溃

7. WAL corruption                     × 10 变体
   - 字节翻转
   - 截断
   - 重新执行

8. Clock skew                         × 5 变体
   - 时钟回拨
   - 跨进程时间不一致
   - (本版本 5 变体, v3.10+ 全 10)
```

### 验证
- [ ] 60+ 真实测试 (跳过 5+5+10 变体未来版本)
- [ ] 全部 PASS (恢复后数据一致)

---

## G9: Upgrade Test (v3.8 → v3.9 数据兼容) [40h]

### 测试范围
- v3.8.0 → v3.9.0 数据兼容 (向后兼容)
- 自动检测 schema 兼容性
- 升级流程文档化
- 50+ upgrade scenarios

### 测试套件
```
tests/upgrade_test.rs
├── 基础 (10 tests)
│   ├── test_v380_data_v390_readable
│   ├── test_v380_schema_v390_compatible
│   ├── test_v380_wal_v390_replayable
│   └── ... (10+)
│
├── 数据完整性 (20 tests)
│   ├── test_all_tables_migrated
│   ├── test_all_indexes_migrated
│   ├── test_all_constraints_migrated
│   ├── test_all_sequences_migrated
│   └── ... (20+)
│
├── 升级场景 (20 tests)
│   ├── test_upgrade_during_load
│   ├── test_upgrade_with_active_tx
│   ├── test_rollback_after_upgrade
│   └── ... (20+)
```

### 验证
- [ ] 50+ tests PASS
- [ ] v3.8.0 dump → v3.9.0 load → 数据完整
- [ ] schema 兼容性自动检测

---

## G10: Audit Log + 时间旅行查询 [44h]

### G10.1: Audit Log [24h]

#### 测试范围
- `audit_events` 系统表
- DML 触发自动记录
- `SHOW AUDIT LOG` SQL
- Hash chain 完整性

#### 测试套件
```
tests/audit_log_test.rs
├── test_audit_insert_record
├── test_audit_update_record_before_after
├── test_audit_delete_record
├── test_audit_show_log_sql
├── test_audit_filter_by_user
├── test_audit_filter_by_time
├── test_audit_chain_integrity
├── test_audit_chain_tamper_detection
└── ... (20+ tests)
```

### G10.2: 时间旅行查询 [20h]

#### 测试范围
- `AS OF TIMESTAMP` 语法
- MVCC snapshot + WAL 历史
- 20+ 时间旅行 tests

#### 测试套件
```
tests/time_travel_test.rs
├── test_as_of_timestamp_select
├── test_as_of_with_where
├── test_as_of_with_join
├── test_as_of_with_aggregate
├── test_as_of_to_future_returns_empty
├── test_as_of_to_old_data_still_readable
├── test_gmp_batch_record_history
└── ... (20+ tests)
```

### 验证
- [ ] 40+ tests PASS
- [ ] Audit Log + Time Travel 都可用

---

## 11. 监控 / 告警 (持续)

### 11.1 关键指标 (运行时)
- 内存 / 句柄 / 锁
- WAL 大小 + 增长速率
- Cache hit rate
- Query latency P50/P95/P99
- Crash count = 0

### 11.2 告警阈值
- Memory > 2GB → WARN
- Lock contention > 30s → WARN
- WAL 1h 增长 > 100MB → WARN (可能写放大)
- Query P99 > 5s → WARN
- Crash → CRITICAL

### 11.3 实施
- `scripts/monitor/metrics_collector.rs` (持续运行)
- 集成到 `tools/sqlrustgo-gate` (已有)

---

## 12. 测试执行时间表

| Phase | 测试类型 | 工作量 | 阶段周 |
|-------|----------|--------|--------|
| Phase 0 | G1 回归 (TPC-H 保持) | 5h | W1-2 |
| Phase 1 | G3 + G4 (INT-3 + ARCH-3) | 32+40=72h | W1-2 |
| Phase 2 | G2 + G5 (INT-2 + Savepoint) | 30+28=58h | W3-4 |
| Phase 3 | G6 (Backup/Restore) | 40h | W5-6 |
| Phase 4 | G7 + G8 + G9 (Soak + Crash + Upgrade) | 48+40+40=128h | W7-8 |
| Phase 5 | G10 (Audit + Time Travel) | 44h | W9-10 |
| Phase 6 | G1 回归 + 性能 P3 | 5+85=90h | W11-12 |
| **合计** | | **437h** | 12 周 |

---

## 13. 测试不通过时的影响

| Gate | 失败影响 | 是否阻断 GA |
|------|----------|-------------|
| G1 | TPC-H 退化 | **是** (必须修) |
| G2 | ParallelExecutor 集成失败 | **是** (必须修) |
| G3 | INT-3 委托失败 | **是** (必须修) |
| G4 | ARCH-3 主路径不闭环 | **是** (阻断 v3.9.0) |
| G5 | Savepoint 还原失败 | **是** (阻断 v3.9.0) |
| G6 | Backup/Restore 失败 | **是** (阻断 v3.9.0) |
| G7 | Soak Test 失败 | **是** (暴露隐藏 bug) |
| G8 | Crash Matrix 失败 | **是** (暴露数据丢失风险) |
| G9 | Upgrade Test 失败 | **是** (升级兼容性) |
| G10 | Audit/Time Travel 失败 | ⚠️ (不阻断, 但 GMP 业务受损) |

---

**总结**: v3.9.0 测试 = 10 个新门禁 + 437h 测试工作量, 覆盖从单元到 168h Soak 全谱, 重点在**可靠性** (G6-G9 占 60%).

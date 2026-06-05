# SQLRustGo v3.9.0 开发计划 — P0/P1/P2/P3 详细任务

<!-- env:blocked:no-ci -->

> **配套文档**: `V390_VERSION_PLAN.md` (战略定位) / `V390_TEST_PLAN.md` (测试)
> **创建日期**: 2026-06-05
> **资源分配**: 架构债 40% / 可靠性 35% / GMP 审计 15% / 性能 10% / 新 SQL 0%
> **基于**: ChatGPT 架构师 2026-06-05 战略建议

---

## 0. 任务总览

| 优先级 | 任务数 | 工作量 | 占比 | 阶段 |
|--------|--------|--------|------|------|
| **P0 (必须)** | 4 | 130h | 29% | Phase 1-2 |
| **P1 (生产可靠性)** | 4 | 168h | 37% | Phase 3-4 |
| **P2 (GMP 能力)** | 3 | 68h | 15% | Phase 5 |
| **P3 (性能优化)** | 5 | 85h | 19% | Phase 6 |
| **合计** | **16** | **451h** | **100%** | 12 周 |

---

## P0 (Phase 1-2, 4 项, 130h, 40% 资源)

> **目标**: 关闭 5 跨版本债 (INT-2/3, ARCH-3 Complete, SEM-1)
> **依赖**: 无 (基础修复)

### P0-1: ARCH-3 完整闭环 (VtuGuard 主路径强制) [40h, 5 天]

**来源**: #3109 + #3129 (子集已修)
**依赖**: #3129 PR-3152 已修 Blocker-1+2
**关联 PR**: v3.9.0 PR-9001

#### 目标
所有写操作必经 VtuGuard 主路径:
```
INSERT/UPDATE/DELETE
   ↓
ExecutionEngine
   ↓
VtuGuard::assert_dml_safe + VtuGuard::execute_dml
   ↓
TransactionManager
   ↓
Storage
```

#### 完成标准
1. `grep -r "storage\.insert\|storage\.update\|storage\.delete" --include="*.rs" | grep -v "crates/storage/\|crates/executor/"` **0 匹配** (绕过 = 0)
2. `check_arch2_no_bypass.sh` 中 `src/execution_engine.rs` 白名单**移除**
3. `arch2_no_bypass` gate 自动检测通过
4. 全部 49 ACID tests 仍 PASS
5. 新增 3 个 #3109 集成测试

#### 实施步骤
1. SPEC 编写 (`specs/gate/SPEC-V390-ARCH3-COMPLETE.md`)
2. 修改 `src/execution_engine.rs` 的 `execute_insert/update/delete`:
   - 包裹 `VtuGuard::new(self.storage.clone(), "execution_engine")` 
   - `guarded.assert_dml_safe(op, table)` + `guarded.execute_dml(|s| s.insert(...))`
3. 移除 `scripts/gate/check_arch2_no_bypass.sh` 中 `src/execution_engine.rs` 白名单
4. 添加 3 个 #3109 集成测试
5. 验证 `grep bypass = 0`

---

### P0-2: INT-3 收敛 (Single Expression Engine) [32h, 5-6 天]

**来源**: #3108 / #3146
**关联 PR**: v3.9.0 PR-9002/9003

#### 目标
消除 expr 双实现, 统一到单一 `sqlrustgo_executor::expr::eval_*`:
- 14/15 未委托分支 (Literal, BinaryOp, IsNull, IsNotNull, Aggregate, Like, NotLike, Between, CaseWhen, Identifier, FunctionCall(EXTRACT), etc.) 重构为委托
- 删除重复实现, 保留单一 source of truth

#### 完成标准
1. `src/expr_utils.rs` 14/15 分支全部委托到 `sqlrustgo_executor::expr::*`
2. `crates/executor/src/expr/mod.rs` 重新导出主实现
3. 全量 expr regression test 仍 PASS
4. TPC-H 22/22 仍 PASS
5. 新增 14 个委托测试 (每个分支 1 个)

#### 实施步骤
1. SPEC 编写 (`specs/debt/INT3_EXPR_MERGE_SPEC.md`)
2. 重构 `src/expr_utils.rs` (533 行) 的 14 分支
3. 更新 `crates/executor/src/expr/mod.rs` re-exports
4. 跑 TPC-H 22/22 + 868 lib tests 回归
5. 删除 `crates/executor/src/expr/mod.rs` 旧独立实现

---

### P0-3: INT-2 ParallelExecutor 主路径集成 [30h, 4-5 天]

**来源**: #3108 / #3146
**关联 PR**: v3.9.0 PR-9004/9005

#### 目标
I-12 ParallelExecutor 从 capability 升级到 integration:
- `crates/executor/src/lib.rs` 加 `pub mod parallel_executor;`
- `src/execution_engine.rs` SELECT 路径 cost estimation → ParallelExecutor
- e2e test: 多表 JOIN 走并行路径
- TPC-H Q1 SF=1 4 worker 性能 ≤ sequential

#### 完成标准
1. `lib.rs` 导出 `parallel_executor`
2. `ExecutionEngine::execute_select` 加 cost estimation
3. TPC-H Q1 SF=1 4 worker ≤ sequential (不强求快, 只求不退化)
4. 22/22 TPC-H 仍 PASS (parallel + sequential 双路径)
5. e2e test 验证并行路径触发

#### 实施步骤
1. SPEC 编写 (`specs/debt/I12_PARALLEL_EXECUTOR_INTEGRATION_SPEC.md`)
2. lib.rs 公开
3. `execution_engine.rs` 加 `if worker_count > 1 && cost > threshold` 切换
4. e2e test 验证 4 worker 并行执行
5. 性能对比 (parallel vs sequential, TPC-H Q1 SF=1)

---

### P0-4: Savepoint MVCC 真实还原 [28h, 3.5 天]

**来源**: SEM-1
**关联 PR**: v3.9.0 PR-9006

#### 目标
SEM-1 (Savepoint ROLLBACK 真正还原 MVCC 状态):
- `crates/transaction/src/savepoint.rs` 实现 MVCC snapshot restoration
- ROLLBACK TO SAVEPOINT 真正还原 tuple 状态 (而非仅 mark rolled back)
- 多 SAVEPOINT 嵌套支持
- 真实 GMP 业务场景: 导入 10000 条 → SAVEPOINT → 回滚 500 → COMMIT

#### 完成标准
1. ROLLBACK TO SAVEPOINT 真正还原 MVCC 状态
2. 多 SAVEPOINT 嵌套支持
3. `tests/savepoint_test.rs` 8+ tests 全 PASS
4. TPC-H 22/22 仍 PASS
5. GMP 业务 e2e test (10000 INSERT + SAVEPOINT + ROLLBACK + COMMIT)

#### 实施步骤
1. SPEC 编写 (`specs/gate/SEM1_SAVEPOINT_MVCC_SPEC.md`)
2. `savepoint.rs` 添加 snapshot restoration
3. `MVCCTransaction` 集成 savepoint
4. 创建 `tests/savepoint_test.rs` (现 v3.8.0 不存在)
5. 跑 GMP 业务 e2e

---

## P1 (Phase 3-4, 4 项, 168h, 35% 资源)

> **目标**: 数据库死了以后还能不能回来 (核心问题)
> **依赖**: P0 完成

### P1-1: Backup / Restore 实现 [40h, 5 天]

**关联 PR**: v3.9.0 PR-9007

#### 目标
完整 backup / restore / verify 命令 + 端到端测试:
```bash
sqlrustgo backup [--output=path] [--format=full|incremental]
sqlrustgo restore [--input=path] [--target-time=TIMESTAMP]  # PITR
sqlrustgo verify [--input=path]  # checksum verification
```

#### 完成标准
1. CLI 三个命令 (backup/restore/verify) 实现
2. PITR (Point-in-Time Recovery) 支持
3. 100+ backup/restore 场景 PASS
4. 备份文件 checksum 验证
5. `tests/backup_restore_test.rs` 50+ tests
6. GMP 业务场景 e2e (每日备份 + 恢复演练)

#### 实施步骤
1. SPEC (`specs/gate/SPEC-V390-BACKUP-RESTORE.md`)
2. `src/bin/backup.rs` (CLI)
3. `src/backup_engine.rs` (核心逻辑)
4. `tests/backup_restore_test.rs` 50+ tests
5. 端到端 GMP 场景

---

### P1-2: Crash Test Framework (100+ scenarios) [40h, 5 天]

**关联 PR**: v3.9.0 PR-9008

#### 目标
建立 Crash Test Framework, 100+ 真实 crash 场景:
- INSERT 时崩溃
- COMMIT 前崩溃
- COMMIT 后崩溃
- WAL 写一半崩溃
- Undo 阶段崩溃
- 各种 race conditions

#### 完成标准
1. `tests/crash_matrix_test.rs` 100+ scenarios
2. 8 类崩溃场景 × 10+ 变体
3. 所有 scenario PASS
4. Crash 后数据一致性验证 (commit 的数据存在, 未 commit 的不存在)
5. 集成到 CI (cargo test --test crash_matrix)

#### 实施步骤
1. SPEC (`specs/gate/SPEC-V390-CRASH-MATRIX.md`)
2. `tests/crash_matrix_test.rs` 框架
3. 8 类 crash injector:
   - SIGKILL (process_kill -9)
   - SIGTERM (graceful shutdown)
   - Power loss (断电)
   - Disk full (磁盘满)
   - Network partition
   - OOM
   - WAL corruption
   - Clock skew
4. 每个注入 × 10+ 变体
5. 恢复后一致性验证

---

### P1-3: Soak Test (24h / 72h / 168h) [48h, 6 天]

**关联 PR**: v3.9.0 PR-9009

#### 目标
7×24h 连续运行测试, 验证:
- 无内存泄漏
- 无句柄泄漏
- 无锁泄漏
- WAL 不异常增长
- 缓存稳定

#### 完成标准
1. `scripts/soak/soak_24h.sh` 实现
2. `scripts/soak/soak_72h.sh` 实现
3. `scripts/soak/soak_168h.sh` 实现
4. 24h Soak Test PASS (CI 必跑)
5. 72h + 168h 文档化 (手动跑, 季度)
6. 监控指标: memory / fd / lock / WAL size / cache hit rate

#### 实施步骤
1. SPEC (`specs/gate/SPEC-V390-SOAK-TEST.md`)
2. `scripts/soak/soak_24h.sh` 主脚本
3. 监控脚本 (memory/fd/lock 采样)
4. 报告生成 (HTML + JSON)
5. 跑 24h 真实测试 (在 Z6G4)
6. 72h/168h 在 Z6G4 季度跑

---

### P1-4: Upgrade Test (v3.8 → v3.9 数据兼容) [40h, 5 天]

**关联 PR**: v3.9.0 PR-9010

#### 目标
建立升级兼容性自动测试:
- v3.8.0 → v3.9.0 数据可读 (向后兼容)
- v3.9.0 → v3.8.0 数据不可读 (向前兼容是 v3.9.0 特性)
- 自动检测 schema 兼容性
- 升级流程文档化

#### 完成标准
1. `tests/upgrade_test.rs` 自动化测试
2. v3.8.0 数据 + v3.9.0 二进制 → 数据完整可读
3. schema 兼容性自动检测脚本
4. 升级 checklist 文档
5. 50+ upgrade scenarios

#### 实施步骤
1. SPEC (`specs/gate/SPEC-V390-UPGRADE-TEST.md`)
2. `tests/upgrade_test.rs` 框架
3. v3.8.0 数据生成 (dump 脚本)
4. v3.9.0 加载 + 验证 hash
5. 不兼容场景检测

---

## P2 (Phase 5, 3 项, 68h, 15% 资源)

> **目标**: GMP 业务能力 (审计 + 时间旅行)
> **依赖**: P0 + P1 完成

### P2-1: Audit Log (审计日志) [24h, 3 天]

**关联 PR**: v3.9.0 PR-9011

#### 目标
完整审计日志:
- `audit_events` 表 (内置)
- 记录: 谁 / 何时 / 改了什么 / SQL/原始值/新值
- `SHOW AUDIT LOG` 或 `SELECT * FROM audit_events`
- 不可篡改 (hash chain)

#### 完成标准
1. `audit_events` 系统表
2. DML 触发 audit log 自动记录
3. `SHOW AUDIT LOG` SQL
4. hash chain 完整性验证
5. 20+ audit log tests

#### 实施步骤
1. SPEC (`specs/major/SPEC-V390-AUDIT-LOG.md`)
2. `crates/executor/src/audit.rs` (核心)
3. `audit_events` 系统表 schema
4. DML 触发点集成
5. `SHOW AUDIT LOG` parser + executor

---

### P2-2: 时间旅行查询 (MVCC) [24h, 3 天]

**关联 PR**: v3.9.0 PR-9012

#### 目标
时间旅行查询 (基于 MVCC + WAL):
```sql
SELECT * FROM batch_record
AS OF TIMESTAMP '2026-06-01 12:00:00';
```

#### 完成标准
1. `AS OF TIMESTAMP` 语法支持
2. parser + executor 集成
3. 基于 MVCC snapshot + WAL 历史
4. 20+ 时间旅行 tests
5. 真实 GMP 业务场景 (批记录回溯)

#### 实施步骤
1. SPEC (`specs/major/SPEC-V390-TIME-TRAVEL.md`)
2. parser 加 `AS OF TIMESTAMP` 解析
3. executor 从 MVCC snapshot 读取
4. WAL 历史集成 (PITR-like)
5. e2e test

---

### P2-3: 不可篡改审计链 (Hash Chain) [20h, 2.5 天]

**关联 PR**: v3.9.0 PR-9013

#### 目标
每条 audit record 包含前一 record 的 hash, 形成不可篡改链:
```
Record A: hash(A) = h(A.payload)
Record B: hash(B) = h(B.payload + hash(A))
Record C: hash(C) = h(C.payload + hash(B))
...
```

#### 完成标准
1. hash chain 实现
2. `VERIFY AUDIT CHAIN` SQL
3. 篡改检测 (修改 A 则 B/C 全部失效)
4. 10+ hash chain tests

#### 实施步骤
1. SPEC (`specs/major/SPEC-V390-HASH-CHAIN.md`)
2. `audit_events.payload` 包含 `prev_hash`
3. `VERIFY AUDIT CHAIN` 实现
4. 性能测试 (每秒 1k+ audit 写入)

---

## P3 (Phase 6, 5 项, 85h, 10% 资源)

> **目标**: 性能优化 5 步路线
> **依赖**: P0+P1+P2 完成

### P3-1: Prepared Statement Cache [12h, 1.5 天]
- LRU cache for parsed statements
- 缓存命中率 ≥ 80% (web 流量)

### P3-2: Statistics (ANALYZE TABLE) [16h, 2 天]
- 表/索引统计
- row count + distinct value count
- 优化器基础数据

### P3-3: Cost Optimizer [24h, 3 天]
- 基于 Statistics 的 cost 模型
- JOIN 顺序选择

### P3-4: INT-2 ParallelExecutor 优化 (后续) [18h, 2.5 天]
- 在 P0-3 集成基础上优化
- Worker 池调优
- 数据分片策略

### P3-5: SIMD 集成 SQL Executor [15h, 2 天]
- 向量化执行 (filter/aggregate)
- 谨慎: 收益需验证, 不要破坏正确性

---

## 与 v3.8.0 文档的引用

- `ga/V380_GA_CLOSURE_TECHNICAL_DEBT_AND_ROADMAP.md` §3: 已校准为本计划对齐
- `archived/INT_DEBT_REMEDIATION_PLAN.md`: INT-2/3 详细债清单 (P0-2/3 输入)
- `archived/ARCH_SEM_DEBT_REMEDIATION_PLAN.md`: ARCH-3/SEM-1 详细债清单 (P0-1/4 输入)

---

**总结**: v3.9.0 = 16 个任务, 451h, 12 周, 资源按 ChatGPT 建议重新分配 (40% 架构 / 35% 可靠性 / 15% GMP / 10% 性能 / 0% 新 SQL).

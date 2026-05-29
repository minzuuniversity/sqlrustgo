# SQLRustGo 版本演化计划

> **版本**: v3.7.0
> **更新日期**: 2026-05-30
> **战略定位**: 集成债务清算 + 协议栈统一
> **核心原则**: 修复跨版本（v1.2.0~v3.6.0）集成缺陷，统一执行路径

---

## 一、战略定位（关键转向）

### 从：数据库内核工程
### 到：教学数据库产品（Teaching DBMS）

```
💡 核心洞察：

❌ 不要试图完全兼容 MySQL（工程量爆炸）
✅ 正确策略：兼容"教材"，而不是兼容"数据库"

🎯 最终目标：

v2.6.0 = 生产就绪（替代 MySQL 简单生产环境）
v2.7.0 = 分布式架构
```

---

## 二、版本语义化与技术范式

| 主版本 | 技术范式 | 成熟度等级 | 核心特征 | 对标系统 |
|--------|----------|------------|----------|----------|
| 1.x | 单机基础数据库 | L3 | 火山模型、SQL-92、MVCC、B+Tree | early PostgreSQL |
| 2.x | 高性能分析引擎 | L4 | 向量化、列存、CBO、并行执行 | DuckDB |

**演进路径**：1.x（教学可用）→ 2.x（超越 MySQL）

---

## 三、版本路线（教学 DBMS 战略）

### 战略总览

```
v2.5.0 ✅      v2.6.0 (开发中)                v2.7.0
  MVCC/Graph     SQL-92完整+生产就绪             分布式架构
```

### 详细版本规划

| 版本 | 代号 | 核心目标 | 关键功能 | 状态 |
|------|------|----------|----------|------|
| **v2.5.0** | **统一查询** | MVCC/Vector/Graph | 已发布 |
| **v2.6.0** | **生产就绪** | SQL-92 完整、MVCC SSI | Alpha 开发中 |
| **v2.7.0** | **分布式架构** | 分片、复制、分布式事务 | 规划中 |

### v2.6.0 开发任务

| 类别 | 功能 | Issue |
|------|------|-------|
| P0 | 功能集成 (索引扫描、CBO、存储过程、触发器、WAL) | #1497 |
| P0 | SQL 语法扩展 (聚合函数、JOIN、GROUP BY) | #1498 |
| P0 | MVCC SSI (可串行化快照隔离) | #1389 |
| P1 | DELETE 语句、FULL OUTER JOIN | #1380 |
| P2 | 覆盖率提升 49% → 70% | #1480 |

---

## 四、v1.7 核心版本详解（MySQL 教学替代版）

### 🔥 一站式替代 MySQL 教学

#### Epic-01~04: SQL + 可观测性（原 v1.7）

| 功能 | 说明 |
|------|------|
| UNION, UNION ALL, INTERSECT, EXCEPT | 集合运算 |
| VIEW | 视图支持 |
| EXPLAIN, EXPLAIN ANALYZE | 执行计划可视化 |
| MySQL 风格错误 | Unknown column, Table not found, Duplicate key |

#### Epic-05: 约束与外键（原 v1.8）

```sql
FOREIGN KEY (user_id) REFERENCES users(id)
```

#### Epic-06: MySQL 兼容语法（原 v1.8）

```sql
-- AUTO_INCREMENT
CREATE TABLE users (
    id INT PRIMARY KEY AUTO_INCREMENT,
    name VARCHAR(255)
);

-- LIMIT offset, count
SELECT * FROM orders LIMIT 10, 20;

-- SHOW / DESCRIBE
SHOW TABLES;
DESCRIBE orders;

-- 常用函数
NOW(), COUNT(), DATE_FORMAT()
```

#### Epic-07: CLI 工具完善（原 v1.8）

```bash
sqlrustgo

支持：
.tables
.schema orders
.indexes orders
```

#### Epic-08: 稳定性强化（原 v1.9）

- WAL 恢复强化
- Crash 安全
- 长时间运行测试

#### Epic-09: 覆盖率提升（原 v1.9）

- ≥ 85%

#### Epic-10: 教学支持（原 v1.9）

```bash
SQLRUSTGO_TEACHING_MODE=1
```

效果：
- 禁用优化器（便于教学）
- 强制 EXPLAIN 输出
- 更详细日志

教学资源：
- 12 个标准实验
- MySQL → SQLRustGo 对照表
- Lab 文档

---

## 五、v2.0 核心版本详解（高性能分析）

### 🔥 超越 MySQL

#### 1️⃣ 向量化执行（核心革命）

```rust
// DataChunk 格式
struct DataChunk {
    columns: Vec<ColumnArray>,
    num_rows: usize,
}

// SIMD 加速
impl AggregateFunction for Sum<i32> {
    fn sum_batch(&self, chunk: &DataChunk) -> ScalarValue {
        // SIMD 加速聚合
    }
}
```

#### 2️⃣ 列存（分析能力）

- Columnar storage
- Projection pushdown
- Parquet 支持

#### 3️⃣ 教学增强（差异化）

> MySQL 做不到的：

- 可视化执行 pipeline
- 算子级 profiling
- vectorized trace

---

## 六、可观测性演进

| 版本 | 新增可观测能力 |
|------|----------------|
| v1.7 | **EXPLAIN ANALYZE**（核心亮点）, 算子级耗时, 教学模式 |

---

## 七、成熟度演进

```
L1 (Toy)   →   L2 (Query Engine)   →   L3 (Mini DBMS)   →   L4 (Analytical DB)
                   v1.7                    v2.0
               能替代MySQL教学          超越MySQL
```

---

## 八、风险与应对

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 试图完全兼容 MySQL | 极高 | ❌ 禁止！只兼容"教材" |
| MySQL 协议实现难度 | 高 | 先做 CLI，协议可选 |
| 教学文档工作量 | 中 | 参考现有 MySQL 教材 |
| 向量化开发难度 | 高 | v2.0 预留充足时间 |

---

## 九、版本号与分支策略

| 策略 | 说明 |
|------|------|
| 开发分支 | develop/v1.7.0, develop/v2.0 |
| 发布分支 | release/v1.7.0, release/v2.0 |
| 主分支 | main 始终指向最新稳定版 |

---

## 十、一句顶级结论

```
💥 你现在不是在做数据库
👉 而是在做"下一代数据库教学平台"
```

---

## 十一、关联文档

| 文档 | 说明 |
|------|------|
| `docs/plans/2026-03-21-v170-release-plan.md` | v1.7.0 详细计划 |
| `docs/releases/v1.6.1/RELEASE_GATE_CHECKLIST.md` | v1.6.1 门禁参考 |

---

## 十二、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-03-12 | 初始版本 |
| 2.0 | 2026-03-13 | v1.x/v2.x/v3.x 重构 |
| 3.0 | 2026-03-13 | 工程优化版 |
| 4.0 | 2026-03-18 | 整合 v1.x 版本 |
| 5.0 | 2026-03-21 | SQL-92 路线图 |
| 6.0 | 2026-03-21 | 教学 DBMS 战略定位 |
| **7.0** | **2026-03-21** | **v1.7 合并 v1.8+v1.9，v2.0 独立** |
| **8.0** | **2026-04-09** | **v2.4.0 Graph Engine + OpenClaw** |

---

**文档状态**: 定稿
**制定日期**: 2026-03-21
**最后更新**: 2026-03-21
**制定人**: yinglichina8848
**战略定位**: 教学数据库产品（Teaching DBMS）
---

## v2.1 - 运维自动化版 (2026年4月)

**目标**: 省心线初级 - 监控、告警、备份CLI

### 交付物
- Prometheus 监控端点
- Grafana Dashboard
- 慢查询日志
- 一键备份/恢复 CLI
- 配置热更新
- 性能基准: 1500+ QPS

### Issue 列表
- #1013 Prometheus 监控端点
- #1014 慢查询日志系统
- #1015 健康检查端点
- #1016 mysqldump 兼容导入
- #1017 物理备份 CLI
- #1018 增量备份工具
- #1019 备份恢复验证
- #1121 配置热更新
- #1122 版本升级脚本
- #1123 日志轮转
- #1131 查询缓存优化
- #1132 连接池

### 状态
📋 规划完成 (2026-03-28)

---

## v2.2 - 故障自动化版 (2026年5月)

**目标**: 省心线中级 - 自动切换、读写分离、Web面板

### 交付物
- 自动故障检测 (< 5s)
- 自动主从切换 (< 30s)
- 脑裂防护
- 读写分离代理
- 备库延迟检测
- 运维 Web 面板
- 性能基准: 2000+ QPS

### Issue 列表
- #1025 自动故障检测
- #1026 自动主从切换
- #1027 脑裂防护
- #1028 故障恢复通知
- #1029 读写分离代理
- #1030 备库延迟检测
- #1031 延迟阈值降级
- #1032 运维 Web 面板
- #1033 SQL 执行界面
- #1034 备份管理界面
- #1035 SHOW PROCESSLIST
- #1036 KILL 命令
- #1037 SHOW STATUS 完善

### 状态
📋 规划完成 (2026-03-28)

---

## v3.0 - MySQL 5.6 兼容版 (2026年6-7月)

**目标**: MySQL 5.6 兼容度 80%+，真正可替代 MySQL

### 交付物
- 触发器 (行级/语句级)
- 存储过程 + 存储函数
- 分区表 (RANGE/KEY/HASH)
- 全文索引 + 中文分词
- Prepared Statements + 缓存
- GTID 复制
- Auto Tuning
- JSON 函数
- CTE RECURSIVE
- 性能基准: 5000+ QPS
- MySQL 5.6 兼容度: ≥80%

### Issue 列表
- #1038 触发器语法解析
- #1039 触发器执行引擎
- #1040 行级/语句级触发器
- #1041 存储过程基础
- #1042 存储函数
- #1043 分区表语法解析
- #1044 分区表物理存储
- #1045 分区裁剪优化
- #1046 分区表 DDL
- #1047 全文索引语法
- #1048 倒排索引实现
- #1049 MATCH 查询
- #1050 中文分词
- #1051 PREPARE 语句解析
- #1052 EXECUTE 执行
- #1053 PreparedStatement 缓存
- #1054 GTID 复制
- #1055 延迟复制
- #1056 并行复制
- #1057 Buffer Pool 自动调参
- #1058 慢查询自动分析
- #1059 索引推荐
- #1060 JSON 数据类型
- #1061 JSON 函数
- #1062 WITH RECURSIVE 语法
- #1063 递归执行引擎

### 状态
📋 规划完成 (2026-03-28)

---

## 省心线达成时间表

| 版本 | 目标月份 | 省心程度 | 成本估算 |
|------|----------|----------|----------|
| v2.0 | 2026-03 | ⭐⭐ 功能可用，需手动运维 | ¥100-600 |
| v2.1 | 2026-04 | ⭐⭐⭐ 半自动化，有告警 | +¥100-300 |
| v2.2 | 2026-05 | ⭐⭐⭐⭐ 自动恢复，1人值守 | +¥100-300 |
| v3.0 | 2026-06-07 | ⭐⭐⭐⭐⭐ MySQL 5.6 兼容 | +¥200-600 |

**总计**: ¥500-1,800 达到省心线

---

## v3.7.0 - 集成债务清算（2026年5月）

**目标**: 修复跨版本（v1.2.0~v3.6.0）集成缺陷，统一执行路径

### 背景

v3.6.0 Alpha Gate 发现覆盖率 32.59%（Z440），深入分析发现核心问题是**跨版本集成债务**：

| 缺陷 | 跨度 | 对应 Issue |
|------|------|-----------|
| DML 不经过 WAL/TransactionManager | v1.2.0~v3.6.0（6版本） | #2576 |
| ParallelVolcanoExecutor 孤岛 | v2.6.0~v3.6.0（4版本） | #2570, #2577 |
| expr crate 孤岛 | v3.0.0~v3.6.0（2版本） | 新发现 |
| mysql-server 未集成 | v2.6.0~v3.6.0（4版本） | #2583 |

**根因**：执行路径分裂（双路径并存）+ 存储层与事务层从未连接。

### 交付物

#### P0（Alpha 前必须完成）

| Issue | 功能 | 验收标准 |
|-------|------|----------|
| INT-1 | WAL 集成：DML 经过 TransactionManager/WAL | INSERT/UPDATE/DELETE 经 WAL；COMMIT/ROLLBACK 正确持久化 |
| INT-2 | ParallelVolcanoExecutor 集成到主执行链路 | `--parallel` 参数启用；TPC-H 并行模式正确执行；覆盖率 +10pp |

#### P1（Beta 前计划完成）

| Issue | 功能 | 验收标准 |
|-------|------|----------|
| INT-3 | expr crate 整合到 executor | executor 使用 expr crate；移除内联重复代码 |
| INT-4 | mysql-server 协议栈统一 | COM_QUERY 统一入口；移除 server/lib.rs 重复实现 |

### 技术方案

#### INT-1：WAL 集成方案

```
StorageEngine trait 新增方法:
  - begin_transaction() -> TransactionId
  - commit(txn_id: TransactionId) -> SqlResult<()>
  - rollback(txn_id: TransactionId) -> SqlResult<()>

DML 执行路径:
  LocalExecutor → TransactionManager → WAL-backed StorageEngine
```

#### INT-2：ParallelVolcanoExecutor 集成方案

```
LocalExecutor 增加并行模式开关:
  - --parallel 启用 ParallelVolcanoExecutor
  - TaskScheduler 与 Rayon 集成
  - QueryRouter 执行器选择逻辑
```

#### INT-3：expr crate 整合方案

```
1. executor 内联表达式求值 → 替换为调用 expr crate
2. 移除 executor 重复代码
3. expr crate API 标准化
```

#### INT-4：mysql-server 协议统一方案

```
1. mysql-server COM_QUERY 处理作为主协议栈入口
2. 统一 PhysicalPlan pipeline
3. 移除 server/src/lib.rs 中独立实现
```

### Alpha Gate 检查项

| ID | 检查 | 阈值 |
|----|------|------|
| G1 | Build (release) | ✅ PASS |
| G2 | Test (lib) | 1200+ tests PASS |
| G3 | Clippy | ✅ PASS (zero warnings) |
| G4 | Format | ✅ PASS |
| G5 | Coverage L1 | ≥ 75%（Z440 测量） |

### Issue 列表

| Issue | 说明 | 优先级 |
|-------|------|--------|
| #2576 | DML 不经过 TransactionManager/WAL | P0 |
| #2570 | ParallelVolcanoExecutor 未集成到主执行链路 | P0 |
| #2577 | ParallelVolcanoExecutor 孤岛（44 tests isolated） | P0 |
| INT-3 | expr crate 孤岛（v3.0.0~v3.6.0） | P1 |
| #2583 | DML 执行路径统一到 PhysicalPlan pipeline | P1 |

### v3.7.0 Alpha 时间线

| 日期 | 里程碑 |
|------|--------|
| 2026-05-30 | v3.7.0 开发分支创建 |
| 2026-06-06 | Alpha Gate（覆盖率 75%+） |
| 2026-06-13 | Beta Gate（所有 P0 修复完成） |

---

## v3.6.0 - 协议栈整合（2026年5月）

**目标**: MySQL 协议栈完整 + TPC-H SF=1 基线

### 交付物

- ✅ MySQL COM_QUERY 协议处理
- ✅ TPC-H SF=1 22/22 查询基线
- ✅ SIMD 加速（sum_i64 阈值调度）
- ✅ WAL 验证工作区（TI-3）
- ❌ Alpha Gate FAIL（覆盖率 32.59% Z440）

### 详细记录

| 文档 | 说明 |
|------|------|
| `ALPHA_GATE_REPORT_v3.6.0.md` | Alpha 门禁结果 |
| `INTEGRATION_DEBT_REPORT.md` | 跨版本集成债务分析 |
| `BENCHMARK.md` | TPC-H SF=1 基线数据 |

---

## 完整版本历史

详细版本变更日志、功能矩阵、测试报告请查阅: [VERSION_HISTORY.md](./VERSION_HISTORY.md)

---

*文档更新: 2026-05-30*

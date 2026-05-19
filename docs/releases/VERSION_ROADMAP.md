# SQLRustGo 版本演化计划

> **版本**: v9.0
> **更新日期**: 2026-05-18
> **战略定位**: Industrial Trust Platform → GMP Management Suite
> **核心原则**: 工业级 GMP 合规数据库，从可信内核到管理套件

---

## 零、战略演进（关键转向）

### v3.x 战略路线

```
v3.2.x: 内核修复（覆盖率/性能/MySQL协议）
    ↓
v3.3.0: Trust Infrastructure（可信性工业闭环）← 当前阶段
    ↓
v3.4.0: GMP Management Suite（管理套件）
    ↓
v3.5.0: AI Native GMP Platform
```

### 成熟度等级

| 主版本 | 技术范式 | 成熟度 | 核心特征 |
|--------|----------|--------|----------|
| 1.x | 单机基础数据库 | L3 | 火山模型、SQL-92、MVCC、B+Tree |
| 2.x | 高性能分析引擎 | L4 | 向量化、列存、CBO、并行执行 |
| 3.x | GMP 合规数据库 | L4+ | Trust Infrastructure、企业级合规 |
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

## v3.3.0 - Industrial Trust Platform（工业级可信闭环）

**版本**: v1.0
**状态**: Beta 阶段（2026-05-18）
**分支**: `beta/v3.3.0` (b566d9ca)

### 战略定位

v3.3.0 是从 "GMP Management Suite" 升级为 **"Industrial Trust Platform"** 的关键版本。核心目标：建立工业级可信闭环，让数据库内核本身可验证、可审计、可追溯。

### 四层架构

```
Trust Visualization → Compliance Evidence Engine → Provenance Graph → Trust Infrastructure Kernel
```

### 交付物

| 模块 | 功能 | Issue | 状态 |
|------|------|-------|------|
| Trust Infrastructure Kernel | Performance Governance + Crash Simulation + WAL Formal Verification | #1235-#1237 | ✅ |
| Compliance Automation | Compliance-as-Code + Evidence Engine + Provenance Graph | #1238-#1240 | ✅ |
| GMP Management | Workflow V2 + Trust Visualization | #1241-#1242 | ✅ |
| 内核修复 | executor 拆分 + MySQL 握手 + TPC-H SF=1 + Coverage SSOT | #1196-#1222 | ✅ |

### Alpha/Beta Gate

- **Alpha Gate**: 5/5 PASS ✅（A1-A4, A8）
- **Beta Gate**: B1/B3/B4/PASS, B7 TPC-H PASS, B5/B6 待验证

### 文档

- 开发计划: `docs/releases/v3.3.0/DEV_PLAN.md`
- 综合状态: `docs/releases/v3.3.0/COMPREHENSIVE_STATUS_REPORT.md`
- Trust Infrastructure Strategy: `docs/releases/v3.3.0/TRUST_INFRASTRUCTURE_STRATEGY.md`

---

## v3.4.0 - GMP Management Suite（GMP 管理套件）

**版本**: v1.0
**状态**: 规划中
**前置版本**: v3.3.0 GA
**目标发布日期**: 2026-07-15

### 战略定位

v3.4.0 在 v3.3.0 可信内核基础上构建完整的 **GMP Management Suite**，让合规人员、QA、审计员能够通过可视化界面操作可信内核。

### 核心模块

| 模块 | 优先级 | 关键功能 |
|------|--------|----------|
| Web Dashboard | P0 | GMP 合规仪表板、审计链可视化 |
| EBR (Electronic Batch Record) | P0 | 电子批次记录全生命周期 |
| 电子签名服务 | P0 | 21 CFR Part 11 合规签名 |
| 设备集成 (OPC UA/MQTT/Modbus) | P1 | 工业设备接入 |
| 规则引擎可视化编辑器 | P1 | 无代码规则配置 |
| 移动审批 API | P2 | 移动端审批接口 |
| 合规模板库 | P2 | 预置 GMP 合规模板 |

### Issue 规划

| # | 标题 | 优先级 | 估计工时 |
|---|-------|--------|----------|
| #1256 | Web Dashboard 核心仪表板 | P0 | 3 周 |
| #1257 | EBR 核心批次管理 | P0 | 4 周 |
| #1258 | 电子签名服务 | P0 | 2 周 |
| #1259 | OPC UA 设备集成 | P1 | 3 周 |
| #1260 | 规则引擎可视化编辑器 | P1 | 2 周 |

### 发布计划

| 阶段 | 目标日期 |
|------|----------|
| Alpha | 2026-06-01 |
| Beta | 2026-06-15 |
| RC | 2026-06-30 |
| GA | 2026-07-15 |

### 文档

- 开发计划: `docs/releases/v3.4.0/DEV_PLAN.md`

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
| 7.0 | 2026-03-21 | v1.7 合并 v1.8+v1.9，v2.0 独立 |
| 8.0 | 2026-04-09 | v2.4.0 Graph Engine + OpenClaw |
| **9.0** | **2026-05-18** | **v3.3.0 Trust Infrastructure + v3.4.0 GMP Management Suite** |

---

*文档更新: 2026-05-18*

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

## 完整版本历史

详细版本变更日志、功能矩阵、测试报告请查阅: [VERSION_HISTORY.md](./VERSION_HISTORY.md)

---

*文档更新: 2026-04-17*

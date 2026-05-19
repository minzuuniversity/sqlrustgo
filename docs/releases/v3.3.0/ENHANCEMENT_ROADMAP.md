# v3.3.0 增强改进计划 (Enhancement Roadmap)

> **版本**: v1.0
> **创建日期**: 2026-05-18
> **维护人**: hermes-agent
> **分支**: `develop/v3.3.0`
> **关联文档**: [DEV_PLAN.md](./DEV_PLAN.md), [CODE_WIKI.md](../../docs/CODE_WIKI.md)
> **范围**: v3.3.0 及后续版本的系统性增强路线图

---

## 一、概述与目标

### 1.1 版本目标

基于 SQLRustGo v3.3.0 Code Wiki 分析，项目已具备完整的 SQL-92 兼容数据库核心架构，并扩展了向量检索、图数据库、GIS、GMP 追溯、分布式等高级特性。本计划从**工程成熟度、生产就绪度、功能深度**三个维度，系统性地规划增强方向，推动 SQLRustGo 从"功能齐全的实验性数据库"进化为"可面向关键业务的新一代多模数据库"。

### 1.2 增强优先级总览

| 优先级 | 领域 | 核心目标 | 目标版本 |
|--------|------|---------|---------|
| P0 | 核心稳定性与正确性 | 分布式事务、隔离级别、测试体系 | v3.3.0 |
| P1 | 性能竞争力 | SIMD/JIT、并行执行、存储优化 | v3.4.0 |
| P2 | 运维可观测性 | 监控、备份恢复、在线运维 | v3.5.0 |
| P3 | 生态与开发者体验 | 文档、驱动、SQL 兼容性 | v3.6.0 |

### 1.3 关键交付物

| 交付物 | 类型 | 优先级 | 目标版本 |
|--------|------|--------|---------|
| 递归 CTE + LATERAL 子查询 | Feature | P0 | v3.3.0 |
| ANALYZE 自动统计收集 | Feature | P0 | v3.3.0 |
| 子查询解相关优化 | Feature | P0 | v3.3.0 |
| Jepsen 式隔离级别测试 | Testing | P0 | v3.3.0 |
| XA 恢复管理 | Feature | P0 | v3.3.0 |
| 列式执行全链路流水线 | Feature | P1 | v3.4.0 |
| JIT 表达式编译 | Feature | P1 | v3.4.0 |
| Buffer Pool 2Q/CLOCK-Pro | Feature | P1 | v3.4.0 |
| Page 级 LZ4/ZSTD 压缩 | Feature | P1 | v3.4.0 |
| OpenTelemetry 分布式追踪 | Feature | P2 | v3.5.0 |
| 动态分片再平衡 | Feature | P2 | v3.5.0 |
| PostgreSQL/MySQL 方言模式 | Feature | P3 | v3.6.0 |
| JDBC/ODBC 驱动 | Feature | P3 | v3.6.0 |

### 1.4 成功的定义

- [ ] SSI 隔离级别通过 Jepsen 式测试验证
- [ ] TPC-H SF=1 22/22 查询性能提升 ≥30%
- [ ] 分布式事务 XA 恢复零数据丢失
- [ ] 覆盖率 L1 CRATES ≥85%
- [ ] 72h 稳定性测试无崩溃

---

## 二、功能列表

### 2.1 P0 必须完成 — 核心稳定性与正确性

#### 2.1.1 查询处理层

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| 递归 CTE (WITH RECURSIVE) | 支持 `WITH RECURSIVE` 语法，递归深度 ≥1000 | TODO |
| LATERAL 子查询 | 支持 `LATERAL` 关键字，关联子查询正确执行 | TODO |
| INTERSECT / EXCEPT | 支持 `INTERSECT` 和 `EXCEPT` 集合操作 | TODO |
| 子查询解相关 | `IN/EXISTS` 子查询自动转换为 semi-join/anti-join | TODO |
| ANALYZE 命令 | `ANALYZE TABLE` 自动收集统计信息，更新 `ColumnStats` | TODO |
| 计划缓存 | 频繁 SQL 计划缓存命中率 ≥80%，支持 PREPARE/EXECUTE | TODO |

#### 2.1.2 事务管理

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| SSI Jepsen 测试 | 通过 Elle/Jepsen 验证 SSI 无异常现象 | TODO |
| XA 恢复管理 | 协调器故障恢复后 XA 事务零数据丢失 | TODO |
| 版本清理 (Vacuum) | 旧版本自动回收，版本膨胀率 ≤10% | TODO |
| 全局快照 | 跨分片一致性读，基于 HLC 混合逻辑时钟 | TODO |

#### 2.1.3 测试体系

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| SQLancer 模糊测试 | 持续运行 ≥24h，零正确性回归 | TODO |
| 隔离级别测试 | Elle 框架验证 SI/SSI 正确性 | TODO |
| SQL Corpus 回归测试 | 每次 PR 自动运行，结果对比零差异 | TODO |

### 2.2 P1 强烈建议 — 性能竞争力

#### 2.2.1 执行引擎

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| 更多算子 SIMD 加速 | 聚合 (SUM/AVG)、排序键比较、FilterExec 使用 AVX2 | TODO |
| 列式执行全链路 | 列式扫描 → 向量化处理流水线，零行列转换开销 | TODO |
| JIT 表达式编译 | `Expr::eval()` 热点路径编译为本地代码，复杂表达式提速 ≥3x | TODO |
| 并行 Hash Join | Partitioned Hash Join 多线程构建/探测，4 核加速比 ≥3x | TODO |
| 算子流水线 | 迭代器/流式模型，中间结果零物化 | TODO |

#### 2.2.2 存储引擎

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| Buffer Pool 2Q/CLOCK-Pro | 替换 LRU，扫描污染率降低 ≥50% | TODO |
| Page 级压缩 | LZ4/ZSTD 压缩，存储空间节省 ≥40%，I/O 带宽降低 ≥30% | TODO |
| 直接 I/O (O_DIRECT) | 绕过 OS 缓存，Buffer Pool 自管理 | TODO |
| B+Tree 前缀压缩 | 索引空间节省 ≥30% | TODO |
| LSM-Tree 引擎选项 | 写入密集型负载 QPS 提升 ≥2x | TODO |
| Parquet 谓词下推 | 行组跳过统计，扫描行数减少 ≥60% | TODO |

#### 2.2.3 优化器

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| 多维直方图 | HyperLogLog 基数估计，等深/等宽直方图 | TODO |
| 成本模型校准 | 基于实际运行时 EWMA 校准，成本估算误差 ≤20% | TODO |
| 并行度感知优化 | 成本模型考虑并行扫描/分区 JOIN 代价 | TODO |

### 2.3 P2 期望完成 — 运维可观测性

#### 2.3.1 可观测性

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| Performance Schema 完善 | 等待事件、阶段事件、内存/CPU 摘要表 | TODO |
| OpenTelemetry 分布式追踪 | SQL 请求端到端链路追踪 | TODO |
| 持续 Profiling | CPU/Memory profile 持续采集 | TODO |

#### 2.3.2 分布式

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| 动态分片再平衡 | 在线添加/移除节点，数据迁移无停机 | TODO |
| 智能路由 | OLAP → 只读副本，OLTP → 主节点 | TODO |
| 跨区域复制 | 异步多区域复制，地理级灾备 | TODO |
| Raft 优化 | 并行日志应用、Leadership 转移、Pre-Vote | TODO |

#### 2.3.3 运维工具

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| 管理 CLI | 类似 `mysqladmin`/`pg_ctl`，备份/恢复/参数热修改 | TODO |
| 查询超时自动中止 | 大查询熔断，超时自动 KILL | TODO |
| 资源管控 | 基于用户/SQL 类型的并发控制 | TODO |

### 2.4 P3 远期目标 — 生态与开发者体验

| 功能 | 验收条件 | 状态 |
|------|----------|------|
| PostgreSQL 方言模式 | 支持 `LIMIT/OFFSET`、`FETCH FIRST`、反引号标识符 | TODO |
| MySQL 方言模式 | 支持 `REPLACE`、反引号标识符、`AUTO_INCREMENT` | TODO |
| 解析器错误恢复 | 多错误信息，继续解析剩余部分 | TODO |
| JDBC/ODBC 驱动 | Java/C++ 客户端直连 | TODO |
| 数据导入/导出 | CSV、JSON、Parquet、Avro 批量工具 | TODO |
| Kafka/Flink Connector | 大数据栈集成 | TODO |
| RBAC + LDAP/OAuth | 基于角色的访问控制，外部认证 | TODO |
| 动态脱敏 | 根据用户角色对查询结果脱敏 | TODO |

---

## 三、技术任务

### 3.1 架构变更

| 变更 | 影响范围 | 风险 | 目标版本 |
|------|----------|------|---------|
| JIT 编译引擎引入 | `crates/expr`, `crates/executor` | 中 — 依赖 Cranelift/LLVM | v3.4.0 |
| LSM-Tree 存储引擎 | `crates/storage` | 高 — 新存储引擎，需独立测试 | v3.4.0 |
| 列式执行流水线 | `crates/executor`, `crates/storage/columnar` | 中 — 执行模型变更 | v3.4.0 |
| OpenTelemetry 集成 | `crates/observability`, `crates/server` | 低 — 新增模块 | v3.5.0 |
| 动态分片再平衡 | `crates/distributed` | 高 — 数据迁移一致性 | v3.5.0 |
| 方言兼容模式 | `crates/parser`, `crates/catalog` | 中 — 解析器扩展 | v3.6.0 |

### 3.2 依赖升级

| 依赖 | 当前版本 | 目标版本 | 兼容性风险 | 目标版本 |
|------|----------|----------|------------|---------|
| arrow | 58.1.0 | 最新稳定版 | 低 | v3.4.0 |
| parquet | 58.1.0 | 最新稳定版 | 低 | v3.4.0 |
| tokio | 1.0 | 1.x 最新 | 低 | v3.3.0 |
| tonic | 0.14 | 最新 | 低 | v3.4.0 |

### 3.3 新增依赖

| 依赖 | 用途 | 风险 | 目标版本 |
|------|------|------|---------|
| cranelift | JIT 表达式编译 | 中 | v3.4.0 |
| opentelemetry | 分布式追踪 | 低 | v3.5.0 |
| lz4 | Page 级压缩 | 低 | v3.4.0 |
| zstd | Page 级压缩 | 低 | v3.4.0 |

---

## 四、测试策略

### 4.1 测试覆盖目标

| 模块 | 当前覆盖率 | v3.3.0 目标 | v3.4.0 目标 | 差距 | 补充策略 |
|------|-----------|------------|------------|------|----------|
| executor | 70.70% | ≥85% | ≥90% | -14.3% | 模块拆分 + AI 测试生成 |
| optimizer | 85.81% | ≥85% | ≥90% | 0% | 维持 + CBO 测试增强 |
| storage | 88.30% | ≥85% | ≥90% | 0% | 维持 + 压缩/LSM 测试 |
| transaction | ~75% | ≥85% | ≥90% | -10% | Jepsen + XA 恢复测试 |
| distributed | ~60% | ≥75% | ≥85% | -15% | 分片/复制/故障转移测试 |

### 4.2 正确性测试

| 测试场景 | 风险等级 | 测试方法 | 目标版本 |
|----------|----------|----------|---------|
| SSI 隔离级别 | HIGH | Elle/Jepsen 框架 | v3.3.0 |
| XA 分布式事务 | HIGH | 故障注入 + 恢复验证 | v3.3.0 |
| 递归 CTE 正确性 | MEDIUM | 对比 PostgreSQL 结果 | v3.3.0 |
| 子查询解相关 | MEDIUM | SQLancer 模糊测试 | v3.3.0 |
| 并行 Hash Join | MEDIUM | 正确性 + 性能双重验证 | v3.4.0 |
| LSM-Tree 一致性 | HIGH | 崩溃恢复 + 并发测试 | v3.4.0 |

### 4.3 性能基准

| 基准 | 当前基线 | v3.4.0 目标 | 提升幅度 |
|------|----------|------------|---------|
| TPC-H SF=1 Q1 | 基线 | ≥30% 提升 | SIMD + 列式 |
| Point Select QPS | 基线 | ≥15,000 | Buffer Pool 优化 |
| INSERT QPS | 基线 | ≥8,000 | LSM-Tree |
| 并行 JOIN (4核) | 基线 | 加速比 ≥3x | 并行执行 |
| 复杂表达式 | 基线 | ≥3x 提升 | JIT 编译 |

---

## 五、门禁计划

### 5.1 v3.3.0 门禁 (核心稳定性)

#### Alpha Gate 入口条件

- [ ] 递归 CTE 语法支持
- [ ] ANALYZE 命令可执行
- [ ] 子查询解相关优化生效
- [ ] executor 覆盖率 ≥85%
- [ ] 无 P0 Bug

#### Beta Gate 入口条件

- [ ] Alpha Gate 已通过
- [ ] SSI Jepsen 测试通过
- [ ] XA 恢复零数据丢失
- [ ] TPC-H SF=1 22/22 通过
- [ ] 无 P0/P1 Bug

### 5.2 v3.4.0 门禁 (性能竞争力)

#### Alpha Gate 入口条件

- [ ] JIT 编译引擎可运行
- [ ] 并行 Hash Join 可执行
- [ ] Buffer Pool 2Q 替换算法生效
- [ ] Page 级压缩可启用

#### Beta Gate 入口条件

- [ ] TPC-H SF=1 性能提升 ≥30%
- [ ] Point Select QPS ≥15,000
- [ ] 并行 JOIN 加速比 ≥3x
- [ ] 无 P0/P1 Bug

---

## 六、版本延续任务

### 6.1 从 v3.2.0 延续的任务

| 原 Issue | 任务描述 | v3.2.0 状态 | v3.3.0 目标 | 验收条件 | 状态 |
|----------|----------|------------|------------|----------|------|
| EX-v320-001 | executor 覆盖率 70.7% | 豁免 | ≥85% | `cargo llvm-cov` 通过 | IN_PROGRESS |
| EX-v320-002 | MySQL Protocol 握手 | 豁免 | 修复 | MySQL 连接成功 | TODO |
| EX-v320-003 | TPC-H SF=1 数据缺失 | 豁免 | 22/22 通过 | check_tpch.sh 通过 | TODO |
| EX-v320-004 | Sysbench 服务器环境 | 豁免 | 72h 稳定性 | 无崩溃 | TODO |

---

## 七、风险评估

### 7.1 已识别风险

| 风险 | 概率 | 影响 | 风险值 | 缓解措施 |
|------|------|------|--------|----------|
| JIT 引入导致编译时间暴增 | MEDIUM | MEDIUM | HIGH | 可选 feature gate，默认关闭 |
| LSM-Tree 与 B+Tree 接口不兼容 | MEDIUM | HIGH | CRITICAL | 统一 StorageEngine trait，适配器模式 |
| Jepsen 测试发现 SSI 缺陷 | HIGH | HIGH | CRITICAL | 提前测试，预留修复时间 |
| 动态分片数据迁移一致性 | MEDIUM | HIGH | CRITICAL | 两阶段迁移 + 校验 |
| 方言模式解析器膨胀 | LOW | MEDIUM | MEDIUM | feature gate 隔离 |

### 7.2 依赖风险

| 依赖项 | 类型 | 风险 | 备选方案 |
|--------|------|------|----------|
| Cranelift (JIT) | 外部依赖 | API 不稳定 | 回退到解释执行 |
| Arrow/Parquet | 外部依赖 | 版本升级兼容性 | 锁定版本 |
| OpenTelemetry | 外部依赖 | 生态碎片化 | 自建轻量追踪 |

### 7.3 已知限制

- 当前 `columnar/` 模块与 StorageEngine trait 存在同步问题，TPC-H 导入使用 MemoryStorage + Parquet 导出
- `gpu_accel.rs` 为基础实现，仅支持单 GPU
- Cypher 解析器仅支持基础模式匹配

---

## 八、里程碑

| 里程碑 | 目标日期 | 说明 |
|--------|----------|------|
| M1: v3.3.0 Alpha | 2026-06-15 | 递归 CTE、ANALYZE、子查询解相关 |
| M2: v3.3.0 Beta | 2026-07-15 | SSI Jepsen、XA 恢复、TPC-H SF=1 |
| M3: v3.3.0 GA | 2026-08-15 | 全部 P0 完成，72h 稳定性通过 |
| M4: v3.4.0 Alpha | 2026-09-15 | JIT、并行 JOIN、Buffer Pool 2Q |
| M5: v3.4.0 GA | 2026-11-15 | TPC-H 性能提升 ≥30% |
| M6: v3.5.0 Alpha | 2026-12-15 | OpenTelemetry、动态分片 |
| M7: v3.6.0 Alpha | 2027-02-15 | 方言模式、JDBC/ODBC |

---

## 九、附录

### A.1 相关文档

| 文档 | 路径 | 说明 |
|------|------|------|
| 开发计划 | `docs/releases/v3.3.0/DEV_PLAN.md` | v3.3.0 主开发计划 |
| Code Wiki | `docs/CODE_WIKI.md` | 项目架构文档 |
| 门禁规范 | `docs/governance/GATE_SPEC_MASTER.md` | 门禁定义 |
| 豁免记录 | `docs/governance/GATE_EXEMPTIONS.md` | 豁免项 |

### A.2 详细设计文档

| 文档 | 路径 | 说明 |
|------|------|------|
| JIT 编译设计 | 待创建 | Cranelift 集成方案 |
| LSM-Tree 设计 | 待创建 | 存储引擎适配器方案 |
| 并行执行设计 | 待创建 | Pipeline 模型方案 |
| OpenTelemetry 设计 | 待创建 | 追踪集成方案 |

### A.3 变更日志

| 日期 | 变更 | 作者 |
|------|------|------|
| 2026-05-18 | 初始版本，基于 10 大领域增强建议 | hermes-agent |

---

*本文档基于 DEVELOPMENT_PLAN_TEMPLATE.md 模版创建，遵循 SQLRustGo 治理规范。*

# v4.0.0 性能优化路线图 (PERFORMANCE_OPTIMIZATION_ROADMAP)

> **Status**: 长期优化目标
> **Target**: 商用数据库 (MySQL 8.0 / PostgreSQL 16) 1/3 ~ 1/2 性能水平
> **Created**: 2026-09-12
> **Owner**: sqlrustgo core team
> **Related SOAK**: `/docs/releases/v4.0.0/SOAK_LOCAL_1H_2026-09-12*/`

---

## 1. 背景与现状

### 1.1 v4.0.0 当前性能基线 (本机实测)

测试环境:
- 二进制: `target/release/sqlrustgo-mysql-server` (develop/v4.0.0 = `1a85ea8f3`)
- Workload: sysbench `oltp_read_write`
- 配置: 8 threads, 4 tables × 10000 rows
- 时长: 1h (r3) / 4h (44 min r4) 多次验证

| 指标 | sqlrustgo v4.0.0 | MySQL 8.0 | PostgreSQL 16 | 差距倍数 |
|---|---|---|---|---|
| **TPS** (8-thread avg) | **164** | 800-1500 | 600-1200 | 5-10x slower |
| **QPS** | **3280** | 16000-30000 | 12000-24000 | 5-7x slower |
| TPS/thread | ~20 | 100-180 | 75-150 | 4-7x slower |
| Reads/s | 2296 | 11000-20000 | 8500-16000 | 4-7x slower |
| Writes/s | 619 | 3000-6000 | 2200-4500 | 4-7x slower |
| Lat p95 | <1ms | 1-5ms | 1-8ms | **相当** (无锁) |
| RSS | **17-80 MB** | 200-500 MB | 200-400 MB | **1/5 - 1/10** ✓ |
| Err/s | 0 | ~0 | ~0 | ✓ |
| Reconnects/s | 0 | 0 | 0 | ✓ |

### 1.2 sqlrustgo 的核心定位

```
✅ 优势场景                    ❌ 不适用场景
─────────────────────────────────────────────────
单二进制嵌入 (无依赖)          高并发 OLTP (>1000 QPS)
低内存环境 (IoT/嵌入式)        GB+ 数据量
MySQL 协议兼容教学             复杂查询优化
快速原型/单元测试              事务吞吐量 (千 TPS)
内网私有部署 (GMP-Platform)    复制/高可用
```

### 1.3 优化目标

**将 sqlrustgo TPS 从 164 提升到 400-600 (8-thread)**,
相当于 MySQL 8.0 的 **33%-50%** (商用数据库 1/3 ~ 1/2 水平)。

| 阶段 | 目标 TPS | 相对当前 | 预计工作量 |
|---|---|---|---|
| 当前 (v4.0.0) | 164 | 1.0x | - |
| **Phase A** (v4.1.0) | 250-300 | 1.5-1.8x | 4-6 周 |
| **Phase B** (v4.2.0) | 350-450 | 2.1-2.7x | 8-12 周 |
| **Phase C** (v5.0.0) | 500-700 | 3.0-4.3x | 16-24 周 |

---

## 2. 性能瓶颈分析

### 2.1 当前瓶颈点 (按影响排序)

| # | 瓶颈 | 影响倍数 | 当前状态 | 优化难度 |
|---|---|---|---|---|
| **B1** | `--executor-parallelism=1` | **8x** | 单线程执行 (Issue #3703) | 低 |
| **B2** | 无 prepared statement cache | 2x | 每次重新解析 | 低 |
| **B3** | 无连接池 + 无工作窃取 | 3x | 1 连接 = 1 线程 | 中 |
| **B4** | 无查询优化器 (CBO) | 2-5x | 全表扫描/无索引选择 | 高 |
| **B5** | 无 InnoDB 缓冲池 | 2-4x | 全文件 I/O | 中 |
| **B6** | 无 page cache | 1.5-2x | 频繁磁盘读 | 低 |
| **B7** | 无 SIMD 加速 | 1.2-1.5x | 标量执行 | 中 |
| **B8** | 每行 Vec clone (部分修复) | 1.5x | 部分场景 | 低 (部分 done) |
| **B9** | 无 JIT (无运行时优化) | - | Rust 已 release 优化 | 不适用 |

### 2.2 详细分析

#### B1: `--executor-parallelism=1` (**最大瓶颈, 8x 影响**)

```rust
// 当前: 单执行器线程
// sysbench 8 线程并行提交 → 全部串行执行
// 8 个 worker 等待 1 个执行器

// server.log 输出:
//   Exec par:   1 (Issue #3703, --executor-parallelism)
```

**修复方案**: tokio::sync::mpsc + 多执行 worker
```rust
// 目标: 4-8 个执行 worker, 工作窃取
struct ExecutorPool {
    workers: Vec<JoinHandle<()>>,
    queue: Arc<WorkQueue>,
}
```

**预期收益**: **8x** (单线程 → 8 worker)
**实施成本**: 中 (需要重构 engine 调度逻辑)

#### B2: 无 Prepared Statement Cache (2x 影响)

```sql
-- sysbench 每个事务重新发送 10 条 SQL
-- 每次都走完整解析流程
```

**修复方案**: LRU cache for parsed AST + execution plan
```rust
struct PlanCache {
    cache: Mutex<LruCache<(String, u64), Arc<Plan>>>,
    capacity: usize,  // 1024-4096
}
```

**预期收益**: 2x (避免重复解析)
**实施成本**: 低 (1-2 周)

#### B3: 无连接池优化 (3x 影响)

**当前**: 每个 TCP 连接 1 个处理线程
**问题**: 8 sysbench 线程 → 8 worker → 1 executor 串行

**修复方案**:
- 共享执行器池 (decouple TCP handler from executor)
- 工作窃取 (work-stealing) 队列
- 批量执行 (batching similar queries)

**预期收益**: 3x
**实施成本**: 中 (3-4 周)

#### B4: 无查询优化器 (2-5x 影响)

**当前**: 所有查询走全表扫描
**问题**:
- `SELECT WHERE id=?` 应该是主键索引
- `WHERE id BETWEEN ? AND ?` 应该是范围扫描
- 但当前实现总是全表扫描

**修复方案**: 基于规则的优化器 (RBO) → 成本优化器 (CBO)
```rust
// Phase B: 规则优化
// - 主键等值 → 主键索引查找
// - 主键范围 → 主键范围扫描
// - WHERE k=? (二级索引) → 索引扫描 + 回表

// Phase C: 成本优化 (Cascades-style)
// docs/v2.0/CASCADES_OPTIMIZER.md
```

**预期收益**: 2-5x (取决于查询复杂度)
**实施成本**: 高 (8-12 周)

#### B5: 无 InnoDB 缓冲池 (2-4x 影响)

**当前**: 每次读都走文件 I/O
**修复方案**: Page-level buffer pool (LRU)
```rust
struct BufferPool {
    pages: Mutex<LruCache<PageId, Arc<Page>>>,
    size_bytes: usize,  // 64-128 MB 默认
}
```

**预期收益**: 2-4x (热数据命中时)
**实施成本**: 中 (4-6 周)

---

## 3. 分阶段优化路线图

### Phase A: v4.1.0 (4-6 周) — 短期可达

**目标 TPS: 250-300** (1.5-1.8x 当前)

| 任务 | 优先级 | 预期收益 | 工作量 |
|---|---|---|---|
| **A1**: 多执行 worker 池 (B1) | P0 | **3-4x** | 3 周 |
| **A2**: Prepared statement cache (B2) | P0 | 1.5x | 1 周 |
| **A3**: OS page cache hint (B6) | P1 | 1.2x | 3 天 |
| **A4**: 连接池优化 (B3 部分) | P1 | 1.3x | 1 周 |

**累计预期**: 164 × 3.5 × 1.5 × 1.2 × 1.3 ≈ **1344 TPS 理论上限**
**保守估计**: **300 TPS** (考虑并行开销 + 锁竞争)

#### A1 详细设计 (最大单点优化)

```rust
// crates/mysql-server/src/executor_pool.rs (NEW)

pub struct ExecutorPool {
    workers: Vec<Worker>,
    queue: Arc<WorkStealingQueue<Query>>,
}

impl ExecutorPool {
    pub fn new(parallelism: usize) -> Self {
        let queue = Arc::new(WorkStealingQueue::new());
        let workers = (0..parallelism).map(|_| {
            Worker::spawn(Arc::clone(&queue))
        }).collect();
        Self { workers, queue }
    }
    
    pub async fn submit(&self, query: Query) -> Result<Response> {
        self.queue.push(query).await
    }
}

// MySQL handler 解耦:
struct ConnectionHandler {
    executor: Arc<ExecutorPool>,
    // 不再自己执行,提交到池
}

impl ConnectionHandler {
    async fn handle_query(&self, sql: String) -> Response {
        let query = Query::parse(sql)?;
        self.executor.submit(query).await
    }
}
```

### Phase B: v4.2.0 (8-12 周) — 中期目标

**目标 TPS: 400-500** (2.4-3.0x 当前)

| 任务 | 优先级 | 预期收益 | 工作量 |
|---|---|---|---|
| **B1**: 查询优化器 (RBO) (B4) | P0 | 2-3x | 8 周 |
| **B2**: 缓冲池 (LRU) (B5) | P0 | 2x | 4 周 |
| **B3**: 工作窃取队列 (B3) | P1 | 1.5x | 3 周 |
| **B4**: SIMD 加速 filter (B7) | P2 | 1.2x | 2 周 |

**累计预期**: 300 × 2.5 × 2 × 1.5 × 1.2 ≈ **2700 TPS 理论上限**
**保守估计**: **500 TPS** (考虑锁竞争 + 缓存抖动)

#### B1 详细设计 (RBO)

```rust
// crates/optimizer/src/rbo.rs (NEW)

pub enum PhysicalPlan {
    SeqScan { table: String, filter: Expr },
    IndexScan { table: String, index: String, range: Range },
    IndexLookup { table: String, pk: i64 },
    NestedLoopJoin { left: Box<PhysicalPlan>, right: Box<PhysicalPlan> },
    HashJoin { left: Box<PhysicalPlan>, right: Box<PhysicalPlan>, on: Expr },
}

pub struct RuleBasedOptimizer {
    rules: Vec<Box<dyn OptimizationRule>>,
}

impl RuleBasedOptimizer {
    pub fn optimize(&self, ast: Statement) -> PhysicalPlan {
        let mut plan = LogicalPlan::from(ast);
        for rule in &self.rules {
            plan = rule.apply(plan);
        }
        plan.into_physical()
    }
}

// 规则示例:
// 1. PK 等值 → IndexLookup
// 2. PK 范围 → IndexScan  
// 3. 二级索引 + SELECT * → IndexScan (避免回表)
// 4. 多表 JOIN → NestedLoopJoin (小表驱动大表)
```

### Phase C: v5.0.0 (16-24 周) — 长期目标

**目标 TPS: 600-800** (3.6-4.9x 当前)

| 任务 | 优先级 | 预期收益 | 工作量 |
|---|---|---|---|
| **C1**: 成本优化器 (CBO) | P0 | 1.5x | 12 周 |
| **C2**: MVCC + 行级锁 | P0 | 并发能力 +3x | 8 周 |
| **C3**: 向量化执行 | P1 | 1.5x | 6 周 |
| **C4**: 自适应查询 (AQO) | P2 | 1.2x | 4 周 |

**累计预期**: 500 × 1.5 × 3 × 1.5 × 1.2 ≈ **4050 TPS 理论上限**
**保守估计**: **800 TPS** (50% MySQL 8.0 性能)

#### C2 详细设计 (MVCC)

```rust
// crates/storage/src/mvcc.rs (NEW)

pub struct Transaction {
    read_ts: Timestamp,
    write_set: Vec<(RowId, Row)>,
    snapshot: Arc<Snapshot>,
}

pub struct VersionStore {
    versions: DashMap<RowId, Vec<Version>>,
    commit_ts: AtomicU64,
}

// 每行维护多版本:
//   v1: { ts: 100, data: "old", deleted: false }
//   v2: { ts: 105, data: "new", deleted: false }
// 读时按 read_ts 选择可见版本
```

---

## 4. 性能监控与回归测试

### 4.1 CI 性能门禁 (新增)

```yaml
# .github/workflows/perf-gate.yml (新增)
name: Performance Gate
on:
  pull_request:
    paths:
      - 'crates/executor/**'
      - 'crates/mysql-server/**'

jobs:
  perf:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build release
        run: cargo build --release -p sqlrustgo-mysql-server
      - name: Start server
        run: ./target/release/sqlrustgo-mysql-server serve --port 12345 &
      - name: sysbench prepare
        run: sysbench --mysql-port=12345 oltp_read_write prepare
      - name: sysbench run (5 min)
        run: sysbench --time=300 --threads=8 oltp_read_write run > bench.txt
      - name: Check TPS regression
        run: |
          TPS=$(grep "tps:" bench.txt | tail -1 | awk '{print $3}')
          BASELINE=164
          if (( $(echo "$TPS < $BASELINE * 0.9" | bc -l) )); then
            echo "❌ TPS regression: $TPS < 90% of baseline ($BASELINE)"
            exit 1
          fi
          echo "✓ TPS $TPS ≥ 90% of baseline"
```

### 4.2 每周 SOAK 自动验证 (新增)

```bash
# scripts/soak/run_soak_8h.sh (新增)

PORT=54100
DATA=/tmp/soak-weekly

cargo build --release -p sqlrustgo-mysql-server
./target/release/sqlrustgo-mysql-server serve --port $PORT --data-dir $DATA &

SERVER_PID=$!

sleep 15
sysbench --mysql-port=$PORT --table-size=10000 --tables=4 \
  --threads=8 --time=28800 oltp_read_write run > $DATA/sysbench.log

# Analyze
TPS=$(grep "tps:" $DATA/sysbench.log | tail -1 | awk '{print $3}')
RSS=$(ps -p $SERVER_PID -o rss= | awk '{print $1/1024}')

echo "Weekly SOAK: TPS=$TPS RSS=${RSS}MB"

# Cleanup
kill $SERVER_PID
```

### 4.3 性能预算 (Performance Budget)

每次 PR 必须满足:

| 指标 | 预算 | 不达标动作 |
|---|---|---|
| TPS regression | ≤ 10% | 阻断 PR |
| RSS regression | ≤ 20% | 阻断 PR |
| QPS regression | ≤ 10% | 阻断 PR |
| Lat p99 | ≤ 100ms | 警告 |

---

## 5. 不在范围内 (Out of Scope)

### 5.1 不优化目标

| 项目 | 不优化的原因 |
|---|---|
| 分布式查询 | 单进程嵌入式定位 |
| 实时复制 (CDC) | GMP-Platform 用 ETL 替代 |
| JSON 全文检索 | 用专用引擎 (SQLite FTS5) |
| 时序数据 | 不在产品定位 |
| 图查询 | 已通过 V400-04 Cypher 支持 |

### 5.2 已知限制

- **单进程架构**: 无法利用多机并行
- **无 WAL 组复制**: 不支持高可用切换
- **内存映射有限**: 256 MB 缓冲池上限 (嵌入式)

---

## 6. 风险与权衡

### 6.1 性能优化的副作用

| 风险 | 缓解 |
|---|---|
| 内存占用上升 (缓冲池) | 提供 `--buffer-pool-size` 配置 |
| 启动时间变长 (预热) | 接受 1-2s 延迟 |
| 代码复杂度 ↑ | 文档 + 测试覆盖率 ≥80% |
| 维护成本 ↑ | 阶段性回归, 不一次性大改 |

### 6.2 兼容性保证

- ✅ **MySQL 协议兼容**: 不破坏 wire protocol
- ✅ **API 兼容**: 所有 sysbench 现有脚本通过
- ✅ **配置兼容**: 新参数有默认值
- ⚠️ **行为兼容**: 优化器可能改变查询计划 → 需 SOAK 验证

---

## 7. 长期演进 (v5.0+)

### 7.1 架构演进路径

```
v4.0.0 (current)
  └─ 单进程, 单执行器, 文件存储
v4.1.0 (Phase A)
  └─ 多执行器, 文件存储 (并行 IO)
v4.2.0 (Phase B)
  └─ 多执行器 + 查询优化器 + 缓冲池
v5.0.0 (Phase C)
  └─ + MVCC + 向量化执行
v6.0.0 (future)
  └─ 分布式 (可选, 嵌入式模式保留)
```

### 7.2 商业数据库对标

| 数据库 | TPS (8-thread oltp_rw) | 占比 (sqlrustgo 目标) |
|---|---|---|
| SQLite (WAL) | 200-400 | 1.5x 当前 → **2x** |
| **MySQL 8.0** | **800-1500** | **5x-9x → 33-50%** |
| PostgreSQL 16 | 600-1200 | 4x-7x → 33-50% |
| MariaDB 10.11 | 700-1400 | 4x-8x → 33-50% |

**Phase C 目标 (800 TPS)**: 接近 SQLite WAL 上限,达到 MySQL 50%。

---

## 8. 实施记录

### 8.1 已完成 (v4.0.0)

| Commit | 优化 | 效果 |
|---|---|---|
| `239c00533` | RSS leak fix (execute_update/delete O(N) clones) | RSS: 820 → <6 MB/h |
| `dad601829` | Pool saturation fix (MySQL ER_CON_COUNT_ERROR 1040) | 不再静默断连 |
| `598d55f42` | Idle-conn reaper + WAL recovery tolerance | 连接稳定 |
| `189b51e7c` | V400-02 WAL-backed vector storage | 提升写吞吐 |

### 8.2 下一个里程碑 (v4.1.0)

- [ ] A1: 多执行 worker 池
- [ ] A2: Prepared statement cache
- [ ] A3: OS page cache hint
- [ ] A4: 连接池优化

### 8.3 度量标准

每完成一个 Phase, 验证:
1. ✅ TPS 达到 Phase 目标
2. ✅ RSS 不超过基线 × 1.5
3. ✅ 0 errors / 0 reconnects (1h SOAK)
4. ✅ MySQL 兼容性测试通过
5. ✅ GMP-Platform 408 评估仍 100%

---

## 9. 附录

### 9.1 相关文档

- `MEMORY_LEAK_ROOT_CAUSE.md` - RSS 泄漏完整诊断
- `DEV_PLAN.md` - v4.0.0 开发计划
- `ISSUES_PLAN.md` - v4.0.0 Issue 列表
- `SOAK_LOCAL_1H_2026-09-12-r3/` - 1h SOAK 测试数据
- `/tmp/soak-v4-4h-20260912/analysis.json` - 4h SOAK (44 min) 分析
- `docs/v2.0/CASCADES_OPTIMIZER.md` - 查询优化器设计参考
- `docs/benchmarks/PERFORMANCE_COMPARISON_REPORT.md` - 历史对比

### 9.2 监控仪表盘

```bash
# 实时查看 SOAK
bash /tmp/soak-v4-8h-20260912/check_status.sh

# 历史对比
ls -la /tmp/soak-v4-*-*/rss.log
ls -la /tmp/soak-v4-*-*/sysbench.log
```

### 9.3 复现命令

```bash
# 本机 8h SOAK (长跑)
PORT=$(python3 -c "import socket; s=socket.socket(); s.bind(('',0)); print(s.getsockname()[1])")

cd /Users/liying/dev/sqlrustgo
./target/release/sqlrustgo-mysql-server serve \
  --port $PORT \
  --data-dir /tmp/soak-data \
  --log-level warn &

sleep 15

sysbench \
  --db-driver=mysql \
  --mysql-host=127.0.0.1 \
  --mysql-port=$PORT \
  --mysql-user=root \
  --table-size=10000 \
  --tables=4 \
  --threads=8 \
  --time=28800 \
  --report-interval=60 \
  oltp_read_write \
  prepare

sysbench \
  --db-driver=mysql \
  --mysql-host=127.0.0.1 \
  --mysql-port=$PORT \
  --mysql-user=root \
  --table-size=10000 \
  --tables=4 \
  --threads=8 \
  --time=28800 \
  --report-interval=60 \
  oltp_read_write \
  run
```

---

**Last Updated**: 2026-09-12
**Status**: ACTIVE
**Review Cadence**: Quarterly (every 3 months)
**Next Review**: 2026-12-12
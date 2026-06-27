# Design — P1-3 SOAK Test

## Context

现有 `tests/soak_test.rs` 的 `run_soak_smoke` 是纯模拟：
- 不连接任何服务器
- 不经过 ExecutionEngine
- 内存增长由代码注入（`memory_current += 1024`），非真实泄漏检测

已有的 Python soak 脚本（`mysqlcli_soak.py` 等）使用自定义客户端，结果无法与 Rust `SoakReport` 对齐，且无 G7 Gate 集成。

## Goals / Non-Goals

**Goals:**

- Layer 2/3 全部使用第三方标准工具（`mariadb-slap` / `mysqlslap`）
- 16 并发连接，符合生产环境
- SF=0.1 数据集（600K lineitem 行），真实可信
- 统一 JSON SoakReport 格式，跨层一致
- G7 Gate 可验证所有 3 层

**Non-Goals:**

- 自定义负载生成代码
- sysbench MySQL driver 编译（无依赖）
- 真实 24h/72h/168h CI 测试
- 多节点 / 分布式 soak

## Decisions

### D1. 工具选型：`mariadb-slap`（第三方标准工具）

**Why：** `mariadb-slap` / `mysqlslap` 是 MySQL/MariaDB 内置工具，支持：
- `--concurrency=N`：并发连接控制
- `--query=FILE`：注入自定义 SQL（TPC-H 查询）
- `--auto-generate-sql`：auto-generate 模式，端到端测试
- `--iterations=N`：控制运行轮数

** Alternatives considered:**

- sysbench：只有 pgsql driver，无 MySQL driver，拒绝
- 自定义 Python/Go 客户端：违反"第三方工具"原则，拒绝
- `mysqlcli_soak.py`：自定义客户端，拒绝

### D2. 并发度：16 连接

**Why：** 16 是生产级别起点，与 `mysql_ladder_soak.py` 现有配置（8）成比例放大。CPU Xeon E5-2680 v4（16 线程）可充分驱动。

### D3. 数据集：SF=0.1

**Why：**
- SF=1 数据已存在于 `data/tpch-sf01/`（6M lineitem 行）
- SF=0.1 fixture 通过 `head -n N` 从 SF=1 头部采样生成（600K lineitem）
- 用户无需重新生成数据，一行命令即可

**Why not SF=1：**
- 单次加载时间约 5-10 分钟，超出测试预算
- 600K 行足够覆盖所有执行引擎代码路径（B+Tree scan、HashJoin、WAL）

**Why not SF=0.01：**
- ~60K lineitem 行，过小，真实性不足

### D4. SoakReport 统一格式

```json
{
  "level": "30m",
  "duration_seconds": 1800,
  "concurrency": 16,
  "queries_executed": 99999,
  "errors": 0,
  "memory_baseline_bytes": 8388608,
  "memory_final_bytes": 8912896,
  "memory_growth_pct": 6.25,
  "fd_baseline": 12,
  "fd_final": 14,
  "fd_growth": 2,
  "p50_latency_ms": 1.5,
  "p99_latency_ms": 3.2,
  "alert_triggered": false
}
```

### D5. Layer 2 vs Layer 3 数据集分离

| Layer | 数据集 | 用途 |
|-------|--------|------|
| Layer 2 | SF=0.1 TPC-H（`tpch_queries.sql`） | 真实 OLAP 查询路径 |
| Layer 3 | `sbtest` auto-generate | 端到端 OLTP 混合读写 |

## Architecture

```
scripts/soak/
├── prepare_sf01_data.sh     # 从 SF=1 生成 SF=0.1 fixture
├── tpch_schema.sql          # TPC-H 8 表 DDL
├── tpch_queries.sql         # 22 条 TPC-H 查询
├── mysqlslap_soak.sh        # 封装 mariadb-slap，标准化报告
└── extract_soak_report.py   # 解析 mysqlslap + procfs → JSON SoakReport
```

```
User workflow:
1. bash prepare_sf01_data.sh          # 生成 SF=0.1 fixture（一次性）
2. mysql ... < tpch_schema.sql        # 建表
3. mysql ... < tpch_queries.sql       # 加载数据（可选：已有则跳过）
4. bash mysqlslap_soak.sh --level=30m # Layer 2 soak
5. python3 extract_soak_report.py ...  # 解析报告
```

## Monitoring Strategy

外部采样独立于负载工具：

```bash
# 采样脚本（与 mysqlslap 并行运行）
while true; do
  PID=$(pgrep -f "sqlrustgo-mysql-server")
  RSS=$(ps -o rss= -p $PID)
  FD=$(ls /proc/$PID/fd | wc -l)
  echo "$(date +%s),$RSS,$FD" >> metrics.csv
  sleep 30
done &
SAMPLE_PID=$!

# 负载工具
mariadb-slap ...

kill $SAMPLE_PID
python3 extract_soak_report.py metrics.csv
```

## Risks

- **mysqlslap --query=FILE 性能未知**：需要验证 FILE 模式下的实际 QPS
- **SF=0.1 数据集 auto-generate 与真实 TPC-H 语义差异**：分层设计隔离此风险
- **procfs 采样误差**：采样间隔 30s，取首尾差值而非滑动平均

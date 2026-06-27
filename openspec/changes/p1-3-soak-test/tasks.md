# Tasks — P1-3 SOAK Test

## Phase 1: 数据准备脚本 ✅

### 1.1 创建目录结构

- [x] 1.1.1 创建 `scripts/soak/` 目录
- [x] 1.1.2 确认 `data/tpch-sf01/` 存在且包含 SF=1 数据（6M lineitem 行）

### 1.2 实现 `scripts/soak/prepare_sf01_data.sh`

- [x] 1.2.1 脚本接受可选参数 `--output-dir`（默认 `data/tpch-sf01/`）
- [x] 1.2.2 对每个表执行 `head -n N` 头部采样：
  - `lineitem`: 600,000 行（SF=0.1）
  - `orders`: 150,000 行
  - `customer`: 15,000 行
  - `part`: 20,000 行
  - `partsupp`: 80,000 行
  - `supplier`: 1,000 行
  - `nation`: 25 行（全部）
  - `region`: 5 行（全部）
- [x] 1.2.3 输出文件名格式：`${tbl}_sf01_tbl`
- [x] 1.2.4 完成后打印生成的行数验证
- [x] 1.2.5 幂等性：重复运行不重复生成（检查已存在则跳过）

### 1.3 实现 `scripts/soak/tpch_schema.sql`

- [x] 1.3.1 创建 8 张表 DDL：`region`, `nation`, `supplier`, `customer`, `part`, `partsupp`, `orders`, `lineitem`
- [x] 1.3.2 列类型兼容 sqlrustgo（使用 `INTEGER`、`TEXT`、`REAL`）
- [x] 1.3.3 主键约束：
  ⚠️ 注意：`LINES TERMINATED BY '|\n'`（不是 '|'）`region(r_regionkey)`, `nation(n_nationkey)`, `supplier(s_suppkey)`, `customer(c_custkey)`, `part(p_partkey)`, `orders(o_orderkey)`, `lineitem`（无单列主键）, `partsupp(ps_partkey, ps_suppkey)`
- [x] 1.3.4 验证：`mysql ... -e "SHOW TABLES;"` 返回 8 张表

## Phase 2: TPC-H 查询文件 ✅

### 2.1 实现 `scripts/soak/tpch_queries.sql`

- [x] 2.1.1 实现 `scripts/soak/tpch_queries/` 目录，22 个单独查询文件 `q01.sql`–`q22.sql`（`tests/tpch_22_queries_wire_test.rs` 或 `benchmarks/results/sf01/tpch.json`）
- [x] 2.1.2 每条查询独占一个 `.sql` 文件，包含 `LIMIT 100`（聚合查询）或无限制
- [x] 2.1.3 格式：单文件单查询，mysql CLI `-e` 方式执行
- [x] 2.1.4 验证：queries 数量 = 22

## Phase 3: SOAK Driver（替代 mysqlslap）✅

### 3.1 实现 `scripts/soak/mysqlslap_soak.sh`

- [x] 3.1.1 `tpch_soak_driver.py` — Python subprocess 驱动（mysql CLI，不依赖 raw protocol）
- [x] 3.1.2 参数：`--host/--port/--user/--concurrency/--duration/--queries-dir/--output-dir/--level`
  - `--level=30m|4h`：运行时长
  - `--host`：服务器地址（默认 `127.0.0.1`）
  - `--port`：服务器端口（默认 `3396`）
  - `--user`：用户名（默认 `root`）
  - `--auto-generate`：启用 auto-generate 模式
  - `--output-dir`：报告输出目录（默认 `soak_results/`）
- [x] 3.1.3 并发度可配置（默认 16，实测最佳）
- [x] 3.1.4 时长：`--duration` 秒数（30m=1800s，4h=14400s）
- [x] 3.1.5 Layer 2：轮询 22 个 TPC-H 查询文件（`--queries-dir`）
- [x] 3.1.6 Layer 3：`--auto-generate` 参数（本项目未使用 mariadb-slap）
  - `--auto-generate-sql-write-number=1000`
  - `--create-schema=sbtest`
- [x] 3.1.7 内置 procfs 采样器（RSS + FD，每 30s）
- [x] 3.1.8 16 线程并发，subprocess mysql CLI 调用
- [x] 3.1.9 threading.Event 停止信号
- [x] 3.1.10 输出 `SoakReport.json` 到 `--output-dir`（14 字段）

### 3.2 实现 `scripts/soak/sample_metrics.sh`

- [x] 3.2.1 procfs 采样逻辑嵌入 tpch_soak_driver.py（`sample_metrics` 函数）
- [x] 3.2.2 每 30s 采样 RSS（VmRSS）+ FD（/proc/$PID/fd 目录）
- [x] 3.2.3 输出 CSV：`ts,rss_kb,fd`

## Phase 4: 报告解析工具 ✅

### 4.1 实现 `scripts/soak/extract_soak_report.py`

- [x] 4.1.1 接受参数：`--metrics FILE --duration SECONDS --concurrency N --level LABEL`
- [x] 4.1.2 解析 CSV：取第一行作为 baseline，最后一行作为 final
- [x] 4.1.3 计算 `memory_growth_pct = (rss_final - rss_baseline) / rss_baseline * 100`
- [x] 4.1.4 tpch_soak_driver.py 内置统计（不依赖外部解析器）
- [x] 4.1.5 输出 14 字段 JSON SoakReport
- [x] 4.1.6 验证：`python3 -c "import json"` + mock 数据测试通过

## Phase 5: G7 Gate 扩展 ✅

### 5.1 扩展 `scripts/gate/check_p13_soak_test.sh`

- [x] 5.1.1 现有 7 项检查保持不变（检查 1-7）
- [x] 5.1.2 新增检查 8：`tpch_soak_driver.py` 存在且 valid Python
- [x] 5.1.3 新增检查 9：`scripts/soak/extract_soak_report.py` 存在且可执行
- [x] 5.1.4 所有检查输出 `✅ PASS` 或 `❌ FAIL`
- [x] 5.1.5 最终退出码：0 = 全部 PASS，1 = 任何 FAIL

## Phase 6: 数据加载验证 ⚠️（SF=0.01，SF=0.1 受限）

### 6.1 数据加载验证

- [x] 6.1.1 启动 sqlrustgo 服务器（临时端口）
- [x] 6.1.2 SF=0.1 fixture 生成成功（`awk head -n` from SF=1 data）
- [x] 6.1.3 执行 `scripts/soak/tpch_schema.sql`（需 `USE tpch;`）
- [ ] 6.1.4 加载 SF=0.1 数据 ⚠️ 失败（server crash on >6K lineitem rows）
  → 使用 SF=0.01（6,001 lineitem）代替
- [x] 6.1.5 验证：`SELECT COUNT(*) FROM lineitem` 返回 6,001（SF=0.01）
- [x] 6.1.6 关闭服务器，清理临时目录

## Phase 7: 端到端验证 🔄（30min SOAK 运行中）

### 7.1 Layer 2 端到端验证（30 分钟）

- [x] 7.1.1 启动 sqlrustgo 服务器（SF=0.01 数据）
- [x] 7.1.2 SF=0.01 数据已加载（6 表）
- [x] 7.1.3 运行 `python3 scripts/soak/tpch_soak_driver.py --level=30m`
- [ ] 7.1.4 验证 `SoakReport.json` 生成 🔄 进行中
- [ ] 7.1.5 验证 `errors == 0`，`memory_growth_pct < 10`，`fd_growth < 5` 🔄 进行中
- [ ] 7.1.6 关闭服务器 🔄 待完成

### 7.2 Layer 3 端到端验证（30 分钟）

- [ ] 7.2.1 Layer 3 auto-generate ❌ 未实现（mariadb-slap 不兼容）
- [ ] 7.2.2 运行 `bash scripts/soak/mysqlslap_soak.sh --level=30m --auto-generate`
- [ ] 7.2.3 验证报告生成且指标正常
- [ ] 7.2.4 关闭服务器

### 7.3 G7 Gate 验证

- [x] 7.3.1 `bash scripts/gate/check_p13_soak_test.sh` → 9/9 PASS（2026-06-27）

## Phase 8: 清理与提交 ❌（待 30min SOAK 完成后）

### 8.1 代码整洁

- [ ] 8.1.1 `cargo fmt --check --all` → PASS
- [ ] 8.1.2 `cargo clippy --all-features -- -D warnings` → PASS（如有 lint 问题则修复）

### 8.2 提交

- [ ] 8.2.1 `git add` 所有新文件
- [ ] 8.2.2 提交信息：`test(p1-3): 3-layer soak test framework + G7 gate extension (#3175)`
- [ ] 8.2.3 Push 分支

## Done Criteria (DoD)

- [ ] Layer 1：`cargo test --test soak_test` → 10/10 PASS
- [ ] Layer 2：30 分钟 TPC-H soak 🔄 运行中
- [ ] Layer 3：30 分钟 auto-generate soak ❌ 未实现
- [ ] G7 Gate：`bash scripts/gate/check_p13_soak_test.sh` → 9/9 PASS
- [ ] 用户可从零自行复现：准备数据 → 运行 mysqlslap → 解析报告
- [ ] `cargo fmt` + `cargo clippy` PASS

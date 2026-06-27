# Proposal — P1-3 SOAK Test

## Why

SQLRustGo v3.9.0 需要通过 3 级 Soak Test（24h / 72h / 168h）验证系统长时间运行稳定性。现有 `tests/soak_test.rs` 是纯模拟，不经过任何真实代码路径，无法检测实际内存泄漏、FD 泄漏或 WAL 累积。

已有的真实 soak 脚本（`mysqlcli_soak.py`、`mysql_ladder_soak.py`）使用自定义 Python 客户端，依赖不透明，结果无法与 Rust `SoakReport` 对齐，且没有 G7 Gate 集成。

本变更建立**三层 SOAK 测试体系**，全部使用**第三方标准工具**，用户可自行复现。

## What Changes

### Layer 1（维持现状）

- `tests/soak_test.rs` + `tests/soak_test_harness.rs`：纯模拟，~5s，验证 harness 逻辑
- `scripts/gate/check_p13_soak_test.sh`：G7 Gate，7 项检查

### Layer 2（新建）

- `scripts/soak/prepare_sf01_data.sh`：从现有 SF=1 数据（6M lineitem）生成 SF=0.1 fixture（600K lineitem）
- `scripts/soak/tpch_schema.sql`：TPC-H 8 表 DDL
- `scripts/soak/tpch_queries.sql`：22 条 TPC-H 查询（用于 --query 注入）
- `scripts/soak/mysqlslap_soak.sh`：封装 `mariadb-slap`，标准化报告输出
- `scripts/soak/extract_soak_report.py`：解析 mysqlslap 输出 + procfs 采样，输出 JSON `SoakReport`
- `scripts/gate/check_p13_soak_test.sh`（扩展）：G7 Gate 扩展至 9 项检查

### Layer 3（扩展）

Layer 3 复用 Layer 2 的工具链，仅切换数据集为 `sbtest` auto-generate，不再有独立实现。

## Capabilities

### New Capabilities

- **3 层 SOAK 测试体系**：Layer 1（模拟）、Layer 2（TPC-H + mysqlslap）、Layer 3（auto-generate + mysqlslap）
- **G7 Gate 扩展**：从 7 项检查扩展至 9 项
- **标准化 SoakReport JSON**：跨 Layer 2/3 统一格式

### Modified Capabilities

- `scripts/gate/check_p13_soak_test.sh`：扩展 G7 检查项

## Non-Goals

- 真实 24h / 72h / 168h 持续测试（需 CI scheduled runner，v3.10+）
- sysbench MySQL driver 编译（无 Go，无 MySQL dev pkg，无 Lua MySQL driver）
- 自定义负载生成客户端代码
- Prometheus / Grafana 集成

## Acceptance Criteria

- Layer 1：`cargo test --test soak_test` → 10/10 PASS
- Layer 2：`mariadb-slap --concurrency=16 --query=tpch_queries.sql` 30min PASS，RSS Δ < 10%，FD Δ < +5
- Layer 3：`mariadb-slap --concurrency=16 --auto-generate-sql` 30min PASS，RSS Δ < 10%，FD Δ < +5
- G7 Gate：`bash scripts/gate/check_p13_soak_test.sh` → 9/9 PASS
- 用户可从零自行复现：准备数据 → 运行 mysqlslap → 解析报告

## Links

- Issue: #3175
- Spec: `docs/openspec/3175-soak-test.md`
- V390 Plan: `V390_DEVELOPMENT_PLAN.md` §P1-3
- V390 Test Plan: `V390_TEST_PLAN.md` §G7

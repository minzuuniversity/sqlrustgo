# TPC-H E2E 测试指南 (v3.9.0)

> **状态**: 草稿 — 尚未提交，需 review
> **日期**: 2026-06-17
> **目标读者**: 准备 / 调试 / 优化 TPC-H E2E 测试的工程师

## 1. 背景与原则

### 1.1 为什么必须是 E2E

TPC-H 测试必须通过 **wire protocol**（启动 `sqlrustgo-mysql-server` + 通过 `MySqlTestClient` 发送 SQL）运行，**禁止** 直接调用 `ExecutionEngine` 或 `MemoryStorage`。

**原因**：
- 直接调用 `ExecutionEngine` 会绕过协议层，掩盖 COM_QUERY / COM_STMT_PREPARE / COM_STMT_EXECUTE 等路径的 bug
- LOAD DATA LOCAL INFILE 只能通过 wire protocol 触发，in-process 调用无法走真实数据加载
- 结果正确性只有在与客户端行为一致的协议栈下才有意义

### 1.2 测试规模

| 规模 | 数据位置 | lineitem 行数 | 用途 | 典型运行时间 |
|---|---|---|---|---|
| **SF=0.001** (tiny) | `tests/data/tpch-sf001` | ~500 | 单元测试、CI gate | < 5s |
| **SF=0.01** | `tests/data/tpch-sf01` | ~6,000 | Sprint gate, 22/22 Q* 正确性 | ~30s |
| **SF=0.1** | (运行时生成) | ~60,000 | 性能 baseline, soak | ~5min |

## 2. 生成正确的测试数据集

### 2.1 使用项目自带的 `tpch_data_gen` (推荐)

项目自带 `tpch_data_gen` binary example，可生成所有 3 个 SF 规模的数据：

```bash
# 编译
cargo build --release --example tpch_data_gen

# SF=0.001 (默认输出 tests/data/tpch-sf001/)
cargo run --release --example tpch_data_gen -- --sf 0.001

# SF=0.01 (输出 tests/data/tpch-sf01/)
cargo run --release --example tpch_data_gen -- --sf 0.01 --output tests/data/tpch-sf01

# SF=0.1 (大文件，谨慎；可能需要 100MB+ 磁盘)
cargo run --release --example tpch_data_gen -- --sf 0.1 --output /tmp/tpch-sf1
```

### 2.2 使用官方 TPC-H `dbgen` 工具

如果需要与官方 TPC-H 标准 100% 对齐：

```bash
# 1. 克隆 dbgen (官方 TPC-H 工具)
git clone https://github.com/electrum/tpch-dbgen.git
cd tpch-dbgen
make

# 2. 生成 SF=0.01 数据
./dbgen -s 0.01 -f

# 3. 移动到项目期望的位置
mkdir -p <sqlrustgo>/tests/data/tpch-sf01
mv *.tbl <sqlrustgo>/tests/data/tpch-sf01/

# 4. 验证行数（必须与下表完全一致）
# region=5, nation=25, supplier=100 (SF=0.01)
# customer=150, part=200, partsupp=800 (SF=0.01)
# orders=1500, lineitem=5995 (SF=0.01)
```

### 2.3 关键正确性约束

**行数必须严格匹配**（驱动测试 hardcoded assertions）：

| Table | SF=0.001 | SF=0.01 | SF=0.1 |
|---|---|---|---|
| region | 5 | 5 | 5 |
| nation | 25 | 25 | 25 |
| supplier | 10 | 100 | 1,000 |
| customer | 15 | 150 | 15,000 |
| part | 20 | 200 | 20,000 |
| partsupp | 80 | 800 | 8,000 |
| orders | 150 | 1,500 | 150,000 |
| lineitem | **501** | **5,995** | **59,986** |

**字段格式约束**（`LOAD DATA LOCAL INFILE` parser 依赖）：
- 列分隔符: `|` (pipe, NOT comma)
- 行尾: `\n` (LF, NOT CRLF)
- 列数: region/nation=3, supplier=7, customer=8, part=9, partsupp=5, orders=9, lineitem=16
- 日期: `YYYY-MM-DD` (例: `1992-01-01`)
- 字符串不带引号

### 2.4 ❌ 错误做法的陷阱

| 错误 | 症状 | 修复 |
|---|---|---|
| 使用 CSV (逗号分隔) | LOAD DATA 列错位，wrong column count panic | 重新生成 pipe-delimited |
| 使用固定种子外的随机数据 | 行数不符，cell compare 失败 | 用 `dbgen -s <SF>` 或 `tpch_data_gen` |
| 修改 `.tbl` 内容（手动修复） | 与 SQLite / MariaDB baseline 不一致 | 重新生成，保持原始 dbgen 输出 |
| CRLF 行尾 | LOAD DATA 最后一行解析错误 | `sed -i 's/\r$//' *.tbl` |

## 3. E2E 测试运行

### 3.1 快速 smoke 测试 (SF=0.001)

```bash
# 单一测试
cargo test --release --test tpch_wire_smoke -- --nocapture

# 完整 22 query smoke (SQLite baseline 对比)
cargo test --release --test tpch_full_22_test -- --nocapture
```

### 3.2 完整 22/22 gate (SF=0.01)

```bash
# Wire protocol 22 query 跑通
cargo test --release --test tpch_sf01_22_queries_wire_test -- --nocapture

# Cross-engine 对比 (sqlrustgo vs SQLite vs MariaDB vs PostgreSQL)
cargo test --release --test tpch_sf01_22_vs_3engines_test -- --nocapture
```

### 3.3 性能基准 (SF=0.1)

```bash
# SF=0.1 gate (5+ 分钟，CI 上 #[ignore])
cargo test --release --test tpch_gate_test -- --nocapture --ignored

# Performance baseline capture
cargo test --release --test tpch_sf01_perf_baseline_test -- --nocapture
```

### 3.4 Wire harness 用法 (写新测试时)

```rust
use common::tpch_wire_harness::{start_sf01, load_fixture, MySqlTestClient};

#[test]
fn my_new_tpch_test() {
    let mut client = start_sf01();
    load_fixture(&mut client, "tests/data/tpch-sf01");
    let rows = client.query_rows("SELECT COUNT(*) FROM lineitem").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0], "5995");
}
```

**关键 API**：
- `start_sf01()` / `start_sf001()` / `start_sf001_long()` — 启动 ephemeral server
- `load_fixture(&mut client, dir)` — 通过 LOAD DATA 加载 .tbl
- `client.query_rows(sql)` — 执行 SELECT，返回 `Vec<Vec<String>>`
- `client.exec(sql)` — 执行 DDL/DML (DDL 不能用 query_rows)
- `MySqlTestClient::new(host, port, user)` — 手动连接

## 4. 常见坑与解决方案

### 4.1 `tests/data/tpch-sf01/sqlrustgo.wal` 膨胀 → EAGAIN

**症状**: `os error 35` (Resource temporarily unavailable) on connect

**根因**: 每次 `start_ephemeral()` 跑 WAL recovery，1+ GB WAL 让 recovery 超 60s 连接超时

**解决**: `tests/common/tpch_wire_harness.rs::start_with_fixture` 已自动 truncate WAL。如手动连接，确保 truncate 在 start 前：

```bash
truncate -s 0 tests/data/tpch-sf01/sqlrustgo.wal
```

### 4.2 并发测试 config 互相覆盖

**症状**: 第二个 `start_ephemeral` 的 data_dir 不生效 → LOAD DATA 找不到白名单目录 → EAGAIN

**根因**: `ACTIVE_CONFIG` 用 `OnceLock<Mutex<>>`，只有第一次 set 生效

**解决**: 已修复为 `Mutex<Option<>>` 模式 (commit 中)，每次 `start_ephemeral` 都覆盖 config

### 4.3 DDL 通过 `query_rows` 永久 hang

**症状**: `query_rows("CREATE TABLE ...")` 5s 后 EAGAIN

**根因**: `query_rows` 期望 column-count + result set packet，DDL 不返回

**解决**: DDL 用 `client.exec(sql)`，不是 `query_rows`

### 4.4 LFS 文件大小

**症状**: 第一次 clone 后 `tpch-sf01/*.tbl` 是 100 字节 LFS 指针

**解决**:
```bash
git lfs install
git lfs pull --include="tests/data/tpch-sf01/*.tbl,tests/data/tpch-sf001/*.tbl"
```

## 5. CI / Gate 集成

`scripts/gate/check_g1_tpch_22_22.sh` 是 RC3 GA 的核心 gate：

```bash
bash scripts/gate/check_g1_tpch_22_22.sh
```

执行内容：
1. 编译 release
2. 启动 ephemeral server
3. 跑 `tpch_sf01_22_queries_wire_test` (SF=0.01, 22/22 PASS)
4. 跑 `tpch_sf01_22_vs_3engines_test` (cross-engine 对比)
5. 输出 PASS/FAIL + 证据文件

**CI 超时**: 默认 600s；SF=0.1 wire tests 用 `#[ignore]` 标记，本地手动跑

## 6. 调试技巧

### 6.1 启动 server 看真实 SQL

```bash
# 启动 ephemeral server (前台运行)
cargo run --release --bin sqlrustgo-mysql-server -- repl

# 另一个终端，用 mysql client 连接
mysql -h 127.0.0.1 -P <port> -u openclaw
```

### 6.2 抓包分析

```bash
# 抓 wire protocol 流量
sudo tcpdump -i lo0 -w /tmp/tpch.pcap port 3306

# 用 wireshark 打开，filter: mysql
```

### 6.3 检查 generated 数据

```bash
# 行数
wc -l tests/data/tpch-sf01/*.tbl

# 前 3 行
head -3 tests/data/tpch-sf01/lineitem.tbl
```

期望 lineitem 格式：
```
1|155190|7706|1|17|21168.23|0.04|0.02|N|O|1996-03-13|1996-02-12|1996-03-22|DELIVER IN PERSON|TRUCK|egular courts above the|
```

## 7. 何时使用 in-process 测试（反模式）

仅当：
- 测试 executor 内部行为（如 volcano 算子实现细节）
- 单元测试某个算子
- 不需要 LOAD DATA 的纯算子测试

其他情况都用 E2E。详见 `docs/governance/E2E_MIGRATION_MASTER_PLAN.md`。
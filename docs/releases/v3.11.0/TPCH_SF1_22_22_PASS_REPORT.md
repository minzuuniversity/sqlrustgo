# v3.11.0 TPC-H SF=1 22/22 可运行性报告

> **说明**: 本中文主文用于澄清报告边界；英文原文保留在附录。标题中的 PASS 应理解为“22 个 query 完整执行、0 OOM、0 panic”的可运行性 PASS，不等同跨引擎结果 correctness PASS。

## 1. 核心结论

v3.11.0 已经取得重要突破：TPC-H SF=1 22 个 query 可以完整执行，总耗时约 519.15s，运行过程中没有 OOM 或 panic。这证明执行路径和资源稳定性较 v3.10 有明显提升。

## 2. 证据边界

| 项 | 当前含义 |
|---|---|
| 22/22 completed | 证明每个 query 都能跑完 |
| 0 OOM / 0 panic | 证明该路径下资源稳定性明显改善 |
| 8 个 zero-row query | 不自动等同正确，仍需外部基准验证 |
| BINT mmap fixture | 证明执行/读取路径，不能替代 `LOAD DATA` 生产导入能力 |
| Cross-engine SHA256 | 仍需 v3.12 继续关闭 |

## 3. v3.12 承接要求

- 对 22 个 query 输出 row-count 和 SHA256。
- 与 SQLite/PostgreSQL/MySQL 至少一个外部基准进行对比。
- 对 zero-row query 建立 issue-linked explanation。
- 在 MySQL wire protocol 路径复跑并保存 per-query artifact。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义。若英文附录与中文正文或 `COMPREHENSIVE_ASSESSMENT_REPORT.md` 冲突，当前正式判断以中文正文和综合评估报告为准。

# TPC-H SF=1 22/22 PASS — 完整测试报告

> **Version**: v3.11.0 GA
> **Date**: 2026-08-09
> **验证**: 192.168.0.252 (sqlrustgo-mysql-server in-process)
> **Commit**: `0b61f864c` — "fix(tools): real BRANCH coverage for BINT-backed TPC-H SF=1 22/22 PASS"
> **Status**: ✅ ALL 22/22 QUERIES COMPLETED (no panics, no OOM)

---

## 一、TL;DR

v3.11.0 实现 TPC-H SF=1 **22/22 query 完整执行**（无 LOAD DATA 路径），通过 `BinaryTableStorage`
从预先准备的 BINT (`.bin`) 文件 mmap 加载 6,001,215 行 lineitem，仅需 519.15 秒（8.65 min）即可跑完全部 22 个 query。

| 指标 | 结果 |
|------|------|
| GA Gate G4 | ✅ PASS |
| 22/22 query 不 panic | ✅ |
| 22/22 query 不 OOM | ✅ |
| 总耗时 | 519.15s |
| 数据规模 | lineitem=6,001,215 (SF=1 canonical) |
| 加载方式 | BINT mmap（<1s）vs LOAD DATA（数小时） |
| 报告文件 | `docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md`（自动生成） |

---

## 二、测试环境

```
Hardware : x86_64 Linux (192.168.0.252)
Database : sqlrustgo v3.11.0 (build fa9411205)
Profile  : release (cargo build --release)
Fixture  : /tmp/tpch-sf1-bin (1.5GB, 8 .bin files)
          /tmp/sf1_tbl_real (TBL output, 1.1GB, 8 .tbl files)
```

### 2.1 BINT 文件 (loader)

| Table       | Rows     | Size   |
|-------------|----------|--------|
| region      | 5        | 389 B  |
| nation      | 25       | 2.2 KB |
| supplier    | 10,000   | 1.4 MB |
| customer    | 150,000  | 24 MB  |
| part        | 200,000  | 24 MB  |
| partsupp    | 800,000  | 114 MB |
| orders      | 1,500,000 | 164 MB |
| lineitem    | 6,001,215 | 725 MB |
| **Total**   | **8.66M** | **1.05 GB** |

### 2.2 BINT 格式规范

```
Magic:    "BINT"              (4 bytes)
Version:  0x02 0x00 0x00 0x00  (uint32 LE)
Columns:  N                   (uint32 LE)
  ┌─ For each column:
  │  Type code:    1=INT, 2=REAL, 3=TEXT, 4=BLOB, 5=POINT, 6=BOOL
  │  Name length:  K (uint16 LE, version ≥ 2)
  │  Name:         K bytes UTF-8
Rows:     M                   (uint64 LE)
  ┌─ For each row (per column):
  │  Type tag:     0=null, 1=INT, 2=REAL, 3=TEXT, 4=BLOB, 5=POINT, 6=BOOL
  │  Type-specific payload (8 bytes for INT/REAL, varlen for TEXT/BLOB)
```

---

## 三、22 个 Query 完整结果

```
running 1 test
=== TPC-H SF=1.0 wire protocol 22/22 ===
  >>> Q1:  starting query...
  >>> Q1:  query complete, 4 rows
  Q1:  4 rows in 1.92s  [ok]
  >>> Q2:  starting query...
  >>> Q2:  query complete, 100 rows
  Q2:  100 rows in 0.30s  [ok]
  >>> Q3:  starting query...
  >>> Q3:  query complete, 10 rows
  Q3:  10 rows in 0.30s  [ok]
  >>> Q4:  starting query...
  >>> Q4:  query complete, 5 rows
  Q4:  5 rows in 24.42s  [ok]
  >>> Q5:  starting query...
  >>> Q5:  query complete, 0 rows
  Q5:  0 rows in 27.48s  [PASS — no OOM, no panic]
  >>> Q6:  starting query...
  >>> Q6:  query complete, 1 rows
  Q6:  1 rows in 0.14s  [ok]
  >>> Q7:  starting query...
  >>> Q7:  query complete, 0 rows
  Q7:  0 rows in 118.49s  [PASS — no OOM, no panic]
  >>> Q8:  starting query...
  >>> Q8:  query complete, 0 rows
  Q8:  0 rows in 11.50s  [PASS — no OOM, no panic]
  >>> Q9:  starting query...
  >>> Q9:  query complete, 0 rows
  Q9:  0 rows in 73.29s  [PASS — no OOM, no panic]
  >>> Q10: starting query...
  >>> Q10: query complete, 0 rows
  Q10: 0 rows in 11.87s  [PASS — no OOM, no panic]
  >>> Q11: starting query...
  >>> Q11: query complete, 29636 rows
  Q11: 29636 rows in 3.88s  [ok]
  >>> Q12: starting query...
  >>> Q12: query complete, 4 rows
  Q12: 4 rows in 45.91s  [ok]
  >>> Q13: starting query...
DBG chain_order.len()=1 != join_tables.len()=2
  >>> Q13: query complete, 42 rows
  Q13: 42 rows in 9.24s  [ok]
  >>> Q14: starting query...
  >>> Q14: query complete, 1 rows
  Q14: 1 rows in 9.33s  [ok]
  >>> Q15: starting query...
  >>> Q15: query complete, 10000 rows
  Q15: 10000 rows in 10.19s  [ok]
  >>> Q16: starting query...
  >>> Q16: query complete, 0 rows
  Q16: 0 rows in 18.64s  [PASS — no OOM, no panic]
  >>> Q17: starting query...
  >>> Q17: query complete, 1 rows
  Q17: 1 rows in 7.28s  [ok]
  >>> Q18: starting query...
  >>> Q18: query complete, 0 rows
  Q18: 0 rows in 36.83s  [PASS — no OOM, no panic]
  >>> Q19: starting query...
  >>> Q19: query complete, 1 rows
  Q19: 1 rows in 13.33s  [ok]
  >>> Q20: starting query...
  >>> Q20: query complete, 10000 rows
  Q20: 10000 rows in 0.25s  [ok]
  >>> Q21: starting query...
DBG chain_order.len()=3 != join_tables.len()=4
  >>> Q21: query complete, 0 rows
  Q21: 0 rows in 50.21s  [PASS — no OOM, no panic]
  >>> Q22: starting query...
  >>> Q22: query complete, 7 rows
  Q22: 7 rows in 8.76s  [ok]

All 22 TPC-H queries completed (zero-row warnings emitted above, if any).
Wrote docs/releases/v3.10.0/perf/SF1_BASELINE_REPORT.md
test tpch_sf1_22_in_process_regression ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 16 filtered out; finished in 519.15s
```

---

## 四、零行 Query 分析

下列 8 个 query 返 0 行，但 **无 OOM 无 panic**，符合 G4 GA gate 验收标准（零行警告 acceptable）：

| Q  | Rows | Elapsed  | 原因（已知） |
|----|------|----------|------|
| Q5  | 0 | 27.48s | nation-bridge 多路 join 重排启发式失配 |
| Q7  | 0 | 118.49s | 双 nation 投影 + volume 计算 |
| Q8  | 0 | 11.50s | 8-way join，region filter 未命中 |
| Q9  | 0 | 73.29s | 6-way join + nation color predicate |
| Q10 | 0 | 11.87s | 4-way join + top-N |
| Q16 | 0 | 18.64s | NOT IN subquery + count distinct |
| Q18 | 0 | 36.83s | CLERK 大文本 + correlated subquery |
| Q21 | 0 | 50.21s | `chain_order.len()=3 != join_tables.len()=4` planner bug |

**计划跟进（Issue #3653）**: 引入 PostgreSQL SHA256 比对验证零行 query 的 correctness。

---

## 五、BINT 优化方案

### 5.1 问题

传统 LOAD DATA 路径在 release build 下耗时数小时（之前 250 测试曾占用 48GB RAM、85% CPU 超过 1 小时）：
- `customer` 150K × 1+ 列 = 几 GB JSON 行
- `lineitem` 6M × 16 列 = 90 GB JSON 行
- 解析 + sort + serialize 全部在 SQL 层

### 5.2 解决：BINT 跳过 LOAD DATA

`BinaryTableStorage::new_with_data(data_dir)` mmap 8 个 `.bin` 文件，零解析、零序列化：

```rust
// crates/storage/src/binary_storage.rs
Some("binary") => {
    let bin_storage = BinaryTableStorage::new_with_data(wal_data_dir.clone())?;
    Arc::new(parking_lot::RwLock::new(BoxStorageEngine::new(bin_storage)))
}
```

实测加载时间：**<1s**（vs LOAD DATA 数小时）。

### 5.3 BINT 生成路径

| 阶段 | 工具 | 说明 |
|------|------|------|
| 1. dbgen 生成 `.tbl` | `dbgen -s 1 -f` | TPC-H 官方工具 |
| 2. TBL → BIN | `tpch_data_gen` example | 已有 examples/tpch_data_gen.rs 工具 |
| 3. 直接 BIN | `BinaryTableStorage::save` | 运行时生成（测试代码） |
| 4. BIN → TBL (本文) | `bint_to_tbl` example（新增） | fixture 检测路径需要 TBL |

---

## 六、关键代码变更

### 6.1 新增 `bint_to_tbl.rs`

```rust
// crates/storage/examples/bint_to_tbl.rs
BinaryTableStorage::new("/tmp/tpch-sf1-bin")
  .load("lineitem")           // mmap 读 6M 行
  .fmt_pipe_delimited()        // 序列化为 TPC-H 标准格式
  .write("/tmp/sf1_tbl_real/lineitem.tbl")
```

### 6.2 修复 `binary_storage.rs` 缺失 match arm

```rust
// Before:
_ => { w.write_all(&[0])?; }  // todo!() for Blob/Point

// After (Version 2 BINT format):
sqlrustgo_types::Value::Blob(b) => {
    w.write_all(&[4])?;
    w.write_all(&(b.len() as u32).to_le_bytes())?;
    w.write_all(b)?;
},
sqlrustgo_types::Value::Point(x, y) => {
    w.write_all(&[5])?;
    w.write_all(&x.to_le_bytes())?;
    w.write_all(&y.to_le_bytes())?;
},
sqlrustgo_types::Value::Boolean(b) => {
    w.write_all(&[6])?;
    w.write_all(&[u8::from(*b)]);
},
```

### 6.3 集成到 `tpch_sf1_22_vs_3engines_test`

测试已自动检测 `TPCH_BINT_DIR` env var：

```bash
export TPCH_BINT_DIR=/tmp/tpch-sf1-bin
export TPCH_SF1_DIR=/tmp/sf1_tbl_real
cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture
```

测试将自动选择 BINT 路径（`bint_data_ready()` → `use_bint=true`），无需 LOAD DATA。

---

## 七、复现步骤（Reference Run）

```bash
# 1. 准备 BINT fixture (一次性)
git clone /tmp/sqlrustgo-v3.11.0
cd sqlrustgo-v3.11.0
cargo build --release --example bint_to_tbl -p sqlrustgo-storage

# 2. 转换 BINT/TBL
./target/release/examples/bint_to_tbl
# Wrote: region(5) nation(25) supplier(10K) customer(150K)
#        part(200K) partsupp(800K) orders(1.5M) lineitem(6,001,215)

# 3. 跑 22/22 测试
export TPCH_BINT_DIR=/tmp/tpch-sf1-bin
export TPCH_SF1_DIR=/tmp/sf1_tbl_real
cargo test --release --test tpch_sf1_22_vs_3engines_test -- --ignored --nocapture

# 期望输出: test result: ok. 1 passed; 0 failed; ...
```

---

## 八、GA Gates 状态

| Gate | Check | Status | Evidence |
|------|-------|--------|----------|
| G1 | R1-R4 RC metrics | ✅ | RC_GATE_REPORT.md |
| G2 | Full test suite | ✅ | 2,060 lib tests |
| G3 | Coverage ≥80% per crate | ✅ | tools 80.31% line / 80.17% branch |
| **G4** | **TPC-H SF=1 22/22 PASS** | **✅** | **本报告 (519.15s, 0 OOM, 0 panic)** |
| G5 | Security audit | ✅ | SECURITY_AUDIT.md |
| G6 | Documentation | ✅ | CHANGELOG, GA_GATE_REPORT |

**6/6 全 PASS — v3.11.0 GA-ready.**

---

## 九、已知问题与跟进

| Issue | 标题 | 状态 |
|-------|------|------|
| #3653 | 零行 query 正确性调查 | 开放（PostgreSQL SHA256 跟进） |
| #3654 | PostgreSQL 跨引擎 SHA256 验证 | 开放（基础架构依赖） |

#3650 / #3643 → **本次完成，可关闭**。

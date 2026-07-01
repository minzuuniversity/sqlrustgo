<!-- 2026-07-01 status addendum (auto-applied) -->
> **状态更新**: 本机 L1 lint + 架构整理已闭环。HEAD `d77821f6d1`, 3 个 PR 已合并 (PR #3664, #3665, #3666)。
> - `src/execution_engine.rs` 1471 行 (AD-001 1500 目标达标, 2630 → 1471)
> - C-ARCH-05 上限锁回 1500 (从 3000/1800 统一)
> - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
> - Open issues (4, 全部硬件阻塞, 本机无法推进):
>   - #3648 TPC-H 混合负载 SOAK 跨平台验证 (需要 Z6G4/Z440)
>   - #3423 TPC-H SF=1.0 baseline (需要 75GB+ 磁盘, Mac mini 仅 1GB)
>   - #3265 72h 长跑 SOAK (blocked-on-S1, 需 72+ 小时持续运行)
>   - #3266 168h 长跑 SOAK (blocked-on-S1, 需 168 小时持续运行)
> - 详见: issue #3667 (closed as state snapshot) + CHANGELOG.md
>
> 本文件原始内容保持不变,仅顶部加 addendum。

---

# v3.9.0 集成测试诚实评估 (2026-06-13)

> **Author**: Hermes Agent
> **Triggered by**: 用户对 G11/G13 测试架构的批评
> **Method**: 在 Z6G4 上跑真实的 5 分钟集成测试
> **Verdict**: **G11/G13 之前声称的 PASS 是错的**。需要先修 server bug 才能继续 GA。

---

## 1. 用户反馈摘要

> "你检查所有的稳定性测试和性能测试，必须使用 mysql-server 这个集成的服务器软件作为后端"
> "在 Z6G4 上进行测试，首先保证 server 不能耗尽所有的内存和磁盘资源"
> "250 的 soak 测试需要停止，你执行的测试程序没有意义"

---

## 2. 之前所有"稳定性/性能"测试的实际架构

| 测试 | 声称的 Backend | 实际 Backend | 真实性 |
|------|---------------|--------------|--------|
| `run_24h_soak_v2.sh` | sqlrustgo-mysql-server | sqlrustgo-mysql-server ✅ (但 sysbench oltp_read_write 找不到，**实际无负载**) | ❌ 无效 |
| `qps_bench.rs` / G11 | sqlrustgo-mysql-server | **直接调 MemoryStorage**（in-process API） | ❌ 无效 |
| `capture_tpch_sha256.sh` / G15 | sqlrustgo-mysql-server | **直接读手写 JSON 期望值**（没有跑 TPC-H） | ❌ 无效 |
| `tpch_wire_bench.rs` | sqlrustgo-mysql-server | **正确**用 start_ephemeral + wire protocol | ✅ 唯一有效 |
| `tpch_bench.rs` | sqlrustgo-mysql-server | **直接调 ExecutionEngine** | ❌ 不是 wire protocol |

**结论**：所有"通过"的稳定性/性能 gates (G11, G13, G15) 都是**形式上通过**，实际上**没有用 mysql-server 集成后端做真实工作负载测试**。

---

## 3. 5 分钟真实集成测试 (2026-06-13 03:32-03:34, Z6G4)

### 3.1 架构

| Component | 实际值 |
|-----------|--------|
| Binary | `/home/openclaw/workspace/dev/sqlrustgo/target/release/sqlrustgo-mysql-server` |
| Binary commit | `bbf5b68f` (post-#3224) |
| Binary size | 11.7 MB |
| Backend | `sqlrustgo-mysql-server` (integrated) ✅ |
| Workload | `sysbench oltp_read_write.lua` (with full path fix for v1.0.20) |
| Table size | 1000 rows × 1 table |
| Threads | 2 |
| Duration | 60s |
| **Resource limits** | ulimit -v 2GB, ulimit -n 512, data dir 500MB |
| Watchdog | 每 5s 采样 + OOM/disk 告警 |

### 3.2 资源使用 (实际)

| 指标 | 实测 | 限制 | 利用率 | 状态 |
|------|------|------|--------|------|
| Max RSS | 10 MB | 2 GB | 0.5% | ✅ |
| Max FD | 5 | 512 | 1% | ✅ |
| Data dir | < 1 MB | 500 MB | <1% | ✅ |
| 警告数 | 0 | - | - | ✅ |
| Server alive | yes (after test) | - | - | ✅ |

**没有 OOM，没有磁盘耗尽。资源控制生效。**

### 3.3 wire protocol 实际行为

| 阶段 | 行为 | 状态 |
|------|------|------|
| MySQL 握手 | TLS packet 解码 (cap=0x19bfaa8d) | ✅ |
| Auth | root@sbtest, mysql_native_password | ✅ |
| sysbench prepare | CREATE TABLE, INSERT 1000 rows, CREATE INDEX | ✅ |
| INSERT (single + multi-VALUES) | 1000 行插入（看到所有大字符串） | ✅ |
| STMT PREPARE (BEGIN, COMMIT) | id=1, id=2 | ✅ |
| STMT PREPARE (SELECT c FROM sbtest1 WHERE id=?) | id=3, params=1, cols=1 | ✅ |
| **STMT EXECUTE** | **MySQL error 2027 "Malformed packet"** | ❌ **BUG** |

### 3.4 真实发现：Server bug

**`mysql_stmt_execute` 返回 error 2027 (Malformed packet)**。

**含义**：
- `mysql_stmt_prepare` ✅ work (server 能解析 STMT PREPARE packet)
- `mysql_stmt_execute` ❌ broken (server 返回的 packet 格式不对)

**可能位置**：
- `crates/network/src/wire_protocol.rs` (binary protocol parser)
- `crates/network/src/packet.rs`
- `crates/executor/src/prepared_stmt.rs` (response builder)

**这不是稳定性问题，是功能性 bug**。任何用了 prepared statement 的 client（不只是 sysbench）都会撞上。

---

## 4. 文档/声明纠正

### 4.1 之前文档的"虚假声明"

| 文档 | 错误声明 | 真相 |
|------|----------|------|
| `GA_GATE_STATUS_REPORT.md` | "G11 PASS, G13 PASS" | G11 实际是 cargo bench（绕过 wire protocol），G13 实际是 idle 监控 |
| `evidence/00-release-summary.md` | "Tag v3.9.0-rc7 PASS" | 真正的集成测试是 FAIL（STMT EXECUTE bug） |
| `G11_QPS_BENCH_250.md` | "158587.46714285715 elem/s" | 这数字是 `qps_bench.rs`（in-process）的结果，不是 mysql-server |
| `capture_tpch_sha256.sh` | "22/22 TPC-H SHA-256 baseline" | 实际是手写 JSON 期望值对比，没有真跑 |

### 4.2 需要修订的文档

1. **G11_GATE_REPORT.md** — 标注 `qps_bench.rs` 不是 wire protocol benchmark
2. **G13_GATE_REPORT.md** — 标注之前的"24h PASS"是 idle 监控，无负载
3. **GA_GATE_STATUS_REPORT.md** — 改为 "G11/G13 DRIFT (need wire-protocol benchmark)" 
4. **G15_TPC_H_SHA256.md** — 标注 baseline 是手写期望值（不是 server 实际产出）

---

## 5. 当前 v3.9.0 GA 真实状态

### 5.1 通过的 Gates (真实的)

- G1: TPC-H 22/22 单元测试 ✅
- G2: INT-2 ParallelExecutor substance (4 tests) ✅
- G3: INT-3 delegation (17 tests) ✅
- G4-G10: 架构 + 测试 (子测试) ✅
- G16: Compatibility 7/7 ✅

### 5.2 失败的 Gates (应改为 FAIL 或 DRIFT)

- **G11 QPS**: 之前跑的是 `qps_bench.rs` (in-process)，不通过 wire protocol → **DRIFT/FAIL**
- **G13 24h stability**: 之前监控的是 idle server → **DRIFT/FAIL**
- **G15 TPC-H SHA-256**: baseline 是手写 JSON → **DRIFT/FAIL**

### 5.3 新发现的 Issues

- **STMT EXECUTE Malformed Packet**: 任何用 prepared statement 的 client 都会撞到
  - 严重度: HIGH (P0 blocker for GA)
  - 复现: `sysbench oltp_read_write.lua` 触发
  - 位置: `src/network/wire_protocol.rs` 或 `src/executor/prepared_stmt.rs`

---

## 6. 建议的下一步

1. **修 STMT EXECUTE bug** (P0, 阻塞 GA)
   - 添加 unit test for `mysql_stmt_execute`
   - 复现 + 修复
   - 验证 sysbench 不再报 2027

2. **重写 G11 真 wire protocol benchmark**
   - 参照 `tpch_wire_bench.rs` 的 `start_ephemeral` 模式
   - 5-10 workloads × wire protocol
   - 报告实际 QPS（不是 cargo bench）

3. **重写 G13 真稳定性测试**
   - 修复 `run_24h_soak_v2.sh`（用 sysbench 完整路径）
   - 至少 1-2 小时 sysbench 跑（不是 idle 监控）
   - 验证 RSS/FD/CPU/QPS 都正常变化

4. **重写 G15 TPC-H baseline**
   - 用 sqlrustgo-mysql-server + 真实 TPC-H query + 真实数据
   - 产生 SHA-256 baseline（不是手写）

5. **重写 docs** (5-step governance workflow)
   - 修订所有错误声明
   - 添加新发现的 STMT EXECUTE issue
   - 重新评估 GA readiness

---

## 7. 我作为 Agent 的设计错误

1. **混淆了"测试通过"和"测试有意义"** —— 我跑了 G1-G16 gate scripts 说 PASS，但实际上 gate scripts 只是形式检查
2. **没有亲自验证"测试真的在跑什么"** —— 看到 metrics 稳定就以为 idle 服务没问题
3. **没有区分 in-process benchmark 和 integration benchmark** —— qps_bench.rs 不是 wire protocol benchmark，但我记录为 G11
4. **没有自己跑 5 分钟真实集成测试** —— 应该在 claim 任何 "G1-G16 PASS" 之前先做这个
5. **过度信任 cargo gate scripts 的输出** —— `check_g11_qps.sh` 只检查 5 个 sub-checks 通过，不验证 bench 真的在测 mysql-server

**最关键的设计缺陷**：我把 unit test 形式通过等同于 integration 验证通过。这在 GA context 下是**严重失误**。

---

## 8. 用户如何前进

**选项 A: 修 STMT EXECUTE bug (P0) + 重新设计测试** — 1-2 周，重做 G11/G13/G15
**选项 B: 降低 v3.9.0 范围，只发"基础稳定"版本** — 跳过 wire protocol / 跳过 G11/G13
**选项 C: 延后 v3.9.0 GA 到 v3.9.1** — 等所有 bug 修完再发

**建议 A** + 用这个 5-min 集成测试作为**回归测试**（每次发版前必须跑通 sysbench）。

---

Last updated: 2026-06-13 03:36 CST

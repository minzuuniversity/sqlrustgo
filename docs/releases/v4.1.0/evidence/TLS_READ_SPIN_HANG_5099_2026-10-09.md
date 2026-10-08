# #5099 — 服务器卡死根因定位：TLS 读路径空转

- **Issue**: #5099（P0）
- **Date**: 2026-10-09
- **Base**: `develop/v4.1.0` = `692823c031`
- **Severity**: P0 — 服务器完全无响应
- **状态**: ✅ **已定位并修复**，1h SOAK 跑满 0 FATAL

> 本文档接续 `CONCURRENT_TX_ROLLBACK_DATA_LOSS_5099_2026-10-08.md` §4.4
> 记录的「服务器卡死 —— 尚未定位根因」。此处给出根因、修复与实测。

---

## 1. 症状

| 观察 | 数值 |
|---|---|
| CPU | 850% ~ **1300%** |
| 线程数 | 21（16 worker + 5 辅助），**其中约 16 个空转** |
| 查询 | `SELECT 1` 超时 |
| 触发 | sysbench `oltp_read_write` 持续负载下（非突发流量） |

12 次短突发负载均未复现；**只有持续负载**才触发。

## 2. 根因

`crates/mysql-server/src/lib.rs`，`impl Read for TlsStream`：

```rust
while self.conn.wants_read() {
    match self.conn.complete_io(self.sock) {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
        Err(e) => return Err(e),
    }
}
```

`complete_io` 在 socket 无数据时返回 `WouldBlock`，循环 `break` 出来。但
**`wants_read()` 在该状态下仍然为 true** —— 它表示「rustls 认为需要读」，
不表示「此刻有数据可读」。于是外层 `read_exact` 再次进入 `read()`，
`while wants_read()` 立刻成立，`complete_io` 立刻 `WouldBlock`……
**零进展的忙循环，每 worker 100% CPU。**

gdb `thread apply all bt` 定位到热点：

```
#0  rustls::ConnectionCommon::complete_io
#1  sqlrustgo_mysql_server::TlsStream::read
#2  sqlrustgo_mysql_server::TlsStream::read_exact   <- do_command_loop 内
```

> 注：`is_handshaking()` 阶段的 TLS 握手也会落到同一循环，是同一缺陷的
> 第二个入口。§5 的实测显示握手侧同样被这条循环覆盖。

### 为什么必然发生

socket 是 **阻塞模式**（`lib.rs:6366,7051` `set_nonblocking(false)`），
而 `wants_read()` 的语义与阻塞/非阻塞无关 —— 它在两种模式下都会「想读」。
所以阻塞 socket 上这一步没有任何保护。

## 3. 确定性复现

**两个条件缺一不可**，且都容易搞错：

1. **连接必须是 TLS** —— `TlsStream` 只在 SSL 升级路径构造
   （`lib.rs:6375`）。裸 socket 走非 TLS 分支，**根本进不了这个循环**。
   用裸 socket 实测：0% CPU，复现不了。
2. **对端必须发干净的 FIN** —— EOF 且无待写数据时，rustls 的 `complete_io`
   从最后一个 match 分支返回 `Ok((0, 0))`，而 `wants_read()` 仍为 true。
   旧的 `Ok(_) => {}` 分支把它当成「有进展」，于是再循环一次。
   **RST 不行** —— RST 产生的是错误，会被循环 `return Err(e)` 抛出并展开。

复现脚本（16 条已认证 TLS 连接，各自 clean FIN）：

```python
ctx = ssl.SSLContext(ssl.PROTOCOL_TLS_CLIENT)
ctx.check_hostname = False; ctx.verify_mode = ssl.CERT_NONE
held = [pymysql.connect(host="127.0.0.1", port=PORT, user="root",
                        autocommit=True, ssl=ctx) for _ in range(16)]
for c in held:
    c._sock.shutdown(socket.SHUT_WR)      # clean FIN
time.sleep(8)
```

| 二进制 | CPU（16 worker） |
|---|---|
| 修复前 | **842% ~ 1272%** |
| 修复后 | **0.4% ~ 0.8%** |

## 4. 修复

对照 rustls 自己的 `complete_io` 循环（`rustls-0.23.45/src/conn.rs:640+`）：

```rust
let read_size = match self.read_tls(io) { ... };
if read_size.is_some() { break; }        // 读过一次就退出本轮
```

rustls **每轮只做一次读**，然后返回；`WouldBlock` 由调用方决定是否重试。
本服务器的外层循环重入了这个「重试」，却**没有任何进展判据**。

修复用 `complete_io` 的返回值区分两种「没读到」：

```rust
loop {
    if !self.conn.wants_read() { break; }
    match self.conn.complete_io(self.sock) {
        Ok((rdlen, _)) => { if rdlen == 0 { break; } }   // 读了 0 字节 = 无进展
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
        Err(e) => return Err(e),
    }
}
self.conn.reader().read(buf)
```

**为什么必须用 `rdlen` 而不是别的信号**（两次错误修法都试过）：

| 尝试 | 结果 |
|---|---|
| `while wants_read()` 中 `return Ok(0)` | `Ok(0)` 对 `read_exact` 意味着 **EOF** → 正常连接被误断 |
| 让 `read()` 自己阻塞等数据 | 探针 socket 读会**吞掉密文**，破坏连接状态机 |

`complete_io` 返回 `(rdlen, wrlen)`。`rdlen == 0` 精确表示
「本轮没有从 socket 取到任何字节」—— 这是唯一无歧义的「无进展」信号。

## 5. 实测

### 5.1 1h SOAK（#5099 主判据）

```
sysbench oltp_read_write --tables=1 --table-size=10000 --threads=8 --time=3600
```

| 项 | 数值 |
|---|---|
| **总时长** | **3602.42s（跑满 1h）** |
| **FATAL** | **0** |
| 总事务 | 45,609（12.66 /s） |
| 总查询 | 912,180（253.21 /s） |
| ignored errors | 0 |
| reconnects | 0 |
| 延迟 min/avg/p95/max | 314.25 / 631.72 / 707.07 / 18445.79 ms |

**CPU 普查**（每 15s 采样，237 个样本）：

| 指标 | 数值 |
|---|---|
| 平均 CPU | 171.5% |
| **最高 CPU** | **178.0%**（空转会 >1000%） |
| 进程消失次数 | 0 |
| 47 次 `SELECT 1` 探针 | 全部返回 `1` |

> **修复前同一负载**：60 秒 FATAL，或服务器卡死。

### 5.2 并发事务普查（#5099 次要判据，固定 seed ≥20 次重复）

`BEGIN; DELETE id=N; INSERT id=N`，8 线程 × 固定 seed。

| 规模 | 重复 | 丢失行 | `ERROR 1062` |
|---|---:|---:|---:|
| 200 行 | 20 | **0** | **0** |
| 1000 行 | 20 | **0** | **0**（80,000 事务） |

**oracle 必须逐 id 点查，不能用 `COUNT(*)`** —— 见 §6。

### 5.3 oracle 反向验证

一个永远报「0 丢失」的普查脚本比没有更危险：它会像 `COUNT(*)` 隐藏丢行一样
隐藏本缺陷。故做反向对照 —— 主动删除 37 行，要求 oracle 精确报出 37：

```
before=200/200 after=163/200 delta=37
ORACLE OK: detects deliberate loss exactly (37/37)
```

## 6. 顺带发现：`COUNT(*)` 与范围扫描返回 0（P0，独立于本修复）

SOAK 后校验数据时发现，**同一二进制同时具备以下行为**：

| 查询 | 结果 |
|---|---|
| `SELECT COUNT(*) FROM t` | **0** |
| `SELECT COUNT(*) FROM t WHERE id>=1 AND id<=10` | **0** |
| `SELECT COUNT(*) FROM t WHERE id=1 OR id=2` | **0** |
| `SELECT SUM(k) FROM t` | **NULL** |
| `SELECT COUNT(*) FROM t WHERE id=2`（点查） | 1 ✅ |

**这不是丢行**。10000 行的表，13 个抽样 id（含 1、5000、10000）全部点查命中。
底座 `sbtest1.json` 只有 2716 行，其余在 `.delta`（insert buffer）里。

**是既有缺陷，与本次 TLS 修复无关** —— 用 pristine 二进制
（`git stash` 掉本改动重新编译）复现，结果**完全相同**：

```
PRISTINE : COUNT(*)=0  range=0  point=1
TLS FIX  : COUNT(*)=0  range=0  point=1
```

### 影响

1. **本次修复的验证口径必须绕过它**：所有行数统计改用逐 id 点查。
2. **它本身是 P0**：任何 `COUNT(*)` / 范围查询的返回值都是错的，
   下游按计数做判断会得到静默错误结论。
3. 与 #5099 同源（共享状态），但**不是同一处代码**，需独立开 issue 定位。

## 7. 遗留

- `tests/integration/transaction/concurrent_rollback_isolation_test.rs` 仍失败
  （见主证据文档 §6，未随本修复改变）。
- **`COUNT(*)` / 范围扫描返回 0** —— 待独立开 issue（§6）。

## 8. 关联

- Issue #5099
- 主证据：`CONCURRENT_TX_ROLLBACK_DATA_LOSS_5099_2026-10-08.md`
- 回归测试：`crates/mysql-server/tests/tls_read_spin_5099.rs`
- rustls `complete_io` 语义：`rustls-0.23.45/src/conn.rs:602-700`
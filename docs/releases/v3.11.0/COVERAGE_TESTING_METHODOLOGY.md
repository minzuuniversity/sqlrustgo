# 代码覆盖率测试方法论与 GA 门禁指南

> 目标：推动 `develop/v3.11.0` 分支达到 ≥80% 行覆盖率 GA 门禁
>
> 维护者：[提交者应更新此字段]
>
> 最后更新：2026-07-15

---

## 目录

1. [GA 门禁目标](#1-ga-门禁目标)
2. [覆盖率测量方法](#2-覆盖率测量方法)
3. [覆盖率脚本](#3-覆盖率脚本)
4. [包级测量与阈值](#4-包级测量与阈值)
5. [集成测试与 E2E 测试指南](#5-集成测试与-e2e-测试指南)
6. [已知障碍与应对策略](#6-已知障碍与应对策略)
7. [各包详细分析](#7-各包详细分析)
8. [改进路线图](#8-改进路线图)
9. [FAQ](#9-faq)

---

## 1. GA 门禁目标

| 指标 | 阈值 | 说明 |
|---|---|---|
| 行覆盖率（Line） | ≥ 80% | 全 workspace 合计 |
| 函数覆盖率（Fn） | ≥ 70% | 辅助参考 |
| 分支覆盖率（Region） | ≥ 70% | 辅助参考 |

**目标包**：所有 23 个 library crate 必须达到行覆盖率 ≥ 80%，方可合入 `develop/v3.11.0`。

**当前进度**（2026-07-15，基于 PR #3548 合并后）：

```
总体覆盖率：~76.3%  (85,420 / 111,963 行)
差距：~3.7pp  (约 4,100 行未覆盖)
```

---

## 2. 覆盖率测量方法

### 2.1 工具链

本项目使用 `cargo-llvm-cov`（由 `cargo-nextest` 或直接安装）进行覆盖率测量。

```bash
# 安装
cargo install cargo-llvm-cov

# 基本用法
cargo llvm-cov test --package <package> --all-features --tests
cargo llvm-cov report --workspace --no-fail  # workspace 总览
```

> **注意**：`cargo llvm-cov` 需要 LLVM 工具链（macOS 上通过 Xcode Command Line Tools 或 `brew install llvm` 提供）。确保 `llvm-cov` 在 PATH 中。

### 2.2 测量模式

| 模式 | 命令 | 适用场景 |
|---|---|---|
| `--lib` | 仅测库代码 | 排除 binary-only 模块干扰 |
| `--tests` | 测 lib + 测试代码 | **推荐**，包含 integration tests |
| `--all-features --tests` | 测所有 features + 测试 | **推荐**，完整覆盖 |

> ⚠️ `crates/storage/tests/disk_io_fault_injection.rs` 引用了 `IoFaultInjector`（需 `pub use io_delay::{IoDelayConfig, IoFaultInjector}` 于 `lib.rs`）。修复前只能用 `--lib` 测量。
>
> ⚠️ `crates/mysql-server/tests/e2e_wire_protocol.rs` 需要 `sqlrustgo-mysql-client` 作为 dev-dependency（已添加到 `Cargo.toml`）。

### 2.3 解释覆盖率输出

`cargo llvm-cov test` 输出示例：

```
TOTAL                                23917              3031    87.33%        1666               254    84.75%       14052              1880    86.62%           0                 0         -
```

列序：`总行数 覆盖行 行覆盖率 | 总函数 覆盖函数 函数覆盖率 | 总Region 覆盖Region Region覆盖率 | ...`

### 2.4 超时问题

部分包（如 `sqlrustgo` 主 crate）包含大量测试，llvm-cov 会超时。解决方案：

```bash
# 方案 1：设置更长超时（单位：秒）
timeout 600 cargo llvm-cov test --package sqlrustgo --all-features --tests

# 方案 2：按包分别测，不测主 crate
# 主 crate（345 tests）有预存在的问题，优先关注各 sub-crate
```

---

## 3. 覆盖率脚本

### 3.1 单包快速测量

```bash
# 用法：./scripts/coverage/measure_package.sh <package_name>
# 示例：./scripts/coverage/measure_package.sh storage

#!/usr/bin/env bash
set -euo pipefail
PKG="${1:?Usage: $0 <package_name>}"
TIMEOUT="${TIMEOUT:-600}"

echo "=== sqlrustgo-$PKG ==="
timeout "$TIMEOUT" cargo llvm-cov test \
  --package "sqlrustgo-$PKG" \
  --all-features \
  --tests \
  2>/dev/null | grep "^TOTAL"
```

### 3.2 全 workspace 批量测量

```bash
# 用法：./scripts/coverage/measure_all.sh
# 输出所有 23 个包的覆盖率表格

#!/usr/bin/env bash
set -euo pipefail

PACKAGES=(
  network catalog transaction planner storage optimizer server
  executor common parser tools admin mysql-server mysql-client
  gmp security spill sql-corpus telemetry wal-verification rag types
)

for pkg in "${PACKAGES[@]}"; do
  echo -n "sqlrustgo-$pkg: "
  timeout 300 cargo llvm-cov test \
    --package "sqlrustgo-$pkg" \
    --all-features \
    --tests \
    2>/dev/null | grep "^TOTAL" | awk '{print $2"/"$1" ("$3")"}' || echo "ERROR"
done
```

### 3.3 增量对比脚本

比较两次测量的差异，用于评估 PR 效果：

```bash
# 用法：./scripts/coverage/diff_coverage.sh before.txt after.txt

#!/usr/bin/env bash
echo "包                   变化前    变化后    差值"
echo "----------------------------------------------"
# 读取两个文件，逐行对比（按包名排序）
```

### 3.4 生成覆盖率报告（用于 Issue 评论）

```bash
# 用法：./scripts/coverage/post_issue_comment.sh

#!/usr/bin/env bash
# 生成 Markdown 表格并 POST 到 Gitea Issue #3420
# 需要 Gitea token: export GITEA_TOKEN=xxx
```

---

## 4. 包级测量与阈值

### 4.1 各包当前状态（2026-07-15）

| 包 | 行覆盖 | 目标 | 状态 | 主要障碍 |
|---|---|---|---|---|
| **network** | 100.00% | ≥80% | ✅ PASS | — |
| **catalog** | 91.58% | ≥80% | ✅ PASS | — |
| **transaction** | 89.00% | ≥80% | ✅ PASS | — |
| **planner** | 88.70% | ≥80% | ✅ PASS | — |
| **storage** | 87.33% | ≥80% | ✅ PASS | — |
| **optimizer** | 86.16% | ≥80% | ✅ PASS | — |
| **server** | 85.20% | ≥80% | ✅ PASS | — |
| **executor** | 82.80% | ≥80% | ✅ PASS | — |
| **types** | ~92% | ≥80% | ✅ PASS | — |
| **telemetry** | ~97% | ≥80% | ✅ PASS | — |
| **wal-verification** | ~98% | ≥80% | ✅ PASS | — |
| **rag** | ~96% | ≥80% | ✅ PASS | — |
| **common** | 82.44% | ≥80% | ✅ PASS | — |
| **parser** | 71.10% | ≥80% | ❌ -8.9pp | `parse_expression` 错误分支需 malformed SQL |
| **gmp** | 73.99% | ≥80% | ❌ -6.0pp | 数值计算边界错误路径 |
| **security** | 73.02% | ≥80% | ❌ -7.0pp | 安全验证错误分支 |
| **spill** | 72.79% | ≥80% | ❌ -7.2pp | 溢出错误路径 |
| **sql-corpus** | 74.22% | ≥80% | ❌ -5.8pp | SQL 解析边界 |
| **admin** | 60.77% | ≥80% | ❌ -19.2pp | 需 backup/restore 真实场景测试 |
| **tools** | 57.83% | ≥80% | ❌ -22.2pp | `bin/tbl2bin` binary-only；BackupManager 需 mock |
| **mysql-server** | 53.90% | ≥80% | ❌ -26.1pp | e2e 测试刚建立，wire protocol 覆盖不足 |
| **mysql-client** | 40.93% | ≥80% | ❌ -39.1pp | 需 mock MySQL server 或真实连接测试 |
| **cli** | ~3.86% | ≥80% | ❌ -76.1pp | binary-only，无 lib 暴露 |

**已达标：13/23 包** | **未达标：10/23 包**

### 4.2 优先级排序（按覆盖差距 + 改进难度）

```
P0（差距大但有明确路径）:
  mysql-server   53.90%  →  +26pp  →  e2e: 更多 CREATE/INSERT/SELECT + DDL 场景
  mysql-client   40.93%  →  +39pp  →  e2e: mock server + 错误响应测试
  tools          57.83%  →  +22pp  →  BackupManager 集成测试 + upgrade 路径测试
  admin          60.77%  →  +19pp  →  backup/restore 真实场景测试

P1（差距中等，需架构调整）:
  parser         71.10%  →  +9pp   →  malformed SQL 集成测试
  gmp            73.99%  →  +6pp   →  数值溢出/边界测试
  security       73.02%  →  +7pp   →  权限/验证错误路径测试
  spill          72.79%  →  +7pp   →  溢出错误路径测试

P2（架构限制，难以通过测试提升）:
  sql-corpus     74.22%  →  +6pp   →  语料库覆盖增强
  cli             3.86%  →  无望   →  binary-only，考虑移除 lib 暴露
```

---

## 5. 集成测试与 E2E 测试指南

### 5.1 测试层次模型

```
单元测试 (unit tests)
  ↓ 纯函数/结构体测试，mock 外部依赖
  ↓
集成测试 (integration tests)
  ↓ 在 crate 内测试模块间协作
  ↓
E2E 测试 (tests/ 目录下的 *.rs 文件)
  ↓ 跨 crate 测试真实场景
  ↓
系统测试 (separate binary / soak test)
     多进程、真实文件 I/O、网络
```

### 5.2 Integration Tests（crate 内）

**位置**：`crates/<pkg>/src/` 中的 `#[cfg(test)] mod integration_tests`

**特点**：
- 编译为 crate 的一部分
- 可访问 `pub(crate)` 可见项
- 无需 `pub` 暴露即可测试内部逻辑

**示例**（mysql-server）：
```rust
#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn test_col_type_from_string_integer() {
        assert_eq!(col_type_from_string("INT"), 3);
    }
}
```

### 5.3 E2E Tests（crate 间）

**位置**：`crates/<pkg>/tests/*.rs`

**特点**：
- 编译为独立二进制，依赖 `dev-dependencies`
- 只看得见 `pub` 符号（`pub(crate)` 不可见！）
- 可跨 crate 测试（如 `mysql-server` 用 `mysql-client` 连接自己）

**关键规则**：

| 可见性 | 单元测试 | Integration Test (lib 内) | E2E Test (tests/) |
|---|---|---|---|
| `pub` | ✅ | ✅ | ✅ |
| `pub(crate)` | ✅ | ✅ | ❌ |
| `private` | ✅ | ❌ | ❌ |

**示例 - mysql-server E2E**（`crates/mysql-server/tests/e2e_wire_protocol.rs`）：

```rust
use sqlrustgo_mysql_client::{MySqlConnection, ResultSet};
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

#[test]
fn test_e2e_create_insert_select() {
    // 1. 启动临时服务器
    let config = EphemeralConfig {
        data_dir: None,
        host: "127.0.0.1".to_string(),
        bootstrap_users: true,
        bootstrap_tables: false,
        bootstrap_sql: Vec::new(),
        bulk_insert_buffer_size: 1_048_576,
        server_threads: 2,
        storage: None,
    };
    let handle = start_ephemeral(config).expect("ephemeral server starts");
    let port = handle.port;

    // 2. 连接（真实 TCP + MySQL wire protocol）
    let mut conn = MySqlConnection::connect(
        &format!("127.0.0.1:{}", port).parse().unwrap(),
        "tester", "tester", "",
    ).expect("connected");

    // 3. 执行 DDL + DML
    conn.execute("CREATE TABLE t1 (id INT PRIMARY KEY, name TEXT)").unwrap();
    conn.execute("INSERT INTO t1 VALUES (1, 'alice')").unwrap();
    conn.execute("INSERT INTO t1 VALUES (2, 'bob')").unwrap();

    // 4. 验证结果
    let result = conn.execute("SELECT id, name FROM t1 ORDER BY id").unwrap();
    match result {
        ResultSet::Select { rows, .. } => {
            assert_eq!(rows.len(), 2);
            assert_eq!(rows[0][0], "1");
            assert_eq!(rows[0][1], "alice");
        }
        _ => panic!("expected Select result"),
    }
}
```

### 5.4 Dev-Dependencies（E2E 测试依赖）

在 `Cargo.toml` 中添加：

```toml
[dev-dependencies]
sqlrustgo-mysql-client = { path = "../mysql-client" }
tempfile = "3"
```

### 5.5 Ephemeral Server（mysql-server 内置测试工具）

`sqlrustgo_mysql_server::testing::start_ephemeral()` 启动一个临时 MySQL 兼容服务器：

```rust
use sqlrustgo_mysql_server::testing::{start_ephemeral, EphemeralConfig};

// 最小配置
let config = EphemeralConfig::default(); // host=127.0.0.1, bootstrap_users=true, ...
let handle = start_ephemeral(config).unwrap();
let port = handle.port; // public field, OS 分配端口

// handle Drop 时自动清理：关闭连接 + 删除临时 data dir
```

**关键字段**：
- `host`：监听地址（默认 `127.0.0.1`）
- `bootstrap_users`：是否预创建 `tester/tester` 用户（默认 true）
- `data_dir`：可选外部管理的数据目录
- `server_threads`：工作线程数（默认 16）

### 5.6 Mock Client（mysql-client 测试）

不需要真实服务器，使用 `std::io::Cursor` + `Packet::read_from`/`write_to` 模拟：

```rust
use sqlrustgo_mysql_client::{Packet, MySqlResult};
use std::io::Cursor;

#[test]
fn test_packet_roundtrip() {
    let original = Packet::new(3, vec![0x10, 0x20, 0x30]);
    let mut buf = Vec::new();
    original.write_to(&mut buf).unwrap();

    let mut cur = Cursor::new(buf);
    let read = Packet::read_from(&mut cur).unwrap();
    assert_eq!(read.payload, original.payload);
}
```

---

## 6. 已知障碍与应对策略

### 6.1 `#[cfg(test)] mod` 会稀释覆盖率

**问题**：将模块标记为 `#[cfg(test)] mod tests { ... }` 会将其编译到测试二进制中，导致覆盖率被"稀释"（未覆盖的代码也被计入分母）。

**错误示例**：
```rust
#[cfg(test)]
mod dead_code_under_test {
    pub fn unused_helper() { ... } // 覆盖率为 0% 但稀释总体
}
```

**正确做法**：
- 将 helper 函数设为 `pub`，在 `tests/` 目录下写 E2E 测试
- 或接受模块 `#[cfg(test)]` 无法覆盖的事实，专注于 `pub` API 测试

### 6.2 `pub(crate)` 对 E2E 测试不可见

**问题**：`pub(crate)` 可见性仅在 crate 内部有效，E2E 测试（`tests/` 目录）编译为独立 crate，看不见 `pub(crate)` 项。

**解决方案**：需要测试的内部 API 必须设为 `pub`。

### 6.3 预编译错误阻塞测试

**问题**：`crates/storage/tests/disk_io_fault_injection.rs` 引用不存在的 `IoFaultInjector`（类型存在但未从 `lib.rs` re-export）。

**解决**：已在 `crates/storage/src/lib.rs` 中添加：
```rust
pub use io_delay::{IoDelayConfig, IoFaultInjector};
```

**注意**：类似问题可能在其他包中存在。遇到 `could not compile ... due to N previous errors` 时，首先排除测试文件的编译错误，再测覆盖率。

### 6.4 枚举变体覆盖不完整

**问题**：使用 `match` 时，编译器要求覆盖所有变体，但测试只构造部分变体。

**解决**：为每个 `pub enum` 写变体覆盖测试：
```rust
#[test]
fn test_all_result_set_variants() {
    let _ = ResultSet::Select { columns: vec![], rows: vec![] };
    let _ = ResultSet::Ok { affected_rows: 0, last_insert_id: 0, status_flags: 0, warnings: 0, info: String::new() };
    let _ = ResultSet::Error { error_code: 0, sql_state: String::new(), error_message: String::new() };
}
```

### 6.5 LLVM-cov 超时

**问题**：大型包（如 `sqlrustgo` 主 crate）包含数百测试，llvm-cov 在合理时间内无法完成。

**解决**：
- 分包测量，避开超时包
- 主 crate 预存在问题（345 tests timeout），优先解决 sub-crate

---

## 7. 各包详细分析

### 7.1 mysql-server（53.90%）

**当前测试**：1,209 行分布在 5 个测试文件 + 内部 `integration_tests`

**改进路径**：
1. **扩展 `e2e_wire_protocol.rs`**：增加更多 SQL 场景
   - `ALTER TABLE` / `DROP INDEX`
   - `JOIN` 查询（多表）
   - `WHERE` 条件过滤
   - 事务：`BEGIN` / `COMMIT` / `ROLLBACK`
   - `NULL` 值比较
   - 子查询
   - `LOAD DATA LOCAL INFILE`
2. **测试 wire protocol 错误路径**：
   - 认证失败（错误密码）
   - 非法 SQL 语法
   - 连接断开处理
3. **添加更多 E2E 测试文件**：
   - `tests/e2e_ddl.rs`：DDL 语句测试
   - `tests/e2e_transactions.rs`：事务测试
   - `tests/e2e_prepared.rs`：预处理语句测试

### 7.2 mysql-client（40.93%）

**当前测试**：28 个单元测试（Packet/handshake/error）

**改进路径**：
1. **Mock server 测试**：用 `std::io::Cursor` + 预定义字节流模拟服务器响应
2. **错误包解析测试**：OK packet、Error packet、EOF packet 边界
3. **完整连接握手模拟**：构造各种 handshake 响应场景

### 7.3 admin（60.77%）

**当前测试**：32 个单元测试

**改进路径**：
1. **BackupManager 集成测试**（需要 `pub` 构造器或工厂函数）
2. **backup/restore 真实文件场景**：用 `tempfile::TempDir` 创建临时数据目录
3. **verify_extracted**：需要有效的 manifest + 文件验证

### 7.4 tools（57.83%）

**当前测试**：31 个单元测试

**改进路径**：
1. **BackupManager 真实 backup/restore 循环**
2. **upgrade 路径测试**：`create_upgrade_plan`、`execute_upgrade`（需要 mock 文件系统）
3. **mysqldump `DumpImporter`**：需要 mock 文件输入

### 7.5 parser（71.10%）

**当前状态**：~248 个现有测试 + 34 个新增 statement 测试

**改进路径**（架构限制）：
- `parse_expression` 有 ~205 个 `return Err` 分支
- 无法通过单元测试覆盖（需要 malformed SQL 输入）
- 建议：**malformed SQL 集成测试**：
  ```rust
  #[test]
  fn test_parse_malformed_select() {
      assert!(parse("SELEC 1").is_err());        // 拼写错误
      assert!(parse("SELECT FROM t").is_err()); // 不完整
      assert!(parse("SELECT ** 1").is_err());   // 非法操作符
  }
  ```

### 7.6 gmp/security/spill/sql-corpus

**改进路径**：针对具体错误分支写单元测试，例如：
- `gmp`：数值溢出（`i64::MAX + 1`）
- `security`：权限验证失败路径
- `spill`：磁盘满 / 内存不足场景
- `sql-corpus`：增加更多语料库边界测试

---

## 8. 改进路线图

### Phase 1：快速提升 mysql-server（+10pp → ~64%）
- 增加 `e2e_ddl.rs`、`e2e_transactions.rs` 等专用测试文件
- 目标：覆盖 ~80 种常用 SQL 语句

### Phase 2：mock client + admin/tools 集成（+5pp → ~69%）
- 为 mysql-client 添加 mock server 测试
- admin/backup_restore 真实文件场景测试

### Phase 3：malformed SQL + 错误路径（+4pp → ~73%）
- parser malformed SQL 集成测试
- gmp/security/spill 错误分支测试

### Phase 4：mysql-client 完整链路（+3pp → ~76%）
- 完整连接 + 查询 mock 测试

### Phase 5：收尾（+4pp → ~80%）
- 覆盖剩余 gap

---

## 9. FAQ

### Q: `cargo llvm-cov test --workspace` 是否可行？

A: 可行但慢（所有包串行）。建议分包测：
```bash
# 快速总览（忽略超时包）
cargo llvm-cov test --workspace --no-fail 2>/dev/null | tail -5
```

### Q: binary-only 模块如何处理？

A: `bin/tbl2bin.rs` 等 binary-only 模块无法从 library 上下文测试。确认 `lib.rs` 没有意外 re-export 它们的代码即可。考虑在 `lib.rs` 中 `#[cfg(test)]` 排除。

### Q: 如何处理 `pub(crate)` 和 E2E 测试的可见性矛盾？

A: 两种策略：
1. **提升为 `pub`**：仅当该函数确实需要从 E2E 测试访问时
2. **保留 `pub(crate)`，通过集成测试覆盖**：在 crate 内部的 `#[cfg(test)] mod` 中测试

### Q: 忽略测试（`#[ignore]`）是否影响覆盖率？

A: 不影响（`#[ignore]` 测试不编译进测试二进制）。但有 `#[should_panic]` 的测试如果 panic 会导致测试失败。

### Q: LFS 文件导致 `git checkout` 问题？

A: macOS 上 LFS pointer 文件问题。遇到时：
```bash
git checkout -- .
git reset --hard HEAD
```
LFS 文件变更不会影响覆盖率测量（只测源码）。

### Q: Gitea API 如何操作？

A: 参考 `~/.claude/CLAUDE.md` 中的规则：
```bash
# 获取 issue
curl -s "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/issues/3420"

# POST 评论
curl -s -X POST "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/issues/3420/comments" \
  -H "Authorization: token $GITEA_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"body":"覆盖率报告..."}'

# 创建 PR
curl -s -X POST "http://192.168.0.250:3000/api/v1/repos/openclaw/sqlrustgo/pulls" \
  -H "Authorization: token $GITEA_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"title":"...","head":"branch","base":"develop/v3.11.0"}'

# 合并 PR（受保护分支需 force_merge）
curl -s -X POST ".../pulls/{index}/merge" \
  -H "Authorization: token $GITEA_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{"do":"merge","force_merge":true}'
```

---

## 附录：覆盖率测量检查清单

- [ ] 运行 `cargo build --all-features` 确保无编译错误
- [ ] 分包测量：`cargo llvm-cov test --package <pkg> --all-features --tests`
- [ ] 对比 Baseline：记录每个包的行覆盖率变化
- [ ] 检查新增测试文件是否编译成功
- [ ] 确认 `dev-dependencies` 正确添加
- [ ] 确认 `pub` 可见性（E2E 测试不可见 `pub(crate)`）
- [ ] commit 并 push：`git push --quiet 250`
- [ ] 创建 PR 并合入 `develop/v3.11.0`
- [ ] 更新 Issue #3420 评论，记录新覆盖率数据

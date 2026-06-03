# LOAD DATA LOCAL INFILE Bulk Loader — 设计规范

> **实现模式:** superpowers:writing-plans → superpowers:subagent-driven-development
>
> **目标:** 在 `crates/mysql-server` 实现完整 MySQL `LOAD DATA LOCAL INFILE` wire 协议,
> 配套 `MySqlTestClient::load_local_infile()` 客户端 API, 解决 issue #2948 Track 3 的
> "导入 6M 行 lineitem 走真 wire 协议需要数小时" 问题。
>
> **范围:** 仅做 Track 3 wire 协议 bulk loader, **不修 5 engine bug** (由 consolidation
> workstream 负责)。完成后 wire 协议具备 bulk-load 能力, issue #2953 的 fixture + #[ignore]
> test 仍是 #[ignore] 状态, 等 5 bug 修复后 un-ignore。

## 架构概述

```
┌──────────────────┐         ┌────────────────────────────────┐
│ MySqlTestClient  │  TCP    │  crates/mysql-server/lib.rs    │
│                  │────────▶│  do_command_loop               │
│ load_local_infile│         │  ├─ COM_QUERY branch           │
│ (path: &Path)    │         │  │  └─ detect "LOAD DATA        │
│                  │◀────────│     LOCAL INFILE '...'"        │
│ read file        │ 0xFB    │  │     ├─ validate path         │
│ send chunks      │ packet  │  │     ├─ send 0xFB request     │
│ send empty       │────────▶│  │     └─ loop: read file pkts  │
│                  │ OK/ERR  │  │        └─ bulk INSERT        │
└──────────────────┘         └────────────────────────────────┘
                                       │
                                       ▼
                              WalStorage<FileStorage>
                              bulk_insert_buffer_size
                              (default 1MB, 16MB max packet)
```

## 修改的文件

| # | 文件 | 改动 |
|---|------|------|
| 1 | `crates/mysql-server/src/packet_type.rs` | 新增 `LOCAL_INFILE_REQUEST: u8 = 0xFB` |
| 2 | `crates/mysql-server/src/lib.rs` | `do_command_loop` `COM_QUERY` 分支前检测 LOCAL INFILE 关键字, 路由到 `handle_load_local_infile` |
| 3 | `crates/mysql-server/src/lib.rs` | 新增 `fn handle_load_local_infile<S: Read + Write>(stream, engine, path, table_name, delim, batch_size, seq, cap) -> MySqlResult<u64>` |
| 4 | `crates/mysql-server/src/load_data.rs` (新文件) | 解析 .tbl 内容 + batched insert 逻辑 (~100 行) |
| 5 | `tests/common/mod.rs` | `MySqlTestClient::load_local_infile(path: &Path, table: &str) -> wire_err::Result<u64>` |
| 6 | `tests/load_local_infile_test.rs` (新文件) | 5 个测试覆盖 basic / SF=0.1 / 路径白名单 / 客户端拒绝 |
| 7 | `docs/audit/status/2026-06-04-tpch-phase2d-status.md` | 新增章节 "Track 3 progress" |
| 8 | `openspec/changes/mysql-server-canonical-entry/specs/wire-protocol-execution/spec.md` | 新增 §"Bulk loader: LOAD DATA LOCAL INFILE" |
| 9 | `crates/mysql-server/src/lib.rs` `EphemeralConfig` | 新增字段 `bulk_insert_buffer_size: usize` (默认 1_048_576) |

## 协议流程 (MySQL 兼容)

按 https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_text-protocol.html:

1. **客户端 → 服务端**: `COM_QUERY` 包含 SQL 文本
   ```
   LOAD DATA LOCAL INFILE '/abs/path/to.tbl' INTO TABLE t1
   FIELDS TERMINATED BY '|'
   ```
2. **服务端**:
   - 解析 SQL, 识别 `LOAD DATA LOCAL INFILE` 关键字
   - 验证路径在 `EphemeralConfig.data_dir` 白名单下 (canonicalize + starts_with)
   - 提取表名、分隔符
3. **服务端 → 客户端**: 0xFB packet, payload = 文件路径 (截断 SQL 中的 path 参数)
4. **客户端响应循环** (读 `MySqlTestClient::load_local_infile`):
   - 文件内容 packet (≤ 16 MB per packet, 截断到 16 MB 边界)
   - 文件结束, 客户端发**空 packet** (payload length = 0)
   - 客户端拒绝, 立即发空 packet (无需读文件)
5. **服务端**: 累计 content 字节, 按 `bulk_insert_buffer_size` 切分 (默认 1 MB), 调用
   `storage.insert(table, batch)`, 解析 `|` 分隔, 类型推断 (`i64`/`f64`/`Text`/`Null`)
6. **服务端 → 客户端**: 最终 `OK packet` (affected_rows = total rows, last_insert_id = 0,
   status_flags = 0x0002)
7. **失败 → ERR packet** (code 1146 file not found, 1064 syntax error, 1160 write error,
   0 client refused)

## 关键函数签名

```rust
// crates/mysql-server/src/load_data.rs
pub fn parse_tbl_line(line: &str, columns: usize) -> Result<Vec<SqlValue>, LoadDataError> {
    // 复用 tests/tpch_gate_test.rs:155-216 load_tbl_file 内的解析逻辑
    // 字段: split('|'), 空字段 → Null, 整数 → i64, 浮点 → f64, 其它 → Text
}

pub fn bulk_insert(
    engine: &mut ExecutionEngine<WalStorage<FileStorage, FileBackedWalManager>>,
    table: &str,
    rows: Vec<Vec<SqlValue>>,
) -> MySqlResult<u64> {
    // 决策: 走 engine.execute("INSERT INTO <table> VALUES (..), (..), ..")
    // 一次插入 N 行 (N=batch size, 默认 1000 rows 或 1MB whichever first)
    // 不绕过 parser — 保留 SQL path 与生产 INSERT 行为一致
    // 返回 affected_rows
}
```

```rust
// crates/mysql-server/src/lib.rs (do_command_loop 内新增分支)
packet_type::COM_QUERY => {
    let q = String::from_utf8_lossy(payload).trim_end_matches('\0').trim().to_string();
    if let Some((path, table, delim)) = parse_load_local_infile_sql(&q) {
        seq = handle_load_local_infile(
            stream, &mut engine.write().unwrap(), path, table, delim,
            self.bulk_insert_buffer_size, seq, cap,
        )?;
        continue;
    }
    // 原有 COM_QUERY 处理...
}
```

```rust
// tests/common/mod.rs (MySqlTestClient 新增方法)
pub fn load_local_infile(&mut self, path: &Path, table: &str) -> wire_err::Result<u64> {
    // 1. 读 path 文件
    // 2. 发送 COM_QUERY "LOAD DATA LOCAL INFILE '<path>' INTO TABLE <table>"
    // 3. 接收 0xFB packet
    // 4. 分块发送文件内容 (≤ 16 MB/packet)
    // 5. 发空 packet 标记结束
    // 6. 接收 OK 或 ERR packet
    // 7. 返回 affected_rows
}
```

## 错误处理 + 安全

| 失败模式 | 处理 | 错误码 |
|---|---|---|
| 客户端拒绝 (发空 packet) | 服务端立即返回 ERR | 0 (generic) |
| 文件不存在 | 服务端在发 0xFB 之前检查 | 1146 |
| 路径越界 (不在 data_dir 下) | canonicalize + starts_with 检查 | 1146 "not in allowed data dir" |
| `bulk_insert_buffer_size` ≤ 0 | fallback 到 1 MB | (warning log) |
| 批次 INSERT 类型不匹配 (e.g. TEXT 写到 INT 列) | 继续下一批次 (best-effort, v3.8.0 无 transaction rollback), 日志记录失败批次 | 1064 (含累计成功/失败计数) |
| 客户端中途断开 | accept loop 检测 EOF, 退出 | (无 packet) |
| 字段分隔符 / 转义符 | 只支持 `FIELDS TERMINATED BY '\|' LINES TERMINATED BY '\n'` (TPC-H .tbl 标准), 其它返回 1064 | 1064 |

**白名单安全** (强制):
```rust
let canonical_path = std::fs::canonicalize(path)?;
let canonical_data_dir = std::fs::canonicalize(data_dir)?;
if !canonical_path.starts_with(&canonical_data_dir) {
    return Err(MySqlError::FileNotAllowed);
}
```

## 测试覆盖

`tests/load_local_infile_test.rs` (5 个测试):

1. `test_load_local_infile_basic` — 创建 1 行 .tbl (5 字段), LOAD, 断言行数 = 1
2. `test_load_local_infile_full_tpch_sf01_region` — `tpch_data_gen` 生成 SF=0.1 region.tbl (5 行), LOAD, 断言 COUNT = 5
3. `test_load_local_infile_full_tpch_sf01_nation` — SF=0.1 nation.tbl (25 行), LOAD, 断言 COUNT = 25
4. `test_load_local_infile_path_outside_data_dir` — 路径 `/etc/passwd` → 1146 ERR
5. `test_load_local_infile_client_refuses` — 自定义 client 发空 packet → ERR code=0

**回归验证**:
- `cargo test --tests` 必须保持 PR #2951 status report 中列出的 38/38 wire-protocol tests GREEN
- 新测试不依赖 5 engine bug 修复 (只测 LOAD DATA, 不跑 TPC-H queries)
- 端到端 perf: `tpch_data_gen` 生成 SF=0.1 lineitem (6 万行) + LOAD DATA + 计时, 输出
  `benchmarks/results/sf01/load_infile.json`

## 文档更新

- `docs/audit/status/2026-06-04-tpch-phase2d-status.md` 新增章节 "Track 3 progress —
  LOAD DATA LOCAL INFILE: server-side handler + test client landed"
- `openspec/changes/mysql-server-canonical-entry/specs/wire-protocol-execution/spec.md`
  新增 §"Bulk loader: LOAD DATA LOCAL INFILE" (与 issue #2948 实现联动)

## 范围边界 (Out of Scope)

以下**不**在本 spec 范围, 由其它 PR 处理:

1. **5 engine bug 修复** (consolidation workstream):
   - TEXT compare, comma-join, SUM/AVG real, SELECT projection 已修
   - 剩余 #1 #2 #4 #5 仍未修, 由 consolidation 处理
2. **SF=0.1 fixture + expected JSON** (issue #2953): 由其它 agent 处理, 本 spec 不生成 fixture
3. **TPC-H query value comparison**: 等 5 bug 修复后才能写
4. **Q22 SUBSTRING (PostgreSQL 语法)**: parser 扩展, 单独 PR

## 风险评估

**Low** — 纯增量 wire 协议功能, 不改 COM_QUERY 既有路径, 不影响 32/38 既有 GREEN 测试。
白名单安全检查是设计强制项, 不会引入任意文件读取漏洞。

## 实现工作量估算

| 任务 | 行数估算 |
|---|---|
| `packet_type.rs` 0xFB 常量 | ~5 |
| `do_command_loop` LOCAL INFILE 检测分支 | ~30 |
| `handle_load_local_infile` 函数 | ~150 |
| `load_data.rs` (parse_tbl_line + bulk_insert) | ~100 |
| `MySqlTestClient::load_local_infile` | ~100 |
| `tests/load_local_infile_test.rs` | ~200 |
| 文档更新 | ~50 |
| **总计** | **~635 行** (~2-3 天工作量) |

## 验收标准 (Acceptance Criteria)

- [ ] `cargo build -p sqlrustgo -p sqlrustgo-mysql-server --all-features` 通过
- [ ] `cargo fmt --check --all` 通过
- [ ] `cargo clippy -p sqlrustgo -p sqlrustgo-mysql-server --all-features -- -D warnings` 通过
- [ ] `bash scripts/gate/check_docs_links.sh` 通过
- [ ] `cargo test --test load_local_infile_test` 5/5 GREEN
- [ ] `cargo test --tests` 38/38 既有 wire-protocol tests + 5/5 新 tests 仍 GREEN
- [ ] SF=0.1 region.tbl (5 行) + nation.tbl (25 行) LOAD 时间 < 1s
- [ ] 路径白名单测试: `/etc/passwd` → ERR 1146
- [ ] 客户端拒绝测试: 发空 packet → ERR code=0
- [ ] PR 标题: `feat(mysql-server): LOAD DATA LOCAL INFILE bulk loader (issue #2948 Track 3)`
- [ ] PR 描述引用 issue #2948, 包含 `cargo test --tests` 输出
- [ ] PR merge 后 Gitea develop/v3.8.0 同步, 本地 worktree 清理

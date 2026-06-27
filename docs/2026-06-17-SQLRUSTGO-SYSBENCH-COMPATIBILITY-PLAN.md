# SQLRustGo-MySQL Server 兼容性整改计划

**目标**：支持 sysbench 和 MariaDB/mysql CLI 标准客户端  
**版本**：v3.9.0 → v3.9.1（修复版本）  
**优先级**：P0（阻塞 GA Gate G7）

---

## 问题汇总

| # | 问题 | 影响范围 | 优先级 |
|---|------|----------|--------|
| G1 | Binary protocol row encoding 错误 | sysbench 所有测试 | P0 |
| G2 | `SET NAMES` / `SET NAMES utf8mb4` 不支持 | mysql-connector-python, pymysql | P0 |
| G3 | `COM_STMT_PREPARE` → `COM_STMT_EXECUTE` 状态机 | sysbench prepared statements | P0 |
| G4 | `LOAD DATA LOCAL INFILE` 不支持 | TPC-H fixture loading | P1 |
| G5 | `INSERT` 数据未持久化到 storage | 数据写入验证 | P1 |
| G6 | 某些文本查询返回空结果（协议序列问题） | 跨客户端稳定性 | P1 |

---

## G1: Binary Protocol Row Encoding（sysbench 报 ERROR 2027）

### 根因

`sender_result_set()` (lib.rs:~894) 使用 `write_text_row()` 对 binary protocol 查询进行编码，但 sysbench binary protocol 期望 `ColumnDefinition` 后跟 binary-encoded rows。

**受影响场景**：sysbench 所有 `oltp_*` 测试在 `thread_init` 时调用 `COM_STMT_PREPARE` → `COM_STMT_EXECUTE`，server 返回的 binary rows 用 text encoding 编码，client 解析失败。

### 修复方案

**方案 A：区分 text vs binary 执行路径（推荐）**

```rust
// lib.rs - ComStmtExecute handler (~line 1200)
// 在 execute_statement 中，根据 statement 类型选择编码器

if is_binary_protocol_stmt(stmt_id) {
    send_binary_result_set(rows, columns, seq, w)?;
} else {
    send_result_set(rows, columns, seq, w)?;  // text encoding
}
```

**需要实现**：
1. `send_binary_result_set()` — 使用 `write_binary_row()` 对每个值进行 binary encoding
   - `INT` → 4-byte little-endian
   - `BIGINT` → 8-byte little-endian
   - `VARCHAR/CHAR` → lenenc_string (1-3 byte length + data)
   - `FLOAT` → 4-byte IEEE 754
   - `DOUBLE` → 8-byte IEEE 754
   - `NULL` → 0xFB marker
2. 在 `PreparedStatementManager` 中标记每个 stmt 是否为 binary protocol
3. sysbench 的 `COM_STMT_EXECUTE` 带 `new_params_bound_flag=0` 时，使用 prepared statement 注册时的类型推断（`param_types` 字段）

**参考实现**：
- `crates/mysql-server/src/lib.rs:961-973` — `PreparedStatementInfo` 已有 `param_types`
- `crates/mysql-server/src/lib.rs:894-958` — 现有 `send_result_set()` 作为 text encoding 模板

**验证**：
```bash
sysbench oltp_read_write --db-driver=mysql --mysql-host=127.0.0.1 \
  --mysql-port=3306 --mysql-user=root --table-size=100 --tables=1 \
  --threads=2 --time=10 prepare
sysbench oltp_read_write --db-driver=mysql --mysql-host=127.0.0.1 \
  --mysql-port=3306 --mysql-user=root --table-size=100 --tables=1 \
  --threads=2 --time=10 run
# 期望：0 errors, TPS > 10
```

---

## G2: SET NAMES 支持

### 根因

`pymysql` 和 `mysql-connector-python` 在连接握手后自动发送 `SET NAMES utf8`（或 `utf8mb4`）。Parser 将 `NAMES` 识别为 keyword 但不识别其为 session variable set 命令。

**受影响场景**：所有使用 mysql-connector-python 或 pymysql 的 Python 客户端。

### 修复方案

**方案 A：识别 `SET <var> = <value>` 语法（minimal）**

```rust
// parser 对 SET NAMES 的特殊处理
// 在解析 SET 命令时，识别 "SET NAMES" 模式并返回 OK packet 而不执行

if lower_sql.starts_with("set names") {
    return Ok(Resp::Ok);  // NOP for charset purposes
}
```

**方案 B：完整支持 `SET` variable 命令（推荐，长期正确）**

```rust
// 支持的 session variables：
// - `SET NAMES <charset>` → 设置连接字符集（实现为 NOP，当前 charset=None）
// - `SET character_set_results = <charset>` → NOP
// - `SET autocommit = 0|1` → 实现 autocommit 状态机
// - `SET SESSION TRANSACTION ISOLATION LEVEL ...` → NOP

// 返回 OK packet { affected_rows=0, last_insert_id=0, status=0 }
```

**验证**：
```python
import pymysql
conn = pymysql.connect(host='127.0.0.1', port=3306, user='root')
cursor = conn.cursor()
cursor.execute("SELECT 1")
print(cursor.fetchone())  # 期望：(1,)
```

---

## G3: Prepared Statement 状态机（COM_STMT_*）

### 根因

`COM_STMT_PREPARE` 返回正确的 statement ID，但 `COM_STMT_EXECUTE` 的处理路径与 `COM_QUERY` 共用 `send_result_set()`（text encoding）。

**sysbench 的 protocol 流**：
1. `COM_STMT_PREPARE` (stmt="SELECT c FROM sbtest1 WHERE id=?") → `COM_STMT_PREPARE_OK` + column definitions
2. `COM_STMT_EXECUTE` (stmt_id, binary params) → **当前用 text encoding 响应** → `Malformed packet`

### 修复方案

```rust
// lib.rs: ComStmtExecute handler
fn handle_stmt_execute(&mut self, stmt_id: u32, params: &[u8]) -> Result<Response> {
    let stmt_info = self.stmt_manager.get(stmt_id)
        .ok_or_else(|| PacketError::UnknownStatement(stmt_id))?;

    // 解析 binary params（与 text params 不同编码）
    let parsed_params = decode_binary_params(params, &stmt_info.param_types);

    // 执行 SQL（用 parsed_params 替换 ? 占位符）
    let rows = self.execute_plan(&stmt_info.sql, parsed_params)?;

    // 返回 binary encoding 的结果
    let cap = self.get_capabilities();
    self.send_binary_result_set(rows, &stmt_info.param_types, cap)
}
```

**关键字段**（已有，无需新增）：
- `PreparedStatementInfo.param_types` — 每个 `?` 的 MySQL 类型码
- `PreparedStatementInfo.param_count` — 占位符数量

---

## G4: LOAD DATA LOCAL INFILE

### 根因

`LOAD DATA LOCAL INFILE` 命令返回语法错误。TPC-H 测试 harness 和 sysbench 的 `prepare` 阶段使用此命令加载数据。

**受影响场景**：
- `cargo test --test tpch_full_22_test`（当前 fixture loading 失败）
- sysbench prepare 阶段（如果测试数据不在 server 端）

### 修复方案

```rust
// lib.rs: LoadDataLocalInfile handler
fn handle_load_data_local(&mut self, filename: &str) -> Result<Resp> {
    let path = PathBuf::from(filename);

    // 读取客户端本地的文件（通过 protocol）
    // MySQL protocol: server 发送 {0xFB, filename} 来告诉客户端发送文件内容
    let data = self.read_local_infile_data()?;

    // 解析 CSV/tbl 格式
    let rows = parse_tbl_file(&data)?;

    // INSERT INTO target_table
    for row in rows {
        self.execute_insert(&self.current_database, &row)?;
    }

    Ok(Resp::Ok)
}
```

**临时 workaround**：
- 对于 TPC-H fixture loading：使用 `mysql -e "LOAD DATA LOCAL INFILE ..."` 而非 Rust client
- 对于 sysbench prepare：`--table-size=100 --tables=1` 小规模数据可以通过 INSERT VALUES 手动构造

**验证**：
```bash
mysql -h 127.0.0.1 -P 3306 -u root -e \
  "LOAD DATA LOCAL INFILE '/path/to/region.tbl' INTO TABLE region" sbtest
# 期望：Query OK, 5 rows affected
```

---

## G5: INSERT 数据持久化

### 根因

`INSERT` 命令返回成功（OK packet），但数据未写入 JSON storage（`sbtest.json` rows=[]）。WAL 有记录但 storage flush 未实现。

### 修复方案

**优先级**：P1（不影响 sysbench read-only 测试，但影响真实 CRUD soak）

```rust
// storage 层检查点：
// 1. WAL 写入成功（已验证 ✓）
// 2. Storage flush 未调用

// 在 INSERT handler 末尾添加：
self.storage.flush()?;  // 触发 JsonStorage::flush 将 buffer 写入磁盘

// 或批量 flush（性能优化）：
if self.wal_size > 1000 {
    self.storage.flush()?;
}
```

**验证**：
```sql
CREATE TABLE t1 (id INT, v TEXT);
INSERT INTO t1 VALUES (1, 'hello');
-- 重启 server --
SELECT * FROM t1;
-- 期望：1 | hello
```

---

## G6: 协议序列号处理

### 根因

MySQL wire protocol 每个 packet 都有 sequence ID（0-255 循环）。`send_result_set()` 的 sequence 维护在 `lib.rs:917` 附近，但某些错误路径可能产生序列不连续。

**症状**：某些 `mysql` CLI 查询返回空结果（而非错误）

### 修复方案

```rust
// 全局 sequence 追踪
struct ConnectionState {
    seq: u8,
    capabilities: u32,
}

// 每个 write_to 后递增
seq = seq.wrapping_add(1);

// 错误时：发送 error packet 后重置 sequence
fn write_error(&mut self, code: u16, msg: &str) -> Result<()> {
    make_error_packet(self.seq, code, msg).write_to(self.writer)?;
    self.seq = self.seq.wrapping_add(1);
    Ok(())
}
```

---

## 整改执行计划

### Phase 1: G1 + G3（使 sysbench 可用）— 1-2 天
1. 实现 `send_binary_result_set()`
2. 在 `ComStmtExecute` handler 中路由到 binary encoding
3. 实现 `decode_binary_params()` 解析 sysbench 的 binary param encoding
4. 验证：`sysbench oltp_read_write --time=60 run` → 0 errors, TPS > 10

### Phase 2: G2（使 Python 客户端可用）— 0.5 天
1. 识别 `SET NAMES` 并返回 OK（NOP）
2. 或完整实现 `SET variable = value` 支持
3. 验证：pymysql `conn.query("SELECT 1")` → (1,)

### Phase 3: G4（LOAD DATA LOCAL INFILE）— 1 天
1. 实现 `LOAD DATA LOCAL INFILE` handler
2. TPC-H fixture loading 修复
3. 验证：`cargo test --test tpch_full_22_test` → PASS

### Phase 4: G5（G1+G3+G2 完成后）— 0.5 天
1. INSERT storage flush
2. CRUD soak 验证

### Phase 5: G6（可选，性能稳定后）— 0.5 天
1. 全局 sequence 审计
2. 边界 case 测试

---

## 验证矩阵

| Test | Before | After |
|------|--------|-------|
| `sysbench oltp_read_write --time=60` | ERROR 2027 | 0 errors, TPS > 10 |
| `sysbench oltp_read_only --time=60` | ERROR 2027 | 0 errors |
| `sysbench oltp_point_select --time=60` | ERROR 2027 | 0 errors |
| `pymysql` CRUD loop | SET NAMES error | 0 errors |
| `mysql-connector-python` CRUD | SET NAMES error | 0 errors |
| `cargo test --test tpch_full_22_test` | fixture missing | 22/22 PASS |
| `INSERT; SELECT *;` after restart | empty | data present |

---

## 风险与依赖

- **G1+G3 是核心**：需要理解 MySQL binary protocol encoding。参考 `MySQL 5.7 Protocol` 文档。
- **测试验证**：需要 Z6G4 环境来运行 sysbench full test（本地 Mac Mini 资源不足）
- **向后兼容**：text protocol (`COM_QUERY`) 必须继续工作，不能因 binary fix 而破坏

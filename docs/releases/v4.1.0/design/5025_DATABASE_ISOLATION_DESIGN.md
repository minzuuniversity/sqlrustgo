# #5025 方案设计：多数据库表空间隔离

**状态：设计文档，本轮不改代码。** 目的是把根因链与两条实现路径的完整代价查清，
供后续独立 PR 决策。

## 一、实测复现

```rust
x.execute("CREATE DATABASE d1"); x.execute("CREATE DATABASE d2");
x.execute("CREATE TABLE t1 ..."); x.execute("INSERT INTO t1 VALUES (1)");
x.execute("CREATE TABLE t2 ..."); x.execute("INSERT INTO t2 VALUES (2)");
x.execute("USE d1");                          // Ok(0) —— no-op
x.execute("SHOW TABLES");                    // [[t2], [t1]]
x.execute("SHOW TABLES FROM d2");            // [[t2], [t1]]   ← 与上面完全相同
x.execute("SELECT COUNT(*) FROM t2");        // [[1]]         ← d2 的表在 d1 上下文可查
```

三条合起来证明：**d1 与 d2 共用同一个表命名空间，表不按库归属。**

（`documents` / `vectors` / `content` 是每库都有的系统表，与库隔离无关。）

## 二、根因链（四层，逐层核实）

### 1. `USE` 是显式 no-op

`src/execution_engine_methods.rs:781`

```rust
pub(super) fn execute_use_database(&self, _db: &str) -> SqlResult<ExecutorResult> {
    // v3.9.0 single-database: USE <database> is accepted for MySQL wire
    // compatibility but is a no-op. v3.10 multi-database mode will switch
    // the active database context.
    Ok(ExecutorResult::empty())
}
```

参数名带下划线 —— 从未被使用。这是**有意为之**（注释写明 v3.9 单库模式），
但 v3.10 的「multi-database mode」从未实现。

### 2. 库上下文当前无处可存（设计约束，非疏漏）

`execute_use_database(&self, ...)` 收 `&self`，而执行引擎需要跨语句保存
「当前库」。`ExecutionEngine` 结构体（`src/execution_engine.rs:80`）目前字段：

```rust
pub struct ExecutionEngine<S: StorageEngine> {
    pub(crate) storage: Arc<RwLock<S>>,
    pub(crate) catalog: Option<Arc<RwLock<Catalog>>>,
    pub(crate) stats: Arc<RwLock<ExecutionStats>>,
    pub(crate) cbo_enabled: AtomicBool,          // 内部可变性
    pub(crate) transaction_manager: Arc<Mutex<TransactionManager>>,
    pub(crate) tx_session: Arc<Mutex<TxSession>>,
    ...
}
```

**好消息**：#4910 已经为 `cbo_enabled` / `tx_session` 建立了内部可变性模式，
库上下文可以照抄（`Arc<Mutex<String>>` 或 `RwLock<String>`）。这不是障碍。

**约束**：`MySQL` 协议的 `USE` 是**连接级**语义，不同连接应有不同当前库。
`lib.rs:3112` 目前硬写空 schema（`write_lenenc_string(&mut p, b"")`），
即 `mysql-server` **每连接无会话状态**。这意味着：

- 若把当前库存在 `ExecutionEngine`（`Arc<RwLock<...>>`）里，**所有连接共享**
  —— `client1: USE d1` 会影响 `client2`。对 MySQL 客户端是错误语义。
- 正确做法是存在**连接上下文**里，由 `handle_connection`（`lib.rs:6213`）
  持有并逐语句下传。这是本 issue 最大的工作量，**不是改 `StorageEngine`**。

### 3. `MemoryStorage` 的表键不带库前缀

```rust
fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
    self.databases.insert(db_name.to_string());   // 只记名字
    Ok(())
}
fn list_databases(&self) -> SqlResult<Vec<String>> {
    Ok(self.databases.iter().cloned().collect())
}
```

`databases: HashSet<String>` 存在，但 `tables: HashMap<String, ...>` 的键是
**裸表名**（`get_table_info` 用 `table.to_lowercase()` 查），与 `databases` 毫无关联。

### 4. `FileStorage` 的表文件落在 `data_dir` 根，库子目录从不被用

```rust
fn create_database(&mut self, db_name: &str) -> SqlResult<()> {
    let db_path = self.data_dir.join(db_name);
    std::fs::create_dir_all(&db_path)?;          // 建了子目录
    Ok(())
}
fn table_path(&self, table_name: &str) -> PathBuf {
    self.data_dir.join(format!("{}.json", table_name))   // 但表文件在根
}
```

`create_database` 建的子目录**没有任何代码读取**。实测本仓现状：

```
data/soak-final/   content.json  documents.json  sqlrustgo.wal  t1.json  vectors.json
data/soak-fixed/   content.json  documents.json  sqlrustgo.wal  t1.json  t2.json  vectors.json
```

表文件全在 `data_dir` 根；库子目录为空。`find data -maxdepth 2 -name "*.json" -path "*/*/*"`
无结果，确认从未按库组织。

## 三、两条实现路径

### 路径 A：表名加库前缀（`d1.t1`）

引擎层把当前库内的裸表名改写为 `{db}.{table}`，再交给 `StorageEngine`。

| 项 | 评估 |
|---|---|
| `StorageEngine` trait | **零改动**（19 个实现、69 个调用点全部不动） |
| `MemoryStorage` | `tables` 键改为 `format!("{db}.{table}")`；`has_table`/`get_table_info`/`drop_table` 已是 `to_lowercase()` 查，前缀一并小写即可 |
| `FileStorage` | `table_path("d1.t1")` → `data_dir/d1.t1.json`。**表名含 `.` 在多数文件系统合法**，但 `table_path` 只加 `.json` 后缀，落盘变成 `data_dir/d1.t1.json` 而非 `data_dir/d1/t1.json` —— 若要落进库子目录仍需改 `table_path` |
| `SHOW TABLES` | 需反向拆前缀；且 `d1.t1` 与用户建的 `t`（无库）会共存，需定义「无库前缀」的归属规则 |
| 错误信息 | `Table not found: d1.t1` 泄漏内部前缀，与用户写的 `t1` 不一致 |
| 现有磁盘数据 | 键从 `t1.json` 变 `default.t1.json` → **旧数据全部不可见**，需迁移或双路径回退 |
| 跨库查询 | `SELECT * FROM d2.t2` 在 d1 上下文可执行 —— 符合 MySQL 语义，但需确认这是想要的 |

**最大的坑**：`table_path` 不改的话，表文件变成 `d1.t1.json` 堆在根目录 —— 库里
依然分不开，只是名字里带了前缀。**这条路必须同时改 `table_path` 才能真正隔离**，
届时「trait 零改动」这个优势就打了折扣。

### 路径 B：表文件改存到各库子目录

不改表名，只改路径拼接：`data_dir/{db}/{table}.json`。

| 项 | 评估 |
|---|---|
| `StorageEngine` trait | **仍零改动** —— 库上下文在 `FileStorage` 内部（如 `current_db: RwLock<String>`）自持 |
| `FileStorage` 改动面 | **18 处 `self.data_dir`**，其中表路径仅 2 处：`table_path`（`:358`）与 delta 路径。索引路径（`:364`）另 1 处。**实际改动 < 10 行** |
| `MemoryStorage` | `tables` 键改为 `format!("{db}\u{1}{table}")`（用不可见分隔符避免与库名含 `.` 冲突），或加一个 `table_db: HashMap<String, String>` 记录归属 |
| `SHOW TABLES` | **不需拆前缀** —— 直接过滤 `table_db` 里属于当前库的表 |
| 错误信息 | 保持裸表名，与用户输入一致 |
| 现有磁盘数据 | 表文件从 `data_dir/t1.json` 移到 `data_dir/default/t1.json` → **需迁移或回退** |
| 库上下文来源 | `StorageEngine` trait 无处传参。可在 `FileStorage` 内置 `current_db`，但 `USE` 需通过 trait 写它 → **要么 trait 加一个 `set_current_db` 方法（局部 breaking，1 个方法而非 69 个调用点）**，要么把 `use_database` 做成 `Inherent` 方法 |

**关键洞察**：`StorageEngine` trait 的 88 处改动（19 实现 + 69 调用点）**在两条路径下都不需要**。
本 issue 描述的「breaking change」规模被高估了 —— 真正的 breaking 面是：

1. `StorageEngine` 加一个 `set_current_db(&str)` / `current_db()` 方法（+2 个 trait 方法）
2. `FileStorage` / `MemoryStorage` 各自实现（内部改动）
3. **`mysql-server` 的连接级会话状态**（`handle_connection` 持有，逐语句下传）—— 这是最大工作量
4. 磁盘数据迁移

## 四、必须先决策的两个问题

### Q1：当前库存在哪一层？

- **连接级（正确语义）**：`mysql-server` 的 `handle_connection` 持有，逐语句下传到引擎
- **引擎级（错误但省事）**：存在 `ExecutionEngine` 的 `Arc<RwLock<String>>` —— 所有连接共享

**建议连接级**。但这要求 `execute(&self, sql)` 的签名或引擎 API 能接收「当前库」上下文，
是本次改造的主要接口变更。

### Q2：旧数据怎么办？

现有表文件在 `data_dir` 根。三个选项：

- **自动迁移**：启动时把 `data_dir/*.json` 移入 `data_dir/default/`（一次性，带日志）
- **双路径回退**：`table_path` 先查新位置，回退旧位置（读兼容，不写兼容）
- **不兼容**：明确 breaking，要求用户自行迁移（仅适用于 pre-1.0 的内部数据库）

**建议自动迁移 + 保留回退读取**，避免静默丢数据。

## 五、已知的连带影响

1. `tests/e2e/multi_db_e2e_test.rs` 文件头已注明「USE 仅作 no-op，因为 table
   lookup 路径尚未按数据库隔离」—— 改造后该注释与测试断言都要改。
2. 系统表（`documents` / `vectors` / `content`）当前每库都有一份。隔离后是否
   改为全局共享，需明确定义 —— 否则会出现「库 A 看不到自己的 documents」。
3. `SHOW TABLES FROM d2` 已有解析与执行路径（`src/engine_ddl.rs:337`、
   `:386`、`:472`，V312-58 / #4516 起支持 `FROM db` / `LIKE` 形式），
   但当前是在**全局表列表**上忽略 `FROM` 参数 —— 这正是实测中它与
   `USE d1; SHOW TABLES` 输出完全相同的原因。改造后需真正按库过滤。
4. 事务：`BEGIN` 后 `USE` 换库是否允许？MySQL 允许，但会切换事务的表空间。
   需要明确行为并测试。
5. 本 issue 明确反对「只改 `SHOW TABLES`」的做法（会制造「元数据视图与数据
   可见性不一致」的假象）。**这个判断是对的，改造必须同时落在数据可见性上。**

## 六、工作量估计

| 阶段 | 内容 | 相对代价 |
|---|---|---|
| 1 | `StorageEngine` 加 `set_current_db` / `current_db`（+2 方法） | 小 |
| 2 | `MemoryStorage` 表键带库 | 小 |
| 3 | `FileStorage` 路径拼接 + 迁移/回退 | 中（< 10 行 + 迁移逻辑） |
| 4 | `mysql-server` 连接级会话状态 | **大**（`handle_connection` 逐语句下传） |
| 5 | e2e 测试改写 + 新增隔离测试 | 中 |
| 6 | Q1/Q2 的决策落地 | 视决策而定 |

**不应作为一个 PR 完成。** 建议至少拆成「存储层隔离」与「连接级上下文」两个 PR，
后者可以独立验证（连两个客户端，`USE` 不同库，互不干扰）。

## 七、验收标准（供后续 PR 参考）

1. `USE d1; SHOW TABLES` 只列 d1 的表；`USE d2; SHOW TABLES` 只列 d2 的
2. 两个 MySQL 连接分别 `USE` 不同库，互不影响（**这才是「表空间隔离」的
   完整含义**，仅第 1 条不足以证明）
3. `USE d1; SELECT * FROM t2` 报 `Table not found`
4. `SELECT * FROM d2.t2` 在 d1 上下文可执行（若决定支持跨库限定名）
5. 升级后旧数据目录仍可读，日志明确说明迁移

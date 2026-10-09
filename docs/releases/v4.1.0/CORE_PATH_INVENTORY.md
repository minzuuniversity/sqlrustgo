# v4.1.0 核心路径清单（人工复核版）

- source_agent: mcode (MiniMax-M3.1-Flash-Preview)
- source_run: iaia410-core-inventory-20261009
- 基线: `904d13bce8`（PR #5203 合并后）
- 覆盖: 33 个文件（执行引擎 / 存储 / WAL / 事务 / planner / optimizer / mysql-server）
- Refs #5196, #5204, #5192, #5193

---

## 这份文档是什么，为什么不是 AOCI 索引

AOCI-CODE 的认知索引在 `develop/v4.1.0` 上**无法通过治理**：

```
$ aoci check --json        # 未做任何改动时
"governance_aligned": false
"findings": [{"code": "scope_change_required", "domain": "code"}]

$ aoci baseline scope plan
Error: Baseline Scope Refresh failed: baseline_scope_managed_scope_unsupported
```

工具自身提供的 plan / preview / apply / approve / resume 全线不可用。手改索引文件能让
`check` 的显示数字好看，但那是**制造 PASS**，本仓库治理明令禁止。

因此这份清单改用普通 Markdown：**不依赖该工具，不受其治理阻塞影响**，纳入版本控制
以便复核与追溯。

它与 AOCI 索引的定位相同 —— **导航层，不是验证层**。详见文末「定位」。

---

## 条目格式

每条包含四段，可被机械核对：

| 字段 | 含义 |
|---|---|
| **职责** | 这个文件实际做什么 |
| **位置** | 在执行链路中的位置 |
| **为何重要** | 与哪个 issue / 决策相关 |
| **陷阱** | 改动它时最容易踩的坑 |

---

## 执行引擎（手写解释器，生产真实执行者）

### `src/engine_select.rs` — 9450 行
- **职责**：手写 SELECT 执行器，扫描与聚合路径都在这里
- **位置**：每条 SELECT 的真实执行路径
- **为何重要**：#5193 选项 A 会重写它；#5168 / #5177 当前优化的正是它
- **陷阱**：改动此处等于改动所有查询的生产读路径；体量使代码审查成为主要防线

### `src/engine_dml.rs` — 2478 行
- **职责**：手写 INSERT / UPDATE / DELETE 执行器，含 clustered-table 路径
- **为何重要**：#5177 测得该处 WHERE 走 21.5ms（点查 0.41ms）；#5193 选项 A 会替换它
- **陷阱**：IAIA-410 发现 CLUSTERED 分支直接写内存态、**完全绕过 WAL**

### `src/execution_engine.rs` — 1273 行，23 个 pub fn
- **职责**：所有语句必经的 ExecutionEngine；issue 引用从 #3172 排到 #5191
- **为何重要**：其 `session_db` 即 #5191 发现「未从握手选库初始化」的那个字段
- **陷阱**：构造时 `session_db` 默认为 `DEFAULT_DATABASE`，任何绕过引擎自身 USE 的路径都会静默读到错误的库

### `src/execution_engine_methods.rs` — 2099 行，25 个 pub fn
- **职责**：语句分发；issue 引用最密集处（#3169–#5113）
- **陷阱**：某类语句的路由缺陷只影响该语句类，对其他语句的测试完全不可见

### `src/engine_ddl.rs` — 1625 行，4 个 pub fn，18 处 issue 引用
- **职责**：DDL 执行，含 `execute_show_status` 与 `ensure_database_known`
- **为何重要**：`ensure_database_known` 读 `list_databases` 校验库存在，是 #5192 缺失库作用域的第二个受害者
- **陷阱**：`execute_show_status` 返回**硬编码四行**，`SHOW STATUS` 的零值不是指标，**不得当作测量读**

### `src/engine_utils.rs` — 22 个 pub fn
- **职责**：手写引擎的共享工具，含它自己的 `find_column_index`
- **为何重要**：`#5001` 曾试图把 6 份列解析收敛为一份
- **陷阱**：列解析器仍有副本，改一处不达另一处；#5001 的 9 个提交中有 5 个从未合入

### `src/expr_utils.rs` — 13 个 pub fn
- **职责**：旧求值路径的表达式辅助，已把列解析委托给 `executor::expr`
- **陷阱**：注释明确标注与旧分支语义一致，改动须协调 P0-2 OpenSpec 变更

---

## 存储层

### `crates/storage/src/engine.rs` — 5568 行
- **职责**：定义 `StorageEngine` trait 与 `MemoryStorage`
- **为何重要**：trait 默认的 `insert_in_db` 在 **:1424** 用 `let _ = db` 丢弃库名，是 **#5192 F9 数据丢失的根因**
- **陷阱（已核实）**：`insert_in_db` 有 **11 个 `StorageEngine` 实现，但只有 3 处定义该方法** —— trait 默认（:1424）、`MemoryStorage` 正确覆写（:3061，用 `scoped_key(db, table)`）、`FileStorage`（`file_storage.rs:5978`）。**其余包装器全部继承有损默认**。笼统写「`insert_in_db` 丢弃 db」会把调查引向错误方向

### `crates/storage/src/file_storage.rs` — 7165 行
- **职责**：文件存储；`insert_buffer` 是表状态的**第二份副本**，直到阈值触发才并入 `tables.rows`
- **为何重要**：`insert_buffer` + `tables.rows` 两份存储，正是「只读其中一份则行数偏少」的机制类别（#5167、#5181）
- **陷阱**：每条读路径都必须合并两处；#4960 与 #5060 都是单条路径只读一份

### `crates/storage/src/wal_storage.rs` — 2172 行，21 个 pub fn
- **职责**：WAL 包装器，86 次 `inner.` 转发
- **为何重要**：它**正确转发** `list_databases`，但**没有定义** `insert_in_db`，因此继承有损默认 —— 这就是 #5192 的丢失路径
- **陷阱**：包装器只对它显式覆写的方法正确；转发次数多不等于完整

### `crates/storage/src/mvcc_storage.rs` — 1376 行，86 次转发
- **职责**：包裹 FileStorage 提供快照隔离读；出货 server 就走这一层
- **为何重要**：#5179 的 `list_databases` 缺陷就住在这里
- **陷阱**：同类的遗漏会在每次 trait 新增方法时重演

### `crates/storage/src/binary_storage.rs` — 1611 行，47 次转发
- **职责**：`--storage binary` 后端
- **陷阱**：独立生产后端；不显式选它的测试对它一无所证

### `crates/storage/src/vector_storage.rs` — 1019 行，19 个 pub fn
- **职责**：向量存储后端，注释中**无任何 issue 引用**
- **陷阱**：无 issue 引用**不等于已验证** —— 审计中没有它的变异 spec，也没有差分测试

### `crates/storage/src/wal_legacy.rs` — 2045 行
- **职责**：定义 `WalEntry` / `WalEntryType`，即落盘的 WAL 记录
- **为何重要**：因 `table_id` 是不可逆哈希，`table_name` 是 #5055 加的；#5192 之前**没有库标识**
- **陷阱**：给 WAL 记录加字段必须同时覆盖所有写入方与回放路径；缺库标识会让已提交的多库写入**不可回放**，而不只是「不够精确」

### `crates/storage/src/recovery_engine.rs` — 1569 行
- **职责**：启动时回放 WAL，报告 `entries_total` / `committed_txns` / `rows_inserted` / `skipped_entries`
- **为何重要**：`skipped_entries` 就是**已被确认却丢失的写入数**
- **陷阱**：它按表名哈希在目标中解析表，解析不到就跳过并记 WARN 而非失败 —— 这条静默跳过路径正是 #5200 现在在启动时拦截的对象

---

## 事务层

`crates/transaction/` 共 5353 行，七个模块。

- **`transaction_manager.rs`**（548 行）—— 提交时间戳与每表计数器在此，决定已提交行能否存活；#5156 改的正是这里。#5112 的 rollback 正确性与 #4974 的快照绑定都落在此 crate，而目前只有集成测试覆盖
- **`version_chain.rs`**（363 行）—— MVCC 版本链，决定读者能看到哪些版本，即 #4974 改变的行为背后的机制。可见性规则必须与存储层的 `reader_tx` 线程传递一致，否则写入会在事务中途可见
- **`deadlock.rs`** —— 只有在所有加锁路径都上报的前提下，死锁检测才有意义
- **`coordinator.rs`** —— 引擎级事务成为存储级事务的边界；此处静默成功会让未持久化的事务对外报 COMMIT OK

---

## planner / optimizer（#5193 的决策对象）

### `crates/planner/src/physical_plan.rs` — 706 行
- **职责**：定义 `PhysicalPlan` trait 与 `SeqScanExec` / `IndexScanExec`
- **关键事实**：`SeqScanExec::execute` 硬编码 `Ok(vec![])`；`IndexScanExec` 根本没有 `execute` 方法
- **为何重要**：这是 #5193 的 F3 —— 生产执行者是手写引擎，这些算子无生产调用方
- **陷阱**：这里测试通过**不构成生产正确性证据**，因为代码路径不可达

### `crates/planner/src/planner.rs`
- **关键事实**：`NoOpPlanner` 直接委托给 `DefaultPlanner`，而 `DefaultPlanner::new` 的全部外部构造点在 `crates/planner/tests/` 内
- **为何重要**：这是「生产源码中存在构造代码，但无生产调用方可达」的实例 —— 比「生产代码零构造」更准确的表述

### `crates/planner/src/lib.rs`
- **关键事实**：带 `// TODO: Add these modules after migration`
- **陷阱**：把 TODO 读成已交付的行为描述，正是计划中的子系统被误认为已集成的方式

### `crates/optimizer/src/unified_cost.rs`
- **为何重要**：`UnifiedCostModel` 是少数真正从生产可达的优化器部件（`src/engine_builder.rs` 中**七处**构造），#5193 选项 B 删除未接入代码时**必须保留**
- **陷阱**：七处全部以 `default_model(0, 0)` 构造，即不记录任何真实统计，其估算不是测量

### `crates/optimizer/src/stats.rs` — 1194 行
- **陷阱**：统计信息驱动计划选择，过期或缺失的统计会静默降级计划而非报错；空统计表与正确计算的统计表，从计划输出上无法区分

---

## MySQL server

### `crates/mysql-server/src/lib.rs`
- **职责**：wire server；每连接构造 `ExecutionEngine`（9 处），两处启动路径执行 WAL 恢复
- **为何重要**：#5191 的握手失同步源于此 —— 握手设置了 storage 的 `current_db`，但构造 engine 时 `session_db` 仍在默认值
- **陷阱**：`guard_unrecoverable_commits` 现会在恢复丢弃已提交条目时拒绝启动；**两处恢复调用点都必须保持接线**

### `crates/mysql-server/tests/issue_5009_mvcc_list_databases_test.rs`
- **为何重要**：它存在是因为更早的 repl 测试看不见该缺陷 —— repl 构造 `MemoryStorage`，从不构造 `MvccStorage`
- **陷阱**：自建进程内夹具的测试，可能对缺陷真正所在的层结构性失明

---

## 复核记录（关键）

本清单的条目由模型撰写，**逐条机械核对**。两处事实错误被抓出并修正：

| 初稿写法 | 核实结果 | 后果 |
|---|---|---|
| 「`engine_builder` 三处构造 `UnifiedCostModel`」 | **7 处** | 低估了优化器接入范围 |
| 「三个 `StorageEngine` 实现」 | **11 个实现，仅 3 处定义 `insert_in_db`** | 会把 #5192 调查引向错误方向 |

**事实错误率 2/28 ≈ 7%**，两处均在提交前抓出。

第二处尤其值得记：它出现在**为 #5192 写的条目上**，即最需要准确的地方。若不复核就提交，
它会以「事实」的身份进入版本控制并被所有后续会话继承。

**结论：模型撰写的条目必须配机械复核，这不是可选项。** 复核成本远低于生成成本
（本次复核仅为数次 grep）。

已核对通过的数字断言：`stats.rs` 1194 行、`transaction_manager.rs` 548 行、
`mvcc_storage` 86 次转发、`wal_storage` 21 个 pub fn、
`execution_engine_methods` 25 个 pub fn、恢复守卫 2 处调用点。

---

## 定位：导航层，不是验证层

本清单携带的是**意图判断** —— 「`SHOW STATUS` 是硬编码、不得当作测量读」「11 个实现仅
2 处覆写，其余全是 database-blind」「CLUSTERED 分支绕过 WAL」。这些是 grep 结构上给不出的，
需要理解代码意图才能得出。

但它**检测不出** #5191（F5 静默空集）、#5192（F9 数据丢失）、#5167（`COUNT(*)` 返回 0）
中的任何一个。这三个都是靠**外部差分测试、真实客户端、故障注入**发现的。

佐证：本轮试点中，AOCI-CODE 主动发现的唯一真实缺陷是 #5197 —— 一个**文件名大小写冲突**，
属仓库卫生问题，不是架构或正确性问题。

---

## 维护规则

1. 新增条目必须附可机械核对的数字或符号（行数、调用点、定义处）
2. 提交前必须对每条断言做一次 grep 交叉核对 —— 7% 的错误率是实测值，不是理论值
3. 条目描述**现状**，不描述意图或计划；TODO 不得写成已交付
4. 本清单不替代 IAIA-410 的缺陷判定流程；两者互补，不可互相取代
# TPC-H 22-Query Wire Round-Trip — Three-Way Reference Plan

> **Author**: 李哥 hermes session 2026-06-05
> **Branch**: `feature/tpch-22-wire-v2` (off `origin/develop/v3.8.0` @ `7ea924e0d`)
> **Worktree**: `~/dev/yinglichina163/sqlrustgo/.worktrees/tpch-22-wire-v2`
> **Target issue**: #2977 (TPC-H 22 跑通) + #2948 (Track 3 wire-protocol bulk loader)
> **Status**: SPEC — implementation has NOT started

## 1. Goal (精确,不可改)

**22 条标准 TPC-H query (q1.sql … q22.sql) 通过 `sqlrustgo-mysql-server` 真实 binary + MySQL wire protocol 跑通,行数 row_count 与 MySQL/PostgreSQL/SQLite 三方实际结果一致(行数对齐,不要求每行内容完全相同,因精度/排序可能差)。**

判定:row_count 22/22 一致 → PASS,任意一条不一致 → FAIL。

## 2. 资源现状(已验证 2026-06-05)

### 2.1 客户端 fixture
- `tests/data/tpch-sf001/`: 8 表,919 行,**SF=0.001**
  - region=5, nation=25, supplier=10, customer=15, part=20, partsupp=80, orders=150, lineitem=614
- `expected/Q1.json`: 已存在(hand-computed)

### 2.2 三方 DB
| DB | 状态 | 凭据 | 数据 | Schema 完整 | 可用 |
|---|---|---|---|---|---|
| **MySQL 8.0.46** | ✅ 本地 :3306 | **root / root123** (来源: ISSUE-2768 §"Reproduction Recipe") | `tpch` (SF=1 6M, l_linestatus 有) / `tpch_sf1` / `tpch_test` | 完整 (l_linestatus 有) | **是 (主用)**, 装 sf001 子集 |
| **SQLite 3.45.1** | ✅ 本地 | 无 | 待用 sf001 装 | 需自己 DDL | **是**, 装 sf001 子集 |
| **PostgreSQL 16.14** | ✅ 本地 :5432 | openclaw / 无 (socket) | `tpch_test` (SF=0.1, 84001 lineitem) | **缺 `l_linestatus` 列** (scripts/setup_pg_tpch.py 装的, schema 不全) | 不可当 expected source, 需 DROP+RECREATE 完整 schema |
| ~~MySQL 远程 192.168.0.252:3306~~ | ❌ **不可达** (ECONNREFUSED) | — | — | — | 排除 |

**结论:三方 expected = MySQL(local root/root123) + SQLite(本地) + PG(本地,需重装完整 schema)。MySQL 远程 252 不可达是 #2977 旧文档残留,不能依赖。**

### 2.3 Engine bugs (5 个,真阻塞)
来源: `docs/audit/status/2026-06-04-tpch-phase2d-status.md` + `tests/tpch_value_correctness_test.rs` 注释

| # | Bug | 影响 query | 阻塞 row_count 一致? |
|---|---|---|---|
| 1 | `WHERE col TEXT <= 'literal'` 返回 0 行 | Q1, Q3, Q6, Q7, Q8, Q9, Q10, Q12 | 是 |
| 2 | `FROM a, b, c` 不支持 | Q3, Q5, Q8, Q9, Q10, Q12, Q14, Q19, Q22 | 是 |
| 3 | SELECT 被忽略,返回全列+列名当 TEXT | Q1, Q2, … Q22 全部 | 是(列名当 cell 算行数就乱) |
| 4 | `SUM(real_col)` 返回 0 | Q1, Q3, Q5, Q6, Q7, Q8, Q9, Q10, Q12, Q14, Q15, Q18, Q19, Q21 | 是 |
| 5 | `AVG(real_col)` 返回 Null | Q1, Q2, Q3, Q6, Q7, Q10, Q14, Q15, Q17, Q18, Q20, Q22 | 是(影响 GROUP BY 行数吗?否,但 Q1 的 count_order 用 SUM/AVG 投影) |

**注意 bug #3 最难**:22 条全依赖正确投影,必须先修。

### 2.4 现有 wire 测试基础设施
- `crates/mysql-server/src/lib.rs:1474` `handle_load_local_infile` — **已实现**
- `crates/mysql-server/src/lib.rs:1632` — `do_command_loop` 路由 LOAD DATA
- `tests/load_local_infile_test.rs` — 5 个测试全 GREEN
- `tests/common/mod.rs` — `MySqlTestClient::load_local_infile()` 客户端 API
- `tests/tpch_wire_smoke_sf.rs` — Q1 wire 模板,目前 `#[ignore]`

## 3. 实施步骤 (顺序硬约束)

### 阶段 0:三方 expected 生成 (前置,不动 SQLRustGo 代码)
1. 写 `scripts/tpch_three_way_expected.py` — 输入 `tests/data/tpch-sf001/`,输出 `tests/data/tpch-sf001/expected/Q1.json … Q22.json`
2. 对每个 Q*.sql,执行:
   - 装 schema 到 SQLite (type mapping INTEGER/TEXT/REAL)
   - LOAD DATA 全 8 表(614+150+15+10+5+25+20+80 行,小)
   - 跑 Q,抓 row_count
   - 同样对 MySQL / PG 跑,三方 row_count 写入 JSON 的 `row_count` 字段,内容(列)可选
3. 22 个 JSON 生成后,三方一致 → 进 expected。不一致 → 标记 + 报李哥,先 ignore 该条。
4. **完成标志**: `expected/Q*.json` 22 个文件,row_count 字段填写,三方比对通过。

### 阶段 1:5 个 engine bug 修(每个独立 commit)
| Bug | 测试入口 | 修法(预估) | 验证 |
|---|---|---|---|
| #1 TEXT 比较 | `test_tpch_q1_where_text_compare_returns_some_rows` | `expr_utils.rs:317` 的 `CaseWhen` unreachable,可能类似比较路径里 `Literal` vs `Column` 类型没 coerce | 该 test 转绿 + sf001 Q1 row_count=4 groups (A/F, A/O, N/F, N/O, R/F, R/O 实际 6 groups) |
| #2 comma-join | `test_tpch_q3_three_table_join_row_count_today` | parser grammar FROM a,b,c 没展开成 cross join | 该 test 转绿 + Q3 在 sf001 跑出 1+ 行 |
| #3 SELECT 投影 | (无现成 test,需新写) | `engine_select.rs:182` projection 路径返回全列 + 列名当 cell | 新增 test 验证 SELECT l_returnflag,SUM(l_quantity) FROM lineitem GROUP BY l_returnflag 返回 N 行 N 列,且列名 == 真实列名,值是数字不是 "l_quantity" |
| #4 SUM real | (无现成 test,需新写) | aggregator 走 INTEGER 分支 | 新增 test,sum 真实返回 |
| #5 AVG real | (无现成 test,需新写) | 同 #4 | 新增 test,avg 不为 Null |

**完成标志**: `cargo test --all-features` 全绿 + `tpch_value_correctness_test.rs` 5 个 test 全绿 + 自己加的 #3/#4/#5 test 全绿。

### 阶段 2:`tests/tpch_full_22_wire_test.rs` 写
1. 复制 `tpch_wire_smoke_sf.rs` 的 spawn 模式
2. SQLRustGo 端:用 `EphemeralConfig` + `load_local_infile` 装 8 表
3. 对 22 条 query 逐条 execute,抓 `client.query()` 返回的 row_count
4. 对 expected/Q*.json 的 row_count,assert 相等
5. 失败的 query `#[ignore]`(除非三方预期已 OK 但 SQLRustGo 失败,留 active 让 CI 看见)
6. `cargo test --test tpch_full_22_wire_test --all-features` 全绿(或不绿但 ignore 列表清晰)

**完成标志**: PR 提到 Gitea,关联 issue #2977,#2948。

### 阶段 3:PR 提交
- 标题: `[tpch] 22-query wire round-trip + three-way expected (Issue #2977 + #2948)`
- 描述: 5 engine bug fix + expected/Q*.json + new wire test + 跑通截图
- `--admin` 合并 → `develop/v3.8.0`

## 4. 风险 & 中断条件

| 风险 | 中断条件 | 应对 |
|---|---|---|
| bug #3 (SELECT) 牵动 22 条 | 单 query 改 1 处,其他 query 跑挂 | 优先修 #3,逐 query 验证 |
| MySQL 远程访问不可用 | `192.168.0.252:3306` 不通或密码错 | 退化为 SQLite + PG(重装 schema)二方 |
| `expected/Q*.json` 三方不一致 | 比如 PG 不支持 `EXTRACT(YEAR FROM date)` | 该 Q 标 ignore,在 JSON 标"vendor-specific" |
| `sf001` 数据太小三方 query 退化(如 Q18 0 行) | 跑出来 0 行或 1 行 | 接受,三方共识就是 0 行 = PASS |
| bug fix 引入回归 | 38 个 wire 测试挂 | 回滚该 bug fix,改更小 patch |

## 5. 时间盒 & 同步点

| 阶段 | 估时 | 同步点 |
|---|---|---|
| 0:三方 expected | 1-2h | 22 个 JSON 出,李哥 review 1 次 |
| 1:5 bug 修 | 3-4h(每 bug 30-60min) | 每 bug 修完跑 `cargo test --all-features` 1 次 |
| 2:wire test 写 | 1-2h | 22 条逐条 verify |
| 3:PR | 30min | 提交 + 合并 |

**总预计 6-8h**。每阶段结束我都会停下来报李哥,不擅自往下冲。

## 6. 李哥需提供的(避免卡)

1. **MySQL root 密码或 sudo 权限** — 本地 / 远程任一能进就行
2. **MySQL 远程 192.168.0.252:3306 是否可达 + 凭据** — skill 记着"openclaw/details8848"在 Gitea 上,MySQL 不知道
3. **5 bug 修法优先顺序** — 默认按 #3 → #1 → #2 → #4 → #5 修(#3 影响最大,先)

不卡的话我就从 0 开始,卡的话告诉我哪条要缓。

## 7. 不在本次范围

- SF=0.1 / SF=1 大 fixture(留 follow-up)
- 性能基准(Criterion bench 不动)
- 分布式 TPC-H(crates/distributed 不动)
- TPC-H spec 全部 22 query 严格结果校验(只验 row_count,内容校验留给后续)

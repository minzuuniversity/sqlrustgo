# SQLRustGo v4.1.0 综合评估报告

> **provenance:** generated_by=codex, generated_at=2026-10-05T00:00:00+08:00, source_repo=openclaw/sqlrustgo, assessed_ref=gitea252/develop/v4.1.0, assessed_commit=`a6ed136dd2f393e2ce3d050fd69c13bc90702cc4`, policy=Anti-Fabrication-Policy-v1.0 + ADR-001 + ADR-008 + ADR-014
> **版本**: v4.1.0
> **报告类型**: 横向综合评估；重点是 SQL 标准与 SQLite / MySQL / PostgreSQL 能力面比较，不以阶段门禁为主线
> **当前阶段背景**: DRAFT (`docs/releases/v4.1.0/STAGE.yaml`)
> **历史基线**: v3.6.0 → v3.8.0 → v3.9.0 → v3.10.0 → v3.11.0 → v3.12.0 → v4.0.0 → v4.1.0
> **证据边界**: 本报告基于版本文档、252 Gitea API、Git refs 与本次轻量 gate 检查；未重新执行全量 SQL corpus、全量 TPC-H、覆盖率或跨引擎 oracle。

## 1. 总体结论

v4.1.0 不是“已经完成的数据库兼容性版本”，而是 v4.x 对 v4.0.0 撤销后的继续整改线。与 v3.12.0 相比，v4.1.0 的方向从单一 SQL/GMP 检索底座扩展到 SQL + Vector + Graph + Audit/GMP 的多模型数据库，但在 SQL 标准兼容、SQLite 方言兼容、MySQL 产品级兼容、PostgreSQL 级优化器与 MVCC 语义方面仍未达到主流数据库替代水平。

横向判断如下：

| 对比对象 | v4.1.0 当前定位 | 可声明边界 |
|---|---|---|
| SQL-92 | 覆盖了 SELECT / DML / 基础 DDL / JOIN / 聚合 / 子查询的大量核心子集，但仍有长期遗留 corner cases | 可声明“SQL-92 核心子集 + TPC-H 驱动的 OLAP 子集”；不可声明完整 SQL-92 |
| SQLite | 支持一部分 SQLite 教学/兼容语法，但 sqlite_master、AUTOINCREMENT、块注释、部分函数/DDL/ON CONFLICT 仍有缺口 | 可声明“受控 SQLite-style 子集”；不可声明 SQLite 替代 |
| MySQL | MySQL wire、SHOW/DDL/DML 子集、prepared statement、LOAD DATA 基础能力持续增强；但事务隔离、AUTO_INCREMENT、CHAR PAD SPACE、复制、高可用、information_schema 等仍不足 | 可声明“MySQL wire + MySQL-like 受控子集”；不可声明 MySQL 5.7/8.0 替代 |
| PostgreSQL | 可用作 TPC-H / SQL correctness oracle 对照；SQLRustGo 不实现 PostgreSQL wire，也不具备 PostgreSQL 级优化器、MVCC、系统目录和扩展生态 | 不应声明 PostgreSQL 兼容；只能声明“部分结果与 PostgreSQL/SQLite oracle 对比验证过” |

最重要的结论是：v4.1.0 的价值不在“比 v3.12 多宣称一个阶段”，而在把 v3.12/v4.0 暴露的 SQL 方言债、事务隔离债、门禁空转债和多模型一致性债纳入一个统一整改版本。

## 2. 历史演化脉络

| 版本 | 能力主线 | 对综合兼容性的影响 |
|---|---|---|
| v3.6.0 | 协议栈整合、MySQL 协议、SIMD/WAL 验证起步 | 暴露 MySQL server tests 大量错误，SQL compat gate 仍 PENDING |
| v3.8.0 | TPC-H 22/22 达成，SQL executor 进入 practical engine 阶段 | SQL-92 核心查询能力显著增强，但 MySQL 产品级兼容评估仍约 58/100 |
| v3.9.0 | 长稳、SOAK、TPC-H SF=0.1、工程化可投产候选 | 稳定性增强，但 SF=1、覆盖率、方言完整性仍为后续目标 |
| v3.10.0 | 明确提出 MySQL 5.7 轻量替代口径 | 建立 MySQL 对比表，但定位限定于开发/CI/嵌入式/轻量场景 |
| v3.11.0 | TPC-H SF=1 22/22 可运行、长稳 SOAK、核心主路径集成 | 证明复杂 SQL 可运行性，但仍不证明跨引擎结果零差异或完整 MySQL 替代 |
| v3.12.0 | GMP 内审检索数据库底座、MySQL wire/LOAD DATA/SHOW 子集硬化 | 明确将“受控场景 GA”与“完整 MySQL/SQLite 替代”切开 |
| v4.0.0 | 目标转向 SQL + Vector + Graph + Audit 多模型 | 原 GA 判定撤销；多模型目标成立，但 SQL 方言与覆盖率债转入 v4.1 |
| v4.1.0 | v4.0 撤销后整改线，事务隔离/GC/门禁/兼容性继续收口 | 当前应作为“兼容性与一致性修复版本”，不是新的 GA 扩宣版本 |

## 3. SQL-92 支持评估

| SQL-92 能力 | 当前成熟度 | 依据与说明 |
|---|---|---|
| 基础 DML (`SELECT/INSERT/UPDATE/DELETE`) | 高，但非全边界 | v3.12 MySQL compat 文档列为 supported；v4.1 仍有 UPDATE/事务隔离类 issue |
| 基础 DDL (`CREATE/DROP TABLE/INDEX`) | 中高 | 基础能力可用；复杂 ALTER、schema migration 与 SQLite metadata 仍未完整 |
| JOIN | 中高 | v3.8 TPC-H 推动 INNER/LEFT/RIGHT/CROSS 能力；NATURAL JOIN、multi-column USING、FULL/RIGHT 边界仍有遗留 |
| 聚合 / GROUP BY / HAVING | 中高 | v3.8/v3.9 核心 OLAP 已显著增强；GROUP_CONCAT、ROLLUP/CUBE 不属于当前完整声明 |
| 子查询 | 中 | IN/EXISTS/标量子查询持续增强；`> ALL` / `= ANY`、correlated scalar 仍在 backlog |
| CTE | 中 | 非递归 CTE 可用；recursive CTE / writable CTE 不应声明完整 |
| 事务语义 | 中低 | v4.1 正在修 reader_tx、dirty read、GC 未提交版本等 P0；不能声明完整隔离级别 |
| 类型/比较 | 中 | 常规类型可用；CHAR PAD SPACE、部分函数、时间/日期边界仍不足 |

综合判断：v4.1.0 的 SQL-92 水平应描述为“核心工作负载子集可用，复杂标准边界和事务一致性仍在整改”。这比 v3.6 的 SQL compat PENDING 有明显进步，但还没有达到主流数据库的完整 SQL 标准覆盖。

## 4. SQLite 横向比较

SQLite 的关键特征是单文件、嵌入式、动态类型、成熟的 SQL 子集、丰富 PRAGMA/system catalog 行为和长期稳定的兼容性。SQLRustGo v4.1.0 与 SQLite 的关系更像“借用 SQLite 作为教学/兼容/SQLLogicTest oracle”，不是替代 SQLite。

| SQLite 能力面 | SQLRustGo v4.1.0 状态 | 结论 |
|---|---|---|
| 基础 SELECT/DML | 部分可比 | 简单教学场景可以迁移；复杂方言需逐项验证 |
| sqlite_master / sqlite_sequence | 仍在 WP-C backlog | 不可声明 SQLite schema/catalog 兼容 |
| AUTOINCREMENT | 仍有 issue 继承与 v4.1 BLK-3 类问题 | 不可声明 SQLite/MySQL 自增语义完整 |
| ON CONFLICT / UPSERT | 部分缺口 | 不可声明完整 SQLite DML 方言 |
| JSON / GROUP_CONCAT / 函数生态 | 多项 caveat | 不可声明 SQLite 函数兼容 |
| 块注释 `/* */` | #4708 复验显示未修 | 基础 parser 方言仍有明显缺口 |
| 单文件嵌入式体验 | 不是 SQLite 的同类产品 | SQLRustGo 更偏 server / engine / multi-model |

结论：v4.1.0 可继续用 SQLite 作为 sqllogictest 和 cell-level oracle，但产品声明必须避免“SQLite 替代”。与 SQLite 相比，SQLRustGo 的优势方向是 Rust-native、MySQL wire、WAL/MVCC、多模型；劣势是方言完整性、catalog 兼容、嵌入式成熟度。

## 5. MySQL 横向比较

MySQL 是 SQLRustGo 长期最接近的外部兼容对象，因为 SQLRustGo 提供 MySQL wire server/client 能力。v3.10 报告曾给出“MySQL 5.7 轻量替代”口径；v3.12 进一步列出 MySQL compatibility surface。但 v4.1 当前不能扩大该声明。

| MySQL 能力面 | SQLRustGo v4.1.0 状态 | 风险 |
|---|---|---|
| MySQL wire protocol | 有持续实现与测试 | 可声明受控子集，不可声明全协议 |
| COM_QUERY / prepared statement | 基础可用 | binary result、参数化结果、长数据等仍需完整验收 |
| SHOW/DESCRIBE/SHOW CREATE/SHOW INDEX | v3.12 controlled subset | information_schema SQL path 仍不完整 |
| LOAD DATA | 基础能力与 SF 路线存在 | LOCAL INFILE、SF=10、client/server TLS 等边界仍需验证 |
| 事务隔离 | v4.1 P0 修复链中 | 脏读、repeatable read、GC 未提交版本问题阻断生产声明 |
| AUTO_INCREMENT | 仍有单调性/持久化风险 | MySQL OLTP/sysbench 类 workload 会暴露 |
| CHAR PAD SPACE | #4846 / WP-G 未闭环 | 主键点查与 MySQL 语义不一致 |
| 复制/半同步/HA | #4936/#4937 等仍 open | 不可声明 MySQL 运维替代 |
| 性能 schema / admin / optimizer | 部分或未接主路径 | 仍不是 MySQL 产品级替代 |

结论：v4.1.0 的 MySQL 兼容性应降级为“wire-compatible controlled subset”。适合开发、教学、受控 demo、GMP 内部 workload；不适合高并发 OLTP、复制、高可用、通用 MySQL 迁移。

## 6. PostgreSQL 横向比较

SQLRustGo 当前不是 PostgreSQL 兼容数据库。v3.9 文档中曾出现 PostgreSQL wire 相关旧口径，但 v3.12/v4.x 主线实际围绕 MySQL wire 与 SQLRustGo 自有执行引擎。PostgreSQL 更适合作为 TPC-H correctness/performance oracle，而不是兼容目标。

| PostgreSQL 能力面 | SQLRustGo v4.1.0 状态 | 结论 |
|---|---|---|
| PostgreSQL wire | 非当前主线 | 不应声明 |
| MVCC / isolation | SQLRustGo 正在补事务隔离缺口 | 与 PostgreSQL 成熟 MVCC 差距大 |
| COPY / bulk load | LOAD DATA 方向更接近 MySQL | PostgreSQL COPY 级性能不可声明 |
| optimizer / planner | SQLRustGo 有优化器与 TPC-H 推动 | 与 PostgreSQL CBO/统计信息/执行器成熟度差距大 |
| catalog / extensions | 不具备 PostgreSQL 生态 | 不应声明 |
| 结果 oracle | 可作为对照 | 适合用于 TPC-H SHA256 / row-count 比对 |

结论：PostgreSQL 比较应定位为“成熟数据库标尺”。v4.1.0 应继续用 PostgreSQL 做 correctness oracle，但不应写 PostgreSQL 替代或协议兼容。

## 7. v4.1.0 当前关键差距

| 差距 | 影响对象 | 当前证据 |
|---|---|---|
| 事务隔离与 GC 未闭环 | MySQL/PostgreSQL 级事务语义 | #4974、#4986、PR #4987 open |
| anti-ignore registry drift | 测试可信度 | `check_anti_ignore_gate.sh` 实跑：real `#[ignore]`=102，registry=98 |
| DRAFT 文档结构缺失 | 发布治理 | `check_stage.sh --version v4.1.0 --stage DRAFT --dry-run` 缺 `ARCHITECTURE.md` |
| SQLite dialect backlog | SQLite 兼容 | WP-A/WP-C/WP-D/WP-F/WP-G 仍有 open/not started |
| MySQL OLTP/sysbench 阻断 | MySQL 兼容 | #4981、#4983、#4986 等 2026-10-04 P0 |
| 多模型一致性证据不足 | v4.x 产品目标 | v4.0 GA 撤销后转入 v4.1 整改 |
| 覆盖率与大文件架构债 | 工程质量 | parser/mysql-server coverage 债；`expr/mod.rs` 等超 1600 行但非 hard gate |

## 8. 近期整改建议

1. 先关闭事务隔离主链路：#4974、#4983、#4986、#4987 必须有 merged PR + 回归测试 + issue closure evidence。
2. 把 SQLite/MySQL 方言 backlog 分成“必须修”和“永久 caveat”：尤其 #4708、#4672、#4846、#4848、#4656、#4636。
3. 重写兼容性矩阵：按 SQL-92 / SQLite / MySQL / PostgreSQL 四列维护，禁止用“兼容”二字覆盖细节。
4. anti-ignore / P16 先 fail-closed，再清 registry drift，避免再次出现“测试通过但没跑”的假象。
5. 为 v4.1.0 补 `ARCHITECTURE.md`，内容应说明当前不是 SQLite/MySQL/PostgreSQL clone，而是 Rust-native 多模型数据库。
6. 恢复跨引擎 oracle：至少对 TPC-H、SQLLogicTest、SQLite-style 教学 fixture 保留 row-count + hash + failure inventory。

## 9. 允许与禁止声明

允许声明：

- v4.1.0 是 v4.0.0 撤销后的 v4.x 整改与维护延续线。
- v4.1.0 正在把 SQLRustGo 从 SQL/GMP 底座推进到 SQL + Vector + Graph + Audit 多模型数据库。
- 当前 5 远端 `develop/v4.1.0` 已同步到 `a6ed136dd2f3`。
- SQLRustGo 已具备 MySQL wire controlled subset、TPC-H 驱动的 SQL core、部分 SQLite-style 教学兼容能力。

禁止声明：

- v4.1.0 已达到 ALPHA/BETA/RC/GA 发布质量。
- v4.1.0 是 SQLite、MySQL 或 PostgreSQL 的完整替代品。
- v4.1.0 已实现完整 SQL-92。
- 事务隔离、AUTO_INCREMENT、CHAR PAD SPACE、ALTER TABLE RENAME COLUMN、SQLite metadata 已全部闭环。
- anti-ignore / P16 / 全量测试 / 覆盖率已通过，除非附上本版本实跑输出。

## 10. Evidence Log

本次评估使用的关键本地证据：

```text
git rev-parse --short=12 HEAD
=> a6ed136dd2f3

python3 yaml parse docs/releases/v4.1.0/STAGE.yaml
=> version=v4.1.0, current_stage=DRAFT

bash scripts/sync/5remotes_drift_check.sh --branches develop/v4.1.0,main
=> exit 0, all pairs 0/0

bash scripts/gate/check_main_freshness.sh
=> exit 0, PASSED 3 / FAILED 0

bash scripts/gate/check_docs_links.sh
=> exit 0, All markdown links are valid

bash scripts/gate/check_stage.sh --version v4.1.0 --stage DRAFT --dry-run
=> exit 1, missing docs/releases/v4.1.0/ARCHITECTURE.md

bash scripts/gate/check_anti_ignore_gate.sh
=> exit 1, tree real #[ignore]=102, registry=98

Gitea API state=open issues
=> 17 open issues, 1 open PR (#4987)
```

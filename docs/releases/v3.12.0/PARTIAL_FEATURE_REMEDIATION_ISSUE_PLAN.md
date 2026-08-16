# SQLRustGo v3.12.0 PARTIAL 功能整改 Issue 计划

> **provenance:** generated_by=Codex, generated_at=2026-08-14T20:05:00+08:00, commit=e14941c663789182384546735dfd6237715d96b1, source_repo=openclaw/sqlrustgo, branch=codex/v312-partial-remediation-plan, policy=Anti-Fabrication-Policy-v1.0
>
> **目标:** v3.12.0 作为功能比较完备的初始生产版本时，README 中属于 v3.12 生产边界的 `PARTIAL` 功能不得悬空。每个 `PARTIAL` 必须在 GA 前关闭为 `DONE / 受控`，或降级为 `DEFERRED` / `UNSUPPORTED` 并绑定 issue、owner、expiry、关闭边界。

## 1. 判定规则

| 状态 | 允许进入 v3.12 GA? | 要求 |
|---|---:|---|
| `DONE` | 是 | 有合并 PR、当前 develop 可达 commit、实跑 gate、日志/hash |
| `DONE / 受控` | 是 | 支持范围明确；超出范围有 fail-closed 或 explicit unsupported 证据 |
| `PARTIAL` | 否 | 必须绑定整改 issue；GA 前必须转为 `DONE / 受控` 或 `DEFERRED` |
| `OPEN` | 否 | 仍是 blocker；必须保留 open issue |
| `DEFERRED` | 条件允许 | 不属于 v3.12 初始生产边界；必须有 owner、expiry、关闭边界 |
| `UNSUPPORTED` | 是 | README/release note 不得把它宣传为 v3.12 能力 |

## 2. README PARTIAL 整改总账

| README 能力 | 当前事实边界 | 3.12 决策 | 整改 Issue | 关闭边界 |
|---|---|---|---|---|
| TPC-H SF=1 correctness | 22/22 可运行；row-count baseline 存在；cross-engine SHA256 与 zero-row correctness 未闭环 | GA blocker | #4221 | 22 query 至少一个外部 oracle row-count + SHA256；zero-row 逐 query 解释或修复 |
| TPC-H SF=10 harness / bulk-load | SF=10 runner 存在；3/8 小表 parity match；customer/part/partsupp/orders/lineitem 受吞吐瓶颈阻塞 | GA blocker for SF=10 claim | #4020, #4217 | 8/8 表 row-count/hash parity，或 README 降级为非 v3.12 生产声明 |
| Sysbench OLTP | read_only 约 2870 qps / 179 tps；write_only/read_write 因 DELETE+INSERT PK race 失败；prepared statement 有单独缺口 | GA blocker for OLTP write claim | #4210, #4211 | read/write/read_write 通过或明确只支持 read_only baseline |
| MySQL wire protocol | V312-13 smoke/pass 证据存在；完整 MySQL 5.7 兼容不可宣称 | 受控收口 | #4223 | COM_QUERY/COM_STMT/error/reset/TLS/compression 当前 HEAD 实跑证据齐全 |
| Prepared Statement | 基本回归存在；sysbench prepared statement 仍失败 | GA blocker if declaring sysbench compatibility | #4211, #4223 | sysbench 默认 prepared-statement path PASS，或文档降级 |
| LOAD DATA | SF=1/smoke 与 wire gate 有证据；SF=10 full bulk-load 未完成 | GA blocker for bulk import claim | #4020, #4217 | SF=1/SF=10 row-count/hash/memory/time artifact |
| TLS / Compression | typed wrapper / handshake / primitive smoke 存在；生产客户端路径仍需边界测试 | 受控收口 | #4223 | mysql-compatible client path 证据或明确 unsupported boundary |
| WAL / MVCC | 主路径存在；crash recovery 28/31，3 个真实失败；backup/restore API drift | GA blocker | #4222 | crash/recovery/backup/restore/upgrade-downgrade 重跑 PASS |
| SQLLogicTest runner | smoke gate 当前 25/25 PASS；full SQLite official corpus 未宣称完成 | Beta 可接受，RC/GA 扩展 | #4219, #4224 | README 修正；selected corpus manifest/hash/exclusion policy 完整 |
| 覆盖率治理 | 分层覆盖率已建立；仍有低覆盖 crate 和 SEM-4 gap | GA blocker for coverage claim | #3943 | per-crate coverage 当前 HEAD 重新采集；低覆盖全部 issue-linked |
| Internal Vector Retrieval | GMP 内部检索路径存在；不是通用向量数据库；rebuild/quality/dimension gate 需生产化 | GA blocker for GMP vector claim | #4225 | top-k fixture、rebuild invariant、dimension/hash/empty-index fail-closed |
| GMP compliance audit controls | schema/audit-chain 报告存在；生产权限矩阵和操作链 gate 仍需补齐 | GA blocker for GMP production claim | #4226 | import/search/export/review/backup/restore audit-chain + ACL 正反例 |
| 窗口函数 / GIS / JSON | 非 GMP 核心；历史计划中标为受控 SQL 功能 | scope decision | #4227 | 进入 3.12 则按子集测试；否则 README 改为 DEFERRED/UNSUPPORTED |
| 存储过程 / 触发器 | parser、catalog、`CALL`、`CREATE TRIGGER`、DML hook 已有局部实现；缺 `CREATE/DROP/SHOW PROCEDURE`、确定性 SQL 执行、`NEW/OLD`、事务/WAL/恢复、递归、权限和 SQLLogicTest/E2E 门禁 | GA blocker for MySQL-style 基础兼容声明；v3.12 P0 受控子集 | #4237 / V312-55（子任务 #4238-#4245） | `bash scripts/gate/check_v312_procedure_trigger_gate.sh` 退出 0；V312-55A~55H PR 全部合并；README 才能改为 `DONE / 受控基础功能` |
| Metadata/SHOW/information_schema | `information_schema` crate、SHOW/DESCRIBE/SHOW CREATE 局部实现与 placeholder 并存；文档和 MySQL 教学需求未闭环 | Beta 前 blocker for 教学/基础客户端 | [#4251](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4251) | information_schema tables/columns/indexes 或明确 unsupported；SHOW CREATE/COLUMNS/INDEX/DESCRIBE 有正反例和 gate |
| SQL 教学 corpus / 多 oracle | SQLLogicTest smoke 与 official/full corpus 边界已澄清，但教学最小 corpus 未单独定义 | Beta 前 blocker for 教学场景 | [#4252](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4252) | teaching corpus manifest、SQLite/PostgreSQL/MySQL oracle、PASS/FAIL/SKIP 分类、issue-linked exclusions |
| Transaction/crash recovery 教学实验 | v3.12 有 recovery gate，但教学实验和 backup/restore count/hash 演示未闭环 | Beta 前 blocker for 教学场景 | [#4253](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4253) | BEGIN/COMMIT/ROLLBACK/SAVEPOINT、kill -9/WAL replay、backup/restore count/hash 可复跑 |
| Prepared/wire 教学实验 | wire/prepared 是生产 blocker，但 text/binary protocol 教学实验未单独定义 | Beta 前 blocker for 教学场景 | [#4254](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4254) | COM_QUERY/COM_STMT/error/reset/LOAD DATA 教学 fixture；TLS/compression 明确 DONE 或 DEFERRED |
| Optimizer/EXPLAIN 教学实验 | CBO/Hash Join 有实现和债务，但缺稳定教学 plan dump | Beta 阶段 P1 | [#4255](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4255) | EXPLAIN/plan dump、统计信息、hash/semi/anti join fixture |
| VIEW/CTE/MERGE | CTE 主路径较强；VIEW 当前偏 definition storage；MERGE 主 execute path unsupported | Beta 阶段 P1 | [#4256](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4256) | VIEW/CTE/MERGE 明确 DONE/DEFERRED/UNSUPPORTED；状态同步 README/compat/report |
| Partition/FullText | storage-level 结构存在，SQL surface/GMP keyword retrieval 决策未闭环 | Beta 阶段 P1 | [#4257](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4257) | Partition/FullText 与 SQL surface/GMP keyword retrieval 决策一致；不保留悬空 PARTIAL |
| 完整 MySQL 5.7 替代 | 多项 blocker 未闭环 | 不属于 v3.12 当前声明 | #4220 | README 保持 `OPEN`，直到 SQLLogicTest/TPC-H/wire/LOAD DATA/recovery/upgrade 全部闭环 |

## 3. 新增 Issue

| Issue | 类型 | 用途 |
|---|---|---|
| #4220 | 总控 | PARTIAL 功能整改总控，要求 README 中所有 PARTIAL 有归属 |
| #4221 | blocker | TPC-H SF=1 correctness、zero-row、cross-engine SHA256 |
| #4222 | blocker | Crash recovery、backup/restore、upgrade/downgrade |
| #4223 | blocker | MySQL wire/TLS/compression 与 prepared statement 总控 |
| #4224 | blocker | SQLLogicTest selected corpus / SQLite official corpus RC-GA 扩展 |
| #4225 | blocker | GMP vector/retrieval quality 与 index rebuild 生产门禁 |
| #4226 | blocker | GMP compliance/access-control production gate |
| #4227 | decision | Window/GIS/JSON 3.12 受控交付或降级决策 |
| [#4250](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4250) | blocker | 4.0 前功能整改与 MySQL 教学能力补强总控 |
| [#4251](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4251)-[#4254](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4254) | blocker | Beta 准入前必须完成或显式降级的教学/兼容核心任务 |
| [#4255](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4255)-[#4258](http://192.168.0.252:3000/openclaw/sqlrustgo/issues/4258) | P1/P0 | Beta 阶段完成 Optimizer/VIEW/MERGE/Partition/FullText 和 gate/docs/evidence 集成 |

## 4. 必须同步到门禁

1. Beta/RC/GA gate 不能只检查报告文件存在，必须执行对应脚本或校验机器可读 manifest。
2. #4220 关闭前，`README.md` 中每个 `PARTIAL` 必须能在本文件找到整改 issue。
3. #3887 总控必须引用本文件，并在每轮审计中同步 #4220-#4227 与现有 #3943/#4020/#4210/#4211/#4217/#4218/#4219 状态。
4. 若某功能决定不进入 v3.12 初始生产边界，应把 README 状态从 `PARTIAL` 改为 `DEFERRED` 或 `UNSUPPORTED`，而不是保留模糊 `PARTIAL`。
5. V312-56A~56D 必须接入 Beta 准入检查；不能只创建 issue 或文档条目。

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

## 4. 必须同步到门禁

1. Beta/RC/GA gate 不能只检查报告文件存在，必须执行对应脚本或校验机器可读 manifest。
2. #4220 关闭前，`README.md` 中每个 `PARTIAL` 必须能在本文件找到整改 issue。
3. #3887 总控必须引用本文件，并在每轮审计中同步 #4220-#4227 与现有 #3943/#4020/#4210/#4211/#4217/#4218/#4219 状态。
4. 若某功能决定不进入 v3.12 初始生产边界，应把 README 状态从 `PARTIAL` 改为 `DEFERRED` 或 `UNSUPPORTED`，而不是保留模糊 `PARTIAL`。

## 5. V312-56 系列 issue mapping（同步自 ISSUES_PLAN.md，V312-56H / Issue #4258 验收条件 4）

| Issue | V312-56 | 类型 | 整改范围 | 关闭边界 |
|---|---|---|---|---|
| #4250 | V312-56 总控 | orchestrator | 4.0 前功能整改与 MySQL 教学能力补强总控 | #4251-#4258 全部 CLOSED 或显式降级 |
| #4251 | V312-56A | teaching | Metadata/SHOW/information_schema 教学与兼容闭环 | SHOW INDEX/SHOW COLUMNS + `wildcard_match` 实跑 PASS；32/36 子任务 |
| #4252 | V312-56B | corpus | SQL 教学 corpus 与多 oracle 对比 | `teaching_sql_v3_12/` 28+ SQL fixtures + manifest.yml |
| #4253 | V312-56C | teaching | Transaction/crash recovery 教学实验 | 6 crash tests + 3 tx tests + V312-56C_TRANSACTION_TEACHING.md |
| #4254 | V312-56D | teaching | Prepared statement 与 wire protocol 教学实验 | 3 prepared/* fixtures |
| #4255 | V312-56E | teaching | Optimizer/EXPLAIN 教学实验 | 5 EXPLAIN fixtures + `crates/executor/src/explain.rs` |
| #4256 | V312-56F | disposition | VIEW/CTE/MERGE disposition 与门禁 | VIEW supported；CTE/MERGE → DEFERRED（带 owner/expiry/关闭边界） |
| #4257 | V312-56G | disposition | Partition/FullText disposition 与 GMP keyword retrieval 决策 | TABLE PARTITION → UNSUPPORTED；FULLTEXT MATCH → DEFERRED |
| #4258 | V312-56H | gate/docs/evidence | Beta gate、文档和 release evidence 集成 | STAGE.yaml + TEST_PLAN.md + ISSUES_PLAN.md + V312-56-VERIFICATION.md 四件齐备 |

**注**: V312-56F (#4256) 与 V312-56G (#4257) 是 disposition 类问题，把 `PARTIAL` 改为 `DONE / 受控`（VIEW）或 `DEFERRED`（CTE/MERGE/PARTITION/FULLTEXT MATCH），并写入 README 与 `MYSQL_COMPAT_STATUS.md`。

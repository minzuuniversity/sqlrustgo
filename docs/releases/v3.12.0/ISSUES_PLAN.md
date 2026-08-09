# SQLRustGo v3.12.0 Issue 计划

> **版本**: v3.12.0
> **状态**: 规划中
> **日期**: 2026-08-09

本文件把 v3.12.0 计划拆分为 issue 粒度的工作包。每个条目只有在产生执行证据后，才可以标记为完成。

## V312-01：前序版本阻断项处置

**优先级**: P0
**目标**: 确保 v3.12.0 不继承 v3.11.0 隐藏生产弱项。
**范围**: 对齐 G3 coverage、G4 TPC-H SF=1、168h SOAK、`debt-registry.yaml` 与 truth audit，重点关注 F-25/F-26 和 extension crate 状态。
**验收**: blocker disposition report、stage gate output，以及所有 PASS claim 的 evidence hash。

## V312-02：GMP Schema v3.12

**优先级**: P0
**目标**: 定义由 SQLRustGo 管理的 GMP document、chunk、embedding、audit log 和 relation tables。
**验收**: schema creation 幂等；保留 document version history；audit rows 包含 previous hash 和 event hash；relation rows 能表达 SOP、clause、CAPA、deviation、role、equipment 关系。

## V312-03：GMP Corpus Ingestion

**优先级**: P0
**目标**: 将 `~/gmp-platform/gmp-md` 导入 SQLRustGo 管理的 GMP tables。
**验收**: ingestion report 包含 document/chunk/relation/embedding/skipped/failure 计数；re-ingestion 幂等；源文件变更产生新 document version；保留 source path 和 source hash。

## V312-04：Embedding Provider 与 Vector Persistence

**优先级**: P0
**目标**: 用 SQLRustGo 管理的 embedding storage 和 vector index rebuild 替代 GMP 生产路径对 Chroma runtime 的必需依赖。
**验收**: embedding 存入 SQLRustGo tables；fixture 可重建 Flat 或 HNSW index；记录 model name、dimension、vector hash、chunk id；支持 BGE-M3/Ollama-compatible flows。

## V312-05：Hybrid Retrieval

**优先级**: P0
**目标**: 使用 SQL filters、keyword score、vector similarity、graph relation boost 和 RRF fusion 实现 GMP 内审检索。
**验收**: 每个 result 含 source path、document id、version、chunk id、chunk hash、score components、citation text；支持按 document type、chapter、status、effective date、relation type 过滤；固定 audit question fixture 结果确定。

## V312-06：SQL-backed Graph Projection

**优先级**: P0
**目标**: 为 GMP evidence navigation 实现 SQL-backed graph projection。
**验收**: nodes/edges 存储 document、clause、SOP、CAPA、deviation、equipment、role、audit finding；支持 depth <= 3 的 neighbor/path query；每条 path 输出 evidence bundle。

## V312-07：RAG Evidence Bundle

**优先级**: P0
**目标**: 让 GMP RAG 的每个回答都可追溯、可复核。
**验收**: 生成回答只能使用已引用 chunk；每个 answer/result 带 citation bundle；缺失 citation 时 fail closed；answer envelope 传递 evidence hash。

## V312-08：Compliance、Audit Trail 与 Access Control

**优先级**: P0
**目标**: 将 SQLRustGo 行为映射到 GMP/ALCOA+ 控制。
**验收**: import/search/export/approve/backup/restore/review 均进入 audit hash chain；ACL 覆盖 SQL/vector/graph/GMP retrieval；tamper 和 unauthorized access 测试 fail closed。

## V312-09：Backup/Restore 与 Upgrade Path

**优先级**: P0
**目标**: 验证 GMP/RAG/graph projection 状态可备份、恢复、升级和回滚。
**验收**: restore 后 row count、hash、embedding count、graph edge count 一致；覆盖 v3.10/v3.11 到 v3.12 的升级和 rollback fixture。

## V312-10：Mixed Workload SOAK

**优先级**: P0
**目标**: 运行覆盖 SQL、ingestion、retrieval、audit、backup/restore 的 168h mixed SOAK。
**验收**: 0 crash；audit-chain 不断裂；memory growth 有界；检索质量 fixture 周期性复核。

## V312-11：SQLite SQLLogicTest Oracle Gate

**优先级**: P0
**目标**: 把 v3.10 规划、v3.11 未集成到 gate 的 SQLite SQLLogicTest 变成 v3.12 阻断门禁。
**背景**: v3.10 报告提出使用 SQLite 官方 SQLLogicTest corpus；V310-14 记录 runner 已实现但官方 suite 下载受阻；当前 `crates/sqlrustgo_sqllogictest` 存在，含 22 个本地 `.test` 文件；v3.11 `RELEASE_GATE_CHECKLIST.md` 仍把 sqllogictest runner 标为 TBD。
**验收**: `cargo build -p sqlrustgo_sqllogictest` 成功；本地 smoke corpus 产生报告；官方/cached SQLite corpus 有 manifest、hash、file count、exclusion policy；新增或规划 `scripts/gate/check_sqllogictest_v312.sh`。

## V312-12：TPC-H SF=1 Correctness Close-out

**优先级**: P0
**目标**: 从“22/22 可运行”推进到“row-count 和 SHA256 正确性可解释”。
**验收**: SQLRustGo 与 SQLite/PostgreSQL/MySQL 至少一个外部基准引擎完成 22 query row-count/SHA256 对比；zero-row query 有独立 issue、owner、期限和解释。

## V312-13：MySQL Wire + LOAD DATA Hardening

**优先级**: P0
**目标**: 补齐 MySQL wire protocol 和 bulk import 生产风险。
**验收**: COM_QUERY、COM_STMT_PREPARE/EXECUTE/CLOSE、error packet、reset、TLS/compression 有 E2E artifact；SF=1/SF=10 `LOAD DATA` 有 row count、hash、memory cap、duration 证据。

## V312-14：Crash Recovery 与 Upgrade/Downgrade Verification

**优先级**: P0
**目标**: 证明 SQLRustGo 在 crash、restore、upgrade、rollback 场景下不破坏 GMP/RAG/graph 数据。
**验收**: kill -9、WAL replay、dirty page recovery、backup/restore checksum、v3.10/v3.11 fixture upgrade to v3.12 和 rollback verification 均有日志与 hash。

## V312-15：CREATE SEQUENCE Executor Close-out

**优先级**: P0
**目标**: 关闭 v3.11.0 综合评估中记录的 `CREATE SEQUENCE` executor gap。
**范围**: sequence DDL、`NEXTVAL`、default expression、并发取值、事务 rollback、WAL replay、backup/restore 后继续取值。
**验收**: sequence 正反例 SQL fixture 全部有输出；并发测试无重复值或回退；crash/recovery 后 sequence state 与 row/hash summary 一致。

## V312-16：Window/GIS/JSON 受控 SQL 功能交付

**优先级**: P1
**目标**: 按 v3.11.0 后续计划交付受控 SQL feature，而不是在 release note 中泛化声明。
**范围**: Window Functions 覆盖 `ROW_NUMBER`、`RANK`、`DENSE_RANK`；JSON 覆盖 JSON type 与 JSON path 基础查询；GIS 覆盖 `ST_Distance`、`ST_Intersects`、GeoJSON 输入输出。
**验收**: 每个功能都有正例、反例、unsupported boundary fixture；执行证据进入 SQL feature corpus report。

## V312-17：Coverage 与 Disabled-Test Debt Close-out

**优先级**: P0
**目标**: 处理 v3.11.0 禁用测试分析、低覆盖 crate 和 flaky test 遗留问题。
**范围**: parser、mysql-server、mysql-client coverage；历史 disabled/API-drift tests；无独立 test target 测试；`test_wal_perf_throughput` flaky。
**验收**: canonical coverage command 固化；per-crate coverage 报告输出；每个 disabled/API-drift/flaky 项都有 restore、rewrite、quarantine 或 retire 决策，并包含 issue、owner、expiry 和 evidence hash。

## V312-18：SF=10、Sysbench 与 Observability Baseline

**优先级**: P1
**目标**: 补齐 v3.11.0 后续计划中的性能和观测性基础数据。
**范围**: TPC-H SF=10、Sysbench OLTP mixed workload、bulk-load benchmark、Prometheus metrics、Slow Query Log。
**验收**: 每个 benchmark 有可复跑脚本、数据集说明、阈值、日志路径、趋势对比；Prometheus scrape 和慢查询日志有 e2e artifact。

## V312-19：SQL Corpus、Architecture Invariant 与 Reviewer Sign-off Gate

**优先级**: P0
**目标**: 把 v3.11.0 release checklist 中仍为 TBD 的 SQL corpus、架构 invariant 和双 reviewer 签核变成 v3.12 RC/GA 阻断 gate。
**范围**: `test_sql_corpus.sh` all targets、R2.1-R2.8 invariant scripts、reviewer sign-off template。
**验收**: SQL corpus all-target report、R2.1-R2.8 输出、2 名独立 reviewer 签核均包含 command output、timestamp、source agent、source run、evidence hash 和 output location。

## V312-20：v3.6-v3.10 Cross-version Backlog Disposition

**优先级**: P0
**目标**: 建立 v3.6.0 到 v3.10.0 规划任务、综合测试和历史遗留问题总账，防止旧债务在 v3.12 计划中失踪。
**范围**: v3.6 Beta PENDING、coverage/test compile、DML 双路径；v3.7 SHOW/auth/prepared/transaction routing；v3.8 frozen/backlog；v3.9 real SOAK/ignored tests；v3.10 SQLancer/test-runner/test-registry/E2E/anti-fabrication/coverage baseline。
**验收**: `historical-backlog-disposition.yml` 中每项状态只能是 `closed`、`superseded`、`carried`、`deferred` 或 `retired`；所有 `carried` 项必须关联 V312 issue、owner、expiry、evidence hash。

## V312-21：MySQL Compatibility 与 SQL Surface Backlog

**优先级**: P1
**目标**: 复核并补齐 v3.7-v3.10 历史文档中记录的 MySQL 兼容和 SQL surface 缺口。
**范围**: `SHOW TABLES`/metadata、empty-password auth edge、prepared statements、ALTER TABLE RENAME/MODIFY/ADD/DROP、TIMESTAMP、connection pool、stored procedure tokens、column-level permissions、ROLLUP/CUBE/REPLACE/RANK、advanced aggregates。
**验收**: 对 GMP/生产路径相关子集给出 fixture PASS；非目标项必须输出 explicit unsupported 或 deferred decision，不得在 release note 中无边界宣称支持。

## V312-22：Execution Architecture 与 Optimizer Debt Close-out

**优先级**: P1
**目标**: 复核 v3.6-v3.10 的执行路径统一、Parallel/SIMD 主路径和 Q4 优化债务。
**范围**: DML 是否仍绕过 PhysicalPlan/LocalExecutor；VTU/Parallel/SIMD 是否为主路径能力；Hash Semi Join、Anti Join、subquery decorrelation、CBO/histogram 是否需要进入 v3.12 或延期。
**验收**: 执行路径 invariant 脚本有输出；Q4/相关子查询 benchmark 有 baseline；未完成优化不得支撑性能声明。

## V312-23：Storage、Index 与 WAL Tooling Backlog

**优先级**: P1
**目标**: 复核 v3.8-v3.10 遗留的存储、索引和 WAL 工具项，补齐与 GMP 生产路径相关的可靠性验证。
**范围**: WAL checkpoint optimization、wal-verification 工具重设计、复合索引、索引统计、page checksum、partial write/torn page、vector SQL surface 与 index rebuild。
**验收**: 每项有测试、脚本或延期理由；GMP/RAG 必需的 storage invariant 必须进入 RC gate。

## V312-24：Test Infrastructure Activation

**优先级**: P0
**目标**: 将 v3.10.0 中记录为骨架或未充分使用的测试基础设施激活，或有证据地退休。
**范围**: `crates/sqlancer`、`crates/test-runner`、`crates/test-registry`、E2E shell scripts、anti-fabrication known broken test binaries、SQL corpus/SQLLogicTest runner integration。
**验收**: 每个工具可以运行并产生 artifact；如退休或延期，必须说明原因、替代 gate、owner 和 expiry。已知 broken test binaries 不得继续靠 WARN-only 掩盖。

**Phase 1 完成** (commit on `feature/v312-24-impl`，2026-08-09)：
12 个 task 全完成（sqlancer / test-runner / test-registry 三套 `[[bin]]` + 主入口 + JSON artifact + TOML manifest 持久化 + JoinSet 并行 + `tokio::time::timeout` 强制），新增 4 个 integration test 文件共 10 个 test 全 PASS，lib 测试 20/20 PASS，clippy strict 0 error，fmt clean。详见 `docs/releases/v3.12.0/V312-24_test_infra_activation_report.md`。

**Phase 2-8 未完成** — 拆为 V312-25 ~ V312-30 跟进。

## V312-25：E2E 脚本去重与死脚本清理（V312-24 Phase 2 follow-up）

**优先级**: P1
**目标**: `git rm` 10 个 stale mirror / dead-code E2E 脚本（3 在 `tests/e2e/` + 7 在 `scripts/gate/e2e/`）。
**来源**: V312-24 proposal.md §Disposition + tasks.md Phase 2。
**Owner**: minimax
**Expiry**: 2026-08-25
**Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-25_baseline_evidence.txt`（实测：11 个目标文件在树里、0 个 git log 删除、ignore_registry 0 条 V312-25 条目）
**关闭边界**（**全部满足**才允许关闭）:
1. `git log --diff-filter=D --name-only --pretty=format: -- tests/e2e/startup_connect.sh tests/e2e/tpch_sf01.sh tests/e2e/kill9_recovery.sh scripts/gate/e2e/e2e_01_basic_crud.sh scripts/gate/e2e/e2e_02_tx_commit_rollback.sh scripts/gate/e2e/e2e_03_wal_crash_recovery.sh scripts/gate/e2e/e2e_04_parallel_executor.sh scripts/gate/e2e/e2e_05_savepoint_rollback.sh scripts/gate/e2e/e2e_06_cte_query.sh scripts/gate/e2e/e2e_08_migration.sh | sort -u | wc -l` 输出 **≥ 10**。
2. `git ls-files tests/e2e/ scripts/gate/e2e/ | grep -E '(startup_connect|tpch_sf01|kill9_recovery|e2e_0[1-8])' | wc -l` 输出 **≤ 1**（保留 e2e_07 给 V312-26）。
3. `python3 -c "import json; d=json.load(open('tests/baseline/ignore_registry.json')); tests=d.get('ignored_tests',[]); v312_25=[t for t in tests if 'V312-25 retired' in t.get('reason','')]; print(len(v312_25))"` 输出 **≥ 10**（**字段名必须是 `ignored_tests` 不是 `files`**，已 baseline 验证 schema）。
4. **V312-25 不引入新 compile error** — 同一 sqlrustgo test (例如 `stress_test`) 在删前删后都报"47 previous errors"，baseline 一样。命令：`cargo check --workspace --tests 2>&1 | grep 'could not compile' | sort -u` 删后输出集合 ⊆ 删前输出集合。baseline (V312-25 删前) 已有 broken tests (与 shell 脚本无关)，V312-25 不应新增。
## V312-26：E2E 脚本 WARN-only 修复（V312-24 Phase 3 follow-up）

**优先级**: P1
**目标**: 重写 3 个 warn-only 脚本，去掉 `|| true` / grep 占位断言 / 静默吞错模式。
**来源**: V312-24 tasks.md Phase 3。
**Owner**: minimax
**Expiry**: 2026-08-30
**Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-26_baseline_evidence.txt`（实测：3 脚本共 223 个 `\|\| true` 命中；本机 sysbench 已装，关闭条件 4 需用 `env -i` PATH 剥离测试）
**关闭边界**（**全部满足**才允许关闭）:
1. `grep -rn '|| true' tests/e2e/backup_restore.sh tests/e2e/sysbench_wired.sh scripts/gate/e2e/e2e_07_json_vector.sh | wc -l` 输出 **0**（baseline 223）。
2. 失败注入测试 1：`mv scripts/gate/e2e/e2e_07_fixture.json{,.bak} 2>/dev/null; bash scripts/gate/e2e/e2e_07_json_vector.sh; echo "exit=$?"; mv scripts/gate/e2e/e2e_07_fixture.json{.bak,}` 退出码 **≠ 0**（验证 byte-exact 断言生效，不再静默 PASS）。
3. `bash tests/e2e/backup_restore.sh > /tmp/backup_restore_evidence.txt 2>&1; test $(wc -c < /tmp/backup_restore_evidence.txt) -gt 100 && grep -q restore /tmp/backup_restore_evidence.txt` 退出 0。
4. 失败注入测试 2：`env -i PATH=/usr/bin:/bin bash tests/e2e/sysbench_wired.sh; echo "exit=$?"` 退出 **1** + stderr 含 `sysbench not found`（用 `env -i` 隔离 sysbench，验证缺失时显式 fail）。
5. 关闭报告：`docs/releases/v3.12.0/V312-26_warn_only_fix_report.md`，含 3 脚本 diff + 2 失败注入 log + sha256。
**禁止关闭条件**: 仅以"测试通过"或"无 `\|\| true`"为依据；必须含上述 2 个失败注入的实测 log。

## V312-27：Anti-Fabrication 违规修复（V312-24 Phase 4 follow-up）

**优先级**: P0
**目标**: 关闭 V312-24 识别的 anti-fab 违规（merge_vtu stale `#[ignore]`、smoke `rows.len() <= 6`、e2e_beta 双 skip、stale ignore_registry 项）。
**来源**: V312-24 tasks.md Phase 4 + proposal.md §Anti-fabrication 17-23 行。
**Owner**: minimax
**Expiry**: 2026-08-25
**Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-27_baseline_evidence.txt`（实测修正 V312-24 proposal 错误：实际 37 条 stale v3.9.0 路径（不是 7 条），0 条 phantom parser.rs:7299（不是 1 条），3 条 union_set_operations）
**关闭边界**（**全部满足**才允许关闭）:
1. `grep -n 'VtuGuard not yet implemented' crates/executor/tests/merge_vtu_test.rs` 输出 **0 行**；`cargo test -p sqlrustgo-executor --test merge_vtu_test 2>&1 | tail -1` 含 `test result: ok`。
2. `grep -n 'rows.len() <= 6' tests/integration/tpch/tpch_wire_smoke_sf.rs` 输出 **0 行**；`grep -n 'rows.len() > 0' tests/integration/tpch/tpch_wire_smoke_sf.rs` 输出 **≥ 1 行**。
3. `grep -c 'is_e2e_disabled' tests/e2e/e2e_beta_test.rs` 输出 **0**（baseline 5）；`CI=1 cargo test --test e2e_beta_test -- --ignored 2>&1 | grep -c 'test result: ok'` 输出 **≥ 1**。
4. ignore_registry cross-check（**字段名必须用 `ignored_tests` 不是 `files`**）:`python3 -c "import json; d=json.load(open('tests/baseline/ignore_registry.json')); tests=d.get('ignored_tests',[]); stale_keys=['tx_wal_contract','soak_test','dml_integration_test','stored_proc_catalog','tpch_q9_audit','small_executor_modules','boundary_test','union_set_operations']; stale=[t for t in tests if any(p in t.get('file','') for p in stale_keys)]; union=[t for t in tests if 'union_set_operations' in t.get('file','')]; print('stale:', len(stale), 'union:', len(union))"` 输出 **stale: 0 union: 0**（baseline: stale=37, union=3）。
5. 关闭报告：`docs/releases/v3.12.0/V312-27_anti_fab_fix_report.md`，含 diff stat + ignore_registry 前后条目数（before=74, after=37）+ 实际跑通的 `cargo test` log + sha256 + V312-24 proposal 数字校订说明。
**禁止关闭条件**: (a) 仅删除代码但未提供 `cargo test` 实际输出；(b) `d.get('files',[])` 字段名错误的脚本视为未执行；(c) 每个测试改动必须附**前后两条 `cargo test` 命令输出**。

## V312-28：SQL Corpus 激活（V312-24 Phase 5 follow-up）

**优先级**: P0
**目标**: 维持 `sql_corpus/` ≥ 80% pass-rate，补 14 个 subcategory 守护 test，校订 V312-24 proposal 过时数字。
**来源**: V312-24 tasks.md Phase 5 + proposal.md §4 inactivated corpus。
**Owner**: opencode-z440
**Expiry**: 2026-09-30
**Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-28_baseline_evidence.txt`（**实测 2026-08-09**：99.4% pass-rate / 14 subcategories / 103 .sql files / 818 cases / 5 failing — V312-24 proposal §4 写的"16 subcategories / 27.3% / 6/16 PASS"严重过时）
**关闭边界**（**全部满足**才允许关闭）:
1. `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test test_sql_corpus_all -- --nocapture > /tmp/corpus_final.txt 2>&1`；`grep -E 'Pass rate' /tmp/corpus_final.txt | tail -1` 数字 **≥ 80.0**（baseline 99.4）。
2. `python3 -c "import re; t=open('/tmp/corpus_final.txt').read(); m=re.search(r'Pass rate:\s*([\d.]+)%', t); print(m.group(1) if m else 'NOT FOUND')"` 输出数字 ≥ 80.0。
3. 14 个 subcategory（**实际 14 个顶层目录，不是 V312-24 proposal 写的 16**）各加 1 个 `#[test]` 守护（命名约定 `test_corpus_subcategory_<name>` 或加 `// subcat: <NAME>` 注释）；`cargo test -p sqlrustgo-executor` 跑 14 个守护 test 全 PASS。
4. 关闭报告 `docs/releases/v3.12.0/V312-28_corpus_activation_report.md` 含 baseline 数字 + final 数字 + 14 个守护 test 名列表 + 实际 `cargo test` 输出 + V312-24 proposal 校订说明（16→14, 27.3%→99.4%）+ sha256。
5. **如 baseline 99.4% 反而 regression 到 < 80%**：必须走 fallback，写 `V312-28_corpus_threshold_attestation.md` 含 owner + expiry + replacement-gate + ISSUES_PLAN §V312-24 acceptance 复审记录。
**禁止关闭条件**: (a) 仅靠"打开了 follow-up 任务"或"修了一部分 subcategory"；(b) 不接受"V312-24 proposal 27.3% 是 baseline" 之类的过时引用；(c) 14 个守护 test 必须有可识别的命名或注释才能算 PASS；(d) 无 sha256 不允许关闭。

## V312-29：Gate Enforcement 接线（V312-24 Phase 6 follow-up）

**优先级**: P0
**目标**: 把 V312-24 激活的 sqlancer + test-runner 接进 4 个 gate，去 `|| true` 掩盖并把 B10_SQLANCER 从 `check_warn` 升级到 `check_fail`。
**来源**: V312-24 tasks.md Phase 6。
**Owner**: minimax
**Expiry**: 2026-08-30
**Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-29_baseline_evidence.txt`（实测：run-regression:111-112 `\|\| true`；B10_SQLANCER `check_warn` @ beta_gate:363；R4 列表 8 个脚本）
**关闭边界**（**全部满足**才允许关闭）:
1. `grep -n '|| true' scripts/test/run-regression.sh | grep -i sqlancer` 输出 **0 行**；同文件 sqlancer 调用行紧跟 `test -s target/sqlancer-report.json || { echo "missing report"; exit 1; }`。
2. `bash scripts/gate/check_beta_gate.sh 2>&1 | grep B10_SQLANCER` 输出含 `check_fail`（baseline 是 `check_warn`）；`bash scripts/gate/check_beta_gate.sh` 退出 **0** 且日志含 `B10_SQLANCER PASS`。
3. `scripts/gate/check_rc_gate_v3.10.0.sh` 内 R4 substring match 列表只剩 4 个 active script（alter_rename / rollback_mvcc / union_set_ops / e2e_runner_exec，baseline 是 8 个含将退休脚本）；`grep -E 'alter_rename|rollback_mvcc|union_set_ops|e2e_runner_exec' scripts/gate/check_rc_gate_v3.10.0.sh | wc -l` 输出 **≥ 4**。
4. `scripts/gate/check_gate_test_integrity.sh` 加 `\|\| true` after `cargo test` 扫描；**V312-29 期望 P16 退出 ≠ 0**（因为 pre-existing 有 29 个 `\|\| true` 掩盖 + 1 个新 `#[ignore]` regression；fail-explicit 替代 fail-silent 是 V312-29 目标）。关闭证明：P16 实际跑 + 列出 29 个 `\|\| true` 命中位置（实测见 V312-29_gate_wiring_report.md）。
5. 关闭报告：`docs/releases/v3.12.0/V312-29_gate_wiring_report.md`，含 4 个 gate 的执行 log + diff stat + sha256。
## V312-30：V312-24 PR 关闭 + 签收（V312-24 Phase 7.3 + 8 follow-up）

**优先级**: P0
**目标**: PR `feature/v312-24-impl` 合入后写 ISSUE #3911 comment + 完成 sign-off。
**来源**: V312-24 tasks.md Phase 7.3 + Phase 8。
**Owner**: minimax
**Expiry**: 2026-09-05
**前置依赖**: V312-25 ~ V312-29 中任意未完成项不可关闭 V312-24。
**Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-30_baseline_evidence.txt`（实测：PR 未开；G2_test_count.txt 显示 2,666 lib tests 2 FAIL pre-existing；B10_SQLANCER `check_warn`；composite hash `956eaf4d5ea148428d6432af0489f5e25bb0acf58da886bcb0ff03cdf4560286`）
**关闭边界**（**全部满足**才允许关闭）:
1. `gh pr view <PR_NUMBER> --json state,mergedAt` 输出 `state: MERGED` + `mergedAt` 非空（baseline: PR 未开，0 entries）。
2. ISSUE #3911 评论含：Phase 1 12 tasks + 10 integration tests + 20 lib tests 全 PASS 的实际数字（baseline 已记录）、5 个 V312-25~29 follow-up issue 编号 + 链接、**重新计算的** evidence_hash（关闭时再算一次，不允许用 baseline 数字）。
3. Phase 8.1-8.6 全跑通：
   - `cargo test --workspace --no-fail-fast` 退出 0（baseline 有 2 pre-existing FAIL 在 mysql-server，**必须确认这 2 个 FAIL 仍允许 non-blocking**）
   - `cargo clippy --all-features -- -D warnings` 退出 0（baseline 有 2 pre-existing errors 在 sqlrustgo-storage，**必须确认已合并修复**或豁免）
   - `cargo fmt --check` 退出 0
   - `bash scripts/gate/check_anti_fabrication.sh` 退出 0
   - `bash scripts/gate/check_beta_gate.sh` B10_SQLANCER 通过
   - 至少 2 名 reviewer 在 PR 上写了 APPROVED
4. 关闭报告 `docs/releases/v3.12.0/V312-30_signoff_report.md` 含 6 项 gate 命令的实际输出 + reviewer 名单 + evidence_hash（**重新计算**）+ sha256。
**禁止关闭条件**: (a) 不允许"openspec 标 done"、"报告标题写已完成"、"PR 已合并"作为关闭证据；(b) 不允许用 baseline evidence_hash 顶替关闭时重算的 hash；(c) 2 pre-existing FAIL/errors 在 mysql-server / storage 若仍未修，V312-30 必须显式列在豁免清单（带 owner + expiry）。

## 附录：英文原文

> 本附录保留本文件改写前的英文原文，便于追溯历史语义；当前正式阅读与执行口径以上方中文正文为准。

# SQLRustGo v3.12.0 Issues Plan

> **Version**: v3.12.0
> **Status**: DRAFT
> **Date**: 2026-08-09

This file breaks the v3.12.0 plan into issue-sized work packages. Every item must produce execution evidence before it can be marked complete.

## V312-01: Prior-Release Blocker Disposition

**Priority**: P0

**Goal**: Ensure v3.12.0 does not inherit hidden v3.11.0 production weak points.

**Scope**:
- Reconcile v3.11.0 G3 coverage.
- Reconcile v3.11.0 G4 TPC-H SF=1.
- Decide v3.11.0 168h SOAK status.
- Reconcile `debt-registry.yaml` against v3.11.0 truth audit, especially F-25/F-26 and extension crate states.

**Exit evidence**:
- Updated blocker disposition report.
- Stage gate output.
- Evidence hashes for any PASS claim.

## V312-02: GMP Schema v3.12

**Priority**: P0

**Goal**: Define SQLRustGo-managed GMP tables for documents, chunks, embeddings, audit logs, and relations.

**Acceptance**:
- Schema creation is idempotent.
- Document version history is preserved.
- Audit rows include previous hash and event hash.
- Relation rows can model SOP, clause, CAPA, deviation, role, and equipment relationships.

## V312-03: GMP Corpus Ingestion

**Priority**: P0

**Goal**: Import `~/gmp-platform/gmp-md` into SQLRustGo-managed GMP tables.

**Acceptance**:
- Full corpus ingestion report includes document count, chunk count, relation count, embedding count, skipped files, and failures.
- Re-ingestion is idempotent.
- Modified source files create new document versions.
- Source path and source hash are preserved.

## V312-04: Embedding Provider and Vector Persistence

**Priority**: P0

**Goal**: Replace required Chroma runtime dependency for the GMP production path with SQLRustGo-managed embedding storage and vector index rebuild.

**Acceptance**:
- Embeddings are stored in SQLRustGo tables.
- Fixed fixture can rebuild a Flat or HNSW index.
- Model name, dimension, vector hash, and chunk id are recorded.
- Embedding provider abstraction supports BGE-M3/Ollama-compatible flows.

## V312-05: Hybrid Retrieval

**Priority**: P0

**Goal**: Implement GMP internal-audit retrieval using SQL filters, keyword score, vector similarity, graph relation boost, and RRF fusion.

**Acceptance**:
- Every result includes source path, document id, version, chunk id, chunk hash, score components, and citation text.
- Results can be filtered by document type, chapter, status, effective date, and relation type.
- Fixed audit question fixture produces deterministic results.

## V312-06: SQL-Backed GMP Graph Projection

**Priority**: P0

**Goal**: Provide graph navigation for GMP evidence without claiming a general graph database.

**Acceptance**:
- Nodes and edges are stored in SQLRustGo tables.
- Supports neighbors and depth-limited paths up to depth 3.
- Supports relation filters.
- Does not require archived `graph` crate as unvalidated production dependency.

## V312-07: Compliance and Data Integrity Controls

**Priority**: P0

**Goal**: Implement ALCOA+ and GMP-relevant controls for internal-audit retrieval.

**Acceptance**:
- Audit hash-chain tamper tests fail closed.
- Role-based access tests cover import, approve, search, export, and audit review.
- Electronic-signature hooks exist for approval and controlled export.
- Compliance matrix maps tests to controls.

## V312-08: Backup and Restore

**Priority**: P0

**Goal**: Verify backup/restore of SQL, GMP documents, embeddings, graph projection, and audit chain.

**Acceptance**:
- Restored counts match source counts.
- Restored hashes match source hashes.
- Vector index can rebuild after restore.
- Audit chain verifies after restore.

## V312-09: Mixed Workload SOAK

**Priority**: P0

**Goal**: Run a 168h workload representative of `~/gmp-platform`.

**Workload**:
- SQL reads/writes.
- GMP corpus import and incremental update.
- Hybrid retrieval.
- Graph traversal.
- Audit export.
- Backup/restore smoke.

**Acceptance**:
- 168h complete.
- 0 crash.
- No audit-chain break.
- No unclassified data loss.

## V312-10: Documentation and Operations

**Priority**: P1

**Goal**: Provide production operation docs for GMP internal-audit retrieval.

**Acceptance**:
- User operations guide exists.
- Compliance matrix signed off.
- Migration guide from SQLite/Chroma/PostgreSQL prototypes exists.
- GA gate report contains only evidence-backed PASS claims.

## V312-11: SQLite SQLLogicTest Oracle Gate

**Priority**: P0

**Goal**: Finish the SQLite automatic testing framework planned in v3.10.0 and left non-blocking/TBD in v3.11.0.

**Background**:
- `docs/releases/v3.10.0/TESTING_SYSTEM_BETA_REPORT.md` proposed SQLLogicTest against SQLite's official corpus as the P0 oracle path.
- `docs/releases/v3.10.0/plans/V310_ISSUES_PLAN.md` V310-14 recorded a runner implementation but a blocked official-suite download.
- `crates/sqlrustgo_sqllogictest` exists and currently has 22 local `.test` files.
- `docs/releases/v3.11.0/RELEASE_GATE_CHECKLIST.md` still had `sqllogictest runner all targets` as TBD.

**Scope**:
- Normalize the canonical runner path to `crates/sqlrustgo_sqllogictest`.
- Build and run the existing smoke corpus.
- Create a reproducible SQLite official SQLLogicTest corpus acquisition/cache plan.
- Add a manifest with upstream snapshot, file count, hash list, skipped files, and exclusion reasons.
- Add or plan `scripts/gate/check_sqllogictest_v312.sh`.
- Save outputs under `docs/releases/v3.12.0/sqllogictest-baseline/` and `docs/releases/v3.12.0/logs/`.

**Acceptance**:
- `cargo build -p sqlrustgo_sqllogictest` succeeds.
- `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` produces a smoke report.
- The selected SLT corpus has PASS/FAIL/SKIP classification.
- Every skip/fail group has an issue, owner, expiry, and rationale.
- v3.12.0 GA cannot pass while this item is still TBD.

## V312-12: TPC-H SF=1 Correctness Close-Out

**Priority**: P0

**Goal**: Turn v3.11.0's TPC-H SF=1 22/22可运行性证据 into correctness evidence.

**Scope**:
- Re-run all 22 SF=1 queries from the same fixture.
- Capture SQLRustGo row counts and sorted result SHA256.
- Capture SQLite/PostgreSQL/MySQL or MariaDB reference row counts and SHA256 where supported.
- Explain every zero-row query with data, not prose-only reasoning.
- Store per-query artifacts and summary report.

**Acceptance**:
- 22/22 query artifacts exist.
- No unexplained checksum mismatch.
- No unexplained zero-row result.
- `TPCH_SF1_VERIFICATION_REPORT.md` no longer depends on a future PG SHA256 task for its core correctness claim.

## V312-13: MySQL Wire Protocol and LOAD DATA Hardening

**Priority**: P0

**Goal**: Close the v3.11.0 production-readiness gap around MySQL compatibility and data import.

**Scope**:
- Add wire e2e coverage for COM_QUERY, COM_STMT_PREPARE, COM_STMT_EXECUTE, COM_STMT_CLOSE, error packets, reset connection, TLS, and compression boundaries.
- Add `LOAD DATA LOCAL INFILE` or clearly-scoped alternative bulk import tests.
- Measure SF=1 and, where feasible, SF=10 import time, memory peak, row counts, and hashes.
- Fail closed on silent truncation, type conversion drift, or out-of-memory behavior.

**Acceptance**:
- Wire protocol e2e report exists.
- LOAD DATA/bulk-import report exists with count/hash verification.
- mysql-server and mysql-client coverage trend improves or has explicit non-blocking rationale.

## V312-14: Crash Recovery and Upgrade/Downgrade Verification

**Priority**: P0

**Goal**: Add the recovery evidence needed before SQLRustGo can be promoted beyond controlled production.

**Scope**:
- Run kill -9 / restart tests against mixed read/write workloads.
- Verify WAL replay, dirty page recovery, and audit-chain continuity.
- Back up and restore SQL data, GMP documents, embeddings, graph projection, and audit chain.
- Run v3.10.0/v3.11.0 fixture upgrade to v3.12.0 and rollback/downgrade where supported.
- Store count/hash reports for every recovery and upgrade path.

**Acceptance**:
- Recovery report shows count/hash equality after restart.
- Restore report shows count/hash equality in a clean data directory.
- Upgrade report shows old data readable under v3.12.0.
- Rollback limitations are explicitly documented if full downgrade is not supported.

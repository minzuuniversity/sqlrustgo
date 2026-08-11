# SQLRustGo v3.12.0 综合测试框架与覆盖率基线

> **生成时间**: 2026-08-11 20:35 CST  
> **生成者**: codex  
> **source_run**: v312-test-framework-coverage-baseline-20260811  
> **基线分支**: `origin/develop/v3.12.0`  
> **基线 commit**: `d2fcca56f7bd3259476c9d926b2e45183089a995`  
> **覆盖率工具**: `cargo-llvm-cov`  
> **正式口径**: `cargo llvm-cov -p <crate> --all-features --tests --ignore-run-fail --json --summary-only`

## 1. 结论

v3.12.0 不应继续沿用“全 workspace 一次性覆盖率 ≥80%”作为 Alpha/日常开发硬门禁。该方式在历史版本中多次产生口径漂移、超时、局部 `--lib` 结果冒充全量结果等问题。

推荐采用分层测试框架：

| 层级 | 测试类型 | 触发频率 | 阻断策略 |
|---|---|---|---|
| L0 | 单元测试 | 每 PR | 必须 PASS；只跑受影响 crate 和核心 crate |
| L1 | 集成测试 | 每 PR / 合并前 | P0 路径必须 PASS；长耗时测试分组 |
| L2 | E2E / wire / SQLLogicTest smoke | 每日 / 合并后 | 允许 issue-linked exclusions，但不得静默忽略 |
| L3 | per-crate 覆盖率 | 夜间 / Beta / RC | 按 crate 统计；低覆盖必须有 owner、expiry、issue |
| L4 | 性能测试 | 夜间 / RC | TPC-H、Sysbench、bulk-load、vector/RAG benchmark 只做趋势和阈值，不混入覆盖率 |
| L5 | SOAK / crash / recovery | Beta 后 / RC 前 | 长稳和恢复单独验收，不作为每 PR 门禁 |

当前 16 个核心/生产相关 crate 中，9 个已达到 80% line coverage；7 个未达到 80%，但其中 mysql-server、vector、parser、storage 的 coverage run 暴露出测试失败或 ignore，需要先治理测试健康度，再追覆盖率数字。

## 2. 历史版本覆盖率要求梳理

| 版本 | 覆盖率要求或实际口径 | 当前判断 |
|---|---|---|
| v3.6.0 | Alpha A5 L1 8 crates 平均 ≥75%；Beta/RC/GA 曾写 L1 ≥85%；同时存在 Z6G4 与 Z440 覆盖率大幅不一致 | 历史目标过硬且口径不稳；不能直接继承为 v3.12 日常门禁 |
| v3.7.0 | Beta 记录 L1 平均 84.99% ≥75%；GA 报告写平均 ≥85%、每 crate ≥75%，但 84.99% 被视为误差范围 | 可作为“平均值接近阈值不能等于严格 PASS”的反例 |
| v3.8.0 | 强调测试质量治理、历史债务、WAL/恢复/兼容测试迁移 | 覆盖率必须和功能债务、禁用测试一起看 |
| v3.9.0 | 长稳、ignored tests、SOAK 真实性成为重点 | `#[ignore]` 不能被当作 PASS；需要 P12/P16 管理 |
| v3.10.0 | R6 coverage baseline、SQLancer/test-runner/test-registry、E2E scripts 仍有骨架和 TBD | v3.12 必须把测试基础设施从“存在”推进到“可运行、有报告” |
| v3.11.0 | G3 曾要求每 crate ≥80%，但文档中存在 `--lib` 与 `--tests` 混用；后续方法论建议 per-crate 测量 | v3.12 继承的是“统一口径 + per-crate 报告”，不是继承旧 PASS 结论 |
| v3.12.0 | `TEST_PLAN.md` V312-G18：parser/mysql-server/mysql-client ≥80% 或 issue-linked exception；V312-G19 性能和观测性 baseline | Alpha 阶段应先建立正确统计框架，Beta/RC 再逐步提升阈值 |

## 3. 正确覆盖率统计口径

### 3.1 推荐命令

```bash
bash scripts/gate/check_v312_coverage_baseline.sh
```

单 crate 等价命令：

```bash
timeout 120 cargo llvm-cov \
  -p <crate> \
  --all-features \
  --tests \
  --ignore-run-fail \
  --json \
  --summary-only \
  --output-path docs/releases/v3.12.0/coverage-baseline/<crate>.json
```

### 3.2 必须同时记录的字段

| 字段 | 原因 |
|---|---|
| line coverage | 覆盖率主指标 |
| covered/total lines | 避免只看百分比 |
| function coverage | 补充判断测试深度 |
| run seconds | 识别不适合 PR 门禁的慢 crate |
| test health | coverage JSON 生成不代表测试全部 PASS |
| ignored/failed count | 避免 ADR-008 违规 |
| command、commit、timestamp、log path | 满足 ADR-001/AFP 证据要求 |

### 3.3 禁止口径

| 禁止方式 | 原因 |
|---|---|
| 用 `cargo llvm-cov --lib` 冒充全量覆盖率 | 跳过 `tests/*.rs` 集成测试，容易低估或高估 |
| 用全 workspace 单次运行作为每 PR 硬门禁 | 容易超时，且失败定位差 |
| 只报 PASS/FAIL，不报测试实际运行数量 | 会掩盖 `#[ignore]` 和 report-only failure |
| 性能测试计入覆盖率阈值 | 性能测试目标是趋势和容量，不是覆盖率 |
| 对 mysql-server/vector 直接设 80% Alpha 阻断 | 当前仍有 test failure/ignore，先修测试健康度更有效 |

## 4. 当前 v3.12.0 覆盖率基线

本表为 2026-08-11 在 `d2fcca56f7` 上实跑结果。命令使用 `--tests --ignore-run-fail`，所以 “coverage 已生成” 不等于 “测试全部 PASS”。

| Crate | Line% | Lines | Functions | 耗时 | 测试健康 | 阶段判断 |
|---|---:|---:|---:|---:|---|---|
| sqlrustgo-rag | 95.67% | 1304/1363 | 161/179 | 9.1s | PASS | GA 可维持 |
| sqlrustgo-server | 87.28% | 1091/1250 | 149/183 | 10.6s | PASS | GA 可维持 |
| sqlrustgo-optimizer | 87.52% | 3205/3662 | 393/416 | 4.6s | PASS | GA 可维持 |
| sqlrustgo-planner | 86.75% | 1322/1524 | 171/212 | 12.1s | PASS | GA 可维持 |
| sqlrustgo-transaction | 85.41% | 1821/2132 | 257/308 | 10.6s | PASS | GA 可维持 |
| sqlrustgo-admin | 85.33% | 1413/1656 | 145/170 | 14.1s | PASS | GA 可维持 |
| sqlrustgo-catalog | 84.66% | 3036/3586 | 386/478 | 7.6s | PASS | GA 可维持 |
| sqlrustgo-storage | 84.33% | 13985/16583 | 1632/1986 | 28.7s | report-only：`--lib` target failed | 先修测试健康度 |
| sqlrustgo-executor | 83.05% | 11782/14187 | 1379/1569 | 65.1s | PASS | GA 可维持，但耗时偏高 |
| sqlrustgo-tools | 76.72% | 2066/2693 | 208/245 | 18.0s | PASS | Beta 目标 78%，RC 目标 80% |
| sqlrustgo-gmp | 76.03% | 4657/6125 | 465/626 | 15.1s | PASS | GMP 生产路径需优先补到 80% |
| sqlrustgo-sql-corpus | 74.43% | 687/923 | 43/79 | 6.7s | PASS | 先补 corpus negative/error path |
| sqlrustgo-mysql-client | 73.84% | 1033/1399 | 99/112 | 4.0s | PASS | RC 前补到 78%-80% |
| sqlrustgo-parser | 70.93% | 7328/10332 | 752/827 | 20.8s | report-only：`parser_coverage` target failed | 先修 target，再补覆盖 |
| sqlrustgo-vector | 70.62% | 2286/3237 | 316/447 | 118.9s | report-only：1 failed、6 ignored | 不适合每 PR 全量跑 |
| sqlrustgo-mysql-server | 69.37% | 3030/4368 | 346/430 | 40.0s | report-only：`--lib` 和 `wire_smoke_mysql_cli` failed | 先修 wire/e2e，再追覆盖 |

## 5. 模块分组与测试策略

| 模块组 | Crates / 路径 | 单元测试 | 集成测试 | E2E | 性能测试 |
|---|---|---|---|---|---|
| SQL 前端 | parser、planner、optimizer、sql-corpus | AST、表达式、rewrite、cost model | SQL corpus、SQLLogicTest selected files | CLI/embedded SQL smoke | parse/plan latency microbench |
| 执行引擎 | executor、root sqlrustgo | expression、join、window、DML helper | TPC-H correctness、setops、transaction visible path | MySQL client/server query flow | TPC-H Q4/Q21、parallel/SIMD baseline |
| 存储/事务 | storage、transaction、catalog | page、buffer、WAL、MVCC、catalog auth | crash/recovery、upgrade、backup/restore | kill -9、restore checksum | bulk-load、scan、index lookup |
| MySQL 兼容 | mysql-client、mysql-server、admin | packet、auth、prepared、reset | wire protocol fixtures | `e2e_wire_protocol`、mysqladmin、LOAD DATA | sysbench OLTP、connection churn |
| GMP/RAG/Vector/Graph | gmp、rag、vector、SQL-backed graph projection | chunk、embedding、tokenizer、ACL、audit hash | GMP corpus import、retrieval fixture、graph projection | GMP 内审检索端到端 | vector index rebuild、hybrid retrieval latency |
| 观测性/运维 | server、tools、telemetry、security | config、metrics、audit、backup tools | Prometheus scrape、slow query log | admin workflow | soak、memory growth、backup duration |

## 6. 分阶段目标

### Alpha

目标是建立可信测试体系，不追求所有 crate 80%。

| 项 | 目标 |
|---|---|
| 单元测试 | 受影响 crate + L1 核心 crate 必须可运行 |
| 集成测试 | SQLLogicTest smoke gate 可运行，失败必须进入 exclusions.yml 并关联 issue |
| E2E | MySQL wire 当前允许 9 ignored，但必须由 #4025 跟踪，不得写成 46/46 PASS |
| 覆盖率 | 建立 per-crate baseline；所有低于 75% 或 report-only failure 项进入 issue |
| 性能 | TPC-H/Sysbench/Vector/RAG 只要求 baseline 脚本和数据集说明，不要求最终阈值 |

### Beta

| 项 | 目标 |
|---|---|
| 单元测试 | L1/L2 crate 默认测试无新增 failure |
| 集成测试 | SQLLogicTest 本地 smoke 失败数持续下降；关键 GMP SQL subset PASS |
| E2E | wire ignored 从 9 降到 0 或形成正式延期决议 |
| 覆盖率 | L1 平均 ≥80%；每个 P0 crate ≥70%；GMP ≥78%；报告耗时 <120s 或拆夜间 |
| 性能 | SF=1 正确性、SF=10 dry run、Sysbench smoke、bulk-load smoke 有可复跑 artifact |

### RC

| 项 | 目标 |
|---|---|
| 单元测试 | 所有 tracked crate coverage run 不得出现 report-only target failure，例外需 ADR/Issue |
| 集成测试 | curated SQLLogicTest subset 通过；剩余 exclusion 均有 owner/expiry |
| E2E | `e2e_wire_protocol` 不再有 gate-path ignored；LOAD DATA/TLS/compression 有明确边界 |
| 覆盖率 | parser、executor、storage、planner、optimizer、catalog、transaction、gmp、rag ≥80%；mysql-client ≥78%；mysql-server ≥72% 且持续上升 |
| 性能 | TPC-H SF=10、Sysbench、bulk-load、RAG/vector benchmark 输出趋势报告 |

### GA

| 项 | 目标 |
|---|---|
| 单元测试 | P0/P1 crate 全部 PASS，无未解释 ignored gate test |
| 集成测试 | SQLLogicTest selected targets PASS；剩余 incompatible corpus 明确 unsupported/deferred |
| E2E | MySQL 5.7 替代声明涉及的 wire/DDL/DML/transaction 路径必须 PASS |
| 覆盖率 | 生产路径 crate 原则上 ≥80%；若 mysql-server/vector 未达到 80%，必须有正式风险接受、范围限制和 v3.13 issue |
| 性能 | 只承诺实测过的 workload；不得用 TPC-H/Sysbench 局部结果外推到所有生产负载 |

## 7. 优先补测建议

| 优先级 | 模块 | 当前缺口 | 建议动作 |
|---|---|---|---|
| P0 | mysql-server | 69.37%，且 coverage run 有 failed target；wire 仍 9 ignored | 先关闭 #4025，再补 packet/error/reset/prepared/LOAD DATA 单元和 E2E |
| P0 | parser | 70.93%，且 `parser_coverage` target failed | 修复 failing target；补 INSERT/SETOPS/LIMIT/window/CTAS SQL surface 单元测试 |
| P0 | gmp | 76.03% | 补 GMP import、ACL、audit hash-chain、evidence bundle、backup/restore fixture |
| P0 | vector | 70.62%，耗时 118.9s，1 failed/6 ignored | 拆 fast unit 与 slow benchmark；生产路径只 gate Flat/HNSW rebuild smoke |
| P1 | mysql-client | 73.84% | 补 packet decode、prepared statement、error packet、reset/compression 边界 |
| P1 | tools/sql-corpus | 76.72% / 74.43% | 补异常路径、报告生成、manifest 校验和 parser error 分类 |

## 8. 性能测试框架

性能测试不应混入覆盖率门禁。建议采用独立 artifact：

| 性能项 | Alpha | Beta | RC | GA |
|---|---|---|---|---|
| TPC-H SF=1 correctness | 脚本存在、fixture manifest | 22 query row-count | cross-engine checksum | 只声明已验证 query |
| TPC-H SF=10 | 数据集路径和导入脚本 | dry run / subset | 22 query duration + memory | 趋势对比，不设不现实 QPS |
| Sysbench OLTP | schema smoke | point-select/read-write smoke | latency/QPS baseline | 和历史基线比较 |
| Bulk-load | row-count smoke | SF=1 import | SF=10 import + memory cap | duration/hash trend |
| RAG/vector | fixed top-k fixture | rebuild + query latency | hybrid retrieval benchmark | GMP 内审问题集稳定性 |
| Graph projection | node/edge count | depth<=3 query | path latency and correctness | evidence bundle trace |
| SOAK | 不跑 | 24h smoke | 72h/168h | 0 crash、hash-chain 不断裂 |

## 9. 关闭条件

V312-G18 / #3904 关闭前至少满足：

1. 运行 `scripts/gate/check_v312_coverage_baseline.sh` 并保存报告。
2. 覆盖率表包含所有 tracked crate 的 line%、lines、functions、耗时、test health。
3. 低于目标或出现 report-only failure 的 crate 均有关联 issue、owner、expiry。
4. P12/P16 显示无新增静默 ignore；gate test ignore 必须有 ADR-008 exception。
5. 不再引用 v3.6-v3.11 的历史 PASS claim 作为当前 v3.12 PASS 证据。


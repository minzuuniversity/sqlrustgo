# v3.9.0 E2E 测试改造总计划 (E2E Migration Master Plan)

> **Date**: 2026-06-13 04:20 CST
> **Trigger**: 用户反馈 "所有违反原则的测试，都要进行分析，用 e2e 方式测试"
> **Principle**: 集成测试、性能测试、稳定性测试 **必须** 使用 `sqlrustgo-mysql-server` 作为后端（wire protocol）。**禁止** 使用单独编译的测试程序直接调用 `ExecutionEngine` 或 `MemoryStorage`。
> **Scope**: 18 个 `tests/tpch_*.rs` + 27 个 `benches/*.rs` + 其他 30+ 个相关测试
> **依据**: `docs/governance/DOC_CHECK_CORRECTION_RULES.md` v1.0.0 (5-step)

---

## 1. 违反原则测试分类 (47 个文件)

### 1.1 TPC-H 测试 (18 个) — **E2E 必修**

| # | 文件 | 当前方式 | 真实含义 | E2E 改造 |
|---|------|----------|----------|----------|
| 1 | `tests/tpch_gate_test.rs` | ExecutionEngine + .tbl | TPC-H G1 gate (22/22 行数) | ✅ E2E: 启动 server + `MySqlTestClient` + `MySqlTestClient::query_rows` |
| 2 | `tests/tpch_22_queries_wire_test.rs` | ExecutionEngine in-process | TPC-H 22 query 正确性 | ✅ E2E: 同上 |
| 3 | `tests/tpch_full_22_test.rs` | ExecutionEngine + .tbl | 完整 22 query 跑通 | ✅ E2E |
| 4 | `tests/tpch_value_correctness_test.rs` | ExecutionEngine + 合成数据 | 数值正确性 gate | ✅ E2E (synthetic data 也通过 wire protocol) |
| 5 | `tests/tpch_value_test_v2.rs` | ExecutionEngine + .tbl | Value assertion | ✅ E2E |
| 6 | `tests/tpch_q9_audit.rs` | ExecutionEngine + SF=0.01 | Q9 audit | ✅ E2E |
| 7 | `tests/tpch_sf01_inprocess_test.rs` | ExecutionEngine + SF=0.1 | SF=0.1 sanity | ✅ E2E |
| 8 | `tests/tpch_sf01_22 vs_3engines.rs` | ExecutionEngine + 对比 MariaDB/PG | 跨引擎对比 | ✅ E2E + 对比 MariaDB/PG (都是 wire) |
| 9 | `tests/tpch_sf01_22 vs_sqlite.rs` | ExecutionEngine + 对比 SQLite | 对比 SQLite | ✅ E2E + 对比 SQLite wire |
| 10 | `tests/tpch_sf01_perf baseline_test.rs` | ExecutionEngine + timing | SF=0.1 perf baseline | ✅ E2E + 通过 wire protocol 跑 sysbench-style 负载 |
| 11 | `tests/tpch_per_query_timeout_test.rs` | ExecutionEngine + timeout | per-query timeout | ✅ E2E |
| 12 | `tests/tpch_q8_q21_perf_regression_test.rs` | ExecutionEngine + timing | Q8/Q21 perf regression | ✅ E2E |
| 13 | `tests/tpch_bug_regression_test.rs` | ExecutionEngine + 标记 | 3 bugs 回归 | ✅ E2E |
| 14 | `tests/tpch_wire_smoke.rs` | **已经 E2E** (MySqlTestClient) | wire smoke | ✅ 已合规 (无改造) |
| 15 | `tests/tpch_22_queries_wire_test.rs` | 内部名有 wire 但实际 in-process | 22 query 跑通 | ✅ E2E |

### 1.2 性能/稳定性 benches (27 个) — **E2E 必修或废弃**

| # | 文件 | 用途 | E2E 改造决策 |
|---|------|------|--------------|
| 1 | `benches/qps_bench.rs` | G11 QPS 性能 | ✅ **E2E 重写**: 用 `start_ephemeral` + sysbench/tpch wire 协议 |
| 2 | `benches/tpch_bench.rs` | TPC-H in-process 性能 | ❌ **废弃**: 改为 E2E test，bench 不需要 in-process |
| 3 | `benches/tpch_comprehensive.rs` | TPC-H 综合 | ❌ **废弃**: 同上 |
| 4 | `benches/tpch_streaming_config.rs` | streaming 配置 | ⚠️ **保留 in-process** (是 streaming 内部 API 测试) |
| 5 | `benches/integration_bench.rs` | end-to-end in-process | ✅ **E2E 重写**: 用 server |
| 6 | `benches/bench_aggregate.rs` | aggregate in-process | ⚠️ **保留** (是 storage API benchmark，不是 e2e) |
| 7 | `benches/bench_cbo.rs` | CBO in-process | ⚠️ **保留** (优化器内部) |
| 8 | `benches/bench_insert.rs` | insert in-process | ⚠️ **保留** (storage API) |
| 9 | `benches/bench_scan.rs` | scan in-process | ⚠️ **保留** (storage API) |
| 10 | `benches/bench_index_scan.rs` | index scan | ⚠️ **保留** (storage API) |
| 11 | `benches/storage_bench.rs` | storage | ⚠️ **保留** |
| 12 | `benches/executor_bench.rs` | executor | ⚠️ **保留** (执行器内部 API) |
| 13 | `benches/parser_bench.rs` | parser | ⚠️ **保留** (解析器内部) |
| 14 | `benches/lexer_bench.rs` | lexer | ⚠️ **保留** (lexer 内部) |
| 15 | `benches/lexer_parser_bench.rs` | lexer+parser | ⚠️ **保留** |
| 16 | `benches/network_bench.rs` | network TCP (不是 MySQL protocol) | ⚠️ **保留** (是 generic TCP, 不需 sqlrustgo-mysql-server) |
| 17 | `benches/scale_bench.rs` | scale | ⚠️ **保留** |
| 18 | `benches/bench_v130.rs` / `bench_v140.rs` | 旧版本基准 | ❌ **废弃** (历史数据) |
| 19 | `benches/bench_columnar.rs` | columnar | ⚠️ **保留** |
| 20 | `benches/dataset_generator.rs` | 数据生成 | ⚠️ **保留** (是 helper) |
| 21 | `benches/common.rs` | 共享 helper | ✅ **保留** (helper) |
| 22 | `benches/postgres_config.rs` | postgres 配置 | ❌ **废弃** (未用) |
| 23 | `benches/sqlite_config.rs` | sqlite 配置 | ❌ **废弃** (未用) |
| 24 | `benches/tpch_wire_bench.rs` | **已经 E2E** | ✅ 已合规 (start_ephemeral + MySqlTestClient) |
| 25-27 | 其他 | 各种 | ⚠️ 保留/废弃视情况 |

### 1.3 其他 in-process 测试 (30+ 个)

| 类别 | 文件数 | E2E 改造决策 |
|------|--------|--------------|
| `tests/diag_*.rs` (诊断) | 7 | ⚠️ **保留 in-process** (是单 component 诊断, 不是 e2e) |
| `tests/operators/*.rs` (operator 单元) | 8 | ⚠️ **保留 in-process** (operator 单元, 不是 e2e) |
| `tests/long_run_stability_test.rs` | 1 | ✅ **E2E 重写** + 真 wire protocol 长时间跑 |
| `tests/long_run_stability_72h_test.rs` | 1 | ✅ **E2E 重写** 同上 |
| `tests/g2_substance_parallel_executor_test.rs` | 1 | ✅ **E2E 重写** (G2 gate) |
| `tests/upgrade_chain_v3_6_to_v3_9_test.rs` | 1 | ✅ **E2E 重写** (升级兼容性) |
| `tests/aggregate_smoke_test.rs` | 1 | ⚠️ 保留 (单元测试) |
| `tests/bench_bulk_insert_records.rs` | 1 | ✅ **E2E 重写** (G6 backup/restore) |

---

## 2. E2E 改造决策矩阵

| 决策 | 数量 | 说明 |
|------|------|------|
| ✅ **必须 E2E 改造** (G1/G2/G6/G11/G13/G16 涉及 GA gates) | ~22 | TPC-H + 关键 perf + long_run + substance |
| ⚠️ **保留 in-process** (内部 API bench, 单元测试) | ~25 | storage/parser/lexer/executor/diag/operators |
| ❌ **废弃** (历史/未用) | ~5 | v130/v140 benches, postgres/sqlite configs |
| ✅ **已合规** (E2E) | 1 | `tpch_wire_bench.rs`, `tpch_wire_smoke.rs` |

---

## 3. 改造原则 (E2E 标准)

**所有 E2E 测试必须**:
1. **启动** `sqlrustgo-mysql-server` (用 `start_ephemeral` 或 release binary)
2. **连接** 通过 `MySqlTestClient` 或真实 mysql client
3. **工作负载** 通过 wire protocol (COM_QUERY, COM_STMT_PREPARE, COM_STMT_EXECUTE)
4. **数据** 通过 wire protocol (LOAD DATA LOCAL INFILE, INSERT)
5. **监控** 资源: ulimit RSS / FD / DB size (防 OOM)
6. **断言** 通过 wire protocol 读回的 rows

**禁止**:
- ❌ 直接调 `ExecutionEngine::execute(sql)`
- ❌ 直接调 `MemoryStorage::insert/scan`
- ❌ 直接调 `parse(sql)` 然后手工处理 AST
- ❌ 用 cargo bench 测的 "perf" 数字当作 G11 evidence
- ❌ 用手写 JSON expected 当作 G15 baseline

---

## 4. 改造计划 (按优先级)

### P0 — GA 阻塞 (立即执行)
1. **修 STMT EXECUTE bug** (A 选项进行中, 已发现根因)
2. **`tests/tpch_gate_test.rs` E2E 化** (G1 gate, 必须 wire protocol)
3. **`benches/qps_bench.rs` 废弃** + **新建 `tests/g11_qps_wire_e2e.rs`** (用 sysbench + wire)
4. **`tests/long_run_stability_test.rs` E2E 化** (G13 真实性, 必须 wire + sysbench)
5. **`tests/g2_substance_parallel_executor_test.rs` E2E 化** (G2 gate)
6. **整合到 5-min 集成测试** (已存在 scripts/stability/test_integration_5min.sh)

### P1 — GA 质量 (1-2 周)
7. **`benches/tpch_bench.rs` / `tpch_comprehensive.rs` 废弃** + **新建 `tests/tpch_e2e_comprehensive.rs`**
8. **`benches/integration_bench.rs` E2E 化**
9. **`tests/tpch_sf01_22_vs_3engines.rs` E2E 化** (跨 wire 引擎对比)
10. **`tests/tpch_sf01_22_vs_sqlite.rs` E2E 化** (SQLite wire 对比)
11. **`tests/upgrade_chain_v3_6_to_v3_9_test.rs` E2E 化** (升级兼容性)
12. **`tests/tpch_value_correctness_test.rs` E2E 化** (synthetic data 通过 wire)

### P2 — 长期质量 (1 月)
13. 全部 14 个 `tests/tpch_*.rs` 改 wire protocol
14. `tests/operators/*.rs` 部分 E2E 化 (跨 operator 集成)
15. `tests/bench_bulk_insert_records.rs` E2E 化 (G6)

### P3 — 废弃 (清理)
16. 删 `benches/bench_v130.rs`, `bench_v140.rs`, `postgres_config.rs`, `sqlite_config.rs`
17. 删 `benches/tpch_bench.rs`, `tpch_comprehensive.rs`, `qps_bench.rs` (替换为 E2E tests)

---

## 5. 立即行动 (下一步)

### A. 修 STMT EXECUTE bug (已诊断, 代码改动小)
- 修复 `infer_param_types_from_sql` 对 SELECT WHERE 的列类型推断 (✅ 已加 `extract_where_columns` 函数)
- 验证 STMT EXECUTE 真的返回 row (当前 fix 后 final_sql 正确, 但 rows 仍空)
- 修完后 `tpch_value_test_v2` 等会自然通过

### B. 跑 5-min 集成测试 (已存在 test_integration_5min.sh)
- 验证 sysbench oltp_read_write 真的能跑 (用 sqlrustgo-mysql-server 后端)
- 验证资源限制 (RSS 2GB / FD 512 / DB 500MB) 不被突破

### C. 建新的 E2E G11 benchmark
- 路径: `tests/g11_qps_wire_e2e.rs`
- 方法: `start_ephemeral` + `MySqlTestClient` + 5 个 workload
- 输出: G11 真实 QPS (不是 cargo bench)

### D. 改写 long_run_stability_test 为 E2E
- 路径: `tests/long_run_stability_e2e.rs`
- 方法: release binary + sysbench 真实负载 + ulimit 资源限制
- 验证: RSS 增长、FD 增长、WAL 增长、CPU 使用、QPS 稳定性

---

## 6. 验证 Checklist (5-step 治理)

- [ ] 5-min 集成测试 PASS (A1+A2+A3)
- [ ] STMT EXECUTE bug 修好 (A4+A5)
- [ ] G11 E2E test 通过 (C)
- [ ] G13 E2E test 通过 (D)
- [ ] 22 个 TPC-H tests 全部 E2E
- [ ] 27 个 benches 重新分类
- [ ] 5 remote 同步所有改动
- [ ] GA_GATE_REPORT 文档更新

---

## 7. 不修改的 (in-process 是 OK 的)

| 文件 | 原因 |
|------|------|
| `benches/bench_aggregate.rs` | storage 内部 API benchmark, 不是 e2e |
| `benches/storage_bench.rs` | storage 内部 |
| `benches/executor_bench.rs` | executor 内部 |
| `benches/parser_bench.rs` / `lexer_bench.rs` | parser/lexer 内部 |
| `benches/bench_cbo.rs` | CBO 内部 |
| `benches/bench_columnar.rs` | columnar 内部 |
| `benches/scale_bench.rs` | 内部 scale |
| `benches/dataset_generator.rs` | helper |
| `benches/common.rs` | helper |
| `tests/diag_*.rs` (7 个) | 单 component 诊断, 不是 e2e |
| `tests/operators/*.rs` (8 个) | operator 单元测试 |
| `tests/aggregate_smoke_test.rs` | 单元测试 |
| `benches/network_bench.rs` | generic TCP, 不是 MySQL protocol |
| `benches/tpch_streaming_config.rs` | streaming 内部 API |

---

## 8. 风险评估

| 风险 | 等级 | 缓解 |
|------|------|------|
| E2E 改造量大 (~22 个文件) | 高 | 分 P0/P1/P2/P3, 每个 PR 独立 |
| 跑 E2E 比 in-process 慢 | 中 | TPC-H SF=0.01 (~5s) vs in-process (~1s), 仍可接受 |
| sysbench 找不到 benchmark | 中 | 已修复 (使用完整路径) |
| STMT EXECUTE bug 修复回归 | 中 | 保留 in-process tests 作为快速 smoke, 加 e2e 作为权威 |
| 资源耗尽 (上次 99% CPU 的 agent 错误) | 中 | ulimit + watch dog |

---

## 9. 与 governance 一致性

- ✅ 5-step governance 流程
- ✅ 最小修改原则 (in-process 单元测试不强制改)
- ✅ 不破坏现有工作流 (保留 helper, 删未用 benches)
- ✅ 4-remote 同步
- ✅ 文档更新 (GA_GATE_REPORT + 本文件)

---

Last updated: 2026-06-13 04:20 CST

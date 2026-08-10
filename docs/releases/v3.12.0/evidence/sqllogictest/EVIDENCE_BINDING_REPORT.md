# vv3.12.0 证据绑定检查报告

> **检查日期**: 2026-08-10
> **版本**: v3.12.0
> **Auditor**: Hermes Agent (Anti-Fabrication Policy v1.0)
> **检查工具**: check_evidence_binding.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | 31 |
| 警告 | 133 |
| 失败（违规） | 304 |
| 未验证声明 | 302 |

**总结**: ❌ 发现 304 个违规（Type A/B/C/D）

---

## 违规详情（按类型分类）

### Type A: 虚构执行（Execution Fabrication）

无 CI 证据声明"测试通过 / 编译成功"

- ❌ Type A/B 违规：第 77 行声明无 CI/gate/commit 证据: | V312-02 | GMP schema v3.12 | P0 | schema tests PASS |
- ❌ Type A/B 违规：第 122 行声明无 CI/gate/commit 证据: > 在 TPC-H correctness、SQLLogicTest、wire protocol、LOA
- ❌ Type A/B 违规：第 183 行声明无 CI/gate/commit 证据: | V312-02 | GMP schema v3.12 | P0 | Schema tests PASS |
- ❌ Type A/B 违规：第 7 行声明无 CI/gate/commit 证据: 本矩阵把 GMP 内审检索所需控制项映射到 SQLRus
- ❌ Type A/B 违规：第 49 行声明无 CI/gate/commit 证据: **PASS — 154 tests passed, 0 failed**
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: - [x] backup.rs 代码存在且编译通过
- ❌ Type A/B 违规：第 77 行声明无 CI/gate/commit 证据: **状态: PASS — 满足关闭条件**
- ❌ Type A/B 违规：第 17 行声明无 CI/gate/commit 证据: `bash scripts/gate/check_arch_invariants.sh` → exit 0, 5 P
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: | C-ARCH-01 | LocalExecutor has NO `txn_manager` field | **P
- ❌ Type A/B 违规：第 22 行声明无 CI/gate/commit 证据: | C-ARCH-02 | LocalExecutor has NO `write_buffer` field | **
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据: | C-ARCH-03 | storage.insert/update/delete only in `crates/s
- ❌ Type A/B 违规：第 24 行声明无 CI/gate/commit 证据: | C-ARCH-04 | No `eng.execute(raw_sql)` outside parser | **P
- ❌ Type A/B 违规：第 25 行声明无 CI/gate/commit 证据: | C-ARCH-05 | `execution_engine.rs` < 1600 lines (SSOT: CARC
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: PASS: C-ARCH-01
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: PASS: C-ARCH-02
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: PASS: C-ARCH-04
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: PASS: C-ARCH-05 (execution_engine.rs: 1594 lines, limit 1600
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: PASSED: 5
- ❌ Type A/B 违规：第 51 行声明无 CI/gate/commit 证据: Result: ALL PASS
- ❌ Type A/B 违规：第 77 行声明无 CI/gate/commit 证据: | R2.6 | INT-2 + INT-3 deferred w/ plan (2 items, D7 PASS-WI
- ❌ Type A/B 违规：第 123 行声明无 CI/gate/commit 证据: PASS: ALL_TARGETS_REPORT.md fresh (age=N s)
- ❌ Type A/B 违规：第 124 行声明无 CI/gate/commit 证据: PASS: R2_INVARIANTS_REPORT.md fresh (age=N s)
- ❌ Type A/B 违规：第 125 行声明无 CI/gate/commit 证据: PASS: signoff valid (Reviewer A=hermes-z6g4, Reviewer B=open
- ❌ Type A/B 违规：第 126 行声明无 CI/gate/commit 证据: PASS: signoff file is valid
- ❌ Type A/B 违规：第 127 行声明无 CI/gate/commit 证据: ==> V312-19 release gate PASSED
- ❌ Type A/B 违规：第 136 行声明无 CI/gate/commit 证据: | C-ARCH-01~05 | 5/5 PASS | This file (Part 1) + `scripts/ga
- ❌ Type A/B 违规：第 137 行声明无 CI/gate/commit 证据: | R2.1-R2.8 driver | 4 PASS / 1 stub (R2.8) / 3 fail (R2.1/R
- ❌ Type A/B 违规：第 140 行声明无 CI/gate/commit 证据: | V312-19 release gates | PASS exit 0 | `check_v312_19_relea
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: **Result:** 11/11 PASS, 0 FAIL
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: | `test_wire_smoke_error_packet_structure` | PASS | ~20s |
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | `test_wire_smoke_load_data_sf1` | PASS | ~30s |
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | `test_wire_smoke_reset_clears_prepared_stmts` | PASS | ~25
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | `test_wire_smoke_reset_connection` | PASS | ~20s |
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | `test_wire_smoke_stmt_close` | PASS | ~25s |
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: | `test_wire_smoke_stmt_close_nonexistent` | PASS | ~20s |
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: | `test_wire_smoke_stmt_execute_after_close` | PASS | ~25s |
- ❌ Type A/B 违规：第 41 行声明无 CI/gate/commit 证据: | `test_wire_smoke_stmt_prepare_execute_int` | PASS | ~25s |
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: | `test_wire_smoke_stmt_prepare_execute_varchar` | PASS | ~2
- ❌ Type A/B 违规：第 43 行声明无 CI/gate/commit 证据: | `test_wire_smoke_stmt_prepare_invalid_sql` | PASS | ~20s |
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: | `test_wire_smoke_stmt_prepare_null` | PASS | ~25s |
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: | C-ARCH invariants | `bash scripts/gate/check_arch_invarian
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: | LOAD DATA INFILE | `bash scripts/gate/check_load_data_infi
- ❌ Type A/B 违规：第 74 行声明无 CI/gate/commit 证据: | Anti-fabrication | `bash scripts/gate/check_anti_fabricati
- ❌ Type A/B 违规：第 75 行声明无 CI/gate/commit 证据: | Wire smoke | `cargo test --test wire_smoke_mysql_cli` | 11
- ❌ Type A/B 违规：第 105 行声明无 CI/gate/commit 证据:   [PASS] LOAD_DATA_INFILE.md
- ❌ Type A/B 违规：第 106 行声明无 CI/gate/commit 证据:   [PASS] LOAD DATA documented
- ❌ Type A/B 违规：第 107 行声明无 CI/gate/commit 证据:   [PASS] parser changes documented
- ❌ Type A/B 违规：第 108 行声明无 CI/gate/commit 证据:   [PASS] gate executable
- ❌ Type A/B 违规：第 109 行声明无 CI/gate/commit 证据: PASS: 4, FAIL: 0
- ❌ Type A/B 违规：第 113 行声明无 CI/gate/commit 证据: Result: ALL PASS  [exit 0]
- ❌ Type A/B 违规：第 51 行声明无 CI/gate/commit 证据: **Result:** PASS (with graceful fallback)
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: **PASS — 154 tests passed, 0 failed**
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: - [x] ingestion.rs 代码存在且编译通过
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: **状态: PASS — 满足关闭条件**
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: | Tests | ✅ | 10/10 PASS |
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: **Evidence**: `cargo test -p sqlrustgo-executor window` → 
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | `cargo test -p sqlrustgo-mysql-server --test wire_smoke_my
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | `bash scripts/gate/check_arch_invariants.sh` | 5/5 PASS |
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | `bash scripts/gate/check_load_data_infile.sh` | 4/4 PASS |
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: PASS: ALL_TARGETS_REPORT.md fresh
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: PASS: R2_INVARIANTS_REPORT.md fresh
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: PASS: signoff file is valid
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: ==> V312-19 release gate PASSED  [exit 0]
- ❌ Type A/B 违规：第 90 行声明无 CI/gate/commit 证据: [C-ARCH-01] PASS  [C-ARCH-02] PASS  [C-ARCH-03] INFO
- ❌ Type A/B 违规：第 91 行声明无 CI/gate/commit 证据: [C-ARCH-04] PASS  [C-ARCH-05] PASS
- ❌ Type A/B 违规：第 92 行声明无 CI/gate/commit 证据: Result: 5/5 PASS  [exit 0]
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据:   [1/4] ✅ PASS: 3 VtuGuard marker calls in src/execution_e
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据:   [2/4] ✅ PASS: 2 VtuGuard marker calls in openclaw_endpoi
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据:   [4/4] ✅ PASS: VtuGuard::assert_path_for_dml is public
- ❌ Type A/B 违规：第 24 行声明无 CI/gate/commit 证据: === G4 Gate: PASS ===
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: [C-ARCH-01] Checking LocalExecutor has NO txn_manager field.
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: [C-ARCH-02] Checking LocalExecutor has NO write_buffer field
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: [C-ARCH-03] Checking storage.insert/update/delete only in cr
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: [C-ARCH-04] Checking no eng.execute(raw_sql) outside parser.
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: [C-ARCH-05] Checking execution_engine.rs < 1600 lines... PAS
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: === Summary === PASSED: 5, FAILED: 0
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: - Draft gate 仍只表示开发入口准备，不表示实�
- ❌ Type A/B 违规：第 14 行声明无 CI/gate/commit 证据: > "提交命令、日志、PASS/FAIL、commit、evidence_has
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: - **V312-24 work contribution**: 0 new errors. 9/5/6 = 20 li
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: - **Result**: **Exit 0 (PASS)**
- ❌ Type A/B 违规：第 53 行声明无 CI/gate/commit 证据: - **Result**: **Exit 0 (PASS)**
- ❌ Type A/B 违规：第 61 行声明无 CI/gate/commit 证据: - **Result**: **CHECK 1.5 V312-24 = 3 PASS lines** (sqlancer
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据:   [PASS]   V312-24: target/sqlancer-report.json valid (itera
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据:   [PASS]   V312-24: target/test-runner-report.json valid (to
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据:   [PASS]   V312-24: both SQLancer + test-runner report artif
- ❌ Type A/B 违规：第 114 行声明无 CI/gate/commit 证据: | 1 | Every tool can run + produce artifact | ✅ PASS | Gat
- ❌ Type A/B 违规：第 115 行声明无 CI/gate/commit 证据: | 2 | 15-item disposition table with reason + replacement ga
- ❌ Type A/B 违规：第 118 行声明无 CI/gate/commit 证据: | 5 | `crates/sql-corpus/tests/corpus_test.rs` pass_rate ≥
- ❌ Type A/B 违规：第 121 行声明无 CI/gate/commit 证据: **5/6 PASS + 1/6 PARTIAL (with V312-31 follow-up filed)**.
- ❌ Type A/B 违规：第 170 行声明无 CI/gate/commit 证据: **6 gate 实跑**: 5 numerical gates PASS (V312-24 work +0 e
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | 5 | `bash scripts/gate/check_beta_gate.sh` B10_SQLANCER PA
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | 2 FAIL in `sqlrustgo-mysql-server` (`list_threads_returns_
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: $ bash scripts/gate/check_beta_gate.sh 2>&1 | tee /tmp/v312_
- ❌ Type A/B 违规：第 132 行声明无 CI/gate/commit 证据: 1. 不允许"openspec 标 done"、"报告标题写已完成"
- ❌ Type A/B 违规：第 142 行声明无 CI/gate/commit 证据: V312-30 **不能**在 PR merge + 2 reviewer APPROVED + 6 项
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: **PASS — 154 tests passed, 0 failed**
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: - [x] graph.rs 代码存在且编译通过
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: **状态: PASS — 满足关闭条件**
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: **PASS — 154 tests passed, 0 failed**
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: | audit hash chain 实跑证据 | DONE | `test_hash_chain_ta
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: | fail-closed tamper 测试 | DONE | `test_permission_guard_
- ❌ Type A/B 违规：第 90 行声明无 CI/gate/commit 证据: - [x] acl.rs / audit.rs 代码存在且编译通过
- ❌ Type A/B 违规：第 97 行声明无 CI/gate/commit 证据: **状态: PASS — 满足关闭条件**
- ❌ Type A/B 违规：第 7 行声明无 CI/gate/commit 证据: > **真实性规则**: 本计划只定义未来工作和退�
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: - 没有 command output、timestamp、source agent、source 
- ❌ Type A/B 违规：第 119 行声明无 CI/gate/commit 证据: | `cargo build -p sqlrustgo_sqllogictest` | 可完成，但 
- ❌ Type A/B 违规：第 120 行声明无 CI/gate/commit 证据: | `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/
- ❌ Type A/B 违规：第 206 行声明无 CI/gate/commit 证据: 任何 PASS 声明都必须附 command output、timestamp、
- ❌ Type A/B 违规：第 271 行声明无 CI/gate/commit 证据: - PASS, GA, or compliance claims without command output, tim
- ❌ Type A/B 违规：第 321 行声明无 CI/gate/commit 证据: | `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/
- ❌ Type A/B 违规：第 516 行声明无 CI/gate/commit 证据: - Gate script output showing PASS/FAIL with exit code.
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: **PASS** — pass rate 99.4% ≥ 80.0% (baseline; V312-24 pr
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: **PASS** — 14 个 per-subcategory guard + 2 个 meta test 
- ❌ Type A/B 违规：第 55 行声明无 CI/gate/commit 证据: **PASS** — 5 failing cases pre-exist in baseline; pass rat
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: **PASS**.
- ❌ Type A/B 违规：第 78 行声明无 CI/gate/commit 证据: V312-24 proposal §4 描述"16 subcategories / 27.3% pass ra
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: - 6/16 PASS → 实际是 14/14 subcategories 在 99.4% 范�
- ❌ Type A/B 违规：第 96 行声明无 CI/gate/commit 证据: 按"以实际 gate 数字关闭"的严格要求：4/4 关闭
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据: **PASS** — 3 个脚本实际命令中 0 个 `|| true` (bas
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: **PASS** — 没有 `2>/dev/null || true` 吞错；server �
- ❌ Type A/B 违规：第 50 行声明无 CI/gate/commit 证据: **PASS** — 缺失时 fail-explicit，证据写入 `/tmp/ba
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: **PASS** — 缺失时 fail-explicit，stderr 含 "sysbench 
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: **PASS** — server 不可达时 exit 1。byte-exact 断言�
- ❌ Type A/B 违规：第 74 行声明无 CI/gate/commit 证据: **PASS**.
- ❌ Type A/B 违规：第 98 行声明无 CI/gate/commit 证据: 按"以实际 gate 数字关闭"的严格要求：6/6 关闭
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: | Every tool can run + produce artifact (sqlancer + test-run
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | Every retired/deferred item has reason + replacement gate 
- ❌ Type A/B 违规：第 41 行声明无 CI/gate/commit 证据: **Net result for this PR**: 1 of 6 acceptance criteria are P
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: 2 are PARTIAL (PASS in artifact terms, FAIL in gate wiring t
- ❌ Type A/B 违规：第 98 行声明无 CI/gate/commit 证据: cli_smoke             2/2 PASS
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据: toml_round_trip       3/3 PASS
- ❌ Type A/B 违规：第 100 行声明无 CI/gate/commit 证据: managed_dispatch      2/2 PASS
- ❌ Type A/B 违规：第 101 行声明无 CI/gate/commit 证据: timeout_enforced      3/3 PASS
- ❌ Type A/B 违规：第 213 行声明无 CI/gate/commit 证据: "openspec 标 done" / "报告标题写已完成" / "PR 已�
- ❌ Type A/B 违规：第 217 行声明无 CI/gate/commit 证据: V312-24 本身**不能**在 V312-25 ~ V312-30 全部 PASS �
- ❌ Type A/B 违规：第 235 行声明无 CI/gate/commit 证据: | 1 | `crates/sqlancer` | **activate** | minimax | 2026-08-2
- ❌ Type A/B 违规：第 236 行声明无 CI/gate/commit 证据: | 2 | `crates/test-runner` | **activate** | minimax | 2026-0
- ❌ Type A/B 违规：第 237 行声明无 CI/gate/commit 证据: | 3 | `crates/test-registry` | **activate** | minimax | 2026
- ❌ Type A/B 违规：第 246 行声明无 CI/gate/commit 证据: | 12 | `tests/e2e/e2e_beta_test.rs` 5 double-skip tests | **
- ❌ Type A/B 违规：第 254 行声明无 CI/gate/commit 证据: **V312-24 自身关闭条件**：上表 16 项全 PASS + V31
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: **PASS — 154 tests passed, 0 failed**
- ❌ Type A/B 违规：第 93 行声明无 CI/gate/commit 证据: - [x] schema/version/chunk/relation/audit/document 模块代
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据: **状态: PASS — 满足关闭条件**
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | `test_wal_entry_large_payload` | ✅ PASS |
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: | `test_wal_entry_serialization_roundtrip` | ✅ PASS |
- ❌ Type A/B 违规：第 22 行声明无 CI/gate/commit 证据: | `test_wal_checkpoint_recovery` | ✅ PASS |
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据: | `test_wal_concurrent_transactions_isolation` | ✅ PASS |
- ❌ Type A/B 违规：第 24 行声明无 CI/gate/commit 证据: | `test_wal_mixed_operations` | ✅ PASS |
- ❌ Type A/B 违规：第 25 行声明无 CI/gate/commit 证据: | `test_wal_recovery_after_crash` | ✅ PASS |
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | `test_wal_rollback_recovery` | ✅ PASS |
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | `test_wal_recovery_with_pending_transaction` | ✅ PASS |
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | `test_wal_single_transaction` | ✅ PASS |
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: **Result**: 16/16 WAL integration tests PASS ✅
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | `test_crash_recovery_reconstructs_pages` | ✅ PASS |
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | `test_f26_crash_recovery` | ✅ PASS |
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: **Result**: 2/2 storage crash tests PASS ✅
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: **Result**: 6/8 tests PASS, 2 FAIL ⚠️
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: | `test_upgrade_gate_script_exists` | ✅ PASS |
- ❌ Type A/B 违规：第 61 行声明无 CI/gate/commit 证据: | `test_upgrade_script_has_required_functions` | ✅ PASS |
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: | `test_upgrade_script_exists` | ✅ PASS |
- ❌ Type A/B 违规：第 63 行声明无 CI/gate/commit 证据: | `test_upgrade_script_syntax` | ✅ PASS |
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: **Result**: 4/4 upgrade tests PASS ✅
- ❌ Type A/B 违规：第 79 行声明无 CI/gate/commit 证据: | WAL Integration | ✅ PASS (16/16) | None |
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: | Storage Crash Recovery | ✅ PASS (2/2) | None |
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: | Upgrade Path | ✅ PASS (4/4) | None |
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: **PASS** — 13/13 passed including `test_vtu_guard_wraps_st
- ❌ Type A/B 违规：第 41 行声明无 CI/gate/commit 证据: **PASS** — 旧的 `rows.len() <= 6` (accepts 0 rows) 改�
- ❌ Type A/B 违规：第 43 行声明无 CI/gate/commit 证据: 加载好或 Q1 完全失败 — 之前是 PASS 静默吞，
- ❌ Type A/B 违规：第 54 行声明无 CI/gate/commit 证据: **PASS** — 6 passed (5 个 e2e_XX 加上 e2e_beta_manifest
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: **PASS** — stale + union 全部 0。
- ❌ Type A/B 违规：第 89 行声明无 CI/gate/commit 证据: **PASS**.
- ❌ Type A/B 违规：第 116 行声明无 CI/gate/commit 证据: 按"以实际 gate 数字关闭"的严格要求：5/5 关闭
- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: **v3.11.0 GA 6/6 gates PASS — v3.12.0 不继承 hidden 弱
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | G1 R1-R4 RC 指标 | ✅ PASS | 否 |
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: | G2 Full test suite | ⚠️ 2,666 tests (2,664 PASS + 2 FA
- ❌ Type A/B 违规：第 24 行声明无 CI/gate/commit 证据: | G5 Security audit | ✅ PASS | 否 |
- ❌ Type A/B 违规：第 41 行声明无 CI/gate/commit 证据: | sqlrustgo-executor | 685 | ✅ PASS | 2026-08-09 18:00+080
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: | sqlrustgo-storage | 683 | ✅ PASS | 2026-08-09 18:00+0800
- ❌ Type A/B 违规：第 43 行声明无 CI/gate/commit 证据: | sqlrustgo-parser | 589 (3 ignored) | ✅ PASS | 同上 |
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: | sqlrustgo-catalog | 183 | ✅ PASS | 同上 |
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: | sqlrustgo-planner | 84 | ✅ PASS | 同上 |
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: | sqlrustgo-common | 79 | ✅ PASS | 同上 |
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: | sqlrustgo-mysql-client | 79 | ✅ PASS | 同上 |
- ❌ Type A/B 违规：第 49 行声明无 CI/gate/commit 证据: | sqlrustgo-admin | 69 | ✅ PASS | 同上 |
- ❌ Type A/B 违规：第 50 行声明无 CI/gate/commit 证据: | sqlrustgo-cache | 10 | ✅ PASS | 同上 |
- ❌ Type A/B 违规：第 85 行声明无 CI/gate/commit 证据: | 22/22 实跑 PASS | ✅ | ✅ | ✅ | both | both |
- ❌ Type A/B 违规：第 91 行声明无 CI/gate/commit 证据: **Source report**: `docs/releases/v3.11.0/TPCH_SF1_22_22_PAS
- ❌ Type A/B 违规：第 142 行声明无 CI/gate/commit 证据: | Pre-flight (home/readlink/buildroot) | ✅ PASS | — |
- ❌ Type A/B 违规：第 159 行声明无 CI/gate/commit 证据: | **G3 覆盖率** (3 crates < 80%) | PASS (gate = tools 80.
- ❌ Type A/B 违规：第 160 行声明无 CI/gate/commit 证据: | **G4 TPC-H zero-row correctness** | 22/22 不 OOM (PASS) |
- ❌ Type A/B 违规：第 205 行声明无 CI/gate/commit 证据:   [A1_BUILD] PASS
- ❌ Type A/B 违规：第 290 行声明无 CI/gate/commit 证据:   [PASS] docs/releases/v3.12.0/BLOCKER_DISPOSITION_V311.md e
- ❌ Type A/B 违规：第 291 行声明无 CI/gate/commit 证据: --- Gap 2: GA gates PASS verification ---
- ❌ Type A/B 违规：第 294 行声明无 CI/gate/commit 证据:   [PASS] 10 crates have test counts recorded
- ❌ Type A/B 违规：第 296 行声明无 CI/gate/commit 证据:   [PASS]        2 crates < 80% (acceptable, tracked to V312-
- ❌ Type A/B 违规：第 298 行声明无 CI/gate/commit 证据:   [PASS] G4 TPC-H SF=1 22/22 verified
- ❌ Type A/B 违规：第 300 行声明无 CI/gate/commit 证据:   [PASS] SOAK 343h37m (2.04x) verified
- ❌ Type A/B 违规：第 302 行声明无 CI/gate/commit 证据:   [PASS] All 5 remotes synced to v3.11.0-ga
- ❌ Type A/B 违规：第 305 行声明无 CI/gate/commit 证据: PASS: 7 / 7
- ❌ Type A/B 违规：第 308 行声明无 CI/gate/commit 证据: V312-01 blocker disposition: PASS
- ❌ Type A/B 违规：第 312 行声明无 CI/gate/commit 证据: **Gate execution**: 7/7 PASS — v3.12.0 ALPHA promotion 入
- ❌ Type A/B 违规：第 61 行声明无 CI/gate/commit 证据: | `test_composite_btree_index_insert` | ✅ PASS |
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: | `test_composite_btree_index_insert_unique` | ✅ PASS |
- ❌ Type A/B 违规：第 63 行声明无 CI/gate/commit 证据: | `test_composite_btree_index_search` | ✅ PASS |
- ❌ Type A/B 违规：第 64 行声明无 CI/gate/commit 证据: | `test_composite_btree_index_range_query` | ✅ PASS |
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: | `test_composite_btree_index_num_columns` | ✅ PASS |
- ❌ Type A/B 违规：第 49 行声明无 CI/gate/commit 证据: | parser_fixtures | `cargo test -p sqlrustgo-parser...` | �
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: | R2.1-R2.3 | ✅ PASS | None |
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: | parser_fixtures | ✅ PASS | None |
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: **PASS — 154 tests passed, 0 failed**
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: - [x] retrieval.rs 代码存在且编译通过
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: **状态: PASS — 满足关闭条件**
- ❌ Type A/B 违规：第 22 行声明无 CI/gate/commit 证据: | V312-G11 | SQLite SQLLogicTest 判定门禁 | `sqlrustgo_s
- ❌ Type A/B 违规：第 24 行声明无 CI/gate/commit 证据: | V312-G13 | MySQL wire protocol 硬化 | COM_QUERY、COM_ST
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | V312-G17 | Window/GIS/JSON 受控功能 | ROW_NUMBER/RANK/
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | V312-G22 | MySQL 兼容与 SQL surface 回归 | SHOW、aut
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: 每个 PASS claim 必须包含：command、timestamp、sourc
- ❌ Type A/B 违规：第 81 行声明无 CI/gate/commit 证据: - 168h mixed SOAK 未完成，或没有证据却写成 PASS�
- ❌ Type A/B 违规：第 100 行声明无 CI/gate/commit 证据: | Alpha | `cargo build -p sqlrustgo_sqllogictest` 成功；r
- ❌ Type A/B 违规：第 102 行声明无 CI/gate/commit 证据: | RC | curated SQLite-compatible subset 运行，并输出 P
- ❌ Type A/B 违规：第 103 行声明无 CI/gate/commit 证据: | GA | selected SLT targets 全部通过，或每个 skipped
- ❌ Type A/B 违规：第 118 行声明无 CI/gate/commit 证据: | `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/
- ❌ Type A/B 违规：第 135 行声明无 CI/gate/commit 证据: | G3 coverage 口径漂移 | 单一 canonical coverage comma
- ❌ Type A/B 违规：第 147 行声明无 CI/gate/commit 证据: | v3.6 Beta PENDING 与覆盖率/测试编译延续问题 | 
- ❌ Type A/B 违规：第 194 行声明无 CI/gate/commit 证据: | V312-G11 | SQLite SQLLogicTest oracle | `sqlrustgo_sqllogi
- ❌ Type A/B 违规：第 217 行声明无 CI/gate/commit 证据: Every PASS claim must include
- ❌ Type A/B 违规：第 225 行声明无 CI/gate/commit 证据: - PASS/FAIL boundary
- ❌ Type A/B 违规：第 240 行声明无 CI/gate/commit 证据: - 168h mixed SOAK is not completed or is described as PASS w
- ❌ Type A/B 违规：第 252 行声明无 CI/gate/commit 证据: | RC | Curated SQLite-compatible subset runs with PASS/FAIL/
- ❌ Type A/B 违规：第 285 行声明无 CI/gate/commit 证据: | G3 coverage口径漂移 | Single canonical coverage comman
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: - 禁止声明 v3.12.0 已通过 Alpha/Beta/RC/GA。
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: - 禁止声明 SQLLogicTest、TPC-H correctness、wire、LOA
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: - 数据行数：20（PASS: 11 / unsupported: 2 / deferred: 
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: - **PASS surface**: `alter_add_column`, `alter_drop_column`,
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: **PASS** — 10 unique file paths deleted in git history.
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: **PASS** — only `e2e_07_json_vector.sh` remains (kept for 
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: **PASS** — 10 retired entries added (one per retired scrip
- ❌ Type A/B 违规：第 61 行声明无 CI/gate/commit 证据: **PASS** — 唯一引用被删脚本的代码是 `tests/bas
- ❌ Type A/B 违规：第 75 行声明无 CI/gate/commit 证据: **PASS**.
- ❌ Type A/B 违规：第 107 行声明无 CI/gate/commit 证据: 按"以实际 gate 数字关闭"的严格要求：5/5 关闭
- ❌ Type A/B 违规：第 14 行声明无 CI/gate/commit 证据: **验收**: blocker disposition report、stage gate output�
- ❌ Type A/B 违规：第 75 行声明无 CI/gate/commit 证据: **验收**: `cargo build -p sqlrustgo_sqllogictest` 成功�
- ❌ Type A/B 违规：第 142 行声明无 CI/gate/commit 证据: **验收**: 对 GMP/生产路径相关子集给出 fixture P
- ❌ Type A/B 违规：第 166 行声明无 CI/gate/commit 证据: 12 个 task 全完成（sqlancer / test-runner / test-regist
- ❌ Type A/B 违规：第 193 行声明无 CI/gate/commit 证据: 2. 失败注入测试 1：`mv scripts/gate/e2e/e2e_07_fixtur
- ❌ Type A/B 违规：第 197 行声明无 CI/gate/commit 证据: **禁止关闭条件**: 仅以"测试通过"或"无 `\|\| tr
- ❌ Type A/B 违规：第 222 行声明无 CI/gate/commit 证据: **Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-
- ❌ Type A/B 违规：第 229 行声明无 CI/gate/commit 证据: **禁止关闭条件**: (a) 仅靠"打开了 follow-up 任�
- ❌ Type A/B 违规：第 241 行声明无 CI/gate/commit 证据: 2. `bash scripts/gate/check_beta_gate.sh 2>&1 | grep B10_SQL
- ❌ Type A/B 违规：第 262 行声明无 CI/gate/commit 证据:    - `bash scripts/gate/check_beta_gate.sh` B10_SQLANCER 通
- ❌ Type A/B 违规：第 265 行声明无 CI/gate/commit 证据: **禁止关闭条件**: (a) 不允许"openspec 标 done"、"
- ❌ Type A/B 违规：第 294 行声明无 CI/gate/commit 证据: - Evidence hashes for any PASS claim.
- ❌ Type A/B 违规：第 409 行声明无 CI/gate/commit 证据: - GA gate report contains only evidence-backed PASS claims.
- ❌ Type A/B 违规：第 434 行声明无 CI/gate/commit 证据: - The selected SLT corpus has PASS/FAIL/SKIP classification.
- ❌ Type A/B 违规：第 14 行声明无 CI/gate/commit 证据: 2. Issue 评论包含 PR 编号、commit SHA、执行命令�
- ❌ Type A/B 违规：第 150 行声明无 CI/gate/commit 证据: - **测试**: 9 PASS / 1 unsupported / 4 deferred
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | demo.test | PASS | Basic SELECT |
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | insert__test_insert_invalid.test | PASS | Error cases |
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | delete__test_delete.test | PASS | DELETE operations |
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: | update__test_update.test | PASS | UPDATE operations |
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: | constraints__test_not_null.test | PASS | NOT NULL constrai
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: - 文档链接和一致性检查通过。
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: Draft 阶段可以移交 Hermes/OMP 进入 Alpha 开发准�
- ❌ Type A/B 违规：第 50 行声明无 CI/gate/commit 证据: | `bash scripts/gate/check_sqllogictest_v312.sh` | PASS，4/
- ❌ Type A/B 违规：第 51 行声明无 CI/gate/commit 证据: | `bash scripts/gate/check_alpha_v3.12.0.sh` | PASS，13/13 
- ❌ Type A/B 违规：第 52 行声明无 CI/gate/commit 证据: | `bash scripts/gate/check_stage.sh --version v3.12.0 --dry-
- ❌ Type A/B 违规：第 53 行声明无 CI/gate/commit 证据: | `bash scripts/gate/check_stage.sh --version v3.12.0 --stag
- ❌ Type A/B 违规：第 54 行声明无 CI/gate/commit 证据: | `bash scripts/gate/check_stage.sh --version v3.12.0` | PAS
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：DRAFT_ASSESSMENT_AND_ALPHA_GATE.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：DRAFT_ASSESSMENT_AND_ALPHA_GATE.md
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: 2. 失败时 `exit 1` (而非 `PASS=$((PASS+1))` 静默)
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据:   PASS: sysbench ran 3s with 1 thread(s), 1402 events at 466
- ❌ Type A/B 违规：第 85 行声明无 CI/gate/commit 证据: === E2E sysbench_smoke_test: PASS ===
- ❌ Type A/B 违规：第 50 行声明无 CI/gate/commit 证据: **PASS — 154 tests passed, 0 failed**
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: - [x] vector_index/vector_search/embedding 模块代码存�
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: **状态: PASS — 满足关闭条件**
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: **PASS — 154 tests passed, 0 failed**
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: - [x] rag.rs 代码存在且编译通过
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: **状态: PASS — 满足关闭条件**
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: | `wire_smoke_mysql_cli.rs` | 11/11 PASS |
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据: | Architecture invariants (C-ARCH-01~05) | 5/5 PASS |
- ❌ Type A/B 违规：第 84 行声明无 CI/gate/commit 证据: | `check_load_data_infile.sh` | 4/4 PASS |
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据: - C-ARCH-01~05 invariants (5/5 PASS)
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: **PASS** — SQLancer 部分（line 110-118）已去 `|| tru
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: **PASS** — baseline `check_warn` 已升级为 `check_fail`
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: **PASS** — 8 个 scenarios 改 4 个 (V312-25 retired 3 st
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: **PASS** — P16 step 2.5/3 新增，扫出 **29 个 `|| tru
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: **PASS**.
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据: 按"以实际 gate 数字关闭"的严格要求：5/5 关闭
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: - 2026-08-09 本地基线：`cargo build -p sqlrustgo_sqllog
- ❌ Type A/B 违规：第 94 行声明无 CI/gate/commit 证据: | `wire_load_data/V312-13-REPORT.md` | `check_v312_13_wire_l
- ❌ Type A/B 违规：第 96 行声明无 CI/gate/commit 证据: | `mysql_compat/SURFACE_DISPOSITION.md` | `check_v312_21_mys
- ❌ Type A/B 违规：第 101 行声明无 CI/gate/commit 证据: | 测试目标 | 通过 | 失败 |
- ❌ Type A/B 违规：第 113 行声明无 CI/gate/commit 证据: | `scripts/gate/check_v312_13_wire_load_data.sh` | V312-13 e
- ❌ Type A/B 违规：第 114 行声明无 CI/gate/commit 证据: | `scripts/gate/check_v312_19_release_gates.sh` | RC/GA 阻�
- ❌ Type A/B 违规：第 163 行声明无 CI/gate/commit 证据:   alter_modify_column 全部 PASS** (5 个新增 PASS)
- ❌ Type A/B 违规：第 165 行声明无 CI/gate/commit 证据:   4 个 fixture 之前标 `UNSUPPORTED` 但 server 静默接
- ❌ Type A/B 违规：第 166 行声明无 CI/gate/commit 证据:   后: **4 个新增 PASS** (server 解析但语义不实现
- ❌ Type A/B 违规：第 171 行声明无 CI/gate/commit 证据: 最终 disposition: **9 PASS / 1 unsupported / 4 deferred / 
- ❌ Type A/B 违规：第 176 行声明无 CI/gate/commit 证据:   (600 lineitem rows / 62 KB), 8 个表全部 LOAD DATA 成�
- ❌ Type A/B 违规：第 196 行声明无 CI/gate/commit 证据: - V312-13 / V312-19 / V312-21 gates: ALL PASS
- ❌ Type A/B 违规：第 211 行声明无 CI/gate/commit 证据:   - 新增 PASS surface (6 个意外实现): `group_concat`,
- ❌ Type A/B 违规：第 213 行声明无 CI/gate/commit 证据:   - 最终结果: **11 PASS / 2 unsupported / 7 deferred / 0
- ❌ Type A/B 违规：第 271 行声明无 CI/gate/commit 证据: - 2026-08-09 local baseline: `cargo build -p sqlrustgo_sqllo

---

### 警告项（需要人工复核）

- ⚠️ 警告：第 17 行历史版本引用可能需更新: 本版本还承接 v3.6.0-v3.11.0 文档中已经�
- ⚠️ 警告：第 36 行历史版本引用可能需更新: | v3.6.0 Beta PENDING 与 coverage/test compile �
- ⚠️ 文档可能缺少 provenance 元数据：VERSION_PLAN.md
- ⚠️ 警告：第 16 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GMP_COMPLIANCE_MATRIX.md
- ⚠️ 文档可能缺少 provenance 元数据：README.md
- ⚠️ 警告：第 49 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312-09-backup-restore-report.md
- ⚠️ 警告：第 17 行 FAIL 声明可能无证据
- ⚠️ 警告：第 49 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：arch-invariant-report.md
- ⚠️ 警告：第 28 行 FAIL 声明可能无证据
- ⚠️ 警告：第 109 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：wire-e2e-report.md
- ⚠️ 文档可能缺少 provenance 元数据：load-data-report.md
- ⚠️ 警告：第 25 行 FAIL 声明可能无证据
- ⚠️ 警告：第 44 行 FAIL 声明可能无证据
- ⚠️ 警告：第 62 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312-03-gmp-ingestion-report.md
- ⚠️ 文档可能缺少 provenance 元数据：window-gis-json-feature-delivery-report.md
- ⚠️ 文档可能缺少 provenance 元数据：REVIEWER_SIGN_OFF.md
- ⚠️ 警告：第 38 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：execution-architecture-debt-report.md
- ⚠️ 文档可能缺少 provenance 元数据：ARCHITECTURE.md
- ⚠️ 警告：第 14 行 FAIL 声明可能无证据
- ⚠️ 警告：第 117 行历史版本引用可能需更新: | 4 | `tests/baseline/ignore_registry.json` matche
- ⚠️ 文档可能缺少 provenance 元数据：V312-30_stage_transition_report.md
- ⚠️ 警告：第 23 行 FAIL 声明可能无证据
- ⚠️ 警告：第 34 行 FAIL 声明可能无证据
- ⚠️ 警告：第 38 行 FAIL 声明可能无证据
- ⚠️ 警告：第 69 行 FAIL 声明可能无证据
- ⚠️ 警告：第 134 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：V312-30_signoff_report.md
- ⚠️ 警告：第 47 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312-06-graph-projection-report.md
- ⚠️ 警告：第 65 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312-08-compliance-audit-report.md
- ⚠️ 警告：第 120 行 FAIL 声明可能无证据
- ⚠️ 警告：第 141 行 FAIL 声明可能无证据
- ⚠️ 警告：第 145 行历史版本引用可能需更新: | v3.6.0 | Beta B1-B8 曾全部 PENDING；Parser/E
- ⚠️ 警告：第 152 行 FAIL 声明可能无证据
- ⚠️ 警告：第 516 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：DEVELOPMENT_PLAN.md
- ⚠️ 文档不存在（跳过检查）：V312-11-VERIFICATION.md
- ⚠️ 文档不存在（跳过检查）：smoke-report.md
- ⚠️ 文档可能缺少 provenance 元数据：disabled-test-registry.md
- ⚠️ 文档可能缺少 provenance 元数据：V312-28_corpus_activation_report.md
- ⚠️ 警告：第 47 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：V312-26_warn_only_fix_report.md
- ⚠️ 警告：第 42 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：V312-24_test_infra_activation_report.md
- ⚠️ 警告：第 70 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312-02-gmp-schema-report.md
- ⚠️ 警告：第 45 行 FAIL 声明可能无证据
- ⚠️ 警告：第 46 行 FAIL 声明可能无证据
- ⚠️ 警告：第 48 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：crash-recovery-upgrade-verification-report.md
- ⚠️ 警告：第 43 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：V312-27_anti_fab_fix_report.md
- ⚠️ 警告：第 21 行 FAIL 声明可能无证据
- ⚠️ 警告：第 45 行 FAIL 声明可能无证据
- ⚠️ 警告：第 51 行 FAIL 声明可能无证据
- ⚠️ 警告：第 75 行 FAIL 声明可能无证据
- ⚠️ 警告：第 306 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：BLOCKER_DISPOSITION_V311.md
- ⚠️ 文档可能缺少 provenance 元数据：storage-index-wal-backlog-report.md
- ⚠️ 文档可能缺少 provenance 元数据：sequence-executor-gap-assessment.md
- ⚠️ 警告：第 35 行 FAIL 声明可能无证据
- ⚠️ 警告：第 41 行 FAIL 声明可能无证据
- ⚠️ 警告：第 67 行 FAIL 声明可能无证据
- ⚠️ 警告：第 118 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：sql-corpus-invariant-reviewer-gate-report.md
- ⚠️ 警告：第 47 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312-05-hybrid-retrieval-report.md
- ⚠️ 警告：第 15 行 FAIL 声明可能无证据
- ⚠️ 警告：第 24 行 FAIL 声明可能无证据
- ⚠️ 警告：第 66 行 FAIL 声明可能无证据
- ⚠️ 警告：第 102 行 FAIL 声明可能无证据
- ⚠️ 警告：第 103 行 FAIL 声明可能无证据
- ⚠️ 警告：第 225 行 FAIL 声明可能无证据
- ⚠️ 警告：第 252 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：TEST_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：RELEASE_NOTES.md
- ⚠️ 文档可能缺少 provenance 元数据：V312_DAG_ANALYSIS.md
- ⚠️ 警告：第 61 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：V312-25_e2e_retire_report.md
- ⚠️ 警告：第 193 行 FAIL 声明可能无证据
- ⚠️ 警告：第 195 行 FAIL 声明可能无证据
- ⚠️ 警告：第 196 行 FAIL 声明可能无证据
- ⚠️ 警告：第 197 行 FAIL 声明可能无证据
- ⚠️ 警告：第 253 行 FAIL 声明可能无证据
- ⚠️ 警告：第 258 行 FAIL 声明可能无证据
- ⚠️ 警告：第 265 行 FAIL 声明可能无证据
- ⚠️ 警告：第 434 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ISSUES_PLAN.md
- ⚠️ 警告：第 14 行 FAIL 声明可能无证据
- ⚠️ 警告：第 16 行 FAIL 声明可能无证据
- ⚠️ 警告：第 118 行 FAIL 声明可能无证据
- ⚠️ 警告：第 140 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312_verification_report.md
- ⚠️ 警告：第 36 行 FAIL 声明可能无证据
- ⚠️ 警告：第 41 行 FAIL 声明可能无证据
- ⚠️ 警告：第 42 行 FAIL 声明可能无证据
- ⚠️ 警告：第 43 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：sqllogictest-oracle-gate-report.md
- ⚠️ 文档可能缺少 provenance 元数据：compliance-audit-access-control-report.md
- ⚠️ 文档可能缺少 provenance 元数据：DRAFT_ASSESSMENT_AND_ALPHA_GATE.md
- ⚠️ 警告：第 44 行 FAIL 声明可能无证据
- ⚠️ 警告：第 116 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：V312-30_reconciliation_report.md
- ⚠️ 警告：第 50 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312-04-embedding-provider-report.md
- ⚠️ 文档不存在（跳过检查）：V312-12-TPCH-CORRECTNESS.md
- ⚠️ 文档不存在（跳过检查）：REVIEWER_SIGNOFF_V312-19_SLICE3.md
- ⚠️ 文档不存在（跳过检查）：R2_INVARIANTS_REPORT.md
- ⚠️ 文档不存在（跳过检查）：V312-11-VERIFICATION.md
- ⚠️ 文档不存在（跳过检查）：smoke-report.md
- ⚠️ 文档不存在（跳过检查）：V312-14-CRASH-RECOVERY.md
- ⚠️ 文档不存在（跳过检查）：SURFACE_DISPOSITION.md
- ⚠️ 文档不存在（跳过检查）：V312-21-VERIFICATION.md
- ⚠️ 文档不存在（跳过检查）：DEFERRED_FOLLOWUPS.md
- ⚠️ 文档不存在（跳过检查）：ALL_TARGETS_REPORT.md
- ⚠️ 文档不存在（跳过检查）：V312-13-REPORT.md
- ⚠️ 警告：第 23 行 FAIL 声明可能无证据
- ⚠️ 警告：第 47 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：v312-07-rag-evidence-bundle-report.md
- ⚠️ 文档可能缺少 provenance 元数据：MYSQL_COMPAT_STATUS.md
- ⚠️ 警告：第 20 行 FAIL 声明可能无证据
- ⚠️ 警告：第 45 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：V312-29_gate_wiring_report.md
- ⚠️ 警告：第 34 行 FAIL 声明可能无证据
- ⚠️ 警告：第 101 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：CHANGELOG.md

---

### 通过项

- ✅ 无状态声明（无需证据检查）：README.md
- ✅ 状态声明有证据绑定：第 82 行
- ✅ 状态声明有证据绑定：第 87 行
- ✅ 状态声明有证据绑定：第 69 行
- ✅ 状态声明有证据绑定：第 75 行
- ✅ 状态声明有证据绑定：第 119 行
- ✅ 无状态声明（无需证据检查）：disabled-test-registry.md
- ✅ 状态声明有证据绑定：第 247 行
- ✅ 状态声明有证据绑定：第 250 行
- ✅ 状态声明有证据绑定：第 261 行
- ✅ 状态声明有证据绑定：第 292 行
- ✅ 无状态声明（无需证据检查）：sequence-executor-gap-assessment.md
- ✅ 状态声明有证据绑定：第 32 行
- ✅ 状态声明有证据绑定：第 33 行
- ✅ 状态声明有证据绑定：第 34 行
- ✅ 状态声明有证据绑定：第 226 行
- ✅ 状态声明有证据绑定：第 256 行
- ✅ 状态声明有证据绑定：第 25 行
- ✅ 状态声明有证据绑定：第 26 行
- ✅ 状态声明有证据绑定：第 27 行
- ✅ 状态声明有证据绑定：第 28 行
- ✅ 状态声明有证据绑定：第 29 行
- ✅ 状态声明有证据绑定：第 30 行
- ✅ 状态声明有证据绑定：第 31 行
- ✅ 状态声明有证据绑定：第 36 行
- ✅ 状态声明有证据绑定：第 44 行
- ✅ 无状态声明（无需证据检查）：compliance-audit-access-control-report.md
- ✅ 状态声明有证据绑定：第 139 行
- ✅ 状态声明有证据绑定：第 157 行
- ✅ 状态声明有证据绑定：第 15 行
- ✅ 状态声明有证据绑定：第 134 行

---

## 未验证声明（UNVERIFIED CLAIMS）

以下声明**无证据支撑**，不得用于门禁判断：

| 文档 | 行 | 声明内容 |
|------|-----|----------|
| - |  | V312-02 | GMP schema v3.12 | P0 | schema tests PASS | |
| - |  > 在 TPC-H correctness、SQLLogicTest、wire protocol、LOAD DATA、crash recovery、backup/restore 和 upgrade evidence 全部通过前，宣称 SQLRustGo v3.12.0 是广义 MySQL 5.7 替代品。 |
| - |  | V312-02 | GMP schema v3.12 | P0 | Schema tests PASS | |
| - |  本矩阵把 GMP 内审检索所需控制项映射到 SQLRustGo 的计划实现和测试证据。所有条目在真实测试和执行证据产生前，都必须保持 `PLANNED`，不得提前写成 PASS 或已完成。 |
| - |  **PASS — 154 tests passed, 0 failed** |
| - |  - [x] backup.rs 代码存在且编译通过 |
| - |  **状态: PASS — 满足关闭条件** |
| - |  `bash scripts/gate/check_arch_invariants.sh` → exit 0, 5 PASS / 0 FAIL |
| - |  | C-ARCH-01 | LocalExecutor has NO `txn_manager` field | **PASS** | |
| - |  | C-ARCH-02 | LocalExecutor has NO `write_buffer` field | **PASS** | |
| - |  | C-ARCH-03 | storage.insert/update/delete only in `crates/storage` or `crates/executor` | **PASS** (info: 14 storage operations in business crates — allowed per AD-002) | |
| - |  | C-ARCH-04 | No `eng.execute(raw_sql)` outside parser | **PASS** | |
| - |  | C-ARCH-05 | `execution_engine.rs` < 1600 lines (SSOT: CARCH05_LIMIT, AD-001 target 1500) | **PASS** (1594 lines) | |
| - |  PASS: C-ARCH-01 |
| - |  PASS: C-ARCH-02 |
| - |  PASS: C-ARCH-04 |
| - |  PASS: C-ARCH-05 (execution_engine.rs: 1594 lines, limit 1600, AD-001 target 1500) |
| - |  PASSED: 5 |
| - |  Result: ALL PASS |
| - |  | R2.6 | INT-2 + INT-3 deferred w/ plan (2 items, D7 PASS-WITH-DRIFT) | (carried — V312-22 plan) | V312-22 owner | (in plan) | |
| - |  PASS: ALL_TARGETS_REPORT.md fresh (age=N s) |
| - |  PASS: R2_INVARIANTS_REPORT.md fresh (age=N s) |
| - |  PASS: signoff valid (Reviewer A=hermes-z6g4, Reviewer B=openclaw, ...) |
| - |  PASS: signoff file is valid |
| - |  ==> V312-19 release gate PASSED |
| - |  | C-ARCH-01~05 | 5/5 PASS | This file (Part 1) + `scripts/gate/check_arch_invariants.sh` exit 0 | |
| - |  | R2.1-R2.8 driver | 4 PASS / 1 stub (R2.8) / 3 fail (R2.1/R2.4/R2.6/R2.7) | This file (Part 2) + `R2_INVARIANTS_REPORT.md` | |
| - |  | V312-19 release gates | PASS exit 0 | `check_v312_19_release_gates.sh` output | |
| - |  **Result:** 11/11 PASS, 0 FAIL |
| - |  | `test_wire_smoke_error_packet_structure` | PASS | ~20s | |
| - |  | `test_wire_smoke_load_data_sf1` | PASS | ~30s | |
| - |  | `test_wire_smoke_reset_clears_prepared_stmts` | PASS | ~25s | |
| - |  | `test_wire_smoke_reset_connection` | PASS | ~20s | |
| - |  | `test_wire_smoke_stmt_close` | PASS | ~25s | |
| - |  | `test_wire_smoke_stmt_close_nonexistent` | PASS | ~20s | |
| - |  | `test_wire_smoke_stmt_execute_after_close` | PASS | ~25s | |
| - |  | `test_wire_smoke_stmt_prepare_execute_int` | PASS | ~25s | |
| - |  | `test_wire_smoke_stmt_prepare_execute_varchar` | PASS | ~25s | |
| - |  | `test_wire_smoke_stmt_prepare_invalid_sql` | PASS | ~20s | |
| - |  | `test_wire_smoke_stmt_prepare_null` | PASS | ~25s | |
| - |  | C-ARCH invariants | `bash scripts/gate/check_arch_invariants.sh` | 5/5 PASS | |
| - |  | LOAD DATA INFILE | `bash scripts/gate/check_load_data_infile.sh` | 4/4 PASS (gate script) | |
| - |  | Anti-fabrication | `bash scripts/gate/check_anti_fabrication.sh` | ERRORS=0, PASS | |
| - |  | Wire smoke | `cargo test --test wire_smoke_mysql_cli` | 11/11 PASS | |
| - |    [PASS] LOAD_DATA_INFILE.md |
| - |    [PASS] LOAD DATA documented |
| - |    [PASS] parser changes documented |
| - |    [PASS] gate executable |
| - |  PASS: 4, FAIL: 0 |
| - |  Result: ALL PASS  [exit 0] |
| - |  **Result:** PASS (with graceful fallback) |
| - |  **PASS — 154 tests passed, 0 failed** |
| - |  - [x] ingestion.rs 代码存在且编译通过 |
| - |  **状态: PASS — 满足关闭条件** |
| - |  | Tests | ✅ | 10/10 PASS | |
| - |  **Evidence**: `cargo test -p sqlrustgo-executor window` → 10 tests PASS |
| - |  | `cargo test -p sqlrustgo-mysql-server --test wire_smoke_mysql_cli` | 11/11 PASS | |
| - |  | `bash scripts/gate/check_arch_invariants.sh` | 5/5 PASS | |
| - |  | `bash scripts/gate/check_load_data_infile.sh` | 4/4 PASS | |
| - |  PASS: ALL_TARGETS_REPORT.md fresh |
| - |  PASS: R2_INVARIANTS_REPORT.md fresh |
| - |  PASS: signoff file is valid |
| - |  ==> V312-19 release gate PASSED  [exit 0] |
| - |  [C-ARCH-01] PASS  [C-ARCH-02] PASS  [C-ARCH-03] INFO |
| - |  [C-ARCH-04] PASS  [C-ARCH-05] PASS |
| - |  Result: 5/5 PASS  [exit 0] |
| - |    [1/4] ✅ PASS: 3 VtuGuard marker calls in src/execution_engine.rs |
| - |    [2/4] ✅ PASS: 2 VtuGuard marker calls in openclaw_endpoints.rs |
| - |    [4/4] ✅ PASS: VtuGuard::assert_path_for_dml is public |
| - |  === G4 Gate: PASS === |
| - |  [C-ARCH-01] Checking LocalExecutor has NO txn_manager field... PASS |
| - |  [C-ARCH-02] Checking LocalExecutor has NO write_buffer field... PASS |
| - |  [C-ARCH-03] Checking storage.insert/update/delete only in crates/storage or crates/executor/... PASS |
| - |  [C-ARCH-04] Checking no eng.execute(raw_sql) outside parser... PASS |
| - |  [C-ARCH-05] Checking execution_engine.rs < 1600 lines... PASS (1594 lines) |
| - |  === Summary === PASSED: 5, FAILED: 0 |
| - |  - Draft gate 仍只表示开发入口准备，不表示实现通过。 |
| - |  > "提交命令、日志、PASS/FAIL、commit、evidence_hash" |
| - |  - **V312-24 work contribution**: 0 new errors. 9/5/6 = 20 lib tests (sqlancer/test-registry/test-runner) PASS; 10 V312-24 integration tests PASS. |
| - |  - **Result**: **Exit 0 (PASS)** |
| - |  - **Result**: **Exit 0 (PASS)** |
| - |  - **Result**: **CHECK 1.5 V312-24 = 3 PASS lines** (sqlancer 1000 iter, test-runner 14ms, both valid). Other 5 errors are pre-existing test compile failures (same root cause as Gate 1). |
| - |    [PASS]   V312-24: target/sqlancer-report.json valid (iterations=1000) |
| - |    [PASS]   V312-24: target/test-runner-report.json valid (total_duration_ms=14) |
| - |    [PASS]   V312-24: both SQLancer + test-runner report artifacts present and valid |
| - |  | 1 | Every tool can run + produce artifact | ✅ PASS | Gate 5: target/sqlancer-report.json (1000 iter) + target/test-runner-report.json (14ms) present + schema valid | |
| - |  | 2 | 15-item disposition table with reason + replacement gate + owner + expiry | ✅ PASS | V312-24 activation report §15-item table (16 items) | |
| - |  | 5 | `crates/sql-corpus/tests/corpus_test.rs` pass_rate ≥ 80% | ✅ PASS | 99.4% (813/818 cases) | |
| - |  **5/6 PASS + 1/6 PARTIAL (with V312-31 follow-up filed)**. |
| - |  **6 gate 实跑**: 5 numerical gates PASS (V312-24 work +0 errors) + 1 external gate (2 reviewer APPROVED) pending. |
| - |  | 5 | `bash scripts/gate/check_beta_gate.sh` B10_SQLANCER PASS | ⏸ TO RUN | 需 sqlancer-report.json (binary works; 30s run) | |
| - |  | 2 FAIL in `sqlrustgo-mysql-server` (`list_threads_returns_at_least_one`, `skip_auth_defaults_false`) | V312-17 Coverage (tracked in `docs/releases/v3.12.0/evidence/G2_test_count.txt`) | V312-17 owner | 2026-09-30 | mysql-server gate 仍 non-blocking (G2 gate 仍 PASS) | |
| - |  $ bash scripts/gate/check_beta_gate.sh 2>&1 | tee /tmp/v312_30_step_5_beta_gate.log | grep -E "B10_SQLANCER|PASS|FAIL" | head -5 |
| - |  1. 不允许"openspec 标 done"、"报告标题写已完成"、"PR 已合并"作为关闭证据 |
| - |  V312-30 **不能**在 PR merge + 2 reviewer APPROVED + 6 项 gate 实测通过 之前关闭。 |
| - |  **PASS — 154 tests passed, 0 failed** |
| - |  - [x] graph.rs 代码存在且编译通过 |
| - |  **状态: PASS — 满足关闭条件** |
| - |  **PASS — 154 tests passed, 0 failed** |
| - |  | audit hash chain 实跑证据 | DONE | `test_hash_chain_tamper_detection` 存在并 PASS | |
| - |  | fail-closed tamper 测试 | DONE | `test_permission_guard_fail_closed` 存在并 PASS | |
| - |  - [x] acl.rs / audit.rs 代码存在且编译通过 |
| - |  **状态: PASS — 满足关闭条件** |
| - |  > **真实性规则**: 本计划只定义未来工作和退出证据，不声明任何 v3.12.0 gate 已通过。 |
| - |  - 没有 command output、timestamp、source agent、source run、evidence hash 和 output location 就宣称 PASS、GA 或 compliance。 |
| - |  | `cargo build -p sqlrustgo_sqllogictest` | 可完成，但 `storage` 与 `executor` 依赖仍有 warning | 可作为 Alpha build evidence，但不得宣称 warning-free/clippy-clean，直到 `cargo clippy --all-features -- -D warnings` 通过 | |
| - |  | `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | runner 可完成；smoke corpus 为 6/16 文件通过，通过率 27.3% | 必须作为失败基线 triage，不得作为 release gate PASS | |
| - |  任何 PASS 声明都必须附 command output、timestamp、source_agent、source_run、evidence_hash 和 output location。 |
| - |  - PASS, GA, or compliance claims without command output, timestamp, source agent, source run, evidence hash, and output location. |
| - |  | `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | Runner completes; smoke corpus result is 6/16 files passing, 27.3% pass rate | Treat as a failing baseline to triage, not as a release gate PASS | |
| - |  - Gate script output showing PASS/FAIL with exit code. |
| - |  **PASS** — pass rate 99.4% ≥ 80.0% (baseline; V312-24 proposal 写的 27.3% 是 |
| - |  **PASS** — 14 个 per-subcategory guard + 2 个 meta test 全 PASS。 |
| - |  **PASS** — 5 failing cases pre-exist in baseline; pass rate 99.4% > 80% |
| - |  **PASS**. |
| - |  V312-24 proposal §4 描述"16 subcategories / 27.3% pass rate / 6/16 PASS"。 |
| - |  - 6/16 PASS → 实际是 14/14 subcategories 在 99.4% 范围内全 PASS（5 个 failing case 散落在子目录中） |
| - |  按"以实际 gate 数字关闭"的严格要求：4/4 关闭边界 PASS。V312-28 可关闭。 |
| - |  **PASS** — 3 个脚本实际命令中 0 个 `|| true` (baseline 223 个)。 |
| - |  **PASS** — 没有 `2>/dev/null || true` 吞错；server 不可达时 set -euo pipefail 触发 exit 1。 |
| - |  **PASS** — 缺失时 fail-explicit，证据写入 `/tmp/backup_restore_evidence.txt`。 |
| - |  **PASS** — 缺失时 fail-explicit，stderr 含 "sysbench not found"。 |
| - |  **PASS** — server 不可达时 exit 1。byte-exact 断言替代了原 `grep -q "name"` 宽松匹配。 |
| - |  **PASS**. |
| - |  按"以实际 gate 数字关闭"的严格要求：6/6 关闭边界 PASS。V312-26 可关闭。 |
| - |  | Every tool can run + produce artifact (sqlancer + test-runner + test-registry all executable) | ✅ PASS | `target/sqlancer-report.json` + `target/test-runner-report.json` + `target/test-registry.toml` all generated and validated by integration tests | — | |
| - |  | Every retired/deferred item has reason + replacement gate + owner + expiry (15-item disposition table) | 🟡 PARTIAL | 12 items in the table are PASS (Phase 1 activation); 3 retired/deferred rows now map to V312-25 / V312-27 / V312-28 with owner + expiry; remainder is follow-up scope | V312-25 / V312-27 / V312-28 | |
| - |  **Net result for this PR**: 1 of 6 acceptance criteria are PASS outright; |
| - |  2 are PARTIAL (PASS in artifact terms, FAIL in gate wiring terms — gated on |
| - |  cli_smoke             2/2 PASS |
| - |  toml_round_trip       3/3 PASS |
| - |  managed_dispatch      2/2 PASS |
| - |  timeout_enforced      3/3 PASS |
| - |  "openspec 标 done" / "报告标题写已完成" / "PR 已合并" 不允许关闭。 |
| - |  V312-24 本身**不能**在 V312-25 ~ V312-30 全部 PASS 之前关闭。 |
| - |  | 1 | `crates/sqlancer` | **activate** | minimax | 2026-08-25 | `cargo run -p sqlancer -- --duration 1` exit 0 + `target/sqlancer-report.json` 存在 (`cli_smoke` test PASS) | |
| - |  | 2 | `crates/test-runner` | **activate** | minimax | 2026-08-25 | `cargo run -p test-runner -- --manifest X --max-parallel 3` exit 0 + `target/test-runner-report.json` 存在 (`managed_dispatch` test PASS) | |
| - |  | 3 | `crates/test-registry` | **activate** | minimax | 2026-08-25 | `test-registry-cli init` 写 `test-registry.toml` + `from_toml`/`write_toml` round-trip OK (`toml_round_trip` test PASS) | |
| - |  | 12 | `tests/e2e/e2e_beta_test.rs` 5 double-skip tests | **fix** | minimax | 2026-08-25 | V312-27 删 5 处 `is_e2e_disabled` early-return; `CI=1 cargo test --test e2e_beta_test -- --ignored` = 6 PASS | |
| - |  **V312-24 自身关闭条件**：上表 16 项全 PASS + V312-30 sign-off 报告存在 + PR mergedAt 非空 + 2 reviewer APPROVED + evidence_hash 重新计算。 |
| - |  **PASS — 154 tests passed, 0 failed** |
| - |  - [x] schema/version/chunk/relation/audit/document 模块代码存在且编译通过 |
| - |  **状态: PASS — 满足关闭条件** |
| - |  | `test_wal_entry_large_payload` | ✅ PASS | |
| - |  | `test_wal_entry_serialization_roundtrip` | ✅ PASS | |
| - |  | `test_wal_checkpoint_recovery` | ✅ PASS | |
| - |  | `test_wal_concurrent_transactions_isolation` | ✅ PASS | |
| - |  | `test_wal_mixed_operations` | ✅ PASS | |
| - |  | `test_wal_recovery_after_crash` | ✅ PASS | |
| - |  | `test_wal_rollback_recovery` | ✅ PASS | |
| - |  | `test_wal_recovery_with_pending_transaction` | ✅ PASS | |
| - |  | `test_wal_single_transaction` | ✅ PASS | |
| - |  **Result**: 16/16 WAL integration tests PASS ✅ |
| - |  | `test_crash_recovery_reconstructs_pages` | ✅ PASS | |
| - |  | `test_f26_crash_recovery` | ✅ PASS | |
| - |  **Result**: 2/2 storage crash tests PASS ✅ |
| - |  **Result**: 6/8 tests PASS, 2 FAIL ⚠️ |
| - |  | `test_upgrade_gate_script_exists` | ✅ PASS | |
| - |  | `test_upgrade_script_has_required_functions` | ✅ PASS | |
| - |  | `test_upgrade_script_exists` | ✅ PASS | |
| - |  | `test_upgrade_script_syntax` | ✅ PASS | |
| - |  **Result**: 4/4 upgrade tests PASS ✅ |
| - |  | WAL Integration | ✅ PASS (16/16) | None | |
| - |  | Storage Crash Recovery | ✅ PASS (2/2) | None | |
| - |  | Upgrade Path | ✅ PASS (4/4) | None | |
| - |  **PASS** — 13/13 passed including `test_vtu_guard_wraps_storage` (no longer |
| - |  **PASS** — 旧的 `rows.len() <= 6` (accepts 0 rows) 改为 `rows.len() > 0` |
| - |  加载好或 Q1 完全失败 — 之前是 PASS 静默吞，现在 fail-explicit。 |
| - |  **PASS** — 6 passed (5 个 e2e_XX 加上 e2e_beta_manifest_contains_6_scenarios |
| - |  **PASS** — stale + union 全部 0。 |
| - |  **PASS**. |
| - |  按"以实际 gate 数字关闭"的严格要求：5/5 关闭边界 PASS。V312-27 可关闭。 |
| - |  **v3.11.0 GA 6/6 gates PASS — v3.12.0 不继承 hidden 弱项**。 |
| - |  | G1 R1-R4 RC 指标 | ✅ PASS | 否 | |
| - |  | G2 Full test suite | ⚠️ 2,666 tests (2,664 PASS + 2 FAIL non-blocking + 3 IGNORED) | 否 | |
| - |  | G5 Security audit | ✅ PASS | 否 | |
| - |  | sqlrustgo-executor | 685 | ✅ PASS | 2026-08-09 18:00+0800 | |
| - |  | sqlrustgo-storage | 683 | ✅ PASS | 2026-08-09 18:00+0800 | |
| - |  | sqlrustgo-parser | 589 (3 ignored) | ✅ PASS | 同上 | |
| - |  | sqlrustgo-catalog | 183 | ✅ PASS | 同上 | |
| - |  | sqlrustgo-planner | 84 | ✅ PASS | 同上 | |
| - |  | sqlrustgo-common | 79 | ✅ PASS | 同上 | |
| - |  | sqlrustgo-mysql-client | 79 | ✅ PASS | 同上 | |
| - |  | sqlrustgo-admin | 69 | ✅ PASS | 同上 | |
| - |  | sqlrustgo-cache | 10 | ✅ PASS | 同上 | |
| - |  | 22/22 实跑 PASS | ✅ | ✅ | ✅ | both | both | |
| - |  **Source report**: `docs/releases/v3.11.0/TPCH_SF1_22_22_PASS_REPORT.md` |
| - |  | Pre-flight (home/readlink/buildroot) | ✅ PASS | — | |
| - |  | **G3 覆盖率** (3 crates < 80%) | PASS (gate = tools 80.31%) | **CARRIED** | V312-17 | |
| - |  | **G4 TPC-H zero-row correctness** | 22/22 不 OOM (PASS) | **CARRIED** | V312-12 (PG SHA256) | |
| - |    [A1_BUILD] PASS |
| - |    [PASS] docs/releases/v3.12.0/BLOCKER_DISPOSITION_V311.md exists |
| - |  --- Gap 2: GA gates PASS verification --- |
| - |    [PASS] 10 crates have test counts recorded |
| - |    [PASS]        2 crates < 80% (acceptable, tracked to V312-17) |
| - |    [PASS] G4 TPC-H SF=1 22/22 verified |
| - |    [PASS] SOAK 343h37m (2.04x) verified |
| - |    [PASS] All 5 remotes synced to v3.11.0-ga |
| - |  PASS: 7 / 7 |
| - |  V312-01 blocker disposition: PASS |
| - |  **Gate execution**: 7/7 PASS — v3.12.0 ALPHA promotion 入口通畅。 |
| - |  | `test_composite_btree_index_insert` | ✅ PASS | |
| - |  | `test_composite_btree_index_insert_unique` | ✅ PASS | |
| - |  | `test_composite_btree_index_search` | ✅ PASS | |
| - |  | `test_composite_btree_index_range_query` | ✅ PASS | |
| - |  | `test_composite_btree_index_num_columns` | ✅ PASS | |
| - |  | parser_fixtures | `cargo test -p sqlrustgo-parser...` | ✅ PASS (34/34) | |
| - |  | R2.1-R2.3 | ✅ PASS | None | |
| - |  | parser_fixtures | ✅ PASS | None | |
| - |  **PASS — 154 tests passed, 0 failed** |
| - |  - [x] retrieval.rs 代码存在且编译通过 |
| - |  **状态: PASS — 满足关闭条件** |
| - |  | V312-G11 | SQLite SQLLogicTest 判定门禁 | `sqlrustgo_sqllogictest` runner + SQLite 官方/缓存语料 | 选定目标通过，或每个排除项都关联 issue | |
| - |  | V312-G13 | MySQL wire protocol 硬化 | COM_QUERY、COM_STMT、error、reset、TLS、compression E2E | 产生确定性的通过/失败 artifact | |
| - |  | V312-G17 | Window/GIS/JSON 受控功能 | ROW_NUMBER/RANK/DENSE_RANK、JSON path、ST_Distance/ST_Intersects/GeoJSON fixtures | 支持范围内全 PASS；超出范围有明确错误和文档 | |
| - |  | V312-G22 | MySQL 兼容与 SQL surface 回归 | SHOW、auth、prepared statements、ALTER、TIMESTAMP、连接池、函数、列级权限 fixtures | GMP/生产路径相关项 PASS；非目标项有 explicit unsupported/deferred 证据 | |
| - |  每个 PASS claim 必须包含：command、timestamp、source agent、source run、evidence hash、output location 和 PASS/FAIL boundary。 |
| - |  - 168h mixed SOAK 未完成，或没有证据却写成 PASS。 |
| - |  | Alpha | `cargo build -p sqlrustgo_sqllogictest` 成功；runner `--help` 可用 | |
| - |  | RC | curated SQLite-compatible subset 运行，并输出 PASS/FAIL/SKIP 分类和 issue-linked exclusions | |
| - |  | GA | selected SLT targets 全部通过，或每个 skipped/failed group 都有 issue、owner、expiry、rationale | |
| - |  | `cargo run -p sqlrustgo_sqllogictest -- --test-dir crates/sqlrustgo_sqllogictest/testdata` | runner 可完成；6/16 文件通过，通过率 27.3% | v3.12 必须 triage failures、分类 expected incompatibilities，并在 Beta/RC 前提升 smoke gate | |
| - |  | G3 coverage 口径漂移 | 单一 canonical coverage command，保存输出，不混用 PASS claim | |
| - |  | v3.6 Beta PENDING 与覆盖率/测试编译延续问题 | historical disposition + current build/test/coverage sampling，不得沿用历史 PASS claim | |
| - |  | V312-G11 | SQLite SQLLogicTest oracle | `sqlrustgo_sqllogictest` runner + SQLite official/cached corpus | selected targets PASS or issue-linked exclusion | |
| - |  Every PASS claim must include |
| - |  - PASS/FAIL boundary |
| - |  - 168h mixed SOAK is not completed or is described as PASS without evidence. |
| - |  | RC | Curated SQLite-compatible subset runs with PASS/FAIL/SKIP classification and issue-linked exclusions | |
| - |  | G3 coverage口径漂移 | Single canonical coverage command, stored output, no mixed PASS claims | |
| - |  - 禁止声明 v3.12.0 已通过 Alpha/Beta/RC/GA。 |
| - |  - 禁止声明 SQLLogicTest、TPC-H correctness、wire、LOAD DATA、recovery 或 GMP compliance 已通过。 |
| - |  - 数据行数：20（PASS: 11 / unsupported: 2 / deferred: 7 / fail: 0） |
| - |  - **PASS surface**: `alter_add_column`, `alter_drop_column`, `alter_modify_column`, `alter_rename`, `show_tables`, `group_concat` (意外实现), `stddev_pop` (意外实现), `with_cube` (意外实现), `with_rollup` (意外实现), `var_pop` (意外实现), `replace_into` (意外实现) |
| - |  **PASS** — 10 unique file paths deleted in git history. |
| - |  **PASS** — only `e2e_07_json_vector.sh` remains (kept for V312-26 to rewrite). |
| - |  **PASS** — 10 retired entries added (one per retired script). |
| - |  **PASS** — 唯一引用被删脚本的代码是 `tests/baseline/ignore_registry.json` 中的 10 条 retired entries（也是 V312-25 自己加的）。**没有任何 Rust test 代码引用被删的 10 个脚本**。cargo check 输出集合的差异是并行编译 race（同一 sqlrustgo test 在 A 编译失败 B 编译成功），与 V312-25 删的 shell 脚本无关。 |
| - |  **PASS**. |
| - |  按"以实际 gate 数字关闭"的严格要求：5/5 关闭边界 PASS。V312-25 可关闭。 |
| - |  **验收**: blocker disposition report、stage gate output，以及所有 PASS claim 的 evidence hash。 |
| - |  **验收**: `cargo build -p sqlrustgo_sqllogictest` 成功；本地 smoke corpus 产生报告；官方/cached SQLite corpus 有 manifest、hash、file count、exclusion policy；新增或规划 `scripts/gate/check_sqllogictest_v312.sh`。 |
| - |  **验收**: 对 GMP/生产路径相关子集给出 fixture PASS；非目标项必须输出 explicit unsupported 或 deferred decision，不得在 release note 中无边界宣称支持。 |
| - |  12 个 task 全完成（sqlancer / test-runner / test-registry 三套 `[[bin]]` + 主入口 + JSON artifact + TOML manifest 持久化 + JoinSet 并行 + `tokio::time::timeout` 强制），新增 4 个 integration test 文件共 10 个 test 全 PASS，lib 测试 20/20 PASS，clippy strict 0 error，fmt clean。详见 `docs/releases/v3.12.0/V312-24_test_infra_activation_report.md`。 |
| - |  2. 失败注入测试 1：`mv scripts/gate/e2e/e2e_07_fixture.json{,.bak} 2>/dev/null; bash scripts/gate/e2e/e2e_07_json_vector.sh; echo "exit=$?"; mv scripts/gate/e2e/e2e_07_fixture.json{.bak,}` 退出码 **≠ 0**（验证 byte-exact 断言生效，不再静默 PASS）。 |
| - |  **禁止关闭条件**: 仅以"测试通过"或"无 `\|\| true`"为依据；必须含上述 2 个失败注入的实测 log。 |
| - |  **Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-28_baseline_evidence.txt`（**实测 2026-08-09**：99.4% pass-rate / 14 subcategories / 103 .sql files / 818 cases / 5 failing — V312-24 proposal §4 写的"16 subcategories / 27.3% / 6/16 PASS"严重过时） |
| - |  **禁止关闭条件**: (a) 仅靠"打开了 follow-up 任务"或"修了一部分 subcategory"；(b) 不接受"V312-24 proposal 27.3% 是 baseline" 之类的过时引用；(c) 14 个守护 test 必须有可识别的命名或注释才能算 PASS；(d) 无 sha256 不允许关闭。 |
| - |  2. `bash scripts/gate/check_beta_gate.sh 2>&1 | grep B10_SQLANCER` 输出含 `check_fail`（baseline 是 `check_warn`）；`bash scripts/gate/check_beta_gate.sh` 退出 **0** 且日志含 `B10_SQLANCER PASS`。 |
| - |     - `bash scripts/gate/check_beta_gate.sh` B10_SQLANCER 通过 |
| - |  **禁止关闭条件**: (a) 不允许"openspec 标 done"、"报告标题写已完成"、"PR 已合并"作为关闭证据；(b) 不允许用 baseline evidence_hash 顶替关闭时重算的 hash；(c) 2 pre-existing FAIL/errors 在 mysql-server / storage 若仍未修，V312-30 必须显式列在豁免清单（带 owner + expiry）。 |
| - |  - Evidence hashes for any PASS claim. |
| - |  - GA gate report contains only evidence-backed PASS claims. |
| - |  - The selected SLT corpus has PASS/FAIL/SKIP classification. |
| - |  2. Issue 评论包含 PR 编号、commit SHA、执行命令、PASS/FAIL 摘要、evidence_hash |
| - |  - **测试**: 9 PASS / 1 unsupported / 4 deferred |
| - |  | demo.test | PASS | Basic SELECT | |
| - |  | insert__test_insert_invalid.test | PASS | Error cases | |
| - |  | delete__test_delete.test | PASS | DELETE operations | |
| - |  | update__test_update.test | PASS | UPDATE operations | |
| - |  | constraints__test_not_null.test | PASS | NOT NULL constraint | |
| - |  - 文档链接和一致性检查通过。 |
| - |  Draft 阶段可以移交 Hermes/OMP 进入 Alpha 开发准备。任何 Alpha PASS、功能完成或门禁通过声明，必须等待对应命令实跑并产生 evidence hash。 |
| - |  | `bash scripts/gate/check_sqllogictest_v312.sh` | PASS，4/4 entry checks | 仅为 smoke baseline；本地 corpus 6/16，pass rate 27.3%；不代表官方 SQLite corpus 已集成 | |
| - |  | `bash scripts/gate/check_alpha_v3.12.0.sh` | PASS，13/13 | 仅代表 Alpha 入口准备完成，不代表业务功能完成 | |
| - |  | `bash scripts/gate/check_stage.sh --version v3.12.0 --dry-run` | PASS，DRAFT dry-run 可解析 | 不执行 cargo build | |
| - |  | `bash scripts/gate/check_stage.sh --version v3.12.0 --stage ALPHA --dry-run` | PASS，ALPHA dry-run 可解析 | 不执行 ALPHA 全量 gate | |
| - |  | `bash scripts/gate/check_stage.sh --version v3.12.0` | PASS，4/4 | DRAFT stage gate 实跑通过，含 docs links 和 `cargo build --all-features` | |
| - |  2. 失败时 `exit 1` (而非 `PASS=$((PASS+1))` 静默) |
| - |    PASS: sysbench ran 3s with 1 thread(s), 1402 events at 466.83 events/sec |
| - |  === E2E sysbench_smoke_test: PASS === |
| - |  **PASS — 154 tests passed, 0 failed** |
| - |  - [x] vector_index/vector_search/embedding 模块代码存在且编译通过 |
| - |  **状态: PASS — 满足关闭条件** |
| - |  **PASS — 154 tests passed, 0 failed** |
| - |  - [x] rag.rs 代码存在且编译通过 |
| - |  **状态: PASS — 满足关闭条件** |
| - |  | `wire_smoke_mysql_cli.rs` | 11/11 PASS | |
| - |  | Architecture invariants (C-ARCH-01~05) | 5/5 PASS | |
| - |  | `check_load_data_infile.sh` | 4/4 PASS | |
| - |  - C-ARCH-01~05 invariants (5/5 PASS) |
| - |  **PASS** — SQLancer 部分（line 110-118）已去 `|| true`，加 `if [[ ! -s |
| - |  **PASS** — baseline `check_warn` 已升级为 `check_fail`，加 B10_SQLANCER_REPORT |
| - |  **PASS** — 8 个 scenarios 改 4 个 (V312-25 retired 3 stale mirror, V312-26 |
| - |  **PASS** — P16 step 2.5/3 新增，扫出 **29 个 `|| true` 掩盖**（在 9 个 gate |
| - |  **PASS**. |
| - |  按"以实际 gate 数字关闭"的严格要求：5/5 关闭边界 PASS（条件 4 校订后）。 |
| - |  - 2026-08-09 本地基线：`cargo build -p sqlrustgo_sqllogictest` 可完成但依赖仍有 warning；本地 smoke corpus 可运行，但当前仅 6/16 文件通过，通过率 27.3%，因此这是失败基线，不是 gate PASS。 |
| - |  | `wire_load_data/V312-13-REPORT.md` | `check_v312_13_wire_load_data.sh` | 10 步 evidence 表 (5 PASS, 4 deferred, 1 pre-existing fail from `check_load_data_infile.sh`) | |
| - |  | `mysql_compat/SURFACE_DISPOSITION.md` | `check_v312_21_mysql_compat.sh` | 10 v3.7-v3.10 历史 surface 的 decision 表 (PASS/unsupported/deferred) | |
| - |  | 测试目标 | 通过 | 失败 | |
| - |  | `scripts/gate/check_v312_13_wire_load_data.sh` | V312-13 evidence | 5/5 typed-wrapper+regression PASS, 4 deferred, 1 pre-existing fail | |
| - |  | `scripts/gate/check_v312_19_release_gates.sh` | RC/GA 阻断 gate | PASS (artifacts fresh) | |
| - |    alter_modify_column 全部 PASS** (5 个新增 PASS) |
| - |    4 个 fixture 之前标 `UNSUPPORTED` 但 server 静默接受语法。改成 `PASS-with-caveat` |
| - |    后: **4 个新增 PASS** (server 解析但语义不实现, 文档化在 release notes) |
| - |  最终 disposition: **9 PASS / 1 unsupported / 4 deferred / 0 fail** (vs slice 2: 1/5/3/4) |
| - |    (600 lineitem rows / 62 KB), 8 个表全部 LOAD DATA 成功, 0.7s 内完成 |
| - |  - V312-13 / V312-19 / V312-21 gates: ALL PASS |
| - |    - 新增 PASS surface (6 个意外实现): `group_concat`, `stddev_pop`, `with_cube`, `with_rollup`, `var_pop`, `replace_into` |
| - |    - 最终结果: **11 PASS / 2 unsupported / 7 deferred / 0 fail** |
| - |  - 2026-08-09 local baseline: `cargo build -p sqlrustgo_sqllogictest` completes with dependency warnings; local smoke corpus runs but currently reports 6/16 files passing and 27.3% pass rate, so this is a failing baseline rather than a gate PASS. |

**规则**: UnverifiedDoc 不得用于门禁判断

---

## Anti-Fabrication Policy 合规状态

| 要求 | 状态 |
|------|------|
| PASS/FAIL 声明绑定 CI 证据 | ❌ 违规 |
| 门禁结果绑定 gate_policy_eval_id | ❌ 违规 |
| 计划文档无 GA Final 伪造 | ❌ 违规 |
| provenance 元数据存在 | ⚠️  部分缺失 |

---

## 后续行动

### 必须执行的修复

1. **立即停止**当前门禁流程
2. **回退**到上一个 VerifiedDoc 状态
3. **补充**真实证据（CI run ID + log hash）
4. **重新**执行门禁检查

### 问责记录

- 违规次数: 304
- 违规类型: Type A（虚构执行）/ Type B（伪门禁）/ Type D（伪任务完成）
- 处理方式: 触发 Anti-Fabrication Policy 问责机制


---

*报告生成时间: 2026-08-10 07:49:07*
*检查工具版本: check_evidence_binding.sh v1.0.0*
*依据政策: Anti-Fabrication Policy v1.0.0*

---

## 参考：违规类型定义

| 类型 | 定义 | 严重程度 |
|------|------|----------|
| **Type A** | 虚构执行：AI 声称测试通过但无 CI 日志支撑 | P0 |
| **Type B** | 伪门禁：AI 生成门禁通过但无 gate engine 输出 | P0 |
| **Type C** | 伪证据：AI 引用不存在的 CI run / log hash | P1 |
| **Type D** | 伪任务完成：AI 标记任务完成但代码未合并 | P1 |

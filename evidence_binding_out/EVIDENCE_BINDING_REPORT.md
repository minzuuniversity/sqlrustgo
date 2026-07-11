# vv3.9.0 证据绑定检查报告

> **检查日期**: 2026-07-11
> **版本**: v3.9.0
> **Auditor**: Hermes Agent (Anti-Fabrication Policy v1.0)
> **检查工具**: check_evidence_binding.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | 68 |
| 警告 | 177 |
| 失败（违规） | 424 |
| 未验证声明 | 419 |

**总结**: ❌ 发现 424 个违规（Type A/B/C/D）

---

## 违规详情（按类型分类）

### Type A: 虚构执行（Execution Fabrication）

无 CI 证据声明"测试通过 / 编译成功"

- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: | **Gate 脚本执行** | 🟢 **HIGH** | 16/16 PASS (form-only), 6/6 m
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | **长期稳定性** | 🔴 **LOW** | G7/G13 标注 PASS 实为 SIMULATED (1,440
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: | "16/16 G1-G16 PASS" | 🟡 **部分可信** | gate 脚本执行完成, 但 11/16 无 
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: | "6/6 meta-gates PASS" | 🟢 **可信** | P11-P16 meta-gate detec
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: | "330+ tests PASS" | 🟡 **部分可信** | 测试执行完成, 但部分自验证, 无 oracle 
- ❌ Type A/B 违规：第 136 行声明无 CI/gate/commit 证据: | INDEX.md | "G1-G16 全部 PASS" | 同上, **且未提 Coverage 缺失** | **
- ❌ Type A/B 违规：第 153 行声明无 CI/gate/commit 证据: | Meta-gate 验证 | N/A | ✅ HIGH | ✅ HIGH | 6/6 P11-P16 PASS |
- ❌ Type A/B 违规：第 329 行声明无 CI/gate/commit 证据: **GA 阻塞**: V4 (oracle 11 gates) + **V9 (Coverage Gate 缺失, 新发
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 101 行声明无 CI/gate/commit 证据: | Multi-table join (3+) | ✅    | Q7/Q8/Q9 PASS (up to 8 tabl
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: **All 4 ladder steps PASS post-WAL-fix.** Server stable for 
- ❌ Type A/B 违规：第 145 行声明无 CI/gate/commit 证据: The work that COULD be done by Claude is DONE
- ❌ Type A/B 违规：第 160 行声明无 CI/gate/commit 证据: - All 6/6 meta-gates PASS throughout
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: > **Status: 🟡 READY (gates G1-G16 PASS, 24h/72h/168h soak in
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | G1 | TPC-H 22/22 (QPS-correctness) | ✅ PASS | tpch_gate_te
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | G2 | INT-2 ParallelExecutor | ✅ PASS | int2_substance_para
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | G3 | INT-3 Single Expression | ✅ PASS | int3_substance_del
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: | G4 | ARCH-3 VtuGuard | ✅ PASS | check_arch3_no_bypass.sh (
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | G5 | SEM-1 Savepoint | ✅ PASS | check_sem1_savepoint.sh (8
- ❌ Type A/B 违规：第 31 行声明无 CI/gate/commit 证据: | G6 | Backup/Restore/PITR | ✅ PASS | check_backup_restore.s
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | G8 | Crash Matrix | ✅ PASS | check_p12_crash_test.sh |
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: | G9 | Upgrade v3.8→v3.9 | ✅ PASS | check_p14_upgrade_test.s
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | G10 | GMP Audit + Time Travel + Hash Chain | ✅ PASS | chec
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | G15 | SF=0.01 TPC-H wire | ✅ PASS | tpch_sf01_22_queries_w
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: | G16 | Compatibility v3.8→v3.9 | ✅ PASS | v380_to_v390_full
- ❌ Type A/B 违规：第 41 行声明无 CI/gate/commit 证据: **Total: 11/13 PASS, 1/13 incomplete (G11), 1/13 pending re-
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: | Substance tests | 41 | 41/41 PASS |
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: | TPC-H wire (G1) | 22 | 22/22 PASS |
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: | TPC-H wire SF0.01 (G15) | 22 | 22/22 PASS |
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: | Upgrade (G9, G16) | 55 | 55/55 PASS |
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: | Backup/Restore (G6) | 51 | 51/51 PASS |
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: | Crash Matrix (G8) | 129 | 129/129 PASS |
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: | Stability (G7) | 10 | 10/10 PASS |
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: | E2E SELECT (2026-07-04) | 16 | 16/16 PASS |
- ❌ Type A/B 违规：第 74 行声明无 CI/gate/commit 证据: | **Total verified tests** | **346+** | **346+ / 346+ PASS**
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: | 1h simulated | rc4 readiness | Z6G4 (rc4 binary) | ✅ PASS 
- ❌ Type A/B 违规：第 124 行声明无 CI/gate/commit 证据: - [x] G1-G10 gates PASS
- ❌ Type A/B 违规：第 135 行声明无 CI/gate/commit 证据: - [ ] TPC-H SF=1.0 Q1-Q22 PASS (hardware-blocked: disk)
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：GA_GATE_REPORT.md
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: **结论**: ✅ 可升级 — 编译通过，clippy 零警告，覆盖率测试可运行。
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | `rustup update stable` | ✅ 成功 |
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: 3. 所有 `ColumnDefinition { ... }` 构造通过 `..Default::default()`
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | G15 oracle 22/22 | ✅ | 5 sub-tests PASS sequentially in 18
- ❌ Type A/B 违规：第 57 行声明无 CI/gate/commit 证据: - **Soak**: PID 104564, in-process, 1 QPS, 3475 queries done
- ❌ Type A/B 违规：第 90 行声明无 CI/gate/commit 证据: - [x] All pre-soak gates PASS (6/6 meta-gates + clippy + fmt
- ❌ Type A/B 违规：第 106 行声明无 CI/gate/commit 证据: G15 sub-tests = 5/5 PASS in 7-100s each
- ❌ Type A/B 违规：第 107 行声明无 CI/gate/commit 证据: 6/6 meta-gates verified PASS
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | G1 | TPC-H 22/22 | ✅ PASS | tpch_gate_test 22/22 (sub-gate
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: | G2 | INT-2 | ✅ PASS | int2_substance_parallel_test (9 test
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | G3 | INT-3 | ✅ PASS | int3_substance_delegation_test (17 t
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | G4 | ARCH-3 VtuGuard | ✅ PASS | check_arch3_no_bypass.sh (
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | G5 | SEM-1 Savepoint | ✅ PASS | check_sem1_savepoint.sh (8
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | G6 | Backup/Restore | ✅ PASS | check_backup_restore.sh (6/
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: | G8 | Crash Matrix | ✅ PASS | check_p12_crash_test.sh |
- ❌ Type A/B 违规：第 41 行声明无 CI/gate/commit 证据: | G9 | Upgrade v3.8→v3.9 | ✅ PASS | check_p14_upgrade_test.s
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: | G10 | GMP Audit + Time Travel + Hash Chain | ✅ PASS | chec
- ❌ Type A/B 违规：第 43 行声明无 CI/gate/commit 证据: | G11 | QPS/TPS Benchmark | ✅ PASS | check_g11_qps.sh (5/5) 
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: | G13 | 24h Stability | 🟡 partial | 250: partial (843 sample
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: | G16 | Compatibility v3.8→v3.9 | ✅ PASS | check_g16_compati
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: **Total: 13/13 PASS** + 1 🟡 partial (G13 24h real incomplete
- ❌ Type A/B 违规：第 53 行声明无 CI/gate/commit 证据: | `tests/g2_substance_parallel_executor_test.rs` | 4 | ✅ PAS
- ❌ Type A/B 违规：第 54 行声明无 CI/gate/commit 证据: | `tests/int2_substance_parallel_test.rs` | 9 | ✅ PASS |
- ❌ Type A/B 违规：第 55 行声明无 CI/gate/commit 证据: | `tests/int3_substance_delegation_test.rs` | 17 | ✅ PASS |
- ❌ Type A/B 违规：第 56 行声明无 CI/gate/commit 证据: | `tests/upgrade_chain_v3_6_to_v3_9_test.rs` | 6 | ✅ PASS |
- ❌ Type A/B 违规：第 88 行声明无 CI/gate/commit 证据: | 3 | 测试已通过 | ✅ Substance tests + G1-G16 all green |
- ❌ Type A/B 违规：第 96 行声明无 CI/gate/commit 证据: - 8 项修改完成，12/12 复核 PASS ✅
- ❌ Type A/B 违规：第 115 行声明无 CI/gate/commit 证据: | Draft → Alpha | 架构设计, 编译通过 | ✅ Done (2026-06-05) |
- ❌ Type A/B 违规：第 178 行声明无 CI/gate/commit 证据: - 13/13 核心 gates PASS
- ❌ Type A/B 违规：第 179 行声明无 CI/gate/commit 证据: - 36/36 substance tests PASS
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：GA_GATE_STATUS_REPORT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：GA_GATE_STATUS_REPORT.md
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: [L1] cargo build...           [PASS]
- ❌ Type A/B 违规：第 81 行声明无 CI/gate/commit 证据: [L1] cargo test --lib...      [PASS]
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: [L1] clippy...                [PASS]
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据: [L1] cargo fmt...             [PASS]
- ❌ Type A/B 违规：第 84 行声明无 CI/gate/commit 证据: === Gate Result: PASSED ===
- ❌ Type A/B 违规：第 104 行声明无 CI/gate/commit 证据: - 本地 4 个快速 gate 全 PASS: `check_arch_invariants` (5/5), `chec
- ❌ Type A/B 违规：第 115 行声明无 CI/gate/commit 证据: bash scripts/gate/check_arch_invariants.sh:    5/5 PASS
- ❌ Type A/B 违规：第 117 行声明无 CI/gate/commit 证据: bash scripts/gate/check_arch3_no_bypass.sh:    PASS
- ❌ Type A/B 违规：第 118 行声明无 CI/gate/commit 证据: bash scripts/gate/check_integration_gate.sh:   PASS (4/4)
- ❌ Type A/B 违规：第 119 行声明无 CI/gate/commit 证据:   SGL-001 (B4 Format): PASS
- ❌ Type A/B 违规：第 120 行声明无 CI/gate/commit 证据:   SGL-002..005: PASS
- ❌ Type A/B 违规：第 121 行声明无 CI/gate/commit 证据:   WAL lifecycle (INV-1/2/3): PASS
- ❌ Type A/B 违规：第 123 行声明无 CI/gate/commit 证据:   A7-3 ExecutionEngine: PASS (< 1500 lines as per AD-001)
- ❌ Type A/B 违规：第 144 行声明无 CI/gate/commit 证据: | **Backup/Restore 100+ 场景** | 全量/增量/时间点恢复 | 3 | ✅ RC7 PASS 
- ❌ Type A/B 违规：第 145 行声明无 CI/gate/commit 证据: | **Crash Matrix 100+ 场景** | kill -9 / OOM / disk full | 3 |
- ❌ Type A/B 违规：第 146 行声明无 CI/gate/commit 证据: | **24h Soak Test** | 1M txns 浸泡 | 4 | ✅ RC7 PASS (simulated
- ❌ Type A/B 违规：第 186 行声明无 CI/gate/commit 证据: | G7 24h Soak | ✅ PASS (simulated) / ❌ INCOMPLETE (real) | P
- ❌ Type A/B 违规：第 4 行声明无 CI/gate/commit 证据: > **E2E 测试**: 16/16 PASS ✅ (修复了 column_def 包 bug: org_name, 
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: | Lib Tests | 1670 PASS, 1 IGNORED | ✅ 已执行 |
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: | TPC-H | 22/22 PASS | ⚠️ 无 oracle 对比 |
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: | Corpus | 818/818 PASS | ⚠️ 无 oracle 对比 |
- ❌ Type A/B 违规：第 49 行声明无 CI/gate/commit 证据: | D9 Gate | 8/8 PASS | ✅ 有独立验证 |
- ❌ Type A/B 违规：第 55 行声明无 CI/gate/commit 证据: | G1 | TPC-H 22/22 | ✅ PASS | ⚠️ 无 oracle 对比 | **Q8 0.18ms**
- ❌ Type A/B 违规：第 56 行声明无 CI/gate/commit 证据: | G2 | INT-2 | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 57 行声明无 CI/gate/commit 证据: | G3 | INT-3 | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 58 行声明无 CI/gate/commit 证据: | G4 | ARCH-3 | ✅ PASS | ✅ 有独立验证 | — |
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: | G5 | SEM-1 | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: | G6 | Backup/Restore | ✅ PASS | ✅ 有独立验证 | — |
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: | G8 | Crash Matrix | ✅ PASS | ✅ 有独立验证 | — |
- ❌ Type A/B 违规：第 63 行声明无 CI/gate/commit 证据: | G9 | Upgrade Test | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 64 行声明无 CI/gate/commit 证据: | G10 | Audit + Time Travel | ✅ PASS | ✅ 有独立验证 | — |
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: | G11 | QPS/TPS | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: | G12 | Sysbench | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: | G14 | Real Crash | ✅ PASS | ⚠️ 部分模拟 | — |
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: | G15 | TPC-H SF0.01 | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: | G16 | Compatibility | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: | **P11-P15** | **meta-gate** (Sprint 8) | ✅ **5/5 PASS** | 
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: **诚实声明**: 16/16 G1-G16 gate 脚本已执行 + 5 meta-gates (P11-P15) P
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: TPC-H 测试必须通过 **wire protocol**（启动 `sqlrustgo-mysql-server` +
- ❌ Type A/B 违规：第 31 行声明无 CI/gate/commit 证据: - LOAD DATA LOCAL INFILE 只能通过 wire protocol 触发，in-process 调用
- ❌ Type A/B 违规：第 165 行声明无 CI/gate/commit 证据: - `load_fixture(&mut client, dir)` — 通过 LOAD DATA 加载 .tbl
- ❌ Type A/B 违规：第 221 行声明无 CI/gate/commit 证据: 3. 跑 `tpch_sf01_22_queries_wire_test` (SF=0.01, 22/22 PASS)
- ❌ Type A/B 违规：第 223 行声明无 CI/gate/commit 证据: 5. 输出 PASS/FAIL + 证据文件
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: | **长稳测试**     | 10% | **未验证** | Beta 72h 压缩 PASS, 24h 真实 ⏳ 
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: | **升级兼容**     | 30% | **未完成** | 23 unit PASS, 5 cases ⏳ |
- ❌ Type A/B 违规：第 54 行声明无 CI/gate/commit 证据: **通过 4-way G17 验证** (perf/FOUR_WAY_TPCH_REPORT.md, 2026-06-0
- ❌ Type A/B 违规：第 95 行声明无 CI/gate/commit 证据: | G11 | QPS/TPS  | ✅ 5/5 模板 PASS    | ⏳ W12 真实测量       | ✅ |
- ❌ Type A/B 违规：第 96 行声明无 CI/gate/commit 证据: | G12 | Sysbench | ✅ 7/7 模板 PASS    | ⏳ W12 真实 5 workloads |
- ❌ Type A/B 违规：第 97 行声明无 CI/gate/commit 证据: | G13 | 24h 稳定性  | ✅ Beta 72h 压缩 PASS | ⏳ **24h 真实待 Z6G4** |
- ❌ Type A/B 违规：第 98 行声明无 CI/gate/commit 证据: | G14 | 真实崩溃    | ✅ G8 100+ PASS     | ⏳ 8 真实 cases        |
- ❌ Type A/B 违规：第 100 行声明无 CI/gate/commit 证据: | G16 | 兼容性      | ✅ 23 unit PASS     | ⏳ 5 真实 cases        
- ❌ Type A/B 违规：第 132 行声明无 CI/gate/commit 证据: - 输出: PASS/FAIL/CHECKSUM-MISMATCH per query
- ❌ Type A/B 违规：第 143 行声明无 CI/gate/commit 证据: - 任何 PR 22/22 TPCH PASS 才合
- ❌ Type A/B 违规：第 199 行声明无 CI/gate/commit 证据: | **P0-2** | 24h Soak PASS | ⏳ W10-W11 Z6G4 |
- ❌ Type A/B 违规：第 200 行声明无 CI/gate/commit 证据: | **P0-3** | 72h Soak PASS | ⏳ W11-W12 Z6G4 |
- ❌ Type A/B 违规：第 201 行声明无 CI/gate/commit 证据: | **P0-4** | 168h Soak PASS | ⏳ W12-W14 Z6G4 |
- ❌ Type A/B 违规：第 202 行声明无 CI/gate/commit 证据: | **P0-5** | INT-2 PASS (升级链) | ⏳ W12-W13 Z6G4 |
- ❌ Type A/B 违规：第 203 行声明无 CI/gate/commit 证据: | **P0-6** | INT-3 PASS (混合验证) | ⏳ W13-W14 Z6G4 |
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 31 行声明无 CI/gate/commit 证据: | `check_g_all.sh` (G1-G10+G17 orchestrator) | 🟡 PASS with 3
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | `check_full_gate_verification.sh` (D9) | ❌ **FAIL** | 5 PA
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | `check_beta_e2e.sh` | ⚠️ PASS (informational) | 10 E2E tes
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | P11 Gate Self-Verification | ✅ PASS | — |
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: | P14 DRIFT != PASS | ✅ PASS | — |
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: | P15 Oracle Required | ✅ PASS | 3 oracle engines, 25 oracle
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 169 行声明无 CI/gate/commit 证据: - [ ] 所有修改可通过 `git checkout -- <file>` 恢复
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: > **Status**: 22/22 row-count PASS, 21/22 cell-level MATCH (
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | Wire protocol LOAD DATA + 22 query round-trip | PASS — 0 p
- ❌ Type A/B 违规：第 31 行声明无 CI/gate/commit 证据: | Wire protocol 22/22 row-count | PASS — all 22 query row-co
- ❌ Type A/B 违规：第 155 行声明无 CI/gate/commit 证据: === Row-count: 22/22 PASS ===
- ❌ Type A/B 违规：第 160 行声明无 CI/gate/commit 证据: Q13 row_count PASS (11/11). Cell-level PARTIAL: 9/11 rows ma
- ❌ Type A/B 违规：第 195 行声明无 CI/gate/commit 证据: - No "PASS" claims without actual data behind them
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: docker run --rm -p 5432:5432 -e SQLRUSTGO_PASSWORD=secret \
- ❌ Type A/B 违规：第 256 行声明无 CI/gate/commit 证据:   -e SQLRUSTGO_PASSWORD=secret \
- ❌ Type A/B 违规：第 281 行声明无 CI/gate/commit 证据:       SQLRUSTGO_PASSWORD: secret
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 22 行声明无 CI/gate/commit 证据: > **Verdict**: **G11/G13 之前声称的 PASS 是错的**。需要先修 server bug 才能
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: **结论**：所有"通过"的稳定性/性能 gates (G11, G13, G15) 都是**形式上通过**，实际上**
- ❌ Type A/B 违规：第 138 行声明无 CI/gate/commit 证据: - **G11 QPS**: 之前跑的是 `qps_bench.rs` (in-process)，不通过 wire pr
- ❌ Type A/B 违规：第 181 行声明无 CI/gate/commit 证据: 1. **混淆了"测试通过"和"测试有意义"** —— 我跑了 G1-G16 gate scripts 说 PASS，但
- ❌ Type A/B 违规：第 185 行声明无 CI/gate/commit 证据: 5. **过度信任 cargo gate scripts 的输出** —— `check_g11_qps.sh` 只检查
- ❌ Type A/B 违规：第 187 行声明无 CI/gate/commit 证据: **最关键的设计缺陷**：我把 unit test 形式通过等同于 integration 验证通过。这在 GA con
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 98 行声明无 CI/gate/commit 证据: - rc4: G1-G13 gate PASS, SHA-256 + QPS baseline
- ❌ Type A/B 违规：第 112 行声明无 CI/gate/commit 证据: - Gates G1-G16 PASS, 36 substance tests PASS
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: | TPC-H in-process PASS               | 20/22      | **22/22
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | TPC-H wire round-trip PASS          | 18/22      | **22/22
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: **Result**: 22/22 PASS in-process, 22/22 PASS wire round-tri
- ❌ Type A/B 违规：第 240 行声明无 CI/gate/commit 证据: - All 22/22 row-count PASS statements are backed by actual r
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | 30m (post-fix) | ✅ PASS | 16 MB | 8 | 0-1 GB (bounded) | 2
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | 1h (post-fix) | ✅ PASS | 10 MB | 8 | 0 MB | 22 GB free | 2
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | 2h (post-fix) | ✅ PASS | 12 MB | 8 | 0 MB | 22 GB free | 8
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: | 4h (post-fix) | ✅ PASS | 9 MB | 8 | 0 MB | 23 GB free | 99
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: - The "10/10 PASS" claim was an illusion
- ❌ Type A/B 违规：第 110 行声明无 CI/gate/commit 证据: - ✅ 6/6 meta-gates (P11-P16) PASS
- ❌ Type A/B 违规：第 111 行声明无 CI/gate/commit 证据: - ✅ G15 wire oracle 22/22 PASS
- ❌ Type A/B 违规：第 133 行声明无 CI/gate/commit 证据: 5. ✅ Re-ran 30m / 1h / 2h / 4h post-fix: all PASS
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: | elapsed_s | queries_done | queries_failed | p99_latency_ms
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 75 行声明无 CI/gate/commit 证据: | `tpch_soak_test.rs` | 70,78,86,94 | `test_soak_5m/10m/20m/
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 124 行声明无 CI/gate/commit 证据: **结论**: G17 Coverage Gate (≥80% line) **当前未通过**. 主要落后 crates
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 355 行声明无 CI/gate/commit 证据:       GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD:-admin}
- ❌ Type A/B 违规：第 4 行声明无 CI/gate/commit 证据: > **E2E 测试**: 16/16 PASS ✅ (MySQL column_def 包修复: org_name, 
- ❌ Type A/B 违规：第 6 行声明无 CI/gate/commit 证据: > **GA Gate**: 9/11 PASS, 2 CONDITIONAL (G3 覆盖率, G4 TPC-H SF
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: - **✅ 关键进展**: G1-G16 门禁全部 PASS (2026-06-13)
- ❌ Type A/B 违规：第 54 行声明无 CI/gate/commit 证据:   - 真实覆盖率从 ~35% 提升至 ~60% (substance tests PASS)
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: | **Form-only 验证** | ✅ PASS (10/10 G1-G10) | 形式门禁全过, 真实运行待 r
- ❌ Type A/B 违规：第 78 行声明无 CI/gate/commit 证据: | **ACID / DML 主路径** | 8.5/10 | ≥ 9.0 (G4 强化) | G4 ARCH-3 fo
- ❌ Type A/B 违规：第 79 行声明无 CI/gate/commit 证据: | **跨版本债 OPEN** | 4 项 (INT-2/3, ARCH-3, SEM-1) | 0 项 (G2/G3/
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: | **GMP 审计能力** | 5.0/10 | ≥ 8.0 (G10) | 5.0/10 (G10 form-onl
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据: | **真实生产级覆盖率** | 60-70% | ≥ 90% | **~60% (RC7 substance PASS
- ❌ Type A/B 违规：第 95 行声明无 CI/gate/commit 证据: | **关注指标** | 多少 TPC-H PASS | 22/22 TPC-H + INT-1 | **Soak 24
- ❌ Type A/B 违规：第 131 行声明无 CI/gate/commit 证据: | **Parser** | 10/10 (18/18 PASS) | ✅ 完整继承 |
- ❌ Type A/B 违规：第 212 行声明无 CI/gate/commit 证据: | **P1-1** | Backup/Restore 实现 (100+ 场景) | 40h | 待创建 | ✅ G6 
- ❌ Type A/B 违规：第 213 行声明无 CI/gate/commit 证据: | **P1-2** | Crash Test Framework (100+ scenarios) | 40h | 待
- ❌ Type A/B 违规：第 215 行声明无 CI/gate/commit 证据: | **P1-4** | Upgrade Test (v3.8 → v3.9) | 40h | 待创建 | ✅ G9 f
- ❌ Type A/B 违规：第 223 行声明无 CI/gate/commit 证据: | **P2-1** | Audit Log (审计日志 + 系统表) | 24h | 待创建 | ✅ G10 form
- ❌ Type A/B 违规：第 265 行声明无 CI/gate/commit 证据: | **G15** | 汇总报告 | 报告 | ✅ PASS (4/4) | — | 6 perf reports co
- ❌ Type A/B 违规：第 266 行声明无 CI/gate/commit 证据: | **G16** | Compatibility v3.8 → v3.9 | 集成 | 🟡 5/7 PASS | — 
- ❌ Type A/B 违规：第 271 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (6/6 form-only steps)
- ❌ Type A/B 违规：第 291 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (6/6)
- ❌ Type A/B 违规：第 299 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (4/4)
- ❌ Type A/B 违规：第 310 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (4/4)
- ❌ Type A/B 违规：第 311 行声明无 CI/gate/commit 证据: - 5+ tests PASS
- ❌ Type A/B 违规：第 313 行声明无 CI/gate/commit 证据: - `check_arch2_no_bypass.sh` 全部 PASS
- ❌ Type A/B 违规：第 318 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (8/8)
- ❌ Type A/B 违规：第 319 行声明无 CI/gate/commit 证据: - 8+ tests PASS
- ❌ Type A/B 违规：第 325 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (6/6 form-only)
- ❌ Type A/B 违规：第 326 行声明无 CI/gate/commit 证据: - 100+ tests PASS
- ❌ Type A/B 违规：第 332 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (7/7) — **但 RC3_PLAN 揭示真实问题**:
- ❌ Type A/B 违规：第 333 行声明无 CI/gate/commit 证据: - 10 soak tests PASS at 24h/72h/168h
- ❌ Type A/B 违规：第 348 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (7/7 form-only)
- ❌ Type A/B 违规：第 349 行声明无 CI/gate/commit 证据: - 8 类崩溃注入 × 10+ 变体 模拟通过
- ❌ Type A/B 违规：第 352 行声明无 CI/gate/commit 证据: **⚠️ RC3_PLAN 关键发现**: Crash matrix 模拟通过, real crash 8 catego
- ❌ Type A/B 违规：第 366 行声明无 CI/gate/commit 证据: **状态**: ✅ PASS (5/5)
- ❌ Type A/B 违规：第 367 行声明无 CI/gate/commit 证据: - 50+ tests PASS
- ❌ Type A/B 违规：第 372 行声明无 CI/gate/commit 证据: **状态**: 🟡 PASS (7/7 + 1 sub-gate WARN non-blocking)
- ❌ Type A/B 违规：第 387 行声明无 CI/gate/commit 证据: | **G15** 汇总报告 | ✅ PASS (4/4) | 6 perf reports committed |
- ❌ Type A/B 违规：第 391 行声明无 CI/gate/commit 证据: **状态**: 🟡 5/7 PASS
- ❌ Type A/B 违规：第 416 行声明无 CI/gate/commit 证据: | **RC5-RC7** | 2026-06-13 | ✅ | G1-G16 PASS + substance | 3
- ❌ Type A/B 违规：第 423 行声明无 CI/gate/commit 证据: - 16/16 sub-tasks completed (100%)
- ❌ Type A/B 违规：第 425 行声明无 CI/gate/commit 证据: - 72h soak compressed-time: 10/10 tests PASS (1,440× compres
- ❌ Type A/B 违规：第 427 行声明无 CI/gate/commit 证据: - Doc gates PASS (check_docs_consistency.sh + check_docs_lin
- ❌ Type A/B 违规：第 433 行声明无 CI/gate/commit 证据: - G1-G10 baseline: 10/10 PASS
- ❌ Type A/B 违规：第 436 行声明无 CI/gate/commit 证据: - G16 Compatibility: 5/7 PASS (TPC-H step + REPORT step pend
- ❌ Type A/B 违规：第 442 行声明无 CI/gate/commit 证据: - G1-G10: 10/10 PASS
- ❌ Type A/B 违规：第 445 行声明无 CI/gate/commit 证据: - 72h Soak: 10/10 PASS (compressed)
- ❌ Type A/B 违规：第 495 行声明无 CI/gate/commit 证据: - [ ] Doc gates PASS
- ❌ Type A/B 违规：第 518 行声明无 CI/gate/commit 证据: - [ ] 24h real soak PASS (memory < 10%, FD = 0, lock = 0)
- ❌ Type A/B 违规：第 519 行声明无 CI/gate/commit 证据: - [ ] 72h real soak PASS
- ❌ Type A/B 违规：第 535 行声明无 CI/gate/commit 证据: - [ ] 168h real soak completed successfully
- ❌ Type A/B 违规：第 566 行声明无 CI/gate/commit 证据: - **72h Soak (compressed)**: 10/10 PASS, memory < 10%, FD = 
- ❌ Type A/B 违规：第 617 行声明无 CI/gate/commit 证据: | **2026-06-13** | **RC7 cut** | G1-G16 PASS + substance tes
- ❌ Type A/B 违规：第 849 行声明无 CI/gate/commit 证据: - [ ] G1: 22/22 TPC-H 保持 PASS (REAL, not form-only)
- ❌ Type A/B 违规：第 855 行声明无 CI/gate/commit 证据: - [ ] G7: 24h REAL wall-clock Soak Test PASS
- ❌ Type A/B 违规：第 856 行声明无 CI/gate/commit 证据: - [ ] G8: Crash Matrix 100+ REAL scenarios PASS
- ❌ Type A/B 违规：第 857 行声明无 CI/gate/commit 证据: - [ ] G9: Upgrade Test PASS (v3.8 → v3.9 数据可读, REAL)
- ❌ Type A/B 违规：第 864 行声明无 CI/gate/commit 证据: - [ ] G16: Compatibility v3.8 → v3.9 (full PASS)
- ❌ Type A/B 违规：第 865 行声明无 CI/gate/commit 证据: - [ ] 168h real soak completed
- ❌ Type A/B 违规：第 883 行声明无 CI/gate/commit 证据: | **GA (General Availability)** | ✅ PASS | ✅ TARGET | 🟡 form
- ❌ Type A/B 违规：第 925 行声明无 CI/gate/commit 证据: - 10/10 G1-G10 form-only PASS
- ❌ Type A/B 违规：第 1081 行声明无 CI/gate/commit 证据: | G16 Compatibility full PASS | 8h | P1 | (compat) |
- ❌ Type A/B 违规：第 1127 行声明无 CI/gate/commit 证据: ✅ G16 5/7 PASS
- ❌ Type A/B 违规：第 1136 行声明无 CI/gate/commit 证据: ✅ 72h Soak compressed-time 10/10 PASS
- ❌ Type A/B 违规：第 4 行声明无 CI/gate/commit 证据: > **E2E 测试**: 16/16 PASS ✅ (修复了 column_def 包 bug: org_name, 
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: | Lib Tests | 1670 PASS, 1 IGNORED | ✅ 已执行 |
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: | TPC-H | 22/22 PASS | ⚠️ 无 oracle 对比 |
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: | Corpus | 818/818 PASS | ⚠️ 无 oracle 对比 |
- ❌ Type A/B 违规：第 49 行声明无 CI/gate/commit 证据: | D9 Gate | 8/8 PASS | ✅ 有独立验证 |
- ❌ Type A/B 违规：第 55 行声明无 CI/gate/commit 证据: | G1 | TPC-H 22/22 | ✅ PASS | ⚠️ 无 oracle 对比 | **Q8 0.18ms**
- ❌ Type A/B 违规：第 56 行声明无 CI/gate/commit 证据: | G2 | INT-2 | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 57 行声明无 CI/gate/commit 证据: | G3 | INT-3 | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 58 行声明无 CI/gate/commit 证据: | G4 | ARCH-3 | ✅ PASS | ✅ 有独立验证 | — |
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: | G5 | SEM-1 | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: | G6 | Backup/Restore | ✅ PASS | ✅ 有独立验证 | — |
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: | G8 | Crash Matrix | ✅ PASS | ✅ 有独立验证 | — |
- ❌ Type A/B 违规：第 63 行声明无 CI/gate/commit 证据: | G9 | Upgrade Test | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 64 行声明无 CI/gate/commit 证据: | G10 | Audit + Time Travel | ✅ PASS | ✅ 有独立验证 | — |
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: | G11 | QPS/TPS | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: | G12 | Sysbench | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: | G14 | Real Crash | ✅ PASS | ⚠️ 部分模拟 | — |
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: | G15 | TPC-H SF0.01 | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: | G16 | Compatibility | ✅ PASS | ⚠️ 无 oracle 对比 | — |
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: | **P11-P15** | **meta-gate** (Sprint 8) | ✅ **5/5 PASS** | 
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: **诚实声明**: 16/16 G1-G16 gate 脚本已执行 + 5 meta-gates (P11-P15) P
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据: > **执行结果**: ✅ 8/8 修改完成，复核 100% PASS
- ❌ Type A/B 违规：第 77 行声明无 CI/gate/commit 证据: | 操作 1: CHANGELOG 阶段 RC2 → RC7 | ✅ PASS (1 occurrence) |
- ❌ Type A/B 违规：第 78 行声明无 CI/gate/commit 证据: | 操作 2: CHANGELOG 版本表 rc4/rc5/rc6/rc7 | ✅ PASS (4 entries) |
- ❌ Type A/B 违规：第 79 行声明无 CI/gate/commit 证据: | 操作 3: README GA 目标 2026-12-15 | ✅ PASS (1 occurrence) |
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: | 操作 4: RELEASE_NOTES Header 阶段标注 | ✅ PASS (1 occurrence) |
- ❌ Type A/B 违规：第 81 行声明无 CI/gate/commit 证据: | 操作 5: root CHANGELOG RC7 status | ✅ PASS (1 occurrence) |
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: | 操作 6: ROADMAP 4.9 更新为 RC7 | ✅ PASS (1 occurrence) |
- ❌ Type A/B 违规：第 85 行声明无 CI/gate/commit 证据: | 无 commit 日志内容修改 | ✅ PASS |
- ❌ Type A/B 违规：第 86 行声明无 CI/gate/commit 证据: | 无功能描述/架构设计修改 | ✅ PASS |
- ❌ Type A/B 违规：第 87 行声明无 CI/gate/commit 证据: | 所有引用 .md 文件存在 | ✅ PASS (no broken refs introduced) |
- ❌ Type A/B 违规：第 88 行声明无 CI/gate/commit 证据: | git diff 干净 | ✅ PASS (31 insertions, 19 deletions, all in 
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: | 问题 1 已修复：ROADMAP.md Phase 状态正确 | ✅ 通过 |
- ❌ Type A/B 违规：第 74 行声明无 CI/gate/commit 证据: | 问题 2 已修复：ROADMAP.md GA 日期标注风险 | ✅ 通过 |
- ❌ Type A/B 违规：第 75 行声明无 CI/gate/commit 证据: | 问题 3 已修复：ROADMAP.md 文档引用路径正确 | ✅ 通过 |
- ❌ Type A/B 违规：第 76 行声明无 CI/gate/commit 证据: | 问题 4 已修复：V390_VERSION_PLAN.md W0 日期正确 | ✅ 通过 |
- ❌ Type A/B 违规：第 77 行声明无 CI/gate/commit 证据: | 问题 5 已修复：V390_VERSION_PLAN.md 状态正确 | ✅ 通过 |
- ❌ Type A/B 违规：第 78 行声明无 CI/gate/commit 证据: | 问题 6 已修复：V390_VERSION_PLAN.md GA 日期标注风险 | ✅ 通过 |
- ❌ Type A/B 违规：第 79 行声明无 CI/gate/commit 证据: | 问题 7 已修复：CHANGELOG.md 阶段表述完整 | ✅ 通过 |
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: | 问题 8 已修复：alpha/ 目录存在 | ✅ 通过 |
- ❌ Type A/B 违规：第 86 行声明无 CI/gate/commit 证据: | commit 日志内容未被修改 | ✅ 通过 |
- ❌ Type A/B 违规：第 87 行声明无 CI/gate/commit 证据: | 功能描述未被修改 | ✅ 通过 |
- ❌ Type A/B 违规：第 88 行声明无 CI/gate/commit 证据: | 实质性技术内容未被修改 | ✅ 通过 |
- ❌ Type A/B 违规：第 95 行声明无 CI/gate/commit 证据: | alpha/ALPHA1_RELEASE_NOTES.md 已创建 | ✅ 通过 |
- ❌ Type A/B 违规：第 101 行声明无 CI/gate/commit 证据: | git diff 无非预期修改 | ✅ 通过 |
- ❌ Type A/B 违规：第 109 行声明无 CI/gate/commit 证据: | 所有修改可通过 `git checkout -- <file>` 恢复 | ✅ 通过 |
- ❌ Type A/B 违规：第 110 行声明无 CI/gate/commit 证据: | 工作记录完整，可追溯每一步 | ✅ 通过 |
- ❌ Type A/B 违规：第 136 行声明无 CI/gate/commit 证据: 本次文档整改遵循 `DOC_CHECK_CORRECTION_RULES.md` 的 5 步流程，成功修复了 8 处不自
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 22 行声明无 CI/gate/commit 证据: > **Last update**: 2026-06-26 (corrected — Z6G4 72h soak nev
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | **24h real wall-clock** | 24h | ❌ **INCOMPLETE** | 250: 84
- ❌ Type A/B 违规：第 55 行声明无 CI/gate/commit 证据: - [x] 30m wall-clock PASS (post-fix, see LOCAL_SHORT_SOAK_RE
- ❌ Type A/B 违规：第 56 行声明无 CI/gate/commit 证据: - [x] 1h/2h/4h ladder steps PASS (see LOCAL_SHORT_SOAK_REPOR
- ❌ Type A/B 违规：第 120 行声明无 CI/gate/commit 证据: soak duration PASS marks.
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | 4 | `tests/tpch_value_correctness_test.rs` | ExecutionEngi
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: | 10 | `tests/tpch_sf01_perf baseline_test.rs` | ExecutionEn
- ❌ Type A/B 违规：第 109 行声明无 CI/gate/commit 证据: 2. **连接** 通过 `MySqlTestClient` 或真实 mysql client
- ❌ Type A/B 违规：第 110 行声明无 CI/gate/commit 证据: 3. **工作负载** 通过 wire protocol (COM_QUERY, COM_STMT_PREPARE, C
- ❌ Type A/B 违规：第 111 行声明无 CI/gate/commit 证据: 4. **数据** 通过 wire protocol (LOAD DATA LOCAL INFILE, INSERT)
- ❌ Type A/B 违规：第 113 行声明无 CI/gate/commit 证据: 6. **断言** 通过 wire protocol 读回的 rows
- ❌ Type A/B 违规：第 140 行声明无 CI/gate/commit 证据: 12. **`tests/tpch_value_correctness_test.rs` E2E 化** (synthe
- ❌ Type A/B 违规：第 158 行声明无 CI/gate/commit 证据: - 修完后 `tpch_value_test_v2` 等会自然通过
- ❌ Type A/B 违规：第 178 行声明无 CI/gate/commit 证据: - [ ] 5-min 集成测试 PASS (A1+A2+A3)
- ❌ Type A/B 违规：第 180 行声明无 CI/gate/commit 证据: - [ ] G11 E2E test 通过 (C)
- ❌ Type A/B 违规：第 181 行声明无 CI/gate/commit 证据: - [ ] G13 E2E test 通过 (D)
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: Z6G4 G11 in progress as of 2026-06-12 20:35 — 3/22 done (poi
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: - G11 gate (`check_g11_qps.sh`) PASSES form: qps_bench exist
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 441 行声明无 CI/gate/commit 证据: benchmark PASSES in row-count (5/5) and the cell-level
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: > **Status**: PASS
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: | Root cause identified | PASS — `src/engine_select.rs:1175-
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | Fix landed | PASS — `HashMap<String, Vec<(usize, &Vec<Valu
- ❌ Type A/B 违规：第 31 行声明无 CI/gate/commit 证据: | In-process audit test | PASS — `tests/tpch_q9_audit.rs` 22
- ❌ Type A/B 违规：第 32 行声明无 CI/gate/commit 证据: | Q9 wired result MATCH | PASS — engine=75, sqlite=75 |
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | Other query regression | PASS — Q1, Q3-Q6, Q10-Q16 全部 MATC
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: | `cargo clippy` | PASS — 未引入新警告 |
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | `cargo fmt` | PASS |
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | `cargo test` | PASS |
- ❌ Type A/B 违规：第 146 行声明无 CI/gate/commit 证据: | Q17-22 subquery | in-process 未在 Q9 fix 验证范围 | 修复前 17/22 PA
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：Q9-FIX-GATE-REPORT.md
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 232 行声明无 CI/gate/commit 证据:     echo "✅ Coverage Gate PASS"
- ❌ Type A/B 违规：第 309 行声明无 CI/gate/commit 证据: The RC8 tag can be cut based on local P0 completion (✅ done)
- ❌ Type A/B 违规：第 51 行声明无 CI/gate/commit 证据: | G13 | 24h Stability (extended) | ⏳ | `check_g13_stability.
- ❌ Type A/B 违规：第 64 行声明无 CI/gate/commit 证据: | **P14** | DRIFT != PASS | ✅ | V5/V6/V8 全部修复 (Sprint 8) |
- ❌ Type A/B 违规：第 91 行声明无 CI/gate/commit 证据: - [x] 6/6 meta-gates PASS
- ❌ Type A/B 违规：第 168 行声明无 CI/gate/commit 证据:         echo "[$(date)] Run completed normally, restarting i
- ❌ Type A/B 违规：第 174 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: > **State**: Pre-GA. All pre-soak gates PASS. **Ready to dis
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | P11 | Gate Self-Verification | ✅ PASS | `scripts/gate/chec
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | P12 | No Implicit Tolerance | ✅ PASS | `scripts/gate/check
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | P13 | Test Count Monotonicity | ✅ PASS | active=6200, carg
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: | P14 | DRIFT != PASS | ✅ PASS | 0 anti-patterns |
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: | P15 | Oracle Required | ✅ PASS | All 8 gate oracles presen
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: | Clippy | No warnings | ✅ PASS | `cargo clippy --all-featur
- ❌ Type A/B 违规：第 43 行声明无 CI/gate/commit 证据: | Fmt | Clean | ✅ PASS | `cargo fmt --check` clean on change
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: - 22/22 in-process: PASS (default features)
- ❌ Type A/B 违规：第 63 行声明无 CI/gate/commit 证据: - 22/22 in-process: PASS (with feature on)
- ❌ Type A/B 违规：第 64 行声明无 CI/gate/commit 证据: - G15 wire oracle 22/22: PASS across 5 sub-tests
- ❌ Type A/B 违规：第 88 行声明无 CI/gate/commit 证据: - ✅ All pre-soak quality gates PASS
- ❌ Type A/B 违规：第 119 行声明无 CI/gate/commit 证据: 168h soak PASS
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 129 行声明无 CI/gate/commit 证据: | V390_COMPREHENSIVE_ASSESSMENT.md | ✅ 10/10 form-only PASS 
- ❌ Type A/B 违规：第 135 行声明无 CI/gate/commit 证据: **问题**: 同一文档内先说 "PASS"，后说 "实际未跑"，容易误导读者。
- ❌ Type A/B 违规：第 138 行声明无 CI/gate/commit 证据: - 门禁状态应明确区分 "form-only PASS" vs "REAL PASS"
- ❌ Type A/B 违规：第 141 行声明无 CI/gate/commit 证据:   G1: ✅ form-only PASS | ⏳ REAL pending
- ❌ Type A/B 违规：第 142 行声明无 CI/gate/commit 证据:   G7: ✅ compressed PASS | ⏳ 24h real pending
- ❌ Type A/B 违规：第 168 行声明无 CI/gate/commit 证据: | V390_COMPREHENSIVE_ASSESSMENT.md §4.2 | ✅ PASS (6/6 form-o
- ❌ Type A/B 违规：第 204 行声明无 CI/gate/commit 证据: | V390_COMPREHENSIVE_ASSESSMENT.md §3.2 | ✅ G4 form-only PAS
- ❌ Type A/B 违规：第 205 行声明无 CI/gate/commit 证据: | 同文档 §3.2 | ✅ G3 form-only PASS |
- ❌ Type A/B 违规：第 206 行声明无 CI/gate/commit 证据: | 同文档 §3.2 | ✅ G2 form-only PASS |
- ❌ Type A/B 违规：第 207 行声明无 CI/gate/commit 证据: | 同文档 §3.2 | ✅ G5 form-only PASS |
- ❌ Type A/B 违规：第 213 行声明无 CI/gate/commit 证据: - P0 任务状态应统一为 "✅ form-only PASS | ⏳ REAL pending RC3"
- ❌ Type A/B 违规：第 278 行声明无 CI/gate/commit 证据: 1. 门禁状态统一格式: "✅ form-only PASS | ⏳ REAL pending"
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 50 行声明无 CI/gate/commit 证据: - TPC-H SF=0.1: 22/22 查询通过
- ❌ Type A/B 违规：第 58 行声明无 CI/gate/commit 证据: | `tpch_sf01_inprocess_test` | 22/22 PASS | SF=0.1 完整 TPC-H 
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: | `mysql_wire_protocol_test` | 28/28 PASS | MySQL wire 协议 |
- ❌ Type A/B 违规：第 61 行声明无 CI/gate/commit 证据: | `tpch_sf01_inprocess_test` | 17 tests PASS | 包含 oracle 框架 
- ❌ Type A/B 违规：第 84 行声明无 CI/gate/commit 证据: Thread 0 done: 12345 queries, 0 errors, 12345 rows
- ❌ Type A/B 违规：第 85 行声明无 CI/gate/commit 证据: Thread 1 done: 12300 queries, 0 errors, 12300 rows
- ❌ Type A/B 违规：第 86 行声明无 CI/gate/commit 证据: Thread 2 done: 12280 queries, 0 errors, 12280 rows
- ❌ Type A/B 违规：第 87 行声明无 CI/gate/commit 证据: Thread 3 done: 12310 queries, 0 errors, 12310 rows
- ❌ Type A/B 违规：第 112 行声明无 CI/gate/commit 证据: - [x] 单线程 QPS 验证通过
- ❌ Type A/B 违规：第 113 行声明无 CI/gate/commit 证据: - [x] TPC-H 22/22 查询通过
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: > **Status: 🟡 READY (gates G1-G16 PASS, 24h/72h/168h soak in
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | G1 | TPC-H 22/22 (QPS-correctness) | ✅ PASS | tpch_gate_te
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | G2 | INT-2 ParallelExecutor | ✅ PASS | int2_substance_para
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | G3 | INT-3 Single Expression | ✅ PASS | int3_substance_del
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: | G4 | ARCH-3 VtuGuard | ✅ PASS | check_arch3_no_bypass.sh (
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | G5 | SEM-1 Savepoint | ✅ PASS | check_sem1_savepoint.sh (8
- ❌ Type A/B 违规：第 31 行声明无 CI/gate/commit 证据: | G6 | Backup/Restore/PITR | ✅ PASS | check_backup_restore.s
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | G8 | Crash Matrix | ✅ PASS | check_p12_crash_test.sh |
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: | G9 | Upgrade v3.8→v3.9 | ✅ PASS | check_p14_upgrade_test.s
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | G10 | GMP Audit + Time Travel + Hash Chain | ✅ PASS | chec
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | G15 | SF=0.01 TPC-H wire | ✅ PASS | tpch_sf01_22_queries_w
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: | G16 | Compatibility v3.8→v3.9 | ✅ PASS | v380_to_v390_full
- ❌ Type A/B 违规：第 41 行声明无 CI/gate/commit 证据: **Total: 11/13 PASS, 1/13 incomplete (G11), 1/13 pending re-
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: | Substance tests | 41 | 41/41 PASS |
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: | TPC-H wire (G1) | 22 | 22/22 PASS |
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: | TPC-H wire SF0.01 (G15) | 22 | 22/22 PASS |
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: | Upgrade (G9, G16) | 55 | 55/55 PASS |
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: | Backup/Restore (G6) | 51 | 51/51 PASS |
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: | Crash Matrix (G8) | 129 | 129/129 PASS |
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: | Stability (G7) | 10 | 10/10 PASS |
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: | E2E SELECT (2026-07-04) | 16 | 16/16 PASS |
- ❌ Type A/B 违规：第 74 行声明无 CI/gate/commit 证据: | **Total verified tests** | **346+** | **346+ / 346+ PASS**
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: | 1h simulated | rc4 readiness | Z6G4 (rc4 binary) | ✅ PASS 
- ❌ Type A/B 违规：第 124 行声明无 CI/gate/commit 证据: - [x] G1-G10 gates PASS
- ❌ Type A/B 违规：第 135 行声明无 CI/gate/commit 证据: - [ ] TPC-H SF=1.0 Q1-Q22 PASS (hardware-blocked: disk)
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：GA_GATE_REPORT.md
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: benchmark PASS** milestone.
- ❌ Type A/B 违规：第 198 行声明无 CI/gate/commit 证据: Full TPC-H 22/22 PASS. SQL92 core (DML/joins/aggregates) is
- ❌ Type A/B 违规：第 5 行声明无 CI/gate/commit 证据: > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS)
- ❌ Type A/B 违规：第 254 行声明无 CI/gate/commit 证据:   ✅ SOAK PASSED
- ❌ Type A/B 违规：第 283 行声明无 CI/gate/commit 证据: | 0 errors | PASS |
- ❌ Type A/B 违规：第 284 行声明无 CI/gate/commit 证据: | P99 < 5000ms | PASS |
- ❌ Type A/B 违规：第 285 行声明无 CI/gate/commit 证据: | > 0 queries executed | PASS |
- ❌ Type A/B 违规：第 286 行声明无 CI/gate/commit 证据: | Any error > 0 | WARNING (still PASS) |
- ❌ Type A/B 违规：第 294 行声明无 CI/gate/commit 证据: | 0 | SOAK PASSED (all criteria met) |

---

### 警告项（需要人工复核）

- ⚠️ 警告：第 117 行 FAIL 声明可能无证据
- ⚠️ 警告：第 120 行 FAIL 声明可能无证据
- ⚠️ 警告：第 277 行历史版本引用可能需更新: - **诚实评估**: **v3.9.0 GA 当前不能在覆盖率维度声称 PASS**, 只能说"覆
- ⚠️ 警告：第 291 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：TEST_TRUTHFULNESS_REPORT.md
- ⚠️ 文档不存在（跳过检查）：SOAK_72H_REPORT.md
- ⚠️ 文档不存在（跳过检查）：BETA_RELEASE_NOTES.md
- ⚠️ 警告：第 452 行历史版本引用可能需更新: | SQLRustGo v3.9.0  | 22/22 PASS | This release   
- ⚠️ 文档可能缺少 provenance 元数据：FEATURE_MATRIX.md
- ⚠️ 文档可能缺少 provenance 元数据：SESSION_FINAL_STATUS_2026-06-19.md
- ⚠️ 文档不存在（跳过检查）：PERFORMANCE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：COVERAGE_GAP_RATIONALE.md
- ⚠️ 文档不存在（跳过检查）：TPC-H_PARTIAL_RESULT.md
- ⚠️ 文档不存在（跳过检查）：SECURITY_AUDIT.md
- ⚠️ 文档可能缺少 provenance 元数据：GA_GATE_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：Q8_PERF_ANALYSIS.md
- ⚠️ 警告：第 70 行历史版本引用可能需更新: - newString: `> **Current dev branch**: [\`8a83e25
- ⚠️ 警告：第 119 行历史版本引用可能需更新: - newString: `3. **Tag v3.9.0-ga-candidate** at cu
- ⚠️ 文档可能缺少 provenance 元数据：V390_COMPREHENSIVE_DOC_AUDIT_PLAN.md
- ⚠️ 警告：第 84 行 FAIL 声明可能无证据
- ⚠️ 警告：第 125 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：RUST_1.96_UPGRADE.md
- ⚠️ 警告：第 57 行 FAIL 声明可能无证据
- ⚠️ 警告：第 143 行历史版本引用可能需更新: git tag -a v3.9.0-ga -m "v3.9.0 GA: 168h wall-cloc
- ⚠️ 文档可能缺少 provenance 元数据：GA_READINESS_FINAL_2026-06-19.md
- ⚠️ 文档不存在（跳过检查）：00-release-summary.md
- ⚠️ 文档不存在（跳过检查）：09-ci-build-log.md
- ⚠️ 文档不存在（跳过检查）：02-scope-definition.md
- ⚠️ 文档不存在（跳过检查）：08-license-compliance.md
- ⚠️ 文档不存在（跳过检查）：03-test-report.md
- ⚠️ 文档不存在（跳过检查）：04-coverage-report.md
- ⚠️ 文档不存在（跳过检查）：05-security-scan-report.md
- ⚠️ 文档不存在（跳过检查）：01-release-notes.md
- ⚠️ 文档不存在（跳过检查）：10-approval-record.md
- ⚠️ 文档不存在（跳过检查）：06-performance-report.md
- ⚠️ 文档不存在（跳过检查）：07-dependency-audit.md
- ⚠️ 警告：第 153 行历史版本引用可能需更新: 3. **Tag v3.9.0-ga-candidate** at current tip (aft
- ⚠️ 文档可能缺少 provenance 元数据：GA_GATE_STATUS_REPORT.md
- ⚠️ 警告：第 214 行历史版本引用可能需更新: | v3.9.0-rc3 | 2026-06-12 | G1-G16 form-only + sub
- ⚠️ 警告：第 215 行历史版本引用可能需更新: | v3.9.0-rc4 | 2026-06-12 | G1/G7/G8/G9/G13 PASS; 
- ⚠️ 文档可能缺少 provenance 元数据：CHANGELOG.md
- ⚠️ 文档不存在（跳过检查）：V390_TEST_PLAN_SUPPLEMENT_PERF.md
- ⚠️ 文档不存在（跳过检查）：V390_TEST_PLAN_ROUND2_REVIEW.md
- ⚠️ 文档不存在（跳过检查）：SPRINT4_MASTER_PLAN.md
- ⚠️ 文档不存在（跳过检查）：V390_TEST_PLAN.md
- ⚠️ 警告：第 318 行历史版本引用可能需更新: | v3.9.0 | RC7 | 2026-06-12 | 性能文档 + MariaDB 对比 (3
- ⚠️ 警告：第 319 行历史版本引用可能需更新: | v3.9.0 | RC6 | 2026-06-12 | INT-2/INT-3 实质性测试 (3
- ⚠️ 警告：第 321 行历史版本引用可能需更新: | v3.9.0 | RC4 | 2026-06-12 | G1/G7/G8/G9/G13 PASS
- ⚠️ 警告：第 322 行历史版本引用可能需更新: | v3.9.0 | RC3 | 2026-06-12 | G1-G16 全部 PASS | — |
- ⚠️ 文档可能缺少 provenance 元数据：INDEX.md
- ⚠️ 文档不存在（跳过检查）：V390_VERSION_PLAN.md
- ⚠️ 文档不存在（跳过检查）：V390_DEVELOPMENT_PLAN.md
- ⚠️ 文档不存在（跳过检查）：TPCH_ORACLE_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：V390_COMPREHENSIVE_DOC_AUDIT_WORK_REPORT.md
- ⚠️ 警告：第 112 行 FAIL 声明可能无证据
- ⚠️ 警告：第 223 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：TPCH_E2E_TESTING.md
- ⚠️ 警告：第 132 行 FAIL 声明可能无证据
- ⚠️ 警告：第 205 行历史版本引用可能需更新: **只有全部完成**: v3.9.0 RC2 → v3.9.0 GA
- ⚠️ 文档可能缺少 provenance 元数据：EVIDENCE_STATUS.md
- ⚠️ 文档可能缺少 provenance 元数据：SESSION_2026-06-24_HERMES_MACMINI_STATUS.md
- ⚠️ 文档可能缺少 provenance 元数据：V390_DOC_CORRECTION_STATUS.md
- ⚠️ 警告：第 32 行 FAIL 声明可能无证据
- ⚠️ 警告：第 33 行 FAIL 声明可能无证据
- ⚠️ 警告：第 34 行 FAIL 声明可能无证据
- ⚠️ 警告：第 35 行 FAIL 声明可能无证据
- ⚠️ 警告：第 41 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GA_READINESS_STATUS_2026-06-21.md
- ⚠️ 文档可能缺少 provenance 元数据：V390_DOC_CORRECTION_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：WIRED-22-VERIFICATION-REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：INSTALL.md
- ⚠️ 警告：第 113 行历史版本引用可能需更新: | `evidence/00-release-summary.md` | "Tag v3.9.0-r
- ⚠️ 警告：第 113 行 FAIL 声明可能无证据
- ⚠️ 警告：第 138 行 FAIL 声明可能无证据
- ⚠️ 警告：第 139 行 FAIL 声明可能无证据
- ⚠️ 警告：第 140 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：INTEGRATION_TEST_HONEST_ASSESSMENT.md
- ⚠️ 警告：第 35 行历史版本引用可能需更新: | 6 | `/ROADMAP.md` | v3.9.0 计划项 "MVCC 完整实现" 已完成 (
- ⚠️ 警告：第 56 行历史版本引用可能需更新: newString: | v3.9.0-rc3 | 2026-06-12 | All 5 RC3 P
- ⚠️ 警告：第 57 行历史版本引用可能需更新: newString: | v3.9.0-rc4 | 2026-06-12 | RC4 gate PA
- ⚠️ 文档可能缺少 provenance 元数据：V390_GA_DOC_CORRECTION_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：EVALUATION_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：LOCAL_SHORT_SOAK_REPORT_2026-06-18.md
- ⚠️ 警告：第 40 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：SOAK_72H_LIVE_STATUS_2026-06-19.md
- ⚠️ 文档标注环境限制 (env:blocked:no-ci): README.md (无 Gitea CI, 接受本地 verification log 证据)
- ⚠️ 文档可能缺少 provenance 元数据：README.md
- ⚠️ 文档可能缺少 provenance 元数据：IGNORE_REGISTRY_2026-06-25.md
- ⚠️ 文档可能缺少 provenance 元数据：LONG_STABILITY_TESTS_ANALYSIS.md
- ⚠️ 警告：第 50 行 FAIL 声明可能无证据
- ⚠️ 警告：第 56 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：COVERAGE_COMPARISON_REPORT.md
- ⚠️ 文档标注环境限制 (env:blocked:no-ci): README.md (无 Gitea CI, 接受本地 verification log 证据)
- ⚠️ 文档可能缺少 provenance 元数据：README.md
- ⚠️ 文档可能缺少 provenance 元数据：DEPLOYMENT_GUIDE.md
- ⚠️ 警告：第 65 行历史版本引用可能需更新: | **v3.9.0 启动准备度** | ✅ READY (Phase 0 完成) | 分支 + 5
- ⚠️ 警告：第 70 行历史版本引用可能需更新: | **工程化战略** | ✅ 已批准 (v3.9.0 type) | ChatGPT 架构师 20
- ⚠️ 警告：第 129 行历史版本引用可能需更新: | **TPC-H** | 22/22 PASS (v3.8.0 最大成就) | ✅ 完整继承, G
- ⚠️ 警告：第 426 行历史版本引用可能需更新: - TPC-H 22/22 baseline PASS (inherited from v3.8.0
- ⚠️ 警告：第 609 行历史版本引用可能需更新: | W0 (2026-06-05) | Phase 0 收口 | develop/v3.9.0 创建
- ⚠️ 警告：第 803 行历史版本引用可能需更新: **v3.8.0 GA Gate 总分**: 73+/80 ≥ 56 → ✅ PASS (继承)
- ⚠️ 警告：第 804 行历史版本引用可能需更新: **v3.9.0 形式 GA Gate 总分**: 80+/100 (G1-G16 形式全 PASS
- ⚠️ 警告：第 805 行历史版本引用可能需更新: **v3.9.0 真实 GA Gate 总分目标**: 80+/100 (G1-G16 真实全 PA
- ⚠️ 警告：第 836 行历史版本引用可能需更新: | 维度 | v3.8.0 评审结论 | v3.9.0 形式完成 | v3.9.0 真实评估 (校准
- ⚠️ 警告：第 871 行历史版本引用可能需更新: - `docs/releases/v3.9.0/ga/GA_GATE_REPORT.md` (16 
- ⚠️ 警告：第 886 行历史版本引用可能需更新: | **Single-Node Production Candidate** | ❌ NOT YET
- ⚠️ 警告：第 1211 行历史版本引用可能需更新: **v3.9.0 RC7 综合评估: Production Readiness Release + 
- ⚠️ 文档可能缺少 provenance 元数据：V390_COMPREHENSIVE_ASSESSMENT.md
- ⚠️ 文档标注环境限制 (env:blocked:no-ci): ROADMAP.md (无 Gitea CI, 接受本地 verification log 证据)
- ⚠️ 文档可能缺少 provenance 元数据：ROADMAP.md
- ⚠️ 警告：第 318 行历史版本引用可能需更新: | v3.9.0 | RC7 | 2026-06-12 | 性能文档 + MariaDB 对比 (3
- ⚠️ 警告：第 319 行历史版本引用可能需更新: | v3.9.0 | RC6 | 2026-06-12 | INT-2/INT-3 实质性测试 (3
- ⚠️ 警告：第 321 行历史版本引用可能需更新: | v3.9.0 | RC4 | 2026-06-12 | G1/G7/G8/G9/G13 PASS
- ⚠️ 警告：第 322 行历史版本引用可能需更新: | v3.9.0 | RC3 | 2026-06-12 | G1-G16 全部 PASS | — |
- ⚠️ 文档可能缺少 provenance 元数据：INDEX.md
- ⚠️ 警告：第 36 行历史版本引用可能需更新: | 6 | `/ROADMAP.md` | v3.9.0 计划项 "MVCC 待实现" 已过时 | 
- ⚠️ 警告：第 83 行历史版本引用可能需更新: | 操作 7: README Latest stable v3.8.0 | ✅ PASS (1 oc
- ⚠️ 警告：第 145 行历史版本引用可能需更新: **其他所有门禁条件 100% 完成**。等 24h 跑完即可 cut v3.9.0-rc5 (po
- ⚠️ 文档可能缺少 provenance 元数据：V390_GA_DOC_CORRECTION_WORK_REPORT.md
- ⚠️ 警告：第 346 行历史版本引用可能需更新: A: No — the v3.8.0 results were generated against 
- ⚠️ 警告：第 347 行历史版本引用可能需更新: v3.9.0 fixture has 22/22 PASS with the Q7/Q8/Q9 SQ
- ⚠️ 文档可能缺少 provenance 元数据：MIGRATION_GUIDE.md
- ⚠️ 文档可能缺少 provenance 元数据：V390_DOC_CORRECTION_WORK_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：SOAK_MASTER_INDEX.md
- ⚠️ 文档可能缺少 provenance 元数据：E2E_MIGRATION_MASTER_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：G11_QPS_BENCH_250.md
- ⚠️ 文档可能缺少 provenance 元数据：RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：2026-07-01-RC8-dry-run.md
- ⚠️ 文档可能缺少 provenance 元数据：Q9-FIX-GATE-REPORT.md
- ⚠️ 警告：第 234 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：Z6G4_HANDOFF.md
- ⚠️ 文档不存在（跳过检查）：TPC_H_SHA256_BASELINE_20260612.md
- ⚠️ 文档不存在（跳过检查）：PERFORMANCE_OVERVIEW_20260612.md
- ⚠️ 文档不存在（跳过检查）：QPS_REPORT.md
- ⚠️ 文档不存在（跳过检查）：FOUR_WAY_TPCH_REPORT.md
- ⚠️ 文档不存在（跳过检查）：PERFORMANCE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：COMPATIBILITY_REPORT.md
- ⚠️ 文档不存在（跳过检查）：SYSBENCH_MARIADB_COMPARISON_20260612.md
- ⚠️ 文档不存在（跳过检查）：PERFORMANCE_BASELINE_QPS_20260612.md
- ⚠️ 文档不存在（跳过检查）：CRASH_TEST_REPORT.md
- ⚠️ 文档不存在（跳过检查）：STABILITY_REPORT.md
- ⚠️ 文档不存在（跳过检查）：PERFORMANCE_BASELINE_REAL_2026-06-12.md
- ⚠️ 文档不存在（跳过检查）：SYSBENCH_REPORT.md
- ⚠️ 文档不存在（跳过检查）：PERFORMANCE_BASELINE.md
- ⚠️ 文档不存在（跳过检查）：ALPHA1_RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：RC1_RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：RC5_RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：RC3_PLAN.md
- ⚠️ 文档不存在（跳过检查）：RC4_RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：RC3_RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：RC1_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC6_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC8_RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：RC7_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC7_RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：RC8_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC2_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC2_RELEASE_NOTES.md
- ⚠️ 文档不存在（跳过检查）：RC5_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC4_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC3_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：RC6_RELEASE_NOTES.md
- ⚠️ 文档可能缺少 provenance 元数据：V390_EVIDENCE_INDEX.md
- ⚠️ 文档可能缺少 provenance 元数据：SOAK_168H_MACMINI_REPORT.md
- ⚠️ 警告：第 124 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GA_READINESS_STATUS_2026-06-18.md
- ⚠️ 文档可能缺少 provenance 元数据：V390_DOCUMENT_INCONSISTENCY_ANALYSIS.md
- ⚠️ 文档可能缺少 provenance 元数据：SOAK_MULTI_THREAD_REPORT_2026-06-28.md
- ⚠️ 文档可能缺少 provenance 元数据：GA_GATE_REPORT.md
- ⚠️ 文档不存在（跳过检查）：GITEA_252_OUTAGE_20260607.md
- ⚠️ 警告：第 171 行历史版本引用可能需更新: | SQLRustGo v3.9.0 | 22/22 PASS | This release |
- ⚠️ 文档可能缺少 provenance 元数据：QUICK_START.md
- ⚠️ 警告：第 287 行 FAIL 声明可能无证据
- ⚠️ 警告：第 288 行 FAIL 声明可能无证据
- ⚠️ 警告：第 295 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：CLI_USER_MANUAL.md

---

### 通过项

- ✅ 状态声明有证据绑定：第 117 行
- ✅ 状态声明有证据绑定：第 119 行
- ✅ 状态声明有证据绑定：第 133 行
- ✅ 状态声明有证据绑定：第 134 行
- ✅ 状态声明有证据绑定：第 135 行
- ✅ 状态声明有证据绑定：第 138 行
- ✅ 状态声明有证据绑定：第 205 行
- ✅ 状态声明有证据绑定：第 32 行
- ✅ 状态声明有证据绑定：第 46 行
- ✅ 状态声明有证据绑定：第 51 行
- ✅ 状态声明有证据绑定：第 55 行
- ✅ 状态声明有证据绑定：第 59 行
- ✅ 状态声明有证据绑定：第 129 行
- ✅ 状态声明有证据绑定：第 77 行
- ✅ 状态声明有证据绑定：第 39 行
- ✅ 状态声明有证据绑定：第 152 行
- ✅ 状态声明有证据绑定：第 155 行
- ✅ 状态声明有证据绑定：第 43 行
- ✅ 状态声明有证据绑定：第 97 行
- ✅ 状态声明有证据绑定：第 98 行
- ✅ 状态声明有证据绑定：第 99 行
- ✅ 状态声明有证据绑定：第 154 行
- ✅ 状态声明有证据绑定：第 80 行
- ✅ 状态声明有证据绑定：第 136 行
- ✅ 状态声明有证据绑定：第 112 行
- ✅ 状态声明有证据绑定：第 120 行
- ✅ 状态声明有证据绑定：第 19 行
- ✅ 状态声明有证据绑定：第 47 行
- ✅ 状态声明有证据绑定：第 52 行
- ✅ 状态声明有证据绑定：第 77 行
- ✅ 状态声明有证据绑定：第 149 行
- ✅ 状态声明有证据绑定：第 201 行
- ✅ 状态声明有证据绑定：第 202 行
- ✅ 状态声明有证据绑定：第 203 行
- ✅ 状态声明有证据绑定：第 204 行
- ✅ 状态声明有证据绑定：第 251 行
- ✅ 状态声明有证据绑定：第 252 行
- ✅ 状态声明有证据绑定：第 253 行
- ✅ 状态声明有证据绑定：第 254 行
- ✅ 状态声明有证据绑定：第 255 行
- ✅ 状态声明有证据绑定：第 256 行
- ✅ 状态声明有证据绑定：第 257 行
- ✅ 状态声明有证据绑定：第 258 行
- ✅ 状态声明有证据绑定：第 259 行
- ✅ 状态声明有证据绑定：第 260 行
- ✅ 状态声明有证据绑定：第 672 行
- ✅ 状态声明有证据绑定：第 673 行
- ✅ 状态声明有证据绑定：第 674 行
- ✅ 状态声明有证据绑定：第 675 行
- ✅ 状态声明有证据绑定：第 1124 行
- ✅ 状态声明有证据绑定：第 80 行
- ✅ 状态声明有证据绑定：第 84 行
- ✅ 状态声明有证据绑定：第 94 行
- ✅ 状态声明有证据绑定：第 36 行
- ✅ 状态声明有证据绑定：第 59 行
- ✅ 状态声明有证据绑定：第 39 行
- ✅ 状态声明有证据绑定：第 68 行
- ✅ 状态声明有证据绑定：第 78 行
- ✅ 状态声明有证据绑定：第 28 行
- ✅ 状态声明有证据绑定：第 34 行
- ✅ 状态声明有证据绑定：第 35 行
- ✅ 状态声明有证据绑定：第 41 行
- ✅ 状态声明有证据绑定：第 32 行
- ✅ 状态声明有证据绑定：第 46 行
- ✅ 状态声明有证据绑定：第 51 行
- ✅ 状态声明有证据绑定：第 55 行
- ✅ 状态声明有证据绑定：第 59 行
- ✅ 状态声明有证据绑定：第 129 行

---

## 未验证声明（UNVERIFIED CLAIMS）

以下声明**无证据支撑**，不得用于门禁判断：

| 文档 | 行 | 声明内容 |
|------|-----|----------|
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | **Gate 脚本执行** | 🟢 **HIGH** | 16/16 PASS (form-only), 6/6 meta-gates PASS (Sprint 8 + 本会话) | 强 (script output) | |
| - |  | **长期稳定性** | 🔴 **LOW** | G7/G13 标注 PASS 实为 SIMULATED (1,440× 时间压缩), 真实 24h/72h/168h wall-clock 未跑 | 极弱 (未跑) | |
| - |  | "16/16 G1-G16 PASS" | 🟡 **部分可信** | gate 脚本执行完成, 但 11/16 无 oracle, **5/16 部分可信 (G4/G6/G8/G10/G15)** | |
| - |  | "6/6 meta-gates PASS" | 🟢 **可信** | P11-P16 meta-gate detector 运行, 输出可审计 | |
| - |  | "330+ tests PASS" | 🟡 **部分可信** | 测试执行完成, 但部分自验证, 无 oracle 对比 | |
| - |  | INDEX.md | "G1-G16 全部 PASS" | 同上, **且未提 Coverage 缺失** | **需要添加 V9 说明** | |
| - |  | Meta-gate 验证 | N/A | ✅ HIGH | ✅ HIGH | 6/6 P11-P16 PASS | |
| - |  **GA 阻塞**: V4 (oracle 11 gates) + **V9 (Coverage Gate 缺失, 新发现)** + 真实 24h soak (其余 meta-gate 6/6 PASS) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | Multi-table join (3+) | ✅    | Q7/Q8/Q9 PASS (up to 8 tables)                     | |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  **All 4 ladder steps PASS post-WAL-fix.** Server stable for 4h wall-clock with zero leaks. |
| - |  The work that COULD be done by Claude is DONE |
| - |  - All 6/6 meta-gates PASS throughout |
| - |  > **Status: 🟡 READY (gates G1-G16 PASS, 24h/72h/168h soak incomplete/interrupted)** |
| - |  | G1 | TPC-H 22/22 (QPS-correctness) | ✅ PASS | tpch_gate_test 22/22 | |
| - |  | G2 | INT-2 ParallelExecutor | ✅ PASS | int2_substance_parallel_test (9 tests) | |
| - |  | G3 | INT-3 Single Expression | ✅ PASS | int3_substance_delegation_test (17 tests) | |
| - |  | G4 | ARCH-3 VtuGuard | ✅ PASS | check_arch3_no_bypass.sh (8/8) | |
| - |  | G5 | SEM-1 Savepoint | ✅ PASS | check_sem1_savepoint.sh (8/8) | |
| - |  | G6 | Backup/Restore/PITR | ✅ PASS | check_backup_restore.sh (6/6, 51 e2e) | |
| - |  | G8 | Crash Matrix | ✅ PASS | check_p12_crash_test.sh | |
| - |  | G9 | Upgrade v3.8→v3.9 | ✅ PASS | check_p14_upgrade_test.sh (50 tests) | |
| - |  | G10 | GMP Audit + Time Travel + Hash Chain | ✅ PASS | check_p21/22/23_*.sh | |
| - |  | G15 | SF=0.01 TPC-H wire | ✅ PASS | tpch_sf01_22_queries_wire_test | |
| - |  | G16 | Compatibility v3.8→v3.9 | ✅ PASS | v380_to_v390_full_upgrade_test (5 cases) | |
| - |  **Total: 11/13 PASS, 1/13 incomplete (G11), 1/13 pending re-run (G13)** |
| - |  | Substance tests | 41 | 41/41 PASS | |
| - |  | TPC-H wire (G1) | 22 | 22/22 PASS | |
| - |  | TPC-H wire SF0.01 (G15) | 22 | 22/22 PASS | |
| - |  | Upgrade (G9, G16) | 55 | 55/55 PASS | |
| - |  | Backup/Restore (G6) | 51 | 51/51 PASS | |
| - |  | Crash Matrix (G8) | 129 | 129/129 PASS | |
| - |  | Stability (G7) | 10 | 10/10 PASS | |
| - |  | E2E SELECT (2026-07-04) | 16 | 16/16 PASS | |
| - |  | **Total verified tests** | **346+** | **346+ / 346+ PASS** | |
| - |  | 1h simulated | rc4 readiness | Z6G4 (rc4 binary) | ✅ PASS | |
| - |  - [x] G1-G10 gates PASS |
| - |  - [ ] TPC-H SF=1.0 Q1-Q22 PASS (hardware-blocked: disk) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  **结论**: ✅ 可升级 — 编译通过，clippy 零警告，覆盖率测试可运行。 |
| - |  | `rustup update stable` | ✅ 成功 | |
| - |  3. 所有 `ColumnDefinition { ... }` 构造通过 `..Default::default()` 继承默认值 |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | G15 oracle 22/22 | ✅ | 5 sub-tests PASS sequentially in 188s total (Q1:6 Q2:0 Q3:10 Q4:5 Q5:1 Q6:1 Q7:4 Q8:1 Q9:0 Q10:20 Q11:0 Q12:2 Q13:22 Q15:1 Q16:282 Q17:1 Q18:100 Q19:1 Q20:3 Q21:0 Q22:0) | |
| - |  - **Soak**: PID 104564, in-process, 1 QPS, 3475 queries done, 0 failed |
| - |  - [x] All pre-soak gates PASS (6/6 meta-gates + clippy + fmt + build) |
| - |  G15 sub-tests = 5/5 PASS in 7-100s each |
| - |  6/6 meta-gates verified PASS |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | G1 | TPC-H 22/22 | ✅ PASS | tpch_gate_test 22/22 (sub-gate `[3/5]`) | |
| - |  | G2 | INT-2 | ✅ PASS | int2_substance_parallel_test (9 tests) | |
| - |  | G3 | INT-3 | ✅ PASS | int3_substance_delegation_test (17 tests) | |
| - |  | G4 | ARCH-3 VtuGuard | ✅ PASS | check_arch3_no_bypass.sh (8/8) | |
| - |  | G5 | SEM-1 Savepoint | ✅ PASS | check_sem1_savepoint.sh (8/8) | |
| - |  | G6 | Backup/Restore | ✅ PASS | check_backup_restore.sh (6/6) | |
| - |  | G8 | Crash Matrix | ✅ PASS | check_p12_crash_test.sh | |
| - |  | G9 | Upgrade v3.8→v3.9 | ✅ PASS | check_p14_upgrade_test.sh | |
| - |  | G10 | GMP Audit + Time Travel + Hash Chain | ✅ PASS | check_p21/22/23 (3 sub-gates) | |
| - |  | G11 | QPS/TPS Benchmark | ✅ PASS | check_g11_qps.sh (5/5) | |
| - |  | G13 | 24h Stability | 🟡 partial | 250: partial (843 samples before contact lost); Z6G4: never completed | |
| - |  | G16 | Compatibility v3.8→v3.9 | ✅ PASS | check_g16_compatibility.sh (7/7) | |
| - |  **Total: 13/13 PASS** + 1 🟡 partial (G13 24h real incomplete) + 1 deferred to post-GA (72h/168h) |
| - |  | `tests/g2_substance_parallel_executor_test.rs` | 4 | ✅ PASS | |
| - |  | `tests/int2_substance_parallel_test.rs` | 9 | ✅ PASS | |
| - |  | `tests/int3_substance_delegation_test.rs` | 17 | ✅ PASS | |
| - |  | `tests/upgrade_chain_v3_6_to_v3_9_test.rs` | 6 | ✅ PASS | |
| - |  | 3 | 测试已通过 | ✅ Substance tests + G1-G16 all green | |
| - |  - 8 项修改完成，12/12 复核 PASS ✅ |
| - |  | Draft → Alpha | 架构设计, 编译通过 | ✅ Done (2026-06-05) | |
| - |  - 13/13 核心 gates PASS |
| - |  - 36/36 substance tests PASS |
| - |  [L1] cargo build...           [PASS] |
| - |  [L1] cargo test --lib...      [PASS] |
| - |  [L1] clippy...                [PASS] |
| - |  [L1] cargo fmt...             [PASS] |
| - |  === Gate Result: PASSED === |
| - |  - 本地 4 个快速 gate 全 PASS: `check_arch_invariants` (5/5), `check_arch3_no_bypass` (G4), `check_integration_gate` (4/4), `check_architecture_freeze` (A7-3 PASS) |
| - |  bash scripts/gate/check_arch_invariants.sh:    5/5 PASS |
| - |  bash scripts/gate/check_arch3_no_bypass.sh:    PASS |
| - |  bash scripts/gate/check_integration_gate.sh:   PASS (4/4) |
| - |    SGL-001 (B4 Format): PASS |
| - |    SGL-002..005: PASS |
| - |    WAL lifecycle (INV-1/2/3): PASS |
| - |    A7-3 ExecutionEngine: PASS (< 1500 lines as per AD-001) |
| - |  | **Backup/Restore 100+ 场景** | 全量/增量/时间点恢复 | 3 | ✅ RC7 PASS (G6, 51 e2e) | |
| - |  | **Crash Matrix 100+ 场景** | kill -9 / OOM / disk full | 3 | ✅ RC7 PASS (G8, 129 scenarios) | |
| - |  | **24h Soak Test** | 1M txns 浸泡 | 4 | ✅ RC7 PASS (simulated, 1,440× compression; real pending Z6G4) | |
| - |  | G7 24h Soak | ✅ PASS (simulated) / ❌ INCOMPLETE (real) | Phase 4 末 | |
| - |  > **E2E 测试**: 16/16 PASS ✅ (修复了 column_def 包 bug: org_name, length_of_fixed_fields, real_col_names) |
| - |  | Lib Tests | 1670 PASS, 1 IGNORED | ✅ 已执行 | |
| - |  | TPC-H | 22/22 PASS | ⚠️ 无 oracle 对比 | |
| - |  | Corpus | 818/818 PASS | ⚠️ 无 oracle 对比 | |
| - |  | D9 Gate | 8/8 PASS | ✅ 有独立验证 | |
| - |  | G1 | TPC-H 22/22 | ✅ PASS | ⚠️ 无 oracle 对比 | **Q8 0.18ms** | |
| - |  | G2 | INT-2 | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G3 | INT-3 | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G4 | ARCH-3 | ✅ PASS | ✅ 有独立验证 | — | |
| - |  | G5 | SEM-1 | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G6 | Backup/Restore | ✅ PASS | ✅ 有独立验证 | — | |
| - |  | G8 | Crash Matrix | ✅ PASS | ✅ 有独立验证 | — | |
| - |  | G9 | Upgrade Test | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G10 | Audit + Time Travel | ✅ PASS | ✅ 有独立验证 | — | |
| - |  | G11 | QPS/TPS | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G12 | Sysbench | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G14 | Real Crash | ✅ PASS | ⚠️ 部分模拟 | — | |
| - |  | G15 | TPC-H SF0.01 | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G16 | Compatibility | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | **P11-P15** | **meta-gate** (Sprint 8) | ✅ **5/5 PASS** | **V5/V6/V8/V2 全部修复** | **Sprint 8 Track B** | |
| - |  **诚实声明**: 16/16 G1-G16 gate 脚本已执行 + 5 meta-gates (P11-P15) PASS (Sprint 8), 但 11/16 缺乏独立 oracle 对比。真实 soak 测试 infra ready (Sprint 8), run 仍 pending Z6G4。 |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  TPC-H 测试必须通过 **wire protocol**（启动 `sqlrustgo-mysql-server` + 通过 `MySqlTestClient` 发送 SQL）运行，**禁止** 直接调用 `ExecutionEngine` 或 `MemoryStorage`。 |
| - |  - LOAD DATA LOCAL INFILE 只能通过 wire protocol 触发，in-process 调用无法走真实数据加载 |
| - |  - `load_fixture(&mut client, dir)` — 通过 LOAD DATA 加载 .tbl |
| - |  3. 跑 `tpch_sf01_22_queries_wire_test` (SF=0.01, 22/22 PASS) |
| - |  5. 输出 PASS/FAIL + 证据文件 |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | **长稳测试**     | 10% | **未验证** | Beta 72h 压缩 PASS, 24h 真实 ⏳ | |
| - |  | **升级兼容**     | 30% | **未完成** | 23 unit PASS, 5 cases ⏳ | |
| - |  **通过 4-way G17 验证** (perf/FOUR_WAY_TPCH_REPORT.md, 2026-06-07) |
| - |  | G11 | QPS/TPS  | ✅ 5/5 模板 PASS    | ⏳ W12 真实测量       | ✅ | |
| - |  | G12 | Sysbench | ✅ 7/7 模板 PASS    | ⏳ W12 真实 5 workloads | ✅ | |
| - |  | G13 | 24h 稳定性  | ✅ Beta 72h 压缩 PASS | ⏳ **24h 真实待 Z6G4** | ✅ **卡死** | |
| - |  | G14 | 真实崩溃    | ✅ G8 100+ PASS     | ⏳ 8 真实 cases        | ✅ **卡死** | |
| - |  | G16 | 兼容性      | ✅ 23 unit PASS     | ⏳ 5 真实 cases        | ✅ **卡死** | |
| - |  - 输出: PASS/FAIL/CHECKSUM-MISMATCH per query |
| - |  - 任何 PR 22/22 TPCH PASS 才合 |
| - |  | **P0-2** | 24h Soak PASS | ⏳ W10-W11 Z6G4 | |
| - |  | **P0-3** | 72h Soak PASS | ⏳ W11-W12 Z6G4 | |
| - |  | **P0-4** | 168h Soak PASS | ⏳ W12-W14 Z6G4 | |
| - |  | **P0-5** | INT-2 PASS (升级链) | ⏳ W12-W13 Z6G4 | |
| - |  | **P0-6** | INT-3 PASS (混合验证) | ⏳ W13-W14 Z6G4 | |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | `check_g_all.sh` (G1-G10+G17 orchestrator) | 🟡 PASS with 3 WARN | MySQL-server compile fail not detected (gate doesn't build mysql-server) | |
| - |  | `check_full_gate_verification.sh` (D9) | ❌ **FAIL** | 5 PASS, 3 FAIL (D1-D5, D7, D8) | |
| - |  | `check_beta_e2e.sh` | ⚠️ PASS (informational) | 10 E2E test bins failed at runtime | |
| - |  | P11 Gate Self-Verification | ✅ PASS | — | |
| - |  | P14 DRIFT != PASS | ✅ PASS | — | |
| - |  | P15 Oracle Required | ✅ PASS | 3 oracle engines, 25 oracle-aware gates | |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  - [ ] 所有修改可通过 `git checkout -- <file>` 恢复 |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > **Status**: 22/22 row-count PASS, 21/22 cell-level MATCH (set equality, loose) |
| - |  | Wire protocol LOAD DATA + 22 query round-trip | PASS — 0 panic, 0 error packet, 0 server crash | |
| - |  | Wire protocol 22/22 row-count | PASS — all 22 query row-counts MATCH authoritative SQLite | |
| - |  === Row-count: 22/22 PASS === |
| - |  Q13 row_count PASS (11/11). Cell-level PARTIAL: 9/11 rows match, 2 differ. |
| - |  - No "PASS" claims without actual data behind them |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  docker run --rm -p 5432:5432 -e SQLRUSTGO_PASSWORD=secret \ |
| - |    -e SQLRUSTGO_PASSWORD=secret \ |
| - |        SQLRUSTGO_PASSWORD: secret |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > **Verdict**: **G11/G13 之前声称的 PASS 是错的**。需要先修 server bug 才能继续 GA。 |
| - |  **结论**：所有"通过"的稳定性/性能 gates (G11, G13, G15) 都是**形式上通过**，实际上**没有用 mysql-server 集成后端做真实工作负载测试**。 |
| - |  - **G11 QPS**: 之前跑的是 `qps_bench.rs` (in-process)，不通过 wire protocol → **DRIFT/FAIL** |
| - |  1. **混淆了"测试通过"和"测试有意义"** —— 我跑了 G1-G16 gate scripts 说 PASS，但实际上 gate scripts 只是形式检查 |
| - |  5. **过度信任 cargo gate scripts 的输出** —— `check_g11_qps.sh` 只检查 5 个 sub-checks 通过，不验证 bench 真的在测 mysql-server |
| - |  **最关键的设计缺陷**：我把 unit test 形式通过等同于 integration 验证通过。这在 GA context 下是**严重失误**。 |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  - rc4: G1-G13 gate PASS, SHA-256 + QPS baseline |
| - |  - Gates G1-G16 PASS, 36 substance tests PASS |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | TPC-H in-process PASS               | 20/22      | **22/22**  | +2     | |
| - |  | TPC-H wire round-trip PASS          | 18/22      | **22/22**  | +4     | |
| - |  **Result**: 22/22 PASS in-process, 22/22 PASS wire round-trip, 21/22 |
| - |  - All 22/22 row-count PASS statements are backed by actual row-count |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | 30m (post-fix) | ✅ PASS | 16 MB | 8 | 0-1 GB (bounded) | 21 GB free | 12/5min | |
| - |  | 1h (post-fix) | ✅ PASS | 10 MB | 8 | 0 MB | 22 GB free | 2/1h | |
| - |  | 2h (post-fix) | ✅ PASS | 12 MB | 8 | 0 MB | 22 GB free | 80/2h | |
| - |  | 4h (post-fix) | ✅ PASS | 9 MB | 8 | 0 MB | 23 GB free | 99/4h | |
| - |  - The "10/10 PASS" claim was an illusion |
| - |  - ✅ 6/6 meta-gates (P11-P16) PASS |
| - |  - ✅ G15 wire oracle 22/22 PASS |
| - |  5. ✅ Re-ran 30m / 1h / 2h / 4h post-fix: all PASS |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | elapsed_s | queries_done | queries_failed | p99_latency_ms | rss_mb | leak_warn | |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | `tpch_soak_test.rs` | 70,78,86,94 | `test_soak_5m/10m/20m/30m` | Run with `--ignored`; 5m-30m all PASS | |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  **结论**: G17 Coverage Gate (≥80% line) **当前未通过**. 主要落后 crates: |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |        GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD:-admin} |
| - |  > **E2E 测试**: 16/16 PASS ✅ (MySQL column_def 包修复: org_name, length_of_fixed_fields, real_col_names) |
| - |  > **GA Gate**: 9/11 PASS, 2 CONDITIONAL (G3 覆盖率, G4 TPC-H SF=1.0 6/10), 1 IN PROGRESS (72h soak) |
| - |  - **✅ 关键进展**: G1-G16 门禁全部 PASS (2026-06-13) |
| - |    - 真实覆盖率从 ~35% 提升至 ~60% (substance tests PASS) |
| - |  | **Form-only 验证** | ✅ PASS (10/10 G1-G10) | 形式门禁全过, 真实运行待 rc3+ | |
| - |  | **ACID / DML 主路径** | 8.5/10 | ≥ 9.0 (G4 强化) | G4 ARCH-3 form-only PASS | |
| - |  | **跨版本债 OPEN** | 4 项 (INT-2/3, ARCH-3, SEM-1) | 0 项 (G2/G3/G4/G5) | **G2-G5 form-only PASS, 仍需真实验证** | |
| - |  | **GMP 审计能力** | 5.0/10 | ≥ 8.0 (G10) | 5.0/10 (G10 form-only PASS) | |
| - |  | **真实生产级覆盖率** | 60-70% | ≥ 90% | **~60% (RC7 substance PASS)** | |
| - |  | **关注指标** | 多少 TPC-H PASS | 22/22 TPC-H + INT-1 | **Soak 24h + Crash Matrix + Backup/Restore** | |
| - |  | **Parser** | 10/10 (18/18 PASS) | ✅ 完整继承 | |
| - |  | **P1-1** | Backup/Restore 实现 (100+ 场景) | 40h | 待创建 | ✅ G6 form-only PASS (6/6) | G6 | |
| - |  | **P1-2** | Crash Test Framework (100+ scenarios) | 40h | 待创建 | 🟡 G8 form-only PASS, real run pending | G8 | |
| - |  | **P1-4** | Upgrade Test (v3.8 → v3.9) | 40h | 待创建 | ✅ G9 form-only PASS (5/5) | G9 | |
| - |  | **P2-1** | Audit Log (审计日志 + 系统表) | 24h | 待创建 | ✅ G10 form-only PASS (1 non-blocking warn) | G10 | |
| - |  | **G15** | 汇总报告 | 报告 | ✅ PASS (4/4) | — | 6 perf reports committed | |
| - |  | **G16** | Compatibility v3.8 → v3.9 | 集成 | 🟡 5/7 PASS | — | TPC-H step + REPORT step pending | |
| - |  **状态**: ✅ PASS (6/6 form-only steps) |
| - |  **状态**: ✅ PASS (6/6) |
| - |  **状态**: ✅ PASS (4/4) |
| - |  **状态**: ✅ PASS (4/4) |
| - |  - 5+ tests PASS |
| - |  - `check_arch2_no_bypass.sh` 全部 PASS |
| - |  **状态**: ✅ PASS (8/8) |
| - |  - 8+ tests PASS |
| - |  **状态**: ✅ PASS (6/6 form-only) |
| - |  - 100+ tests PASS |
| - |  **状态**: ✅ PASS (7/7) — **但 RC3_PLAN 揭示真实问题**: |
| - |  - 10 soak tests PASS at 24h/72h/168h |
| - |  **状态**: ✅ PASS (7/7 form-only) |
| - |  - 8 类崩溃注入 × 10+ 变体 模拟通过 |
| - |  **⚠️ RC3_PLAN 关键发现**: Crash matrix 模拟通过, real crash 8 categories pending |
| - |  **状态**: ✅ PASS (5/5) |
| - |  - 50+ tests PASS |
| - |  **状态**: 🟡 PASS (7/7 + 1 sub-gate WARN non-blocking) |
| - |  | **G15** 汇总报告 | ✅ PASS (4/4) | 6 perf reports committed | |
| - |  **状态**: 🟡 5/7 PASS |
| - |  | **RC5-RC7** | 2026-06-13 | ✅ | G1-G16 PASS + substance | 330+ tests | 30m soak PASS | 2026-06-13 | |
| - |  - 16/16 sub-tasks completed (100%) |
| - |  - 72h soak compressed-time: 10/10 tests PASS (1,440× compression) |
| - |  - Doc gates PASS (check_docs_consistency.sh + check_docs_links.sh) |
| - |  - G1-G10 baseline: 10/10 PASS |
| - |  - G16 Compatibility: 5/7 PASS (TPC-H step + REPORT step pending) |
| - |  - G1-G10: 10/10 PASS |
| - |  - 72h Soak: 10/10 PASS (compressed) |
| - |  - [ ] Doc gates PASS |
| - |  - [ ] 24h real soak PASS (memory < 10%, FD = 0, lock = 0) |
| - |  - [ ] 72h real soak PASS |
| - |  - [ ] 168h real soak completed successfully |
| - |  - **72h Soak (compressed)**: 10/10 PASS, memory < 10%, FD = 0, lock = 0, p99 bounded |
| - |  | **2026-06-13** | **RC7 cut** | G1-G16 PASS + substance tests | ✅ DONE | |
| - |  - [ ] G1: 22/22 TPC-H 保持 PASS (REAL, not form-only) |
| - |  - [ ] G7: 24h REAL wall-clock Soak Test PASS |
| - |  - [ ] G8: Crash Matrix 100+ REAL scenarios PASS |
| - |  - [ ] G9: Upgrade Test PASS (v3.8 → v3.9 数据可读, REAL) |
| - |  - [ ] G16: Compatibility v3.8 → v3.9 (full PASS) |
| - |  - [ ] 168h real soak completed |
| - |  | **GA (General Availability)** | ✅ PASS | ✅ TARGET | 🟡 form-only (35%) | |
| - |  - 10/10 G1-G10 form-only PASS |
| - |  | G16 Compatibility full PASS | 8h | P1 | (compat) | |
| - |  ✅ G16 5/7 PASS |
| - |  ✅ 72h Soak compressed-time 10/10 PASS |
| - |  > **E2E 测试**: 16/16 PASS ✅ (修复了 column_def 包 bug: org_name, length_of_fixed_fields, real_col_names) |
| - |  | Lib Tests | 1670 PASS, 1 IGNORED | ✅ 已执行 | |
| - |  | TPC-H | 22/22 PASS | ⚠️ 无 oracle 对比 | |
| - |  | Corpus | 818/818 PASS | ⚠️ 无 oracle 对比 | |
| - |  | D9 Gate | 8/8 PASS | ✅ 有独立验证 | |
| - |  | G1 | TPC-H 22/22 | ✅ PASS | ⚠️ 无 oracle 对比 | **Q8 0.18ms** | |
| - |  | G2 | INT-2 | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G3 | INT-3 | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G4 | ARCH-3 | ✅ PASS | ✅ 有独立验证 | — | |
| - |  | G5 | SEM-1 | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G6 | Backup/Restore | ✅ PASS | ✅ 有独立验证 | — | |
| - |  | G8 | Crash Matrix | ✅ PASS | ✅ 有独立验证 | — | |
| - |  | G9 | Upgrade Test | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G10 | Audit + Time Travel | ✅ PASS | ✅ 有独立验证 | — | |
| - |  | G11 | QPS/TPS | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G12 | Sysbench | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G14 | Real Crash | ✅ PASS | ⚠️ 部分模拟 | — | |
| - |  | G15 | TPC-H SF0.01 | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | G16 | Compatibility | ✅ PASS | ⚠️ 无 oracle 对比 | — | |
| - |  | **P11-P15** | **meta-gate** (Sprint 8) | ✅ **5/5 PASS** | **V5/V6/V8/V2 全部修复** | **Sprint 8 Track B** | |
| - |  **诚实声明**: 16/16 G1-G16 gate 脚本已执行 + 5 meta-gates (P11-P15) PASS (Sprint 8), 但 11/16 缺乏独立 oracle 对比。真实 soak 测试 infra ready (Sprint 8), run 仍 pending Z6G4。 |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > **执行结果**: ✅ 8/8 修改完成，复核 100% PASS |
| - |  | 操作 1: CHANGELOG 阶段 RC2 → RC7 | ✅ PASS (1 occurrence) | |
| - |  | 操作 2: CHANGELOG 版本表 rc4/rc5/rc6/rc7 | ✅ PASS (4 entries) | |
| - |  | 操作 3: README GA 目标 2026-12-15 | ✅ PASS (1 occurrence) | |
| - |  | 操作 4: RELEASE_NOTES Header 阶段标注 | ✅ PASS (1 occurrence) | |
| - |  | 操作 5: root CHANGELOG RC7 status | ✅ PASS (1 occurrence) | |
| - |  | 操作 6: ROADMAP 4.9 更新为 RC7 | ✅ PASS (1 occurrence) | |
| - |  | 无 commit 日志内容修改 | ✅ PASS | |
| - |  | 无功能描述/架构设计修改 | ✅ PASS | |
| - |  | 所有引用 .md 文件存在 | ✅ PASS (no broken refs introduced) | |
| - |  | git diff 干净 | ✅ PASS (31 insertions, 19 deletions, all in 7 files) | |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | 问题 1 已修复：ROADMAP.md Phase 状态正确 | ✅ 通过 | |
| - |  | 问题 2 已修复：ROADMAP.md GA 日期标注风险 | ✅ 通过 | |
| - |  | 问题 3 已修复：ROADMAP.md 文档引用路径正确 | ✅ 通过 | |
| - |  | 问题 4 已修复：V390_VERSION_PLAN.md W0 日期正确 | ✅ 通过 | |
| - |  | 问题 5 已修复：V390_VERSION_PLAN.md 状态正确 | ✅ 通过 | |
| - |  | 问题 6 已修复：V390_VERSION_PLAN.md GA 日期标注风险 | ✅ 通过 | |
| - |  | 问题 7 已修复：CHANGELOG.md 阶段表述完整 | ✅ 通过 | |
| - |  | 问题 8 已修复：alpha/ 目录存在 | ✅ 通过 | |
| - |  | commit 日志内容未被修改 | ✅ 通过 | |
| - |  | 功能描述未被修改 | ✅ 通过 | |
| - |  | 实质性技术内容未被修改 | ✅ 通过 | |
| - |  | alpha/ALPHA1_RELEASE_NOTES.md 已创建 | ✅ 通过 | |
| - |  | git diff 无非预期修改 | ✅ 通过 | |
| - |  | 所有修改可通过 `git checkout -- <file>` 恢复 | ✅ 通过 | |
| - |  | 工作记录完整，可追溯每一步 | ✅ 通过 | |
| - |  本次文档整改遵循 `DOC_CHECK_CORRECTION_RULES.md` 的 5 步流程，成功修复了 8 处不自洽问题： |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > **Last update**: 2026-06-26 (corrected — Z6G4 72h soak never completed; prior claim of "RUNNING" was incorrect) |
| - |  | **24h real wall-clock** | 24h | ❌ **INCOMPLETE** | 250: 843 samples before Z6G4 lost; Z6G4: never started | Z6G4 unreachable; 24h soak not completed | |
| - |  - [x] 30m wall-clock PASS (post-fix, see LOCAL_SHORT_SOAK_REPORT) |
| - |  - [x] 1h/2h/4h ladder steps PASS (see LOCAL_SHORT_SOAK_REPORT) |
| - |  soak duration PASS marks. |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | 4 | `tests/tpch_value_correctness_test.rs` | ExecutionEngine + 合成数据 | 数值正确性 gate | ✅ E2E (synthetic data 也通过 wire protocol) | |
| - |  | 10 | `tests/tpch_sf01_perf baseline_test.rs` | ExecutionEngine + timing | SF=0.1 perf baseline | ✅ E2E + 通过 wire protocol 跑 sysbench-style 负载 | |
| - |  2. **连接** 通过 `MySqlTestClient` 或真实 mysql client |
| - |  3. **工作负载** 通过 wire protocol (COM_QUERY, COM_STMT_PREPARE, COM_STMT_EXECUTE) |
| - |  4. **数据** 通过 wire protocol (LOAD DATA LOCAL INFILE, INSERT) |
| - |  6. **断言** 通过 wire protocol 读回的 rows |
| - |  12. **`tests/tpch_value_correctness_test.rs` E2E 化** (synthetic data 通过 wire) |
| - |  - 修完后 `tpch_value_test_v2` 等会自然通过 |
| - |  - [ ] 5-min 集成测试 PASS (A1+A2+A3) |
| - |  - [ ] G11 E2E test 通过 (C) |
| - |  - [ ] G13 E2E test 通过 (D) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  Z6G4 G11 in progress as of 2026-06-12 20:35 — 3/22 done (point_select/1, /4, on /8). |
| - |  - G11 gate (`check_g11_qps.sh`) PASSES form: qps_bench exists, registers, compiles, runs |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  benchmark PASSES in row-count (5/5) and the cell-level |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > **Status**: PASS |
| - |  | Root cause identified | PASS — `src/engine_select.rs:1175-1215` O(N²) hash match bookkeeping | |
| - |  | Fix landed | PASS — `HashMap<String, Vec<(usize, &Vec<Value>)>>` O(1) 索引进哈希值 | |
| - |  | In-process audit test | PASS — `tests/tpch_q9_audit.rs` 22 query 对比 SQLite baseline | |
| - |  | Q9 wired result MATCH | PASS — engine=75, sqlite=75 | |
| - |  | Other query regression | PASS — Q1, Q3-Q6, Q10-Q16 全部 MATCH SQLite (13/16) | |
| - |  | `cargo clippy` | PASS — 未引入新警告 | |
| - |  | `cargo fmt` | PASS | |
| - |  | `cargo test` | PASS | |
| - |  | Q17-22 subquery | in-process 未在 Q9 fix 验证范围 | 修复前 17/22 PASS 基线保持 | |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |      echo "✅ Coverage Gate PASS" |
| - |  The RC8 tag can be cut based on local P0 completion (✅ done). GA tag requires Z6G4. |
| - |  | G13 | 24h Stability (extended) | ⏳ | `check_g13_stability.sh` (template) | ⚠️ SIMULATED. Real-data: 72h soak INTERRUPTED — 4-min sample then Z6G4 unreachable (2026-06-19); never completed | |
| - |  | **P14** | DRIFT != PASS | ✅ | V5/V6/V8 全部修复 (Sprint 8) | |
| - |  - [x] 6/6 meta-gates PASS |
| - |          echo "[$(date)] Run completed normally, restarting in 2s" >> "${LOG_FILE}" |
| - |  done |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  > **State**: Pre-GA. All pre-soak gates PASS. **Ready to dispatch real wall-clock 24h+ soak to Z6G4.** |
| - |  | P11 | Gate Self-Verification | ✅ PASS | `scripts/gate/check_gate_self_verification.sh` | |
| - |  | P12 | No Implicit Tolerance | ✅ PASS | `scripts/gate/check_ignore_count.sh` (1 pre-existing tpch_sf1_22_vs_3engines ignore marker) | |
| - |  | P13 | Test Count Monotonicity | ✅ PASS | active=6200, cargo=115 (matches baseline) | |
| - |  | P14 | DRIFT != PASS | ✅ PASS | 0 anti-patterns | |
| - |  | P15 | Oracle Required | ✅ PASS | All 8 gate oracles present | |
| - |  | Clippy | No warnings | ✅ PASS | `cargo clippy --all-features -- -D warnings` (1 pre-existing parser unreachable pattern, unrelated) | |
| - |  | Fmt | Clean | ✅ PASS | `cargo fmt --check` clean on changed files | |
| - |  - 22/22 in-process: PASS (default features) |
| - |  - 22/22 in-process: PASS (with feature on) |
| - |  - G15 wire oracle 22/22: PASS across 5 sub-tests |
| - |  - ✅ All pre-soak quality gates PASS |
| - |  168h soak PASS |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  | V390_COMPREHENSIVE_ASSESSMENT.md | ✅ 10/10 form-only PASS | |
| - |  **问题**: 同一文档内先说 "PASS"，后说 "实际未跑"，容易误导读者。 |
| - |  - 门禁状态应明确区分 "form-only PASS" vs "REAL PASS" |
| - |    G1: ✅ form-only PASS | ⏳ REAL pending |
| - |    G7: ✅ compressed PASS | ⏳ 24h real pending |
| - |  | V390_COMPREHENSIVE_ASSESSMENT.md §4.2 | ✅ PASS (6/6 form-only steps) | |
| - |  | V390_COMPREHENSIVE_ASSESSMENT.md §3.2 | ✅ G4 form-only PASS | |
| - |  | 同文档 §3.2 | ✅ G3 form-only PASS | |
| - |  | 同文档 §3.2 | ✅ G2 form-only PASS | |
| - |  | 同文档 §3.2 | ✅ G5 form-only PASS | |
| - |  - P0 任务状态应统一为 "✅ form-only PASS | ⏳ REAL pending RC3" |
| - |  1. 门禁状态统一格式: "✅ form-only PASS | ⏳ REAL pending" |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  - TPC-H SF=0.1: 22/22 查询通过 |
| - |  | `tpch_sf01_inprocess_test` | 22/22 PASS | SF=0.1 完整 TPC-H | |
| - |  | `mysql_wire_protocol_test` | 28/28 PASS | MySQL wire 协议 | |
| - |  | `tpch_sf01_inprocess_test` | 17 tests PASS | 包含 oracle 框架 | |
| - |  Thread 0 done: 12345 queries, 0 errors, 12345 rows |
| - |  Thread 1 done: 12300 queries, 0 errors, 12300 rows |
| - |  Thread 2 done: 12280 queries, 0 errors, 12280 rows |
| - |  Thread 3 done: 12310 queries, 0 errors, 12310 rows |
| - |  - [x] 单线程 QPS 验证通过 |
| - |  - [x] TPC-H 22/22 查询通过 |
| - |  > **Status: 🟡 READY (gates G1-G16 PASS, 24h/72h/168h soak incomplete/interrupted)** |
| - |  | G1 | TPC-H 22/22 (QPS-correctness) | ✅ PASS | tpch_gate_test 22/22 | |
| - |  | G2 | INT-2 ParallelExecutor | ✅ PASS | int2_substance_parallel_test (9 tests) | |
| - |  | G3 | INT-3 Single Expression | ✅ PASS | int3_substance_delegation_test (17 tests) | |
| - |  | G4 | ARCH-3 VtuGuard | ✅ PASS | check_arch3_no_bypass.sh (8/8) | |
| - |  | G5 | SEM-1 Savepoint | ✅ PASS | check_sem1_savepoint.sh (8/8) | |
| - |  | G6 | Backup/Restore/PITR | ✅ PASS | check_backup_restore.sh (6/6, 51 e2e) | |
| - |  | G8 | Crash Matrix | ✅ PASS | check_p12_crash_test.sh | |
| - |  | G9 | Upgrade v3.8→v3.9 | ✅ PASS | check_p14_upgrade_test.sh (50 tests) | |
| - |  | G10 | GMP Audit + Time Travel + Hash Chain | ✅ PASS | check_p21/22/23_*.sh | |
| - |  | G15 | SF=0.01 TPC-H wire | ✅ PASS | tpch_sf01_22_queries_wire_test | |
| - |  | G16 | Compatibility v3.8→v3.9 | ✅ PASS | v380_to_v390_full_upgrade_test (5 cases) | |
| - |  **Total: 11/13 PASS, 1/13 incomplete (G11), 1/13 pending re-run (G13)** |
| - |  | Substance tests | 41 | 41/41 PASS | |
| - |  | TPC-H wire (G1) | 22 | 22/22 PASS | |
| - |  | TPC-H wire SF0.01 (G15) | 22 | 22/22 PASS | |
| - |  | Upgrade (G9, G16) | 55 | 55/55 PASS | |
| - |  | Backup/Restore (G6) | 51 | 51/51 PASS | |
| - |  | Crash Matrix (G8) | 129 | 129/129 PASS | |
| - |  | Stability (G7) | 10 | 10/10 PASS | |
| - |  | E2E SELECT (2026-07-04) | 16 | 16/16 PASS | |
| - |  | **Total verified tests** | **346+** | **346+ / 346+ PASS** | |
| - |  | 1h simulated | rc4 readiness | Z6G4 (rc4 binary) | ✅ PASS | |
| - |  - [x] G1-G10 gates PASS |
| - |  - [ ] TPC-H SF=1.0 Q1-Q22 PASS (hardware-blocked: disk) |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |  benchmark PASS** milestone. |
| - |  Full TPC-H 22/22 PASS. SQL92 core (DML/joins/aggregates) is |
| - |  > - SGL-001 rustfmt drift 修复 (integration gate 4/4 PASS) |
| - |    ✅ SOAK PASSED |
| - |  | 0 errors | PASS | |
| - |  | P99 < 5000ms | PASS | |
| - |  | > 0 queries executed | PASS | |
| - |  | Any error > 0 | WARNING (still PASS) | |
| - |  | 0 | SOAK PASSED (all criteria met) | |

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

- 违规次数: 424
- 违规类型: Type A（虚构执行）/ Type B（伪门禁）/ Type D（伪任务完成）
- 处理方式: 触发 Anti-Fabrication Policy 问责机制


---

*报告生成时间: 2026-07-11 14:19:38*
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

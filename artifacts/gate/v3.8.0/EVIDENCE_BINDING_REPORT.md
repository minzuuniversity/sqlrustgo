# vv3.8.0 证据绑定检查报告

> **检查日期**: 2026-06-01
> **版本**: v3.8.0
> **Auditor**: Hermes Agent (Anti-Fabrication Policy v1.0)
> **检查工具**: check_evidence_binding.sh

---

## 检查结果概览

| 检查项 | 数量 |
|--------|------|
| 通过 | 98 |
| 警告 | 150 |
| 失败（违规） | 530 |
| 未验证声明 | 503 |

**总结**: ❌ 发现 530 个违规（Type A/B/C/D）

---

## 违规详情（按类型分类）

### Type A: 虚构执行（Execution Fabrication）

无 CI 证据声明"测试通过 / 编译成功"

- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: | **单元测试** | ✅ 317/317 + 283/283 PASS | executor + storage c
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: | **WAL Recovery** | ✅ 22/22 PASS | RECOVERY-007 re-enabled 
- ❌ Type A/B 违规：第 252 行声明无 CI/gate/commit 证据: | **Executor** | 317 PASS ✅ | 大量 | ❌ 无 mysql-server E2E | 高 
- ❌ Type A/B 违规：第 253 行声明无 CI/gate/commit 证据: | **Storage** | 283 PASS ✅ | 16 (`wal_integration_test.rs`) 
- ❌ Type A/B 违规：第 371 行声明无 CI/gate/commit 证据: | **错误4**: 混淆 "测试通过" 和 "集成正确" | 🟡 方法论 | PR-870 测试全过但 execute
- ❌ Type A/B 违规：第 377 行声明无 CI/gate/commit 证据: - R3 (Expr Convergence): 类型迁移干净，317 测试通过
- ❌ Type A/B 违规：第 379 行声明无 CI/gate/commit 证据: - PR-830E (WAL Recovery): One-shot guard 正确实现，21/22 测试通过
- ❌ Type A/B 违规：第 394 行声明无 CI/gate/commit 证据: | Executor 单元测试 | 317/317 PASS | ✅ |
- ❌ Type A/B 违规：第 395 行声明无 CI/gate/commit 证据: | Storage 单元测试 | 283/283 PASS | ✅ |
- ❌ Type A/B 违规：第 396 行声明无 CI/gate/commit 证据: | WAL Contract 测试 | 22/22 PASS ✅ | RECOVERY-007 re-enabled +
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: 预期：`{"result":"PASS","reason":"reachability","missing":[]}`
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: 验证 5/5 tests PASS。
- ❌ Type A/B 违规：第 88 行声明无 CI/gate/commit 证据: - 假设通过
- ❌ Type A/B 违规：第 78 行声明无 CI/gate/commit 证据: | Parser Coverage | ✅ | 100 PASS（刚修复 FOR tokenization） |
- ❌ Type A/B 违规：第 91 行声明无 CI/gate/commit 证据: | L1-1 Parser lib | `cargo test -p sqlrustgo-parser --lib` |
- ❌ Type A/B 违规：第 92 行声明无 CI/gate/commit 证据: | L1-2 Parser coverage tests | `cargo test -p sqlrustgo-pars
- ❌ Type A/B 违规：第 154 行声明无 CI/gate/commit 证据: | Dirty Read Prevention | ❌ | 必须 PASS |
- ❌ Type A/B 违规：第 155 行声明无 CI/gate/commit 证据: | Non-repeatable Read | ❌ | 必须 PASS |
- ❌ Type A/B 违规：第 156 行声明无 CI/gate/commit 证据: | Phantom Read | ❌ | 必须 PASS |
- ❌ Type A/B 违规：第 157 行声明无 CI/gate/commit 证据: | Write-Write Conflict | ❌ | 必须 PASS |
- ❌ Type A/B 违规：第 158 行声明无 CI/gate/commit 证据: | Commit crash recovery | ❌ | 必须 PASS |
- ❌ Type A/B 违规：第 159 行声明无 CI/gate/commit 证据: | Rollback crash | ❌ | 必须 PASS |
- ❌ Type A/B 违规：第 160 行声明无 CI/gate/commit 证据: | WAL replay ordering | ❌ | 必须 PASS |
- ❌ Type A/B 违规：第 172 行声明无 CI/gate/commit 证据: | **L1 unit tests** | ✅ 98 PASS | ✅ 98 PASS | 保持 |
- ❌ Type A/B 违规：第 173 行声明无 CI/gate/commit 证据: | **L1 Coverage** | 条件性 PASS（<75% 可接受） | **L1 >= 70%, 每crate
- ❌ Type A/B 违规：第 176 行声明无 CI/gate/commit 证据: | **Truthfulness 原则** | 无明确文档 | **强制**：禁止改文档通过门禁 | **v3.8.0 
- ❌ Type A/B 违规：第 188 行声明无 CI/gate/commit 证据: - **v3.7.0 门禁保持现状**，已经修复到 100 PASS parser_coverage_tests
- ❌ Type A/B 违规：第 318 行声明无 CI/gate/commit 证据: | Q1-Q22 | — | 22/22 PASS |
- ❌ Type A/B 违规：第 235 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 314 行声明无 CI/gate/commit 证据: | 测试类型 | 通过标准 | Gate ID |
- ❌ Type A/B 违规：第 318 行声明无 CI/gate/commit 证据: | L3 E2E Tests | 28/28 files PASS | C2 |
- ❌ Type A/B 违规：第 319 行声明无 CI/gate/commit 证据: | TPC-H SF=1 | 22/22 queries PASS | C3 |
- ❌ Type A/B 违规：第 320 行声明无 CI/gate/commit 证据: | Backward Compatibility | 所有现有 tests 仍然 PASS | A2 |
- ❌ Type A/B 违规：第 357 行声明无 CI/gate/commit 证据: | 测试 | 当前 PASS 率 | PR-800 后 |
- ❌ Type A/B 违规：第 13 行声明无 CI/gate/commit 证据: v3.6.0 Beta Gate 报告显示 `7/9 PASS`，看起来一切正常。
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: | WAL Contract | 21/22 PASS | **0/22**（测试框架损坏） |
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: | Integration | PASS | **架构漂移未检测** |
- ❌ Type A/B 违规：第 56 行声明无 CI/gate/commit 证据: Type A — 虚构执行: "测试通过" 但测试框架损坏
- ❌ Type A/B 违规：第 57 行声明无 CI/gate/commit 证据: Type B — 伪门禁:   Gate PASS 但无真实CI证据
- ❌ Type A/B 违规：第 139 行声明无 CI/gate/commit 证据: │  作用: 在编译/测试通过之上，增加语义层验证，保证规格与实现一致                  │
- ❌ Type A/B 违规：第 167 行声明无 CI/gate/commit 证据: RC Gate:     功能完整            → 性能/回归通过 + 证据链
- ❌ Type A/B 违规：第 168 行声明无 CI/gate/commit 证据: GA Gate:     所有检查通过        → 代码冻结 + 发布
- ❌ Type A/B 违规：第 245 行声明无 CI/gate/commit 证据: **通过标准**: 方案选定，无未解决的架构冲突
- ❌ Type A/B 违规：第 259 行声明无 CI/gate/commit 证据: | **L1句法** | cargo build/test/clippy/fmt | 全部 PASS |
- ❌ Type A/B 违规：第 262 行声明无 CI/gate/commit 证据: **通过标准**: 功能规格实现，基础测试通过，无 P0/P1 问题
- ❌ Type A/B 违规：第 275 行声明无 CI/gate/commit 证据: | SQL Compat | SQL Corpus 通过率 (基线建立) |
- ❌ Type A/B 违规：第 278 行声明无 CI/gate/commit 证据: **通过标准**: 集成测试全部通过，架构不变式满足，无 P0/P1 漂移
- ❌ Type A/B 违规：第 282 行声明无 CI/gate/commit 证据: **目标**: 性能达标，回归通过，证据链完整
- ❌ Type A/B 违规：第 288 行声明无 CI/gate/commit 证据: | 回归测试 | `cargo test` 全部通过 |
- ❌ Type A/B 违规：第 289 行声明无 CI/gate/commit 证据: | SQL Corpus | 通过率 ≥90% |
- ❌ Type A/B 违规：第 294 行声明无 CI/gate/commit 证据: **通过标准**: 性能基线达标，证据链完整，无 regression
- ❌ Type A/B 违规：第 309 行声明无 CI/gate/commit 证据: **通过标准**: 零未解决问题，零 open blocker，代码冻结
- ❌ Type A/B 违规：第 318 行声明无 CI/gate/commit 证据: L1通过 ≠ L2通过 ≠ L3通过
- ❌ Type A/B 违规：第 321 行声明无 CI/gate/commit 证据: L1: cargo build ✓    → 编译通过
- ❌ Type A/B 违规：第 322 行声明无 CI/gate/commit 证据: L2: cargo test ✓     → 22个recovery tests通过
- ❌ Type A/B 违规：第 325 行声明无 CI/gate/commit 证据: 结论: L1/L2通过只是"没明显出错"
- ❌ Type A/B 违规：第 326 行声明无 CI/gate/commit 证据:       L3通过才是"真正正确"
- ❌ Type A/B 违规：第 336 行声明无 CI/gate/commit 证据: │  产出: PASS/FAIL (exit code)                                
- ❌ Type A/B 违规：第 348 行声明无 CI/gate/commit 证据: │  产出: PASS / FAIL / DRIFT (legacy tracked)                 
- ❌ Type A/B 违规：第 361 行声明无 CI/gate/commit 证据: EXIT_PASS  = 0  — 所有检查通过
- ❌ Type A/B 违规：第 375 行声明无 CI/gate/commit 证据: > **背景**: v3.6.0 Beta Gate 7/9 PASS 实际 0/8 通过 —— AI 伪造证据
- ❌ Type A/B 违规：第 381 行声明无 CI/gate/commit 证据: **规则**: 任何 PASS/FAIL 声明必须有 CI 证据，包括：
- ❌ Type A/B 违规：第 390 行声明无 CI/gate/commit 证据: | **Type A** | 虚构执行：声称测试通过但无 CI run ID | P0 | PASS 声明无 CI ru
- ❌ Type A/B 违规：第 399 行声明无 CI/gate/commit 证据: ❌ Beta Gate Report: "7/9 PASS"
- ❌ Type A/B 违规：第 491 行声明无 CI/gate/commit 证据: 结论: ✅ B2 PASS
- ❌ Type A/B 违规：第 515 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 548 行声明无 CI/gate/commit 证据:     done
- ❌ Type A/B 违规：第 549 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 632 行声明无 CI/gate/commit 证据: | R2 | **Test** | `cargo test --lib` 全部通过 | ✅ | ✅ | ✅ | ✅ |
- ❌ Type A/B 违规：第 634 行声明无 CI/gate/commit 证据: | R4 | **Format** | `cargo fmt -- --check` 通过 | ✅ | ✅ | ✅ | 
- ❌ Type A/B 违规：第 638 行声明无 CI/gate/commit 证据: | R8 | **Corpus Gate** | SQL Corpus 通过率不降低（≥基线） | — | ⚠️ | ✅
- ❌ Type A/B 违规：第 723 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 757 行声明无 CI/gate/commit 证据: **检查内容**: SQL Corpus 测试通过率（相对于基线不降低）
- ❌ Type A/B 违规：第 801 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 849 行声明无 CI/gate/commit 证据: **目的**: 保证所有 PASS/FAIL 声明都有真实 CI 证据，防止 AI 伪造
- ❌ Type A/B 违规：第 858 行声明无 CI/gate/commit 证据:     lines=$(grep -nE "(PASS|FAIL|通过|失败)" "$doc")
- ❌ Type A/B 违规：第 865 行声明无 CI/gate/commit 证据:     done
- ❌ Type A/B 违规：第 870 行声明无 CI/gate/commit 证据:     if grep -qE "GA.*PASS|Gate.*PASS" "$gate_doc"; then
- ❌ Type A/B 违规：第 914 行声明无 CI/gate/commit 证据:     done
- ❌ Type A/B 违规：第 915 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 922 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 1018 行声明无 CI/gate/commit 证据: | 0 | 所有检查通过 | PASS |
- ❌ Type A/B 违规：第 1129 行声明无 CI/gate/commit 证据: RC Gate     → 性能收敛（回归通过）
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: 本文档定义 PR-800 的验收标准（Acceptance Criteria）。每个验收标准必须有明确的验证方法和通过标
- ❌ Type A/B 违规：第 31 行声明无 CI/gate/commit 证据: **通过标准**: 所有 test_pr800_* 测试 PASS（预计 30+ tests）
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: Status: ✅ PASS
- ❌ Type A/B 违规：第 52 行声明无 CI/gate/commit 证据: **通过标准**: 所有 test_pr800_* 测试 PASS
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: **通过标准**: E2E 测试 28/28 PASS
- ❌ Type A/B 违规：第 90 行声明无 CI/gate/commit 证据: **通过标准**: 直接 raw_sql 执行路径 = 0 matches
- ❌ Type A/B 违规：第 104 行声明无 CI/gate/commit 证据: **通过标准**: local_executor 被主路径调用（非孤岛组件）
- ❌ Type A/B 违规：第 120 行声明无 CI/gate/commit 证据: **通过标准**: sqlrustgo crate 所有 lib tests PASS
- ❌ Type A/B 违规：第 126 行声明无 CI/gate/commit 证据: **标准**: 现有的 E2E 测试文件全部 PASS。
- ❌ Type A/B 违规：第 133 行声明无 CI/gate/commit 证据: **通过标准**: E2E test result: ok (所有测试通过)
- ❌ Type A/B 违规：第 148 行声明无 CI/gate/commit 证据: **通过标准**: Exit code 0
- ❌ Type A/B 违规：第 161 行声明无 CI/gate/commit 证据: **通过标准**: 0 warnings, 0 errors
- ❌ Type A/B 违规：第 167 行声明无 CI/gate/commit 证据: **标准**: `cargo build --release --workspace` 成功。
- ❌ Type A/B 违规：第 174 行声明无 CI/gate/commit 证据: **通过标准**: "Finished release profile" 或 "Compiling X crates..
- ❌ Type A/B 违规：第 191 行声明无 CI/gate/commit 证据: **通过标准**: 无绕过 Parser 的路径
- ❌ Type A/B 违规：第 206 行声明无 CI/gate/commit 证据: **通过标准**: execution_engine.rs 行数 <6000 行
- ❌ Type A/B 违规：第 223 行声明无 CI/gate/commit 证据: **通过标准**: parser 覆盖率 ≥85%
- ❌ Type A/B 违规：第 236 行声明无 CI/gate/commit 证据: **通过标准**: planner 覆盖率 ≥85%
- ❌ Type A/B 违规：第 249 行声明无 CI/gate/commit 证据: **通过标准**: executor 覆盖率 ≥83% (v3.7.0 baseline)
- ❌ Type A/B 违规：第 143 行声明无 CI/gate/commit 证据: C2: 28 E2E test files PASS
- ❌ Type A/B 违规：第 22 行声明无 CI/gate/commit 证据: | **B2** | WAL Execution Path | `ExecutionEngine::with_wal(P
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | **B-F5** | PR-DAG 一致性 | — | ✅ PASS | DEVELOPMENT_PLAN.md v
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | **B-F6** | Feature Checklist | — | ✅ PASS | docs/releases/
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: | **B-F7** | 无幽灵 PR | — | ✅ PASS | 所有未合并 PR 有说明 |
- ❌ Type A/B 违规：第 95 行声明无 CI/gate/commit 证据:   RECOVERY-001 begin_then_crash_rolls_back       ✅ PASS  (us
- ❌ Type A/B 违规：第 96 行声明无 CI/gate/commit 证据:   RECOVERY-002 insert_then_crash_rolls_back      ✅ PASS
- ❌ Type A/B 违规：第 98 行声明无 CI/gate/commit 证据:   RECOVERY-004 commit_flush_crash_replays        ✅ PASS
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据:   RECOVERY-005 partial_insert_write_recovery     ✅ PASS
- ❌ Type A/B 违规：第 100 行声明无 CI/gate/commit 证据:   RECOVERY-006 partial_update_write_recovery     ✅ PASS
- ❌ Type A/B 违规：第 102 行声明无 CI/gate/commit 证据:   RECOVERY-008 partial_commit_flush_recovery     ✅ PASS
- ❌ Type A/B 违规：第 168 行声明无 CI/gate/commit 证据: The question "how can we enter RC if features aren't done?" 
- ❌ Type A/B 违规：第 204 行声明无 CI/gate/commit 证据: > - B2 WAL Execution Path: 21/22 RECOVERY tests pass — PASS
- ❌ Type A/B 违规：第 216 行声明无 CI/gate/commit 证据: - [x] B1 Build: core 5 crates build PASS
- ❌ Type A/B 违规：第 217 行声明无 CI/gate/commit 证据: - [x] B2 WAL Execution Path: WAL path exists + 21/22 RECOVER
- ❌ Type A/B 违规：第 218 行声明无 CI/gate/commit 证据: - [x] B3 Clippy: 0 warnings PASS
- ❌ Type A/B 违规：第 219 行声明无 CI/gate/commit 证据: - [x] B4 Format: fmt check PASS
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：BETA_GATE_CONTRACT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：BETA_GATE_CONTRACT.md
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: | INT-2: VTU Merge | MERGE via MergeExecutor | PR-870 phase 
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: - R3 expr convergence completed (MergeStatement uses planner
- ❌ Type A/B 违规：第 171 行声明无 CI/gate/commit 证据: | INT-1 | ✅ PROVEN | 22/22 PASS | N/A |
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | Beta | B1 Build / B2 WAL Contract / B3 Clippy | Recovery 7
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | GA | 所有门禁 PASS + TPC-H 22/22 | 100% |
- ❌ Type A/B 违规：第 89 行声明无 CI/gate/commit 证据:     {"id": "A1", "name": "Build", "result": "PASS", "command
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：R5-GATE-REFORM.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：R5-GATE-REFORM.md
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: | A1_BUILD | ✅ PASS | Core 6 crates build successfully |
- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: | A2_TEST | ✅ PASS | 89 tests pass across 6 crates |
- ❌ Type A/B 违规：第 17 行声明无 CI/gate/commit 证据: | A3_CLIPPY | ✅ PASS | Zero warnings on core crates |
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | A6-1_REPLAY | ✅ PASS | REPLAY_v3.7.0_GA.md exists |
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: | A6-2_CLAIM | ✅ PASS | ADR-002-claim-registry.md exists |
- ❌ Type A/B 违规：第 22 行声明无 CI/gate/commit 证据: | A6-3_DECISION | ✅ PASS | ARCHITECTURE_DECISIONS.md exists 
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据: | A6-4_FRESHNESS | ✅ PASS | Freshness markers present |
- ❌ Type A/B 违规：第 24 行声明无 CI/gate/commit 证据: | A6-5_ADR | ✅ PASS | ADR-001~ADR-005 all exist |
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: **PASS: 8/10 | BLOCKERS: 2**
- ❌ Type A/B 违规：第 87 行声明无 CI/gate/commit 证据: Step 3: Re-run check_alpha_v380.sh → expect PASS
- ❌ Type A/B 违规：第 78 行声明无 CI/gate/commit 证据:     /// Recovery completed successfully
- ❌ Type A/B 违规：第 131 行声明无 CI/gate/commit 证据:     "WAL recovery completed: {} committed txns, {} rolled ba
- ❌ Type A/B 违规：第 201 行声明无 CI/gate/commit 证据: cargo test --test wal_tx_contract  → 15+ PASS (RECOVERY test
- ❌ Type A/B 违规：第 120 行声明无 CI/gate/commit 证据:         └── StorageEngine          ← 仅通过 Executor 访问
- ❌ Type A/B 违规：第 247 行声明无 CI/gate/commit 证据: **核心原则**: PhysicalPlan 只通过 `local_executor.rs` 执行，不存在其他执行路径。
- ❌ Type A/B 违规：第 285 行声明无 CI/gate/commit 证据: 单元测试通过，但组件从未在真实执行中被调用——这是集成缺失（Integration Gap）。
- ❌ Type A/B 违规：第 316 行声明无 CI/gate/commit 证据: - 性能回归测试必须通过
- ❌ Type A/B 违规：第 7 行声明无 CI/gate/commit 证据: > **标准**: v3.8.0 必须通过以下全部门禁方可发布 GA
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: | **计划不可伪造** | 开发/测试计划是历史记录，禁止重写原始计划以通过门禁 | 将 VERSION_PLAN.m
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: | **执行结果优先** | 以命令输出、测试结果为依据，禁止编造 | 在 TEST_PLAN.md 写入"PASS"但
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: 3. 在 TEST_PLAN.md 写入 "93 tests PASS" 但未执行 cargo test
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: | L2-1 | Execution consistency harness | `python3 scripts/te
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据: | L2-2 | E2E integration tests | `cargo test -p sqlrustgo-in
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: | L2-3 | TPC-H SF=1 regression | `./target/release/sqlrustgo
- ❌ Type A/B 违规：第 79 行声明无 CI/gate/commit 证据: | L3-01 | Dirty Read Prevention | `python3 scripts/test/isol
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: | L3-02 | Non-repeatable Read | `python3 scripts/test/isolat
- ❌ Type A/B 违规：第 81 行声明无 CI/gate/commit 证据: | L3-03 | Phantom Read | `python3 scripts/test/isolation_tes
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: | L3-04 | Write-Write Conflict | `python3 scripts/test/isola
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据: | L3-05 | Lost Update | `python3 scripts/test/isolation_test
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据: | L3-11 | Same SQL all paths | `python3 scripts/test/executi
- ❌ Type A/B 违规：第 100 行声明无 CI/gate/commit 证据: | L3-12 | NULL handling | `python3 scripts/test/execution_di
- ❌ Type A/B 违规：第 101 行声明无 CI/gate/commit 证据: | L3-13 | Type coercion | `python3 scripts/test/execution_di
- ❌ Type A/B 违规：第 102 行声明无 CI/gate/commit 证据: | L3-14 | Error handling | `python3 scripts/test/execution_d
- ❌ Type A/B 违规：第 114 行声明无 CI/gate/commit 证据: | L4-5 | ParallelVolcanoExecutor integrated | `scripts/test/
- ❌ Type A/B 违规：第 122 行声明无 CI/gate/commit 证据: | L5-1 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli 
- ❌ Type A/B 违规：第 138 行声明无 CI/gate/commit 证据: | L6-5 | SSOT cross-check | `bash scripts/docs/ssot_cross_ch
- ❌ Type A/B 违规：第 194 行声明无 CI/gate/commit 证据: echo "=== GA Gate PASSED ==="
- ❌ Type A/B 违规：第 205 行声明无 CI/gate/commit 证据: cargo test --lib --quiet && echo "L1 PASS" || echo "L1 FAIL"
- ❌ Type A/B 违规：第 206 行声明无 CI/gate/commit 证据: python3 scripts/test/execution_consistency_harness.py --quic
- ❌ Type A/B 违规：第 207 行声明无 CI/gate/commit 证据: grep "eng.execute.*raw_sql" src/ --include="*.rs" && echo "L
- ❌ Type A/B 违规：第 208 行声明无 CI/gate/commit 证据: wc -l src/execution_engine.rs | awk '{if($1<1500) print "L4 
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：GA_GATE_CHECKLIST.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：GA_GATE_CHECKLIST.md
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | Alpha | 编译通过、测试通过、格式干净 | cargo build/test/fmt/clippy |
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: | SGL-001 | B4 format 是 read-only | cargo fmt --check | PASS
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: | SGL-002 | WAL-002: checkpoint advance 在 commit 路径 | grep a
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: | SGL-003 | WAL-003: truncate_before 在 commit 路径 | grep trun
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: | SGL-004 | WAL-004: DELETE replay 幂等 | grep 语义检查 | PASS ✅ |
- ❌ Type A/B 违规：第 129 行声明无 CI/gate/commit 证据:   PASS: 4/4 sections
- ❌ Type A/B 违规：第 132 行声明无 CI/gate/commit 证据: SGL: 4/5 PASS, DRIFT=1
- ❌ Type A/B 违规：第 133 行声明无 CI/gate/commit 证据:   SGL-001~004: PASS ✅
- ❌ Type A/B 违规：第 136 行声明无 CI/gate/commit 证据: WAL: 22/22 PASS (Rust) + 5/5 PASS (invariant script)
- ❌ Type A/B 违规：第 138 行声明无 CI/gate/commit 证据: C-ARCH: 2/2 PASS (C-ARCH-02, C-ARCH-05)
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：INTEGRATION_GATE_PLAN.md
- ❌ Type A/B 违规：第 184 行声明无 CI/gate/commit 证据:         "WAL recovery completed: {} committed txns, {} rolle
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: **PASS 标准**：两个 binary 均可正常执行
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: ./target/release/ingest ci "alpha-check" --status PASS --db 
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: **PASS 标准**：`{"result":"PASS","reason":"reachability","missi
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: - C1: `cargo build -p evidence-graph --release` 成功
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: - C2: `cargo test -p evidence-graph` 全部 PASS（5个测试）
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: - C3: `cargo build -p sqlrustgo-gate --release` 成功
- ❌ Type A/B 违规：第 98 行声明无 CI/gate/commit 证据: - 状态：PASS | FAIL
- ❌ Type A/B 违规：第 107 行声明无 CI/gate/commit 证据: - ❌ 假设通过（必须实际执行命令）
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：ALPHA_GATE_PROMPT.md
- ❌ Type A/B 违规：第 151 行声明无 CI/gate/commit 证据: **保留理由**: LocalExecutor 是标准执行器，所有 PhysicalPlan 都通过它执行。
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: **PASS**：无编译错误，binary 可执行
- ❌ Type A/B 违规：第 42 行声明无 CI/gate/commit 证据: ./target/release/ingest ci "$CI" "$COMMIT" PASS --db $DB
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: **PASS**：`{"result":"PASS","reason":"reachability","missing"
- ❌ Type A/B 违规：第 81 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 118 行声明无 CI/gate/commit 证据: 在 Alpha PASS 基础上执行：
- ❌ Type A/B 违规：第 157 行声明无 CI/gate/commit 证据: 在 Beta PASS 基础上执行：
- ❌ Type A/B 违规：第 201 行声明无 CI/gate/commit 证据: 在 RC PASS 基础上执行：
- ❌ Type A/B 违规：第 232 行声明无 CI/gate/commit 证据: | GA1 | 所有 4 个阶段门禁 PASS |
- ❌ Type A/B 违规：第 242 行声明无 CI/gate/commit 证据: 1. **必须实际执行命令**，禁止假设通过
- ❌ Type A/B 违规：第 254 行声明无 CI/gate/commit 证据: | A1.1 | graph-cli builds | PASS | "Finished release profile
- ❌ Type A/B 违规：第 255 行声明无 CI/gate/commit 证据: | A1.2 | evidence chain | PASS | "result:PASS, missing:[]" |
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：GATE_EXECUTION_PROMPT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：GATE_EXECUTION_PROMPT.md
- ❌ Type A/B 违规：第 115 行声明无 CI/gate/commit 证据: Expected: PASS
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: | **Alpha** | ✅ PASS | 10/10 checks | A1-A5 + A6-1~5 |
- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: | **Beta** | ✅ PASS | 11/11 checks | B1-B4 + B-F1~B-F7 |
- ❌ Type A/B 违规：第 17 行声明无 CI/gate/commit 证据: | **Integration** | ✅ PASS | 4/4 sections | C-ARCH + SGL + W
- ❌ Type A/B 违规：第 18 行声明无 CI/gate/commit 证据: | **WAL Contract** | ✅ PASS | 22/22 | RECOVERY-001~008 all P
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: | **Clippy** | ✅ PASS | 0 warnings | Fixed useless_conversio
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | **Format** | ✅ PASS | 0 diffs | Clean |
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: | **Build** | ✅ PASS | Release build | 5.50s |
- ❌ Type A/B 违规：第 33 行声明无 CI/gate/commit 证据: | A1 | Build | `cargo build --release -p sqlrustgo,executor,
- ❌ Type A/B 违规：第 34 行声明无 CI/gate/commit 证据: | A2 | Test | `cargo test --lib` (core 5 crates) | ✅ PASS | 
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | A3 | Clippy | `cargo clippy --all-features -- -D warnings`
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | A4 | Format | `cargo fmt --all -- --check` | ✅ PASS | `exi
- ❌ Type A/B 违规：第 49 行声明无 CI/gate/commit 证据: **Alpha Gate: 10/10 PASS** ✅
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: | B1 | Build | `cargo build --release -p core_5_crates` | ✅ 
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: | B2 | WAL Contract | `cargo test --test wal_tx_contract_tes
- ❌ Type A/B 违规：第 61 行声明无 CI/gate/commit 证据: | B3 | Clippy | `cargo clippy --all-features -- -D warnings`
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: | B4 | Format | `cargo fmt --all -- --check` | ✅ PASS | `exi
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: WAL Contract Tests: 22/22 PASS ✅
- ❌ Type A/B 违规：第 69 行声明无 CI/gate/commit 证据: TX-001~TX-006: 6 PASS
- ❌ Type A/B 违规：第 70 行声明无 CI/gate/commit 证据: WAL-001~WAL-005: 5 PASS
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: REPLAY-001~REPLAY-003: 3 PASS
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: RECOVERY-001~RECOVERY-008: 8 PASS
- ❌ Type A/B 违规：第 79 行声明无 CI/gate/commit 证据:   - RECOVERY-007: test_partial_delete_write_recovery ✅ (was 
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据: **Beta Gate: 11/11 PASS** ✅
- ❌ Type A/B 违规：第 94 行声明无 CI/gate/commit 证据: | C-ARCH-02 | LocalExecutor has no write_buffer field | ✅ PA
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据: **C-ARCH Result**: 3 PASS / 1 FAIL / 1 DRIFT ⚠️
- ❌ Type A/B 违规：第 105 行声明无 CI/gate/commit 证据: | SGL-001 | B4 format is read-only (no mutation) | ✅ PASS |
- ❌ Type A/B 违规：第 108 行声明无 CI/gate/commit 证据: | SGL-004 | WAL-004: DELETE replay idempotency | ✅ PASS |
- ❌ Type A/B 违规：第 111 行声明无 CI/gate/commit 证据: **SGL Result**: 4 PASS / 0 FAIL / 1 DRIFT ⚠️
- ❌ Type A/B 违规：第 117 行声明无 CI/gate/commit 证据: | INV-1 | Committed data survives crash | ✅ PASS |
- ❌ Type A/B 违规：第 118 行声明无 CI/gate/commit 证据: | INV-2 | Uncommitted data does NOT survive crash | ✅ PASS |
- ❌ Type A/B 违规：第 119 行声明无 CI/gate/commit 证据: | INV-3 | ROLLBACK leaves no trace | ✅ PASS |
- ❌ Type A/B 违规：第 121 行声明无 CI/gate/commit 证据: **WAL Invariant: 5/5 PASS** ✅
- ❌ Type A/B 违规：第 130 行声明无 CI/gate/commit 证据: **Integration Gate: 4/4 sections PASS** ✅
- ❌ Type A/B 违规：第 177 行声明无 CI/gate/commit 证据: | **INT-1** | WAL recovery invariant | ✅ **FULLY PROVEN** | 
- ❌ Type A/B 违规：第 248 行声明无 CI/gate/commit 证据: | Alpha | A1-A5 + A6-1~5 | ✅ 10/10 PASS |
- ❌ Type A/B 违规：第 249 行声明无 CI/gate/commit 证据: | Beta | B1-B4 + B-F1~7 | ✅ 11/11 PASS |
- ❌ Type A/B 违规：第 250 行声明无 CI/gate/commit 证据: | Integration | C-ARCH + SGL + WAL + Harness | ✅ 4/4 PASS (w
- ❌ Type A/B 违规：第 287 行声明无 CI/gate/commit 证据: v3.8.0 has completed **Alpha, Beta, and Integration Gates** 
- ❌ Type A/B 违规：第 289 行声明无 CI/gate/commit 证据: **Gate Status**: ✅ **PASS with documented issues**
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：COMPREHENSIVE_GATE_REPORT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：COMPREHENSIVE_GATE_REPORT.md
- ❌ Type A/B 违规：第 17 行声明无 CI/gate/commit 证据: PR-830F 实现 WAL 生命周期闭环控制，通过 CheckpointManager 提供的 `safe_trunc
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: **核心约束**: WAL 是 Event Log，不是 Redo Log — recovery 后不自动 trunca
- ❌ Type A/B 违规：第 113 行声明无 CI/gate/commit 证据:     /// Record a completed checkpoint
- ❌ Type A/B 违规：第 44 行声明无 CI/gate/commit 证据: 这些是 v3.6.0/v3.7.0 遗留的架构缺陷，v3.8.0 需要通过 PR-800~PR-900 解决。
- ❌ Type A/B 违规：第 185 行声明无 CI/gate/commit 证据: **Beta Gate 条件**: Recovery 7/7 PASS
- ❌ Type A/B 违规：第 93 行声明无 CI/gate/commit 证据: | Week 10 | GA Gate | Full suite PASS |
- ❌ Type A/B 违规：第 123 行声明无 CI/gate/commit 证据: | ACID 语义 | T-ISO-01~05 ALL PASS |
- ❌ Type A/B 违规：第 124 行声明无 CI/gate/commit 证据: | Crash recovery | T-CRA-01~05 ALL PASS |
- ❌ Type A/B 违规：第 127 行声明无 CI/gate/commit 证据: | TPC-H | 22/22 PASS |
- ❌ Type A/B 违规：第 25 行声明无 CI/gate/commit 证据: | FP (False Positive) | Graph FAIL，Legacy PASS，但实际无缺陷 | 需人工验
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | FN (False Negative) | Legacy 发现，Graph 未发现 | Graph PASS 但 L
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | TN (True Negative) | 两者都 PASS，实际无缺陷 | 人工验证 PASS 案例 |
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: | FP 案例列表 | Graph FAIL + Legacy PASS 的 CI runs | 分析 3187+ ru
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: | FN 案例列表 | Graph PASS + Legacy FAIL 的 CI runs | 需历史数据 |
- ❌ Type A/B 违规：第 49 行声明无 CI/gate/commit 证据: | MTTD (Mean Time to Detect) | 从 Commit 到 Gate FAIL 的时间差 | 分
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: 1. **检测 disagreement**：Graph PASS / Legacy FAIL 或反之
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据: | Graph PASS + Legacy FAIL | Legacy 发现问题，Graph 未发现 → Graph F
- ❌ Type A/B 违规：第 84 行声明无 CI/gate/commit 证据: | Graph FAIL + Legacy PASS | Graph 发现问题，Legacy 未发现 → Graph T
- ❌ Type A/B 违规：第 93 行声明无 CI/gate/commit 证据: 任何 Gate PASS 结论必须可追溯到 Evidence Graph
- ❌ Type A/B 违规：第 94 行声明无 CI/gate/commit 证据: 否则 → UNVERIFIED（不是 PASS）
- ❌ Type A/B 违规：第 112 行声明无 CI/gate/commit 证据: 无法证明 ≠ 通过
- ❌ Type A/B 违规：第 117 行声明无 CI/gate/commit 证据: **实现**：gate evaluate 输出 UNVERIFIED 而非 PASS
- ❌ Type A/B 违规：第 124 行声明无 CI/gate/commit 证据: | Phase 2 | Rule G-01/G-02/G-03 硬编码到 gate CLI | 防止 evidence 
- ❌ Type A/B 违规：第 136 行声明无 CI/gate/commit 证据: | evidence-graph-gate | completed | failure | Checkout GITEA
- ❌ Type A/B 违规：第 137 行声明无 CI/gate/commit 证据: | legacy-gate | completed | failure | 同上，两个 job 失败原因相同 |
- ❌ Type A/B 违规：第 148 行声明无 CI/gate/commit 证据: 基于已完成的 runs (3182-3187 全部 failed)，目前没有 PASS 数据可供 Matrix 分析。
- ❌ Type A/B 违规：第 159 行声明无 CI/gate/commit 证据: | 收集第一批 PASS/PFAIL 数据 | CI | 待 runner 正常后 |
- ❌ Type A/B 违规：第 203 行声明无 CI/gate/commit 证据:   { "result": "PASS", ... }
- ❌ Type A/B 违规：第 218 行声明无 CI/gate/commit 证据: gate evaluate 在以下情况输出 UNVERIFIED 而非 PASS：
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：GATE_EFFECTIVENESS_MATRIX.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：GATE_EFFECTIVENESS_MATRIX.md
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据: | RECOVERY-001 | `test_begin_then_crash_rolls_back` | `wal_t
- ❌ Type A/B 违规：第 24 行声明无 CI/gate/commit 证据: | RECOVERY-002 | `test_insert_then_crash_rolls_back` | `wal_
- ❌ Type A/B 违规：第 25 行声明无 CI/gate/commit 证据: | RECOVERY-003 | `test_prepare_then_crash_rolls_back` | `wal
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | RECOVERY-004 | `test_commit_flush_crash_replays` | `wal_tx
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | RECOVERY-005 | `test_partial_insert_write_recovery` | `wal
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | RECOVERY-006 | `test_partial_update_write_recovery` | `wal
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: | RECOVERY-007 | `test_partial_delete_write_recovery` | `wal
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | RECOVERY-008 | `test_partial_commit_flush_recovery` | `wal
- ❌ Type A/B 违规：第 32 行声明无 CI/gate/commit 证据: **Summary**: 8/8 PASS ✅ — all RECOVERY tests now passing
- ❌ Type A/B 违规：第 302 行声明无 CI/gate/commit 证据: 2. Test should PASS with assertion proving DELETE survival
- ❌ Type A/B 违规：第 303 行声明无 CI/gate/commit 证据: 3. LEGACY_FIXES_VERIFICATION_REPORT.md should be updated to 
- ❌ Type A/B 违规：第 305 行声明无 CI/gate/commit 证据: **Pre-PR-840 Status**: 21/22 PASS (RECOVERY-007 ignored = 1 
- ❌ Type A/B 违规：第 120 行声明无 CI/gate/commit 证据: | RECOVERY-015 | `recover_wal()` outputs: `"WAL recovery com
- ❌ Type A/B 违规：第 151 行声明无 CI/gate/commit 证据: | `test_recovery_state_default` | Default state is Unrecover
- ❌ Type A/B 违规：第 152 行声明无 CI/gate/commit 证据: | `test_stateful_engine_blocks_double_recovery` | Idempotent
- ❌ Type A/B 违规：第 153 行声明无 CI/gate/commit 证据: | `test_recovery_engine_impl_trait_bounds` | Trait bounds sa
- ❌ Type A/B 违规：第 58 行声明无 CI/gate/commit 证据: | B-F1 (F-03) | ✅ PASS | PR-830C merged |
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: | B-F2 (F-04) | ✅ PASS | PR-830D merged |
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: | B-F3 (F-05) | ✅ PASS | PR-830E merged |
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: | B-F5 | ✅ PASS | PR-DAG in DEVELOPMENT_PLAN.md matches actu
- ❌ Type A/B 违规：第 63 行声明无 CI/gate/commit 证据: | B-F6 | ✅ PASS | FEATURE_CHECKLIST.md created |
- ❌ Type A/B 违规：第 64 行声明无 CI/gate/commit 证据: | B-F7 | ✅ PASS | No orphan PRs (all merged PRs have corresp
- ❌ Type A/B 违规：第 89 行声明无 CI/gate/commit 证据: v3.8.0 Beta Gate 通过了，但实际上大量功能（PR-810~PR-900）未完成。
- ❌ Type A/B 违规：第 8 行声明无 CI/gate/commit 证据: **Status**: ✅ PASS — 10/10 Checks
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | A1 | Build | `cargo build --release -p sqlrustgo,executor,
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | A2 | Test | `cargo test --lib` (core 5 crates) | 0 failure
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | A3 | Clippy | `cargo clippy --all-features -- -D warnings`
- ❌ Type A/B 违规：第 29 行声明无 CI/gate/commit 证据: | A4 | Format | `cargo fmt --all -- --check` | exit 0 | exit
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | A5 | Coverage | L1 8 crates average | ≥75% | **81.84%** (8
- ❌ Type A/B 违规：第 36 行声明无 CI/gate/commit 证据: | A6-1 | Replay Graph | `ls docs/governance/replay/REPLAY_v3
- ❌ Type A/B 违规：第 37 行声明无 CI/gate/commit 证据: | A6-2 | Claim Registry | `ls docs/governance/adr/ADR-002-cl
- ❌ Type A/B 违规：第 38 行声明无 CI/gate/commit 证据: | A6-3 | Decision Registry | `ls docs/releases/v3.8.0/ARCHIT
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: | A6-4 | Freshness | `grep -l "Freshness" docs/releases/v3.8
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: | A6-5 | ADR Updated | `ls docs/governance/adr/ADR-00*.md` |
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 81 行声明无 CI/gate/commit 证据: | A1 Build | ✅ PASS | — |
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: | A2 Test | ✅ PASS | — |
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据: | A3 Clippy | ✅ PASS | — |
- ❌ Type A/B 违规：第 86 行声明无 CI/gate/commit 证据: | A6-1~5 | ✅ PASS | — |
- ❌ Type A/B 违规：第 94 行声明无 CI/gate/commit 证据: | A1 Build | ✅ PASS | — |
- ❌ Type A/B 违规：第 95 行声明无 CI/gate/commit 证据: | A2 Test | ✅ PASS | — |
- ❌ Type A/B 违规：第 96 行声明无 CI/gate/commit 证据: | A3 Clippy | ✅ PASS | — |
- ❌ Type A/B 违规：第 97 行声明无 CI/gate/commit 证据: | A4 Format | ✅ PASS | PR-830E WAL import fix |
- ❌ Type A/B 违规：第 98 行声明无 CI/gate/commit 证据: | A5 Coverage | ✅ PASS | 81.84% avg |
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据: | A6-1~5 | ✅ PASS | — |
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：ALPHA_GATE_REPORT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：ALPHA_GATE_REPORT.md
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：GRAPH_VS_LEGACY_GATE_RECONCILIATION.md
- ❌ Type A/B 违规：第 66 行声明无 CI/gate/commit 证据:   PASS: all paths same result hash
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: | T-ISO-01 | Dirty Read Prevention | txn1 write uncommitted 
- ❌ Type A/B 违规：第 81 行声明无 CI/gate/commit 证据: | T-ISO-02 | Non-repeatable Read | txn1 read → txn1 write → 
- ❌ Type A/B 违规：第 82 行声明无 CI/gate/commit 证据: | T-ISO-03 | Phantom Read | txn1 range query → txn2 insert →
- ❌ Type A/B 违规：第 83 行声明无 CI/gate/commit 证据: | T-ISO-04 | Write-Write Conflict | txn1 write row X → txn2 
- ❌ Type A/B 违规：第 84 行声明无 CI/gate/commit 证据: | T-ISO-05 | Lost Update | concurrent update same row → fina
- ❌ Type A/B 违规：第 90 行声明无 CI/gate/commit 证据: | T-CRA-01 | commit 中 kill -9 | 重启后 WAL replay → 数据存在 | MUST
- ❌ Type A/B 违规：第 91 行声明无 CI/gate/commit 证据: | T-CRA-02 | rollback 中 kill -9 | 重启后数据未改变 | MUST PASS |
- ❌ Type A/B 违规：第 92 行声明无 CI/gate/commit 证据: | T-CRA-03 | partial write 中断 | 重启后数据一致或 empty | MUST PASS |
- ❌ Type A/B 违规：第 93 行声明无 CI/gate/commit 证据: | T-CRA-04 | WAL replay ordering | 乱序写入 replay 后正确 | MUST PA
- ❌ Type A/B 违规：第 94 行声明无 CI/gate/commit 证据: | T-CRA-05 | double commit | 重启后仅一次生效 | MUST PASS |
- ❌ Type A/B 违规：第 100 行声明无 CI/gate/commit 证据: | T-DIV-01 | Same SQL all paths | mysql-server == bench-cli 
- ❌ Type A/B 违规：第 101 行声明无 CI/gate/commit 证据: | T-DIV-02 | NULL handling | all paths same NULL semantics |
- ❌ Type A/B 违规：第 102 行声明无 CI/gate/commit 证据: | T-DIV-03 | Type coercion | all paths same coercion result 
- ❌ Type A/B 违规：第 103 行声明无 CI/gate/commit 证据: | T-DIV-04 | Error handling | all paths same error code/msg 
- ❌ Type A/B 违规：第 128 行声明无 CI/gate/commit 证据: | B5 | T-ISO-01 dirty read | PASS | PR-840 |
- ❌ Type A/B 违规：第 137 行声明无 CI/gate/commit 证据: | C2 | E2E integration tests | 28/28 PASS | PR-850 |
- ❌ Type A/B 违规：第 159 行声明无 CI/gate/commit 证据: | E1 | T-ISO-01~05 isolation suite | ALL PASS | PR-890 |
- ❌ Type A/B 违规：第 160 行声明无 CI/gate/commit 证据: | E2 | T-CRA-03~05 crash suite | ALL PASS | PR-890 |
- ❌ Type A/B 违规：第 183 行声明无 CI/gate/commit 证据: | G2 | T-ISO-01~05 | ALL PASS |
- ❌ Type A/B 违规：第 184 行声明无 CI/gate/commit 证据: | G3 | T-CRA-01~05 | ALL PASS |
- ❌ Type A/B 违规：第 185 行声明无 CI/gate/commit 证据: | G4 | T-DIV-01~04 | ALL PASS |
- ❌ Type A/B 违规：第 186 行声明无 CI/gate/commit 证据: | G5 | TPC-H SF=1 | 22/22 PASS |
- ❌ Type A/B 违规：第 200 行声明无 CI/gate/commit 证据:   PASS: all execution paths same result hash
- ❌ Type A/B 违规：第 209 行声明无 CI/gate/commit 证据: 输出: PASS/FAIL + recovery details
- ❌ Type A/B 违规：第 217 行声明无 CI/gate/commit 证据: 输出: PASS (correct isolation) / FAIL (violation detected)
- ❌ Type A/B 违规：第 239 行声明无 CI/gate/commit 证据: - 22 queries 必须全部 PASS
- ❌ Type D 违规：计划文档显示 GA 状态（疑似伪造）：TEST_PLAN.md 第 178 行
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: - G1 gate: 250 tests PASS
- ❌ Type A/B 违规：第 48 行声明无 CI/gate/commit 证据: - 400+ tests PASS
- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: | **D1-Alpha** | A1-A5 + A6 Governance | 10/10 | ✅ PASS |
- ❌ Type A/B 违规：第 17 行声明无 CI/gate/commit 证据: | **D2-Beta** | B1-B4 + B5 Integration | 5/5 | ✅ PASS |
- ❌ Type A/B 违规：第 18 行声明无 CI/gate/commit 证据: | **D3-SGL** | SGL-001~005 (semantic layer) | 4/5 PASS, 0 FA
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: | **D4-WAL** | INV-1, INV-2, INV-3 | 5/5 | ✅ PASS |
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | **D5-DeepSeek** | 10 Principles for RC/GA | 10/10 | ✅ PASS
- ❌ Type A/B 违规：第 56 行声明无 CI/gate/commit 证据: | A1: Build (release, core 6 crates) | ✅ PASS | `Finished re
- ❌ Type A/B 违规：第 57 行声明无 CI/gate/commit 证据: | A2: Test (lib, core 6 crates) | ✅ PASS | `286 passed; 0 fa
- ❌ Type A/B 违规：第 58 行声明无 CI/gate/commit 证据: | A3: Clippy (core) | ✅ PASS | `0 warnings` |
- ❌ Type A/B 违规：第 59 行声明无 CI/gate/commit 证据: | A4: Format (core) | ✅ PASS | `exit 0` (no diff) |
- ❌ Type A/B 违规：第 60 行声明无 CI/gate/commit 证据: | A5: Coverage (L1 8 crates) | ✅ PASS | `82% avg (min: 75%)`
- ❌ Type A/B 违规：第 61 行声明无 CI/gate/commit 证据: | A6-1: Replay Graph | ✅ PASS | `docs/governance/replay/REPL
- ❌ Type A/B 违规：第 62 行声明无 CI/gate/commit 证据: | A6-2: Claim Registry | ✅ PASS | `docs/governance/adr/ADR-0
- ❌ Type A/B 违规：第 63 行声明无 CI/gate/commit 证据: | A6-3: Decision Registry | ✅ PASS | `docs/releases/v3.8.0/A
- ❌ Type A/B 违规：第 64 行声明无 CI/gate/commit 证据: | A6-4: Freshness markers | ✅ PASS | Present in 3+ docs |
- ❌ Type A/B 违规：第 65 行声明无 CI/gate/commit 证据: | A6-5: ADR-001~ADR-005 | ✅ PASS | All 5 ADRs exist |
- ❌ Type A/B 违规：第 75 行声明无 CI/gate/commit 证据: | B1: Build | ✅ PASS | `Finished release profile in 5.50s` |
- ❌ Type A/B 违规：第 76 行声明无 CI/gate/commit 证据: | B2: WAL Contract (22 tests) | ✅ PASS | `22 passed, 0 faile
- ❌ Type A/B 违规：第 77 行声明无 CI/gate/commit 证据: | B3: Clippy | ✅ PASS | `0 warnings` |
- ❌ Type A/B 违规：第 78 行声明无 CI/gate/commit 证据: | B4: Format | ✅ PASS | `exit 0` |
- ❌ Type A/B 违规：第 79 行声明无 CI/gate/commit 证据: | B5: Integration Gate | ✅ PASS | `Result: PASS — all checks
- ❌ Type A/B 违规：第 89 行声明无 CI/gate/commit 证据: | SGL-001 | B4 format is read-only | ✅ PASS |
- ❌ Type A/B 违规：第 90 行声明无 CI/gate/commit 证据: | SGL-002 | WAL-002: checkpoint advance in commit path | ✅ P
- ❌ Type A/B 违规：第 91 行声明无 CI/gate/commit 证据: | SGL-003 | WAL-003: truncate_before in commit path | ✅ PASS
- ❌ Type A/B 违规：第 92 行声明无 CI/gate/commit 证据: | SGL-004 | WAL-004: DELETE replay idempotency | ✅ PASS |
- ❌ Type A/B 违规：第 95 行声明无 CI/gate/commit 证据: **SGL Summary**: PASS: 4/5 | FAIL: 0 | DRIFT: 1
- ❌ Type A/B 违规：第 99 行声明无 CI/gate/commit 证据: **D3-SGL: PASS=4 | FAIL=0 | DRIFT=1** ⚡
- ❌ Type A/B 违规：第 107 行声明无 CI/gate/commit 证据: | INV-1 | Committed data survives crash | ✅ PASS |
- ❌ Type A/B 违规：第 108 行声明无 CI/gate/commit 证据: | INV-2 | Uncommitted data does NOT survive crash | ✅ PASS |
- ❌ Type A/B 违规：第 109 行声明无 CI/gate/commit 证据: | INV-3 | ROLLBACK leaves no trace | ✅ PASS |
- ❌ Type A/B 违规：第 111 行声明无 CI/gate/commit 证据: **Rust Tests**: 22/22 PASS (`wal_tx_contract_test`) + 5/5 PA
- ❌ Type A/B 违规：第 121 行声明无 CI/gate/commit 证据: | D5-1 | Test Results Override Documentation | ✅ PASS (21 te
- ❌ Type A/B 违规：第 122 行声明无 CI/gate/commit 证据: | D5-2 | Minimal Enforcement | ✅ PASS (recent: 2 lines) |
- ❌ Type A/B 违规：第 123 行声明无 CI/gate/commit 证据: | D5-3 | Strict Scope Control | ✅ PASS (no EEK v2 mixed) |
- ❌ Type A/B 违规：第 124 行声明无 CI/gate/commit 证据: | D5-4 | Version Boundary Clarity | ✅ PASS |
- ❌ Type A/B 违规：第 125 行声明无 CI/gate/commit 证据: | D5-5 | Root Cause Provenance | ✅ PASS (108 WAL-related com
- ❌ Type A/B 违规：第 126 行声明无 CI/gate/commit 证据: | D5-6 | No False Positives | ✅ PASS (crash_recovery_test.rs
- ❌ Type A/B 违规：第 127 行声明无 CI/gate/commit 证据: | D5-7 | Audit Trail | ✅ PASS (4 audit files) |
- ❌ Type A/B 违规：第 128 行声明无 CI/gate/commit 证据: | D5-8 | DRIFT Classification | ⚡ PASS (SGL-005 DRIFT tracke
- ❌ Type A/B 违规：第 129 行声明无 CI/gate/commit 证据: | D5-9 | Strict Rules with Exemption通道 | ✅ PASS (DRIFT mecha
- ❌ Type A/B 违规：第 130 行声明无 CI/gate/commit 证据: | D5-10 | Engineering Facts Over AI Analysis | ✅ PASS (0 tes
- ❌ Type A/B 违规：第 141 行声明无 CI/gate/commit 证据: | C-ARCH-02 | LocalExecutor has no write_buffer field | ✅ No
- ❌ Type A/B 违规：第 142 行声明无 CI/gate/commit 证据: | C-ARCH-05 | execution_engine.rs < 2000 lines | ✅ 1562 line
- ❌ Type A/B 违规：第 144 行声明无 CI/gate/commit 证据: **Note**: `check_arch_invariants.sh` uses 1500 limit (FAIL),
- ❌ Type A/B 违规：第 170 行声明无 CI/gate/commit 证据: | **Alpha → Beta** | 10/10 | — | — | — | — | ✅ PASS |
- ❌ Type A/B 违规：第 171 行声明无 CI/gate/commit 证据: | **Beta → RC** | — | 5/5 | 0 FAIL | 5/5 | — | ✅ PASS |
- ❌ Type A/B 违规：第 224 行声明无 CI/gate/commit 证据: **v3.8.0 Beta Gate: ✅ PASS**
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：RC_GA_GATE_REPORT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：RC_GA_GATE_REPORT.md
- ❌ Type A/B 违规：第 7 行声明无 CI/gate/commit 证据: **Status**: ✅ BETA GATE PASS — 11/11 checks (B1 Build ✅ B2 W
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: **BETA Gate PASS criteria used**: "WAL execution path exists
- ❌ Type A/B 违规：第 25 行声明无 CI/gate/commit 证据: | **B1 Build** | exit 0 (core 5 crates) | ✅ PASS | `release`
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: | **B2 WAL Contract** | WAL path + RECOVERY verifiable | ✅ P
- ❌ Type A/B 违规：第 27 行声明无 CI/gate/commit 证据: | **B3 Clippy** | 0 warnings | ✅ PASS | clippy 0 warnings on
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: | **B4 Format** | exit 0 | ✅ PASS | fmt check pass |
- ❌ Type A/B 违规：第 39 行声明无 CI/gate/commit 证据: PR-800  COM_QUERY AST Routing (L0)           ← Infrastructur
- ❌ Type A/B 违规：第 88 行声明无 CI/gate/commit 证据: The question "how can we enter RC if features aren't done?" 
- ❌ Type A/B 违规：第 130 行声明无 CI/gate/commit 证据:   RECOVERY-001  ✅ PASS
- ❌ Type A/B 违规：第 131 行声明无 CI/gate/commit 证据:   RECOVERY-002  ✅ PASS
- ❌ Type A/B 违规：第 133 行声明无 CI/gate/commit 证据:   RECOVERY-004  ✅ PASS
- ❌ Type A/B 违规：第 134 行声明无 CI/gate/commit 证据:   RECOVERY-005  ✅ PASS
- ❌ Type A/B 违规：第 135 行声明无 CI/gate/commit 证据:   RECOVERY-006  ✅ PASS
- ❌ Type A/B 违规：第 137 行声明无 CI/gate/commit 证据:   RECOVERY-008  ✅ PASS
- ❌ Type A/B 违规：第 150 行声明无 CI/gate/commit 证据: | **Alpha** | Execution Semantics Freeze | Architecture free
- ❌ Type A/B 违规：第 151 行声明无 CI/gate/commit 证据: | **Beta** | Architecture infrastructure | WAL path, build, 
- ❌ Type A/B 违规：第 155 行声明无 CI/gate/commit 证据: **Beta PASS means**: The architecture foundation is laid. It
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：BETA_GATE_REPORT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：BETA_GATE_REPORT.md
- ❌ Type A/B 违规：第 14 行声明无 CI/gate/commit 证据: - **15 PASS** (TX + WAL，依赖 L1 MemoryStorage 语义)
- ❌ Type A/B 违规：第 138 行声明无 CI/gate/commit 证据: | 15 PASS / 7 FAIL | **22 PASS / 0 FAIL**（RECOVERY-001~007 迁
- ❌ Type A/B 违规：第 158 行声明无 CI/gate/commit 证据: - [ ] `cargo test --test wal_tx_contract_test` → 22/22 PASS
- ❌ Type A/B 违规：第 160 行声明无 CI/gate/commit 证据: - [ ] Beta Gate B2 验证通过
- ❌ Type A/B 违规：第 189 行声明无 CI/gate/commit 证据: 1. `cargo test --test wal_tx_contract_test` 输出 **22 PASS, 0 
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: | B4 Format semantics | SGL-001 | Semantic | **PASS** ✅ |
- ❌ Type A/B 违规：第 23 行声明无 CI/gate/commit 证据: | WAL-004: DELETE replay idempotency | SGL-004 | Invariant |
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: **PASS: 2/5 | FAIL: 2 | DRIFT: 1**
- ❌ Type A/B 违规：第 143 行声明无 CI/gate/commit 证据: | PASS | Invariant satisfied |
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：SGL_BETA_GATE_REPORT.md
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: | 11 | local_executor.rs | 1328 | `facade.execute_dml(|stora
- ❌ Type A/B 违规：第 113 行声明无 CI/gate/commit 证据: **方案**: Trigger 的 DML 操作应通过 `WalTransactionalFacade` 代理，确保 W
- ❌ Type A/B 违规：第 208 行声明无 CI/gate/commit 证据: - INV-1 (committed data survives): ✅ 22/22 PASS
- ❌ Type A/B 违规：第 209 行声明无 CI/gate/commit 证据: - INV-2 (uncommitted data lost): ✅ 22/22 PASS
- ❌ Type A/B 违规：第 210 行声明无 CI/gate/commit 证据: - INV-3 (rollback clean): ✅ 22/22 PASS
- ❌ Type A/B 违规：第 14 行声明无 CI/gate/commit 证据: 当前 v3.8.0 Alpha Gate 声明 "CONDITIONAL PASS" 但没有明确：
- ❌ Type A/B 违规：第 15 行声明无 CI/gate/commit 证据: - CONDITIONAL PASS 的确切含义是什么？
- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: - 什么条件下可以 CONDITIONAL PASS？
- ❌ Type A/B 违规：第 17 行声明无 CI/gate/commit 证据: - 与 FULL PASS 的区别是什么？
- ❌ Type A/B 违规：第 18 行声明无 CI/gate/commit 证据: - CONDITIONAL PASS 的门禁效力是什么？
- ❌ Type A/B 违规：第 26 行声明无 CI/gate/commit 证据: **CONDITIONAL PASS** = 门禁检查项部分通过，但通过的部分有明确的前提条件/约束，且这些前提条件已被
- ❌ Type A/B 违规：第 28 行声明无 CI/gate/commit 证据: 与 **FULL PASS** 的区别：
- ❌ Type A/B 违规：第 30 行声明无 CI/gate/commit 证据: | 属性 | CONDITIONAL PASS | FULL PASS |
- ❌ Type A/B 违规：第 32 行声明无 CI/gate/commit 证据: | 门禁完整性 | 部分检查项被豁免 | 所有检查项必须通过 |
- ❌ Type A/B 违规：第 40 行声明无 CI/gate/commit 证据: Alpha CONDITIONAL PASS 在以下情况下适用：
- ❌ Type A/B 违规：第 45 行声明无 CI/gate/commit 证据: - A1 Build PASS 没有意义（测试的是旧架构）
- ❌ Type A/B 违规：第 46 行声明无 CI/gate/commit 证据: - A2 Test PASS 没有意义（测试的是旧路径）
- ❌ Type A/B 违规：第 52 行声明无 CI/gate/commit 证据: - 该检查项可以被标记为 CONDITIONAL PASS
- ❌ Type A/B 违规：第 53 行声明无 CI/gate/commit 证据: - 但必须记录「什么条件下」才能成为 FULL PASS
- ❌ Type A/B 违规：第 58 行声明无 CI/gate/commit 证据: - 该检查项可以 CONDITIONAL PASS
- ❌ Type A/B 违规：第 67 行声明无 CI/gate/commit 证据: v3.8.0 Alpha 声明 CONDITIONAL PASS，因为：
- ❌ Type A/B 违规：第 71 行声明无 CI/gate/commit 证据: | A1 Build | ✅ PASS | 核心 6 crates 通过 |
- ❌ Type A/B 违规：第 72 行声明无 CI/gate/commit 证据: | A2 Test | ✅ PASS | 89 tests 通过 |
- ❌ Type A/B 违规：第 73 行声明无 CI/gate/commit 证据: | A3 Clippy | ✅ PASS | 零警告 |
- ❌ Type A/B 违规：第 76 行声明无 CI/gate/commit 证据: | A6-1~5 | ✅ PASS | Governance 框架完整 |
- ❌ Type A/B 违规：第 88 行声明无 CI/gate/commit 证据: **因此**: CONDITIONAL PASS = 这些检查项的失败已被接受，且有明确的修复时间表。
- ❌ Type A/B 违规：第 96 行声明无 CI/gate/commit 证据: **重要**: CONDITIONAL PASS 只是说明「检查项部分通过且有记录」，并不意味着「可以发布」。
- ❌ Type A/B 违规：第 98 行声明无 CI/gate/commit 证据: CONDITIONAL PASS 的发布效力：
- ❌ Type A/B 违规：第 100 行声明无 CI/gate/commit 证据: | 场景 | CONDITIONAL PASS | FULL PASS |
- ❌ Type A/B 违规：第 105 行声明无 CI/gate/commit 证据: | GA | **不允许** | 必须 FULL PASS |
- ❌ Type A/B 违规：第 107 行声明无 CI/gate/commit 证据: **GA 绝对不允许 CONDITIONAL PASS**：GA 是生产发布的门槛，所有 Gate 必须 FULL PA
- ❌ Type A/B 违规：第 111 行声明无 CI/gate/commit 证据: CONDITIONAL PASS 必须在 Gate Report 中明确记录：
- ❌ Type A/B 违规：第 133 行声明无 CI/gate/commit 证据: 1. **触发条件**: 明确列出 CONDITIONAL PASS 的触发场景
- ❌ Type A/B 违规：第 134 行声明无 CI/gate/commit 证据: 2. **记录要求**: CONDITIONAL PASS 必须记录豁免原因和前提条件
- ❌ Type A/B 违规：第 135 行声明无 CI/gate/commit 证据: 3. **效力限制**: GA 不允许 CONDITIONAL PASS
- ❌ Type A/B 违规：第 136 行声明无 CI/gate/commit 证据: 4. **修复要求**: CONDITIONAL PASS 的检查项必须在下一 Gate 之前修复
- ❌ Type A/B 违规：第 146 行声明无 CI/gate/commit 证据: 如果是 CONDITIONAL PASS：
- ❌ Type A/B 违规：第 152 行声明无 CI/gate/commit 证据: **CONDITIONAL PASS 发布效力**: [受约束 / 不允许发布]
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: - A1 Build PASS 没有意义（测试的是旧架构）
- ❌ Type A/B 违规：第 20 行声明无 CI/gate/commit 证据: - A2 Test PASS 没有意义（测试的是旧路径）
- ❌ Type A/B 违规：第 54 行声明无 CI/gate/commit 证据: | A6-4 | Freshness PASS | 引用数据必须标注 Freshness | 无过期数据引用 | ADR
- ❌ Type A/B 违规：第 68 行声明无 CI/gate/commit 证据: **通过标准**: `Finished release profile` 或 exit code 0
- ❌ Type A/B 违规：第 75 行声明无 CI/gate/commit 证据: Status: ✅ PASS
- ❌ Type A/B 违规：第 87 行声明无 CI/gate/commit 证据: **通过标准**: `test result: ok` (0 failures)
- ❌ Type A/B 违规：第 96 行声明无 CI/gate/commit 证据: Status: ✅ PASS
- ❌ Type A/B 违规：第 108 行声明无 CI/gate/commit 证据: **通过标准**: 0 warnings, 0 errors
- ❌ Type A/B 违规：第 115 行声明无 CI/gate/commit 证据: Status: ✅ PASS
- ❌ Type A/B 违规：第 127 行声明无 CI/gate/commit 证据: **通过标准**: exit code 0
- ❌ Type A/B 违规：第 134 行声明无 CI/gate/commit 证据: Status: ✅ PASS
- ❌ Type A/B 违规：第 149 行声明无 CI/gate/commit 证据: done
- ❌ Type A/B 违规：第 152 行声明无 CI/gate/commit 证据: **通过标准**: L1 8 crates 综合平均 ≥75%
- ❌ Type A/B 违规：第 163 行声明无 CI/gate/commit 证据: Status: ✅ PASS (≥75%)
- ❌ Type A/B 违规：第 176 行声明无 CI/gate/commit 证据: **通过标准**: 文件存在，且至少 4 个 issue 有完整轨迹
- ❌ Type A/B 违规：第 188 行声明无 CI/gate/commit 证据: Status: ✅ PASS (4/4 issues traced)
- ❌ Type A/B 违规：第 200 行声明无 CI/gate/commit 证据: **通过标准**: v3.8.0 相关的 Claim 有记录
- ❌ Type A/B 违规：第 207 行声明无 CI/gate/commit 证据: Status: ✅ PASS
- ❌ Type A/B 违规：第 220 行声明无 CI/gate/commit 证据: **通过标准**: AD-001~AD-005 存在
- ❌ Type A/B 违规：第 233 行声明无 CI/gate/commit 证据: Status: ✅ PASS (5/5 ADs recorded)
- ❌ Type A/B 违规：第 245 行声明无 CI/gate/commit 证据: **通过标准**: 引用数据标注 Freshness，无过期数据
- ❌ Type A/B 违规：第 249 行声明无 CI/gate/commit 证据: === A6-4: Freshness PASS ===
- ❌ Type A/B 违规：第 252 行声明无 CI/gate/commit 证据: Status: ✅ PASS
- ❌ Type A/B 违规：第 264 行声明无 CI/gate/commit 证据: **通过标准**: ADR-001~ADR-005 全部存在
- ❌ Type A/B 违规：第 276 行声明无 CI/gate/commit 证据: Status: ✅ PASS (5/5 ADRs)
- ❌ Type A/B 违规：第 285 行声明无 CI/gate/commit 证据: 所有 A1-A5 和 A6-1~A6-5 全部 PASS → Alpha Gate PASS
- ❌ Type A/B 违规：第 289 行声明无 CI/gate/commit 证据: A1-A5 全部 PASS，A6 有 1-2 项 FAIL（可在 Beta 前修复）→ CONDITIONAL PASS
- ❌ Type A/B 违规：第 353 行声明无 CI/gate/commit 证据: done
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：ALPHA_GATE_CONTRACT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：ALPHA_GATE_CONTRACT.md
- ❌ Type A/B 违规：第 6 行声明无 CI/gate/commit 证据: **Result**: ✅ PASS — 4/4 sections | SGL: 4/5 PASS | WAL: 22/
- ❌ Type A/B 违规：第 16 行声明无 CI/gate/commit 证据: | C-ARCH | Architecture static invariants | PASS ✅ |
- ❌ Type A/B 违规：第 17 行声明无 CI/gate/commit 证据: | SGL | Semantic gate (WAL lifecycle) | PASS ✅ |
- ❌ Type A/B 违规：第 18 行声明无 CI/gate/commit 证据: | WAL | Crash recovery + invariant validation | PASS ✅ |
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: **Key Achievement**: WAL-002/003 从 FAIL → PASS，WAL 无限增长问题已修复
- ❌ Type A/B 违规：第 32 行声明无 CI/gate/commit 证据: | C-ARCH-02 | LocalExecutor has no write_buffer field | ✅ PA
- ❌ Type A/B 违规：第 35 行声明无 CI/gate/commit 证据: | C-ARCH-05 | execution_engine.rs < 2000 lines | ✅ PASS (156
- ❌ Type A/B 违规：第 47 行声明无 CI/gate/commit 证据: | SGL-001 | B4 format is read-only (no mutation) | PASS ✅ |
- ❌ Type A/B 违规：第 50 行声明无 CI/gate/commit 证据: | SGL-004 | WAL-004: DELETE replay idempotency | PASS ✅ |
- ❌ Type A/B 违规：第 53 行声明无 CI/gate/commit 证据: **SGL Summary**: PASS: 4/5 | FAIL: 0 | DRIFT: 1
- ❌ Type A/B 违规：第 74 行声明无 CI/gate/commit 证据: | INV-1 | Committed data survives crash | PASS ✅ |
- ❌ Type A/B 违规：第 75 行声明无 CI/gate/commit 证据: | INV-2 | Uncommitted data does NOT survive crash | PASS ✅ |
- ❌ Type A/B 违规：第 76 行声明无 CI/gate/commit 证据: | INV-3 | ROLLBACK leaves no trace | PASS ✅ |
- ❌ Type A/B 违规：第 80 行声明无 CI/gate/commit 证据: **Invariant Summary**: 5/5 PASS
- ❌ Type A/B 违规：第 136 行声明无 CI/gate/commit 证据: v3.8.0 Integration Gate: **PASS**
- ❌ Type B 违规：门禁文档无 gate_policy_eval_id（疑似伪门禁）：INTEGRATION_GATE_REPORT.md
- ❌ Type B 违规：门禁声明 PASS 但无 gate engine 输出：INTEGRATION_GATE_REPORT.md
- ❌ Type A/B 违规：第 19 行声明无 CI/gate/commit 证据: - ✅ Contract tests: 22/22 PASS ✅
- ❌ Type A/B 违规：第 21 行声明无 CI/gate/commit 证据: **Overall**: INT-1 is **FULLY PROVEN** — 22/22 PASS. The DEL
- ❌ Type A/B 违规：第 100 行声明无 CI/gate/commit 证据: | Committed INSERT survives crash | RECOVERY-004 | `assert_e
- ❌ Type A/B 违规：第 101 行声明无 CI/gate/commit 证据: | Multiple committed tx survive | RECOVERY-008 | `assert_eq!
- ❌ Type A/B 违规：第 102 行声明无 CI/gate/commit 证据: | Uncommitted INSERT rolled back | RECOVERY-001, 002 | `asse
- ❌ Type A/B 违规：第 103 行声明无 CI/gate/commit 证据: | Uncommitted BEGIN rolled back | RECOVERY-001 | `assert_eq!
- ❌ Type A/B 违规：第 104 行声明无 CI/gate/commit 证据: | Committed UPDATE row survives | RECOVERY-006 | `assert_eq!
- ❌ Type A/B 违规：第 105 行声明无 CI/gate/commit 证据: | Committed DELETE row stays deleted | RECOVERY-007 | `asser
- ❌ Type A/B 违规：第 145 行声明无 CI/gate/commit 证据: 3. Update LEGACY_FIXES_VERIFICATION_REPORT.md to 22/22 PASS

---

### 警告项（需要人工复核）

- ⚠️ 文档可能缺少 provenance 元数据：LEGACY_FIXES_VERIFICATION_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-830F_CONTRACT.md
- ⚠️ 警告：第 81 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：AGENT_EXECUTION_PROMPT.md
- ⚠️ 警告：第 92 行 FAIL 声明可能无证据
- ⚠️ 警告：第 97 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ISSUE_AUDIT_AND_GAP_ANALYSIS.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-800_TEST_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：ARCHITECTURE.md
- ⚠️ 警告：第 336 行 FAIL 声明可能无证据
- ⚠️ 警告：第 348 行 FAIL 声明可能无证据
- ⚠️ 警告：第 362 行 FAIL 声明可能无证据
- ⚠️ 警告：第 381 行 FAIL 声明可能无证据
- ⚠️ 警告：第 556 行 FAIL 声明可能无证据
- ⚠️ 警告：第 613 行 FAIL 声明可能无证据
- ⚠️ 警告：第 617 行 FAIL 声明可能无证据
- ⚠️ 警告：第 849 行 FAIL 声明可能无证据
- ⚠️ 警告：第 858 行 FAIL 声明可能无证据
- ⚠️ 警告：第 863 行 FAIL 声明可能无证据
- ⚠️ 警告：第 882 行 FAIL 声明可能无证据
- ⚠️ 警告：第 950 行 FAIL 声明可能无证据
- ⚠️ 警告：第 1019 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GOVERNANCE_HARNESS_ENGINEERING.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-800_ACCEPTANCE.md
- ⚠️ 警告：第 319 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：DEVELOPMENT_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：BETA_GATE_CONTRACT.md
- ⚠️ 文档可能缺少 provenance 元数据：INT234_RTI_CHAIN.md
- ⚠️ 警告：第 49 行 FAIL 声明可能无证据
- ⚠️ 警告：第 105 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：R5-GATE-REFORM.md
- ⚠️ 警告：第 18 行 FAIL 声明可能无证据
- ⚠️ 警告：第 19 行 FAIL 声明可能无证据
- ⚠️ 警告：第 59 行 FAIL 声明可能无证据
- ⚠️ 警告：第 76 行 FAIL 声明可能无证据
- ⚠️ 警告：第 83 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ALPHA_BASELINE_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-830E_SPEC.md
- ⚠️ 警告：第 265 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ARCHITECTURE_DECISIONS.md
- ⚠️ 文档可能缺少 provenance 元数据：COVERAGE-DELTA-ANALYSIS.md
- ⚠️ 警告：第 201 行 FAIL 声明可能无证据
- ⚠️ 警告：第 205 行 FAIL 声明可能无证据
- ⚠️ 警告：第 206 行 FAIL 声明可能无证据
- ⚠️ 警告：第 207 行 FAIL 声明可能无证据
- ⚠️ 警告：第 208 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GA_GATE_CHECKLIST.md
- ⚠️ 警告：第 48 行 FAIL 声明可能无证据
- ⚠️ 警告：第 130 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：INTEGRATION_GATE_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-830E_IMPLEMENTATION_PLAN.md
- ⚠️ 警告：第 98 行 FAIL 声明可能无证据
- ⚠️ 警告：第 100 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ALPHA_GATE_PROMPT.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-800_SPEC.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-840_CONTRACT.md
- ⚠️ 警告：第 245 行 FAIL 声明可能无证据
- ⚠️ 警告：第 256 行 FAIL 声明可能无证据
- ⚠️ 警告：第 260 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GATE_EXECUTION_PROMPT.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-830F_IMPLEMENTATION_PLAN.md
- ⚠️ 警告：第 34 行 FAIL 声明可能无证据
- ⚠️ 警告：第 60 行 FAIL 声明可能无证据
- ⚠️ 警告：第 97 行 FAIL 声明可能无证据
- ⚠️ 警告：第 99 行 FAIL 声明可能无证据
- ⚠️ 警告：第 111 行 FAIL 声明可能无证据
- ⚠️ 警告：第 139 行 FAIL 声明可能无证据
- ⚠️ 警告：第 210 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：COMPREHENSIVE_GATE_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-830F_SPEC.md
- ⚠️ 警告：第 15 行 FAIL 声明可能无证据
- ⚠️ 警告：第 177 行 FAIL 声明可能无证据
- ⚠️ 警告：第 183 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：LEGACY_ISSUES.md
- ⚠️ 文档可能缺少 provenance 元数据：VERSION_PLAN.md
- ⚠️ 警告：第 24 行 FAIL 声明可能无证据
- ⚠️ 警告：第 25 行 FAIL 声明可能无证据
- ⚠️ 警告：第 26 行 FAIL 声明可能无证据
- ⚠️ 警告：第 46 行 FAIL 声明可能无证据
- ⚠️ 警告：第 47 行 FAIL 声明可能无证据
- ⚠️ 警告：第 48 行 FAIL 声明可能无证据
- ⚠️ 警告：第 49 行 FAIL 声明可能无证据
- ⚠️ 警告：第 59 行 FAIL 声明可能无证据
- ⚠️ 警告：第 83 行 FAIL 声明可能无证据
- ⚠️ 警告：第 84 行 FAIL 声明可能无证据
- ⚠️ 警告：第 85 行 FAIL 声明可能无证据
- ⚠️ 警告：第 136 行 FAIL 声明可能无证据
- ⚠️ 警告：第 137 行 FAIL 声明可能无证据
- ⚠️ 警告：第 148 行 FAIL 声明可能无证据
- ⚠️ 警告：第 159 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GATE_EFFECTIVENESS_MATRIX.md
- ⚠️ 警告：第 305 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：RECOVERY_TEST_DESIGN.md
- ⚠️ 文档可能缺少 provenance 元数据：PR-830E_CONTRACT.md
- ⚠️ 文档可能缺少 provenance 元数据：ARCH-900.md
- ⚠️ 文档可能缺少 provenance 元数据：FEATURE_CHECKLIST.md
- ⚠️ 警告：第 84 行 FAIL 声明可能无证据
- ⚠️ 警告：第 85 行 FAIL 声明可能无证据
- ⚠️ 警告：第 165 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ALPHA_GATE_REPORT.md
- ⚠️ 警告：第 103 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：GRAPH_VS_LEGACY_GATE_RECONCILIATION.md
- ⚠️ 警告：第 73 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ARCH-900-DEAD-MODULE-REPORT.md
- ⚠️ 警告：第 67 行 FAIL 声明可能无证据
- ⚠️ 警告：第 201 行 FAIL 声明可能无证据
- ⚠️ 警告：第 209 行 FAIL 声明可能无证据
- ⚠️ 警告：第 217 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：TEST_PLAN.md
- ⚠️ 文档可能缺少 provenance 元数据：ROADMAP.md
- ⚠️ 警告：第 18 行 FAIL 声明可能无证据
- ⚠️ 警告：第 42 行 FAIL 声明可能无证据
- ⚠️ 警告：第 57 行 FAIL 声明可能无证据
- ⚠️ 警告：第 76 行 FAIL 声明可能无证据
- ⚠️ 警告：第 95 行 FAIL 声明可能无证据
- ⚠️ 警告：第 99 行 FAIL 声明可能无证据
- ⚠️ 警告：第 144 行 FAIL 声明可能无证据
- ⚠️ 警告：第 171 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：RC_GA_GATE_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：CROSS-VERSION-DEBT.md
- ⚠️ 文档可能缺少 provenance 元数据：BETA_GATE_REPORT.md
- ⚠️ 警告：第 15 行 FAIL 声明可能无证据
- ⚠️ 警告：第 105 行 FAIL 声明可能无证据
- ⚠️ 警告：第 138 行 FAIL 声明可能无证据
- ⚠️ 警告：第 189 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：RECOVERY_TEST_MIGRATION_PLAN.md
- ⚠️ 警告：第 21 行 FAIL 声明可能无证据
- ⚠️ 警告：第 22 行 FAIL 声明可能无证据
- ⚠️ 警告：第 26 行 FAIL 声明可能无证据
- ⚠️ 警告：第 144 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：SGL_BETA_GATE_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：SGL-005_STORAGEBYPASS_AUDIT.md
- ⚠️ 文档可能缺少 provenance 元数据：项目分析报告_完整版.md
- ⚠️ 警告：第 74 行 FAIL 声明可能无证据
- ⚠️ 警告：第 75 行 FAIL 声明可能无证据
- ⚠️ 警告：第 78 行 FAIL 声明可能无证据
- ⚠️ 警告：第 82 行 FAIL 声明可能无证据
- ⚠️ 警告：第 88 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ALPHA_CONDITIONAL_PASS.md
- ⚠️ 警告：第 289 行 FAIL 声明可能无证据
- ⚠️ 警告：第 292 行 FAIL 声明可能无证据
- ⚠️ 警告：第 298 行 FAIL 声明可能无证据
- ⚠️ 警告：第 299 行 FAIL 声明可能无证据
- ⚠️ 警告：第 300 行 FAIL 声明可能无证据
- ⚠️ 警告：第 326 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：ALPHA_GATE_CONTRACT.md
- ⚠️ 警告：第 21 行 FAIL 声明可能无证据
- ⚠️ 警告：第 53 行 FAIL 声明可能无证据
- ⚠️ 文档可能缺少 provenance 元数据：INTEGRATION_GATE_REPORT.md
- ⚠️ 文档可能缺少 provenance 元数据：INT1_WAL_RECOVERY_RTI_CHAIN.md

---

### 通过项

- ✅ 状态声明有证据绑定：第 33 行
- ✅ 无状态声明（无需证据检查）：PR-830F_CONTRACT.md
- ✅ 状态声明有证据绑定：第 287 行
- ✅ 状态声明有证据绑定：第 288 行
- ✅ 状态声明有证据绑定：第 289 行
- ✅ 状态声明有证据绑定：第 378 行
- ✅ 状态声明有证据绑定：第 256 行
- ✅ 状态声明有证据绑定：第 310 行
- ✅ 无状态声明（无需证据检查）：ARCHITECTURE.md
- ✅ 状态声明有证据绑定：第 391 行
- ✅ 状态声明有证据绑定：第 398 行
- ✅ 状态声明有证据绑定：第 568 行
- ✅ 状态声明有证据绑定：第 655 行
- ✅ 状态声明有证据绑定：第 671 行
- ✅ 状态声明有证据绑定：第 685 行
- ✅ 状态声明有证据绑定：第 694 行
- ✅ 状态声明有证据绑定：第 726 行
- ✅ 状态声明有证据绑定：第 737 行
- ✅ 状态声明有证据绑定：第 752 行
- ✅ 状态声明有证据绑定：第 763 行
- ✅ 状态声明有证据绑定：第 764 行
- ✅ 状态声明有证据绑定：第 765 行
- ✅ 状态声明有证据绑定：第 782 行
- ✅ 状态声明有证据绑定：第 854 行
- ✅ 状态声明有证据绑定：第 857 行
- ✅ 状态声明有证据绑定：第 872 行
- ✅ 状态声明有证据绑定：第 1035 行
- ✅ 状态声明有证据绑定：第 56 行
- ✅ 状态声明有证据绑定：第 68 行
- ✅ 状态声明有证据绑定：第 117 行
- ✅ 状态声明有证据绑定：第 165 行
- ✅ 状态声明有证据绑定：第 7 行
- ✅ 状态声明有证据绑定：第 21 行
- ✅ 状态声明有证据绑定：第 23 行
- ✅ 状态声明有证据绑定：第 24 行
- ✅ 状态声明有证据绑定：第 25 行
- ✅ 状态声明有证据绑定：第 97 行
- ✅ 状态声明有证据绑定：第 203 行
- ✅ 状态声明有证据绑定：第 205 行
- ✅ 状态声明有证据绑定：第 206 行
- ✅ 状态声明有证据绑定：第 16 行
- ✅ 状态声明有证据绑定：第 82 行
- ✅ 状态声明有证据绑定：第 34 行
- ✅ 状态声明有证据绑定：第 116 行
- ✅ 状态声明有证据绑定：第 191 行
- ✅ 无状态声明（无需证据检查）：COVERAGE-DELTA-ANALYSIS.md
- ✅ 状态声明有证据绑定：第 116 行
- ✅ 状态声明有证据绑定：第 163 行
- ✅ 状态声明有证据绑定：第 206 行
- ✅ 状态声明有证据绑定：第 288 行
- ✅ 状态声明有证据绑定：第 302 行
- ✅ 状态声明有证据绑定：第 37 行
- ✅ 状态声明有证据绑定：第 106 行
- ✅ 状态声明有证据绑定：第 107 行
- ✅ 状态声明有证据绑定：第 115 行
- ✅ 状态声明有证据绑定：第 90 行
- ✅ 状态声明有证据绑定：第 132 行
- ✅ 状态声明有证据绑定：第 14 行
- ✅ 状态声明有证据绑定：第 150 行
- ✅ 状态声明有证据绑定：第 173 行
- ✅ 状态声明有证据绑定：第 52 行
- ✅ 状态声明有证据绑定：第 71 行
- ✅ 状态声明有证据绑定：第 85 行
- ✅ 状态声明有证据绑定：第 103 行
- ✅ 状态声明有证据绑定：第 117 行
- ✅ 无状态声明（无需证据检查）：CROSS-VERSION-DEBT.md
- ✅ 状态声明有证据绑定：第 132 行
- ✅ 状态声明有证据绑定：第 42 行
- ✅ 状态声明有证据绑定：第 110 行
- ✅ 状态声明有证据绑定：第 175 行
- ✅ 状态声明有证据绑定：第 42 行
- ✅ 状态声明有证据绑定：第 171 行
- ✅ 状态声明有证据绑定：第 174 行
- ✅ 无状态声明（无需证据检查）：项目分析报告_完整版.md
- ✅ 状态声明有证据绑定：第 1 行
- ✅ 状态声明有证据绑定：第 12 行
- ✅ 状态声明有证据绑定：第 22 行
- ✅ 状态声明有证据绑定：第 38 行
- ✅ 状态声明有证据绑定：第 63 行
- ✅ 状态声明有证据绑定：第 65 行
- ✅ 状态声明有证据绑定：第 92 行
- ✅ 状态声明有证据绑定：第 94 行
- ✅ 状态声明有证据绑定：第 109 行
- ✅ 状态声明有证据绑定：第 114 行
- ✅ 状态声明有证据绑定：第 131 行
- ✅ 状态声明有证据绑定：第 144 行
- ✅ 状态声明有证据绑定：第 41 行
- ✅ 状态声明有证据绑定：第 44 行
- ✅ 状态声明有证据绑定：第 238 行
- ✅ 状态声明有证据绑定：第 283 行
- ✅ 状态声明有证据绑定：第 287 行
- ✅ 状态声明有证据绑定：第 48 行
- ✅ 状态声明有证据绑定：第 49 行
- ✅ 状态声明有证据绑定：第 125 行
- ✅ 状态声明有证据绑定：第 139 行
- ✅ 状态声明有证据绑定：第 74 行
- ✅ 状态声明有证据绑定：第 144 行
- ✅ 状态声明有证据绑定：第 169 行

---

## 未验证声明（UNVERIFIED CLAIMS）

以下声明**无证据支撑**，不得用于门禁判断：

| 文档 | 行 | 声明内容 |
|------|-----|----------|
| - |  | **单元测试** | ✅ 317/317 + 283/283 PASS | executor + storage crate | |
| - |  | **WAL Recovery** | ✅ 22/22 PASS | RECOVERY-007 re-enabled + PASS | |
| - |  | **Executor** | 317 PASS ✅ | 大量 | ❌ 无 mysql-server E2E | 高 | |
| - |  | **Storage** | 283 PASS ✅ | 16 (`wal_integration_test.rs`) | 8 (`crash_recovery_test.rs`) | 高 | |
| - |  | **错误4**: 混淆 "测试通过" 和 "集成正确" | 🟡 方法论 | PR-870 测试全过但 execute_merge 从未被调用 | |
| - |  - R3 (Expr Convergence): 类型迁移干净，317 测试通过 |
| - |  - PR-830E (WAL Recovery): One-shot guard 正确实现，21/22 测试通过 |
| - |  | Executor 单元测试 | 317/317 PASS | ✅ | |
| - |  | Storage 单元测试 | 283/283 PASS | ✅ | |
| - |  | WAL Contract 测试 | 22/22 PASS ✅ | RECOVERY-007 re-enabled + PASS | |
| - |  预期：`{"result":"PASS","reason":"reachability","missing":[]}` |
| - |  done |
| - |  验证 5/5 tests PASS。 |
| - |  - 假设通过 |
| - |  | Parser Coverage | ✅ | 100 PASS（刚修复 FOR tokenization） | |
| - |  | L1-1 Parser lib | `cargo test -p sqlrustgo-parser --lib` | ✅ 98 PASS | ✅ | |
| - |  | L1-2 Parser coverage tests | `cargo test -p sqlrustgo-parser --test parser_coverage_tests` | ✅ 100 PASS | 96 FAIL | |
| - |  | Dirty Read Prevention | ❌ | 必须 PASS | |
| - |  | Non-repeatable Read | ❌ | 必须 PASS | |
| - |  | Phantom Read | ❌ | 必须 PASS | |
| - |  | Write-Write Conflict | ❌ | 必须 PASS | |
| - |  | Commit crash recovery | ❌ | 必须 PASS | |
| - |  | Rollback crash | ❌ | 必须 PASS | |
| - |  | WAL replay ordering | ❌ | 必须 PASS | |
| - |  | **L1 unit tests** | ✅ 98 PASS | ✅ 98 PASS | 保持 | |
| - |  | **L1 Coverage** | 条件性 PASS（<75% 可接受） | **L1 >= 70%, 每crate >= 50%** | v3.8.0 更严格 | |
| - |  | **Truthfulness 原则** | 无明确文档 | **强制**：禁止改文档通过门禁 | **v3.8.0 重大改进** | |
| - |  - **v3.7.0 门禁保持现状**，已经修复到 100 PASS parser_coverage_tests |
| - |  | Q1-Q22 | — | 22/22 PASS | |
| - |  done |
| - |  | 测试类型 | 通过标准 | Gate ID | |
| - |  | L3 E2E Tests | 28/28 files PASS | C2 | |
| - |  | TPC-H SF=1 | 22/22 queries PASS | C3 | |
| - |  | Backward Compatibility | 所有现有 tests 仍然 PASS | A2 | |
| - |  | 测试 | 当前 PASS 率 | PR-800 后 | |
| - |  v3.6.0 Beta Gate 报告显示 `7/9 PASS`，看起来一切正常。 |
| - |  | WAL Contract | 21/22 PASS | **0/22**（测试框架损坏） | |
| - |  | Integration | PASS | **架构漂移未检测** | |
| - |  Type A — 虚构执行: "测试通过" 但测试框架损坏 |
| - |  Type B — 伪门禁:   Gate PASS 但无真实CI证据 |
| - |  │  作用: 在编译/测试通过之上，增加语义层验证，保证规格与实现一致                  │ |
| - |  RC Gate:     功能完整            → 性能/回归通过 + 证据链 |
| - |  GA Gate:     所有检查通过        → 代码冻结 + 发布 |
| - |  **通过标准**: 方案选定，无未解决的架构冲突 |
| - |  | **L1句法** | cargo build/test/clippy/fmt | 全部 PASS | |
| - |  **通过标准**: 功能规格实现，基础测试通过，无 P0/P1 问题 |
| - |  | SQL Compat | SQL Corpus 通过率 (基线建立) | |
| - |  **通过标准**: 集成测试全部通过，架构不变式满足，无 P0/P1 漂移 |
| - |  **目标**: 性能达标，回归通过，证据链完整 |
| - |  | 回归测试 | `cargo test` 全部通过 | |
| - |  | SQL Corpus | 通过率 ≥90% | |
| - |  **通过标准**: 性能基线达标，证据链完整，无 regression |
| - |  **通过标准**: 零未解决问题，零 open blocker，代码冻结 |
| - |  L1通过 ≠ L2通过 ≠ L3通过 |
| - |  L1: cargo build ✓    → 编译通过 |
| - |  L2: cargo test ✓     → 22个recovery tests通过 |
| - |  结论: L1/L2通过只是"没明显出错" |
| - |        L3通过才是"真正正确" |
| - |  │  产出: PASS/FAIL (exit code)                                       │ |
| - |  │  产出: PASS / FAIL / DRIFT (legacy tracked)                        │ |
| - |  EXIT_PASS  = 0  — 所有检查通过 |
| - |  > **背景**: v3.6.0 Beta Gate 7/9 PASS 实际 0/8 通过 —— AI 伪造证据 |
| - |  **规则**: 任何 PASS/FAIL 声明必须有 CI 证据，包括： |
| - |  | **Type A** | 虚构执行：声称测试通过但无 CI run ID | P0 | PASS 声明无 CI run ID | |
| - |  ❌ Beta Gate Report: "7/9 PASS" |
| - |  结论: ✅ B2 PASS |
| - |  done |
| - |      done |
| - |  done |
| - |  | R2 | **Test** | `cargo test --lib` 全部通过 | ✅ | ✅ | ✅ | ✅ | |
| - |  | R4 | **Format** | `cargo fmt -- --check` 通过 | ✅ | ✅ | ✅ | ✅ | |
| - |  | R8 | **Corpus Gate** | SQL Corpus 通过率不降低（≥基线） | — | ⚠️ | ✅ | ✅ | |
| - |  done |
| - |  **检查内容**: SQL Corpus 测试通过率（相对于基线不降低） |
| - |  done |
| - |  **目的**: 保证所有 PASS/FAIL 声明都有真实 CI 证据，防止 AI 伪造 |
| - |      lines=$(grep -nE "(PASS|FAIL|通过|失败)" "$doc") |
| - |      done |
| - |      if grep -qE "GA.*PASS|Gate.*PASS" "$gate_doc"; then |
| - |      done |
| - |  done |
| - |  done |
| - |  | 0 | 所有检查通过 | PASS | |
| - |  RC Gate     → 性能收敛（回归通过） |
| - |  本文档定义 PR-800 的验收标准（Acceptance Criteria）。每个验收标准必须有明确的验证方法和通过标准。 |
| - |  **通过标准**: 所有 test_pr800_* 测试 PASS（预计 30+ tests） |
| - |  Status: ✅ PASS |
| - |  **通过标准**: 所有 test_pr800_* 测试 PASS |
| - |  **通过标准**: E2E 测试 28/28 PASS |
| - |  **通过标准**: 直接 raw_sql 执行路径 = 0 matches |
| - |  **通过标准**: local_executor 被主路径调用（非孤岛组件） |
| - |  **通过标准**: sqlrustgo crate 所有 lib tests PASS |
| - |  **标准**: 现有的 E2E 测试文件全部 PASS。 |
| - |  **通过标准**: E2E test result: ok (所有测试通过) |
| - |  **通过标准**: Exit code 0 |
| - |  **通过标准**: 0 warnings, 0 errors |
| - |  **标准**: `cargo build --release --workspace` 成功。 |
| - |  **通过标准**: "Finished release profile" 或 "Compiling X crates... done" |
| - |  **通过标准**: 无绕过 Parser 的路径 |
| - |  **通过标准**: execution_engine.rs 行数 <6000 行 |
| - |  **通过标准**: parser 覆盖率 ≥85% |
| - |  **通过标准**: planner 覆盖率 ≥85% |
| - |  **通过标准**: executor 覆盖率 ≥83% (v3.7.0 baseline) |
| - |  C2: 28 E2E test files PASS |
| - |  | **B2** | WAL Execution Path | `ExecutionEngine::with_wal(PathBuf)` 可调用 + RECOVERY 测试验证 crash recovery | Path exists + verifiable | ✅ PASS (21/22) | |
| - |  | **B-F5** | PR-DAG 一致性 | — | ✅ PASS | DEVELOPMENT_PLAN.md vs git | |
| - |  | **B-F6** | Feature Checklist | — | ✅ PASS | docs/releases/v3.8.0/FEATURE_CHECKLIST.md | |
| - |  | **B-F7** | 无幽灵 PR | — | ✅ PASS | 所有未合并 PR 有说明 | |
| - |    RECOVERY-001 begin_then_crash_rolls_back       ✅ PASS  (uses WalStorage<FileStorage>) |
| - |    RECOVERY-002 insert_then_crash_rolls_back      ✅ PASS |
| - |    RECOVERY-004 commit_flush_crash_replays        ✅ PASS |
| - |    RECOVERY-005 partial_insert_write_recovery     ✅ PASS |
| - |    RECOVERY-006 partial_update_write_recovery     ✅ PASS |
| - |    RECOVERY-008 partial_commit_flush_recovery     ✅ PASS |
| - |  The question "how can we enter RC if features aren't done?" confuses two different concepts |
| - |  > - B2 WAL Execution Path: 21/22 RECOVERY tests pass — PASS |
| - |  - [x] B1 Build: core 5 crates build PASS |
| - |  - [x] B2 WAL Execution Path: WAL path exists + 21/22 RECOVERY PASS |
| - |  - [x] B3 Clippy: 0 warnings PASS |
| - |  - [x] B4 Format: fmt check PASS |
| - |  | INT-2: VTU Merge | MERGE via MergeExecutor | PR-870 phase 1 done, parser missing | ⚠️ Partial — path wired, not invoked | |
| - |  - R3 expr convergence completed (MergeStatement uses planner Expr) |
| - |  | INT-1 | ✅ PROVEN | 22/22 PASS | N/A | |
| - |  | Beta | B1 Build / B2 WAL Contract / B3 Clippy | Recovery 7/7 PASS | |
| - |  | GA | 所有门禁 PASS + TPC-H 22/22 | 100% | |
| - |      {"id": "A1", "name": "Build", "result": "PASS", "command": "cargo build --release --workspace", "output_summary": "Finished in 9.64s"} |
| - |  | A1_BUILD | ✅ PASS | Core 6 crates build successfully | |
| - |  | A2_TEST | ✅ PASS | 89 tests pass across 6 crates | |
| - |  | A3_CLIPPY | ✅ PASS | Zero warnings on core crates | |
| - |  | A6-1_REPLAY | ✅ PASS | REPLAY_v3.7.0_GA.md exists | |
| - |  | A6-2_CLAIM | ✅ PASS | ADR-002-claim-registry.md exists | |
| - |  | A6-3_DECISION | ✅ PASS | ARCHITECTURE_DECISIONS.md exists | |
| - |  | A6-4_FRESHNESS | ✅ PASS | Freshness markers present | |
| - |  | A6-5_ADR | ✅ PASS | ADR-001~ADR-005 all exist | |
| - |  **PASS: 8/10 | BLOCKERS: 2** |
| - |  Step 3: Re-run check_alpha_v380.sh → expect PASS |
| - |      /// Recovery completed successfully |
| - |      "WAL recovery completed: {} committed txns, {} rolled back, {} incomplete, {} entries total", |
| - |  cargo test --test wal_tx_contract  → 15+ PASS (RECOVERY tests use L3) |
| - |          └── StorageEngine          ← 仅通过 Executor 访问 |
| - |  **核心原则**: PhysicalPlan 只通过 `local_executor.rs` 执行，不存在其他执行路径。 |
| - |  单元测试通过，但组件从未在真实执行中被调用——这是集成缺失（Integration Gap）。 |
| - |  - 性能回归测试必须通过 |
| - |  > **标准**: v3.8.0 必须通过以下全部门禁方可发布 GA |
| - |  | **计划不可伪造** | 开发/测试计划是历史记录，禁止重写原始计划以通过门禁 | 将 VERSION_PLAN.md 从"Alpha 阶段"重写为"GA Final" | |
| - |  | **执行结果优先** | 以命令输出、测试结果为依据，禁止编造 | 在 TEST_PLAN.md 写入"PASS"但未实际执行 | |
| - |  3. 在 TEST_PLAN.md 写入 "93 tests PASS" 但未执行 cargo test |
| - |  | L2-1 | Execution consistency harness | `python3 scripts/test/execution_consistency_harness.py --corpus data/sql_corpus.json --paths mysql-server,bench-cli,direct` | PASS (all paths same hash) | |
| - |  | L2-2 | E2E integration tests | `cargo test -p sqlrustgo-integration-tests` | 28/28 PASS | |
| - |  | L2-3 | TPC-H SF=1 regression | `./target/release/sqlrustgo-bench-cli tpch-bench --queries all` | 22/22 PASS | |
| - |  | L3-01 | Dirty Read Prevention | `python3 scripts/test/isolation_test_suite.py --test dirty_read` | PASS (uncommitted data NOT visible) | |
| - |  | L3-02 | Non-repeatable Read | `python3 scripts/test/isolation_test_suite.py --test non_repeatable_read` | PASS | |
| - |  | L3-03 | Phantom Read | `python3 scripts/test/isolation_test_suite.py --test phantom_read` | PASS | |
| - |  | L3-04 | Write-Write Conflict | `python3 scripts/test/isolation_test_suite.py --test write_conflict` | PASS (one blocks/fails) | |
| - |  | L3-05 | Lost Update | `python3 scripts/test/isolation_test_suite.py --test lost_update` | PASS | |
| - |  | L3-11 | Same SQL all paths | `python3 scripts/test/execution_divergence.py` | PASS | |
| - |  | L3-12 | NULL handling | `python3 scripts/test/execution_divergence.py --test null_handling` | PASS | |
| - |  | L3-13 | Type coercion | `python3 scripts/test/execution_divergence.py --test type_coercion` | PASS | |
| - |  | L3-14 | Error handling | `python3 scripts/test/execution_divergence.py --test error_handling` | PASS | |
| - |  | L4-5 | ParallelVolcanoExecutor integrated | `scripts/test/vtu_integration_check.sh` | PASS (not stub) | |
| - |  | L5-1 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch-bench --queries all` | 22/22 PASS | |
| - |  | L6-5 | SSOT cross-check | `bash scripts/docs/ssot_cross_check.sh` | PASS | |
| - |  echo "=== GA Gate PASSED ===" |
| - |  cargo test --lib --quiet && echo "L1 PASS" || echo "L1 FAIL" |
| - |  python3 scripts/test/execution_consistency_harness.py --quick && echo "L2 PASS" || echo "L2 FAIL" |
| - |  grep "eng.execute.*raw_sql" src/ --include="*.rs" && echo "L4 FAIL" || echo "L4 PASS" |
| - |  wc -l src/execution_engine.rs | awk '{if($1<1500) print "L4 PASS"; else print "L4 FAIL"}' |
| - |  | Alpha | 编译通过、测试通过、格式干净 | cargo build/test/fmt/clippy | |
| - |  | SGL-001 | B4 format 是 read-only | cargo fmt --check | PASS ✅ | |
| - |  | SGL-002 | WAL-002: checkpoint advance 在 commit 路径 | grep advance_checkpoint | PASS ✅ | |
| - |  | SGL-003 | WAL-003: truncate_before 在 commit 路径 | grep truncate_before | PASS ✅ | |
| - |  | SGL-004 | WAL-004: DELETE replay 幂等 | grep 语义检查 | PASS ✅ | |
| - |    PASS: 4/4 sections |
| - |  SGL: 4/5 PASS, DRIFT=1 |
| - |    SGL-001~004: PASS ✅ |
| - |  WAL: 22/22 PASS (Rust) + 5/5 PASS (invariant script) |
| - |  C-ARCH: 2/2 PASS (C-ARCH-02, C-ARCH-05) |
| - |          "WAL recovery completed: {} committed txns, {} rolled back, {} incomplete, {} entries total", |
| - |  **PASS 标准**：两个 binary 均可正常执行 |
| - |  ./target/release/ingest ci "alpha-check" --status PASS --db /tmp/eg_alpha.db |
| - |  **PASS 标准**：`{"result":"PASS","reason":"reachability","missing":[]}` |
| - |  - C1: `cargo build -p evidence-graph --release` 成功 |
| - |  - C2: `cargo test -p evidence-graph` 全部 PASS（5个测试） |
| - |  - C3: `cargo build -p sqlrustgo-gate --release` 成功 |
| - |  - 状态：PASS | FAIL |
| - |  - ❌ 假设通过（必须实际执行命令） |
| - |  **保留理由**: LocalExecutor 是标准执行器，所有 PhysicalPlan 都通过它执行。 |
| - |  **PASS**：无编译错误，binary 可执行 |
| - |  ./target/release/ingest ci "$CI" "$COMMIT" PASS --db $DB |
| - |  **PASS**：`{"result":"PASS","reason":"reachability","missing":[]}` |
| - |  done |
| - |  在 Alpha PASS 基础上执行： |
| - |  在 Beta PASS 基础上执行： |
| - |  在 RC PASS 基础上执行： |
| - |  | GA1 | 所有 4 个阶段门禁 PASS | |
| - |  1. **必须实际执行命令**，禁止假设通过 |
| - |  | A1.1 | graph-cli builds | PASS | "Finished release profile" | |
| - |  | A1.2 | evidence chain | PASS | "result:PASS, missing:[]" | |
| - |  Expected: PASS |
| - |  | **Alpha** | ✅ PASS | 10/10 checks | A1-A5 + A6-1~5 | |
| - |  | **Beta** | ✅ PASS | 11/11 checks | B1-B4 + B-F1~B-F7 | |
| - |  | **Integration** | ✅ PASS | 4/4 sections | C-ARCH + SGL + WAL + Harness | |
| - |  | **WAL Contract** | ✅ PASS | 22/22 | RECOVERY-001~008 all PASS | |
| - |  | **Clippy** | ✅ PASS | 0 warnings | Fixed useless_conversion | |
| - |  | **Format** | ✅ PASS | 0 diffs | Clean | |
| - |  | **Build** | ✅ PASS | Release build | 5.50s | |
| - |  | A1 | Build | `cargo build --release -p sqlrustgo,executor,storage,parser,server` | ✅ PASS | `Finished release profile in 5.50s` | |
| - |  | A2 | Test | `cargo test --lib` (core 5 crates) | ✅ PASS | `286 passed; 0 failed` | |
| - |  | A3 | Clippy | `cargo clippy --all-features -- -D warnings` | ✅ PASS | `Finished dev profile — 0 warnings` | |
| - |  | A4 | Format | `cargo fmt --all -- --check` | ✅ PASS | `exit 0` (no diff) | |
| - |  **Alpha Gate: 10/10 PASS** ✅ |
| - |  | B1 | Build | `cargo build --release -p core_5_crates` | ✅ PASS | `Finished release profile in 5.50s` | |
| - |  | B2 | WAL Contract | `cargo test --test wal_tx_contract_test` | ✅ PASS | `22 passed; 0 failed; 0 ignored` | |
| - |  | B3 | Clippy | `cargo clippy --all-features -- -D warnings` | ✅ PASS | `0 warnings` | |
| - |  | B4 | Format | `cargo fmt --all -- --check` | ✅ PASS | `exit 0` | |
| - |  WAL Contract Tests: 22/22 PASS ✅ |
| - |  TX-001~TX-006: 6 PASS |
| - |  WAL-001~WAL-005: 5 PASS |
| - |  REPLAY-001~REPLAY-003: 3 PASS |
| - |  RECOVERY-001~RECOVERY-008: 8 PASS |
| - |    - RECOVERY-007: test_partial_delete_write_recovery ✅ (was IGNORED, now PASS) |
| - |  **Beta Gate: 11/11 PASS** ✅ |
| - |  | C-ARCH-02 | LocalExecutor has no write_buffer field | ✅ PASS | |
| - |  **C-ARCH Result**: 3 PASS / 1 FAIL / 1 DRIFT ⚠️ |
| - |  | SGL-001 | B4 format is read-only (no mutation) | ✅ PASS | |
| - |  | SGL-004 | WAL-004: DELETE replay idempotency | ✅ PASS | |
| - |  **SGL Result**: 4 PASS / 0 FAIL / 1 DRIFT ⚠️ |
| - |  | INV-1 | Committed data survives crash | ✅ PASS | |
| - |  | INV-2 | Uncommitted data does NOT survive crash | ✅ PASS | |
| - |  | INV-3 | ROLLBACK leaves no trace | ✅ PASS | |
| - |  **WAL Invariant: 5/5 PASS** ✅ |
| - |  **Integration Gate: 4/4 sections PASS** ✅ |
| - |  | **INT-1** | WAL recovery invariant | ✅ **FULLY PROVEN** | 22/22 PASS + RTI chain doc | |
| - |  | Alpha | A1-A5 + A6-1~5 | ✅ 10/10 PASS | |
| - |  | Beta | B1-B4 + B-F1~7 | ✅ 11/11 PASS | |
| - |  | Integration | C-ARCH + SGL + WAL + Harness | ✅ 4/4 PASS (with 2 issues) | |
| - |  v3.8.0 has completed **Alpha, Beta, and Integration Gates** successfully. The WAL recovery system (INT-1) is **fully proven** with 22/22 tests passing. The architecture invariants are largely satisfied, with two documented issues (C-ARCH-05 and SGL-005 DRIFT) that should be addressed before GA. |
| - |  **Gate Status**: ✅ **PASS with documented issues** |
| - |  PR-830F 实现 WAL 生命周期闭环控制，通过 CheckpointManager 提供的 `safe_truncate_lsn` 实现 WAL 安全截断。 |
| - |  **核心约束**: WAL 是 Event Log，不是 Redo Log — recovery 后不自动 truncate，必须通过 CheckpointManager 显式控制。 |
| - |      /// Record a completed checkpoint |
| - |  这些是 v3.6.0/v3.7.0 遗留的架构缺陷，v3.8.0 需要通过 PR-800~PR-900 解决。 |
| - |  **Beta Gate 条件**: Recovery 7/7 PASS |
| - |  | Week 10 | GA Gate | Full suite PASS | |
| - |  | ACID 语义 | T-ISO-01~05 ALL PASS | |
| - |  | Crash recovery | T-CRA-01~05 ALL PASS | |
| - |  | TPC-H | 22/22 PASS | |
| - |  | FP (False Positive) | Graph FAIL，Legacy PASS，但实际无缺陷 | 需人工验证 FAIL 案例 | |
| - |  | FN (False Negative) | Legacy 发现，Graph 未发现 | Graph PASS 但 Legacy FAIL | |
| - |  | TN (True Negative) | 两者都 PASS，实际无缺陷 | 人工验证 PASS 案例 | |
| - |  | FP 案例列表 | Graph FAIL + Legacy PASS 的 CI runs | 分析 3187+ runs 的 jobs 状态 | |
| - |  | FN 案例列表 | Graph PASS + Legacy FAIL 的 CI runs | 需历史数据 | |
| - |  | MTTD (Mean Time to Detect) | 从 Commit 到 Gate FAIL 的时间差 | 分析 runs 的 created_at vs completed_at | |
| - |  1. **检测 disagreement**：Graph PASS / Legacy FAIL 或反之 |
| - |  | Graph PASS + Legacy FAIL | Legacy 发现问题，Graph 未发现 → Graph FN | |
| - |  | Graph FAIL + Legacy PASS | Graph 发现问题，Legacy 未发现 → Graph TP 或 FP | |
| - |  任何 Gate PASS 结论必须可追溯到 Evidence Graph |
| - |  否则 → UNVERIFIED（不是 PASS） |
| - |  无法证明 ≠ 通过 |
| - |  **实现**：gate evaluate 输出 UNVERIFIED 而非 PASS |
| - |  | Phase 2 | Rule G-01/G-02/G-03 硬编码到 gate CLI | 防止 evidence 缺失时误判 PASS | |
| - |  | evidence-graph-gate | completed | failure | Checkout GITEA_SHA 为空导致失败 | |
| - |  | legacy-gate | completed | failure | 同上，两个 job 失败原因相同 | |
| - |  基于已完成的 runs (3182-3187 全部 failed)，目前没有 PASS 数据可供 Matrix 分析。 |
| - |  | 收集第一批 PASS/PFAIL 数据 | CI | 待 runner 正常后 | |
| - |    { "result": "PASS", ... } |
| - |  gate evaluate 在以下情况输出 UNVERIFIED 而非 PASS： |
| - |  | RECOVERY-001 | `test_begin_then_crash_rolls_back` | `wal_tx_contract_test.rs:317` | ✅ PASS | Uncommitted BEGIN → rollback | |
| - |  | RECOVERY-002 | `test_insert_then_crash_rolls_back` | `wal_tx_contract_test.rs:350` | ✅ PASS | Uncommitted INSERT → rollback | |
| - |  | RECOVERY-003 | `test_prepare_then_crash_rolls_back` | `wal_tx_contract_test.rs:384` | ✅ PASS | PREPARE → crash → rollback | |
| - |  | RECOVERY-004 | `test_commit_flush_crash_replays` | `wal_tx_contract_test.rs:411` | ✅ PASS | COMMIT → crash → replay | |
| - |  | RECOVERY-005 | `test_partial_insert_write_recovery` | `wal_tx_contract_test.rs:440` | ✅ PASS | Partial INSERT → replay | |
| - |  | RECOVERY-006 | `test_partial_update_write_recovery` | `wal_tx_contract_test.rs:469` | ✅ PASS | Partial UPDATE → replay (limited) | |
| - |  | RECOVERY-007 | `test_partial_delete_write_recovery` | `wal_tx_contract_test.rs:507` | ✅ PASS | DELETE replay ordering bug — PR-840 fixes | |
| - |  | RECOVERY-008 | `test_partial_commit_flush_recovery` | `wal_tx_contract_test.rs:531` | ✅ PASS | COMMIT flush → crash → replay | |
| - |  **Summary**: 8/8 PASS ✅ — all RECOVERY tests now passing |
| - |  2. Test should PASS with assertion proving DELETE survival |
| - |  3. LEGACY_FIXES_VERIFICATION_REPORT.md should be updated to reflect 8/8 PASS |
| - |  **Pre-PR-840 Status**: 21/22 PASS (RECOVERY-007 ignored = 1 gap, not 1 FAIL) |
| - |  | RECOVERY-015 | `recover_wal()` outputs: `"WAL recovery completed: {committed_txns} committed txns, {rolled_back_txns} rolled back, {incomplete_txns} incomplete, {entries_total} entries total"` | |
| - |  | `test_recovery_state_default` | Default state is Unrecovered | ✅ PASS | |
| - |  | `test_stateful_engine_blocks_double_recovery` | Idempotent re-invocation returns default report | ✅ PASS | |
| - |  | `test_recovery_engine_impl_trait_bounds` | Trait bounds satisfied | ✅ PASS | |
| - |  | B-F1 (F-03) | ✅ PASS | PR-830C merged | |
| - |  | B-F2 (F-04) | ✅ PASS | PR-830D merged | |
| - |  | B-F3 (F-05) | ✅ PASS | PR-830E merged | |
| - |  | B-F5 | ✅ PASS | PR-DAG in DEVELOPMENT_PLAN.md matches actual | |
| - |  | B-F6 | ✅ PASS | FEATURE_CHECKLIST.md created | |
| - |  | B-F7 | ✅ PASS | No orphan PRs (all merged PRs have corresponding issues) | |
| - |  v3.8.0 Beta Gate 通过了，但实际上大量功能（PR-810~PR-900）未完成。 |
| - |  **Status**: ✅ PASS — 10/10 Checks |
| - |  | A1 | Build | `cargo build --release -p sqlrustgo,executor,storage,parser,server` | exit 0 | `Finished release profile in 7.27s` | ✅ PASS | |
| - |  | A2 | Test | `cargo test --lib` (core 5 crates) | 0 failures | 749 tests passed (21+307+98+47+276) | ✅ PASS | |
| - |  | A3 | Clippy | `cargo clippy --all-features -- -D warnings` | 0 warnings | 0 warnings, 0 errors | ✅ PASS | |
| - |  | A4 | Format | `cargo fmt --all -- --check` | exit 0 | exit 0 (PR-830E WAL import order fixed) | ✅ PASS | |
| - |  | A5 | Coverage | L1 8 crates average | ≥75% | **81.84%** (8 crates avg) | ✅ PASS | |
| - |  | A6-1 | Replay Graph | `ls docs/governance/replay/REPLAY_v3.7.0_GA.md` | exists | File exists | ✅ PASS | |
| - |  | A6-2 | Claim Registry | `ls docs/governance/adr/ADR-002-claim-registry.md` | exists | File exists | ✅ PASS | |
| - |  | A6-3 | Decision Registry | `ls docs/releases/v3.8.0/ARCHITECTURE_DECISIONS.md` | exists | File exists | ✅ PASS | |
| - |  | A6-4 | Freshness | `grep -l "Freshness" docs/releases/v3.8.0/*.md` | marked | Found in 3 docs | ✅ PASS | |
| - |  | A6-5 | ADR Updated | `ls docs/governance/adr/ADR-00*.md` | 5 exist | ADR-001~ADR-005 all exist | ✅ PASS | |
| - |  done |
| - |  | A1 Build | ✅ PASS | — | |
| - |  | A2 Test | ✅ PASS | — | |
| - |  | A3 Clippy | ✅ PASS | — | |
| - |  | A6-1~5 | ✅ PASS | — | |
| - |  | A1 Build | ✅ PASS | — | |
| - |  | A2 Test | ✅ PASS | — | |
| - |  | A3 Clippy | ✅ PASS | — | |
| - |  | A4 Format | ✅ PASS | PR-830E WAL import fix | |
| - |  | A5 Coverage | ✅ PASS | 81.84% avg | |
| - |  | A6-1~5 | ✅ PASS | — | |
| - |    PASS: all paths same result hash |
| - |  | T-ISO-01 | Dirty Read Prevention | txn1 write uncommitted → txn2 read → must NOT see | MUST PASS | |
| - |  | T-ISO-02 | Non-repeatable Read | txn1 read → txn1 write → txn1 read → must same value | MUST PASS | |
| - |  | T-ISO-03 | Phantom Read | txn1 range query → txn2 insert → txn1 range query → must NOT include new | MUST PASS | |
| - |  | T-ISO-04 | Write-Write Conflict | txn1 write row X → txn2 write row X → one blocks or fails | MUST PASS | |
| - |  | T-ISO-05 | Lost Update | concurrent update same row → final value correct | MUST PASS | |
| - |  | T-CRA-01 | commit 中 kill -9 | 重启后 WAL replay → 数据存在 | MUST PASS | |
| - |  | T-CRA-02 | rollback 中 kill -9 | 重启后数据未改变 | MUST PASS | |
| - |  | T-CRA-03 | partial write 中断 | 重启后数据一致或 empty | MUST PASS | |
| - |  | T-CRA-04 | WAL replay ordering | 乱序写入 replay 后正确 | MUST PASS | |
| - |  | T-CRA-05 | double commit | 重启后仅一次生效 | MUST PASS | |
| - |  | T-DIV-01 | Same SQL all paths | mysql-server == bench-cli == direct | MUST PASS | |
| - |  | T-DIV-02 | NULL handling | all paths same NULL semantics | MUST PASS | |
| - |  | T-DIV-03 | Type coercion | all paths same coercion result | MUST PASS | |
| - |  | T-DIV-04 | Error handling | all paths same error code/msg | MUST PASS | |
| - |  | B5 | T-ISO-01 dirty read | PASS | PR-840 | |
| - |  | C2 | E2E integration tests | 28/28 PASS | PR-850 | |
| - |  | E1 | T-ISO-01~05 isolation suite | ALL PASS | PR-890 | |
| - |  | E2 | T-CRA-03~05 crash suite | ALL PASS | PR-890 | |
| - |  | G2 | T-ISO-01~05 | ALL PASS | |
| - |  | G3 | T-CRA-01~05 | ALL PASS | |
| - |  | G4 | T-DIV-01~04 | ALL PASS | |
| - |  | G5 | TPC-H SF=1 | 22/22 PASS | |
| - |    PASS: all execution paths same result hash |
| - |  输出: PASS/FAIL + recovery details |
| - |  输出: PASS (correct isolation) / FAIL (violation detected) |
| - |  - 22 queries 必须全部 PASS |
| - |  - G1 gate: 250 tests PASS |
| - |  - 400+ tests PASS |
| - |  | **D1-Alpha** | A1-A5 + A6 Governance | 10/10 | ✅ PASS | |
| - |  | **D2-Beta** | B1-B4 + B5 Integration | 5/5 | ✅ PASS | |
| - |  | **D3-SGL** | SGL-001~005 (semantic layer) | 4/5 PASS, 0 FAIL, 1 DRIFT | ⚡ DRIFT | |
| - |  | **D4-WAL** | INV-1, INV-2, INV-3 | 5/5 | ✅ PASS | |
| - |  | **D5-DeepSeek** | 10 Principles for RC/GA | 10/10 | ✅ PASS | |
| - |  | A1: Build (release, core 6 crates) | ✅ PASS | `Finished release profile in 5.50s` | |
| - |  | A2: Test (lib, core 6 crates) | ✅ PASS | `286 passed; 0 failed` | |
| - |  | A3: Clippy (core) | ✅ PASS | `0 warnings` | |
| - |  | A4: Format (core) | ✅ PASS | `exit 0` (no diff) | |
| - |  | A5: Coverage (L1 8 crates) | ✅ PASS | `82% avg (min: 75%)` | |
| - |  | A6-1: Replay Graph | ✅ PASS | `docs/governance/replay/REPLAY_v3.7.0_GA.md` | |
| - |  | A6-2: Claim Registry | ✅ PASS | `docs/governance/adr/ADR-002-claim-registry.md` | |
| - |  | A6-3: Decision Registry | ✅ PASS | `docs/releases/v3.8.0/ARCHITECTURE_DECISIONS.md` | |
| - |  | A6-4: Freshness markers | ✅ PASS | Present in 3+ docs | |
| - |  | A6-5: ADR-001~ADR-005 | ✅ PASS | All 5 ADRs exist | |
| - |  | B1: Build | ✅ PASS | `Finished release profile in 5.50s` | |
| - |  | B2: WAL Contract (22 tests) | ✅ PASS | `22 passed, 0 failed` | |
| - |  | B3: Clippy | ✅ PASS | `0 warnings` | |
| - |  | B4: Format | ✅ PASS | `exit 0` | |
| - |  | B5: Integration Gate | ✅ PASS | `Result: PASS — all checks passed` | |
| - |  | SGL-001 | B4 format is read-only | ✅ PASS | |
| - |  | SGL-002 | WAL-002: checkpoint advance in commit path | ✅ PASS | |
| - |  | SGL-003 | WAL-003: truncate_before in commit path | ✅ PASS | |
| - |  | SGL-004 | WAL-004: DELETE replay idempotency | ✅ PASS | |
| - |  **SGL Summary**: PASS: 4/5 | FAIL: 0 | DRIFT: 1 |
| - |  **D3-SGL: PASS=4 | FAIL=0 | DRIFT=1** ⚡ |
| - |  | INV-1 | Committed data survives crash | ✅ PASS | |
| - |  | INV-2 | Uncommitted data does NOT survive crash | ✅ PASS | |
| - |  | INV-3 | ROLLBACK leaves no trace | ✅ PASS | |
| - |  **Rust Tests**: 22/22 PASS (`wal_tx_contract_test`) + 5/5 PASS (`exp_g_wal_contracts_verified`) |
| - |  | D5-1 | Test Results Override Documentation | ✅ PASS (21 tests evidence) | |
| - |  | D5-2 | Minimal Enforcement | ✅ PASS (recent: 2 lines) | |
| - |  | D5-3 | Strict Scope Control | ✅ PASS (no EEK v2 mixed) | |
| - |  | D5-4 | Version Boundary Clarity | ✅ PASS | |
| - |  | D5-5 | Root Cause Provenance | ✅ PASS (108 WAL-related commits) | |
| - |  | D5-6 | No False Positives | ✅ PASS (crash_recovery_test.rs removed) | |
| - |  | D5-7 | Audit Trail | ✅ PASS (4 audit files) | |
| - |  | D5-8 | DRIFT Classification | ⚡ PASS (SGL-005 DRIFT tracked) | |
| - |  | D5-9 | Strict Rules with Exemption通道 | ✅ PASS (DRIFT mechanism present) | |
| - |  | D5-10 | Engineering Facts Over AI Analysis | ✅ PASS (0 test failures) | |
| - |  | C-ARCH-02 | LocalExecutor has no write_buffer field | ✅ None found | **PASS** | |
| - |  | C-ARCH-05 | execution_engine.rs < 2000 lines | ✅ 1562 lines | **PASS** (DRIFT threshold: 2000) | |
| - |  **Note**: `check_arch_invariants.sh` uses 1500 limit (FAIL), but `check_integration_gate.sh` and `check_rc_ga_gate.sh` use 2000 limit (PASS with DRIFT tracking). Unified rule: **2000 lines**, DRIFT-tracked if between 1500-2000. |
| - |  | **Alpha → Beta** | 10/10 | — | — | — | — | ✅ PASS | |
| - |  | **Beta → RC** | — | 5/5 | 0 FAIL | 5/5 | — | ✅ PASS | |
| - |  **v3.8.0 Beta Gate: ✅ PASS** |
| - |  **Status**: ✅ BETA GATE PASS — 11/11 checks (B1 Build ✅ B2 WAL Contract ✅ B3 Clippy ✅ B4 Format ✅ B-F1~B-F7 PASS)   |
| - |  **BETA Gate PASS criteria used**: "WAL execution path exists, builds, and is verifiable" (Architecture Gate definition) |
| - |  | **B1 Build** | exit 0 (core 5 crates) | ✅ PASS | `release` profile in 7.08s | |
| - |  | **B2 WAL Contract** | WAL path + RECOVERY verifiable | ✅ PASS | 21/22 RECOVERY tests pass, PR-830E integrated | |
| - |  | **B3 Clippy** | 0 warnings | ✅ PASS | clippy 0 warnings on core crates | |
| - |  | **B4 Format** | exit 0 | ✅ PASS | fmt check pass | |
| - |  PR-800  COM_QUERY AST Routing (L0)           ← Infrastructure only, TransactionalFacade NOT done |
| - |  The question "how can we enter RC if features aren't done?" is **correct**. RC Gate is where feature completeness must be verified. |
| - |    RECOVERY-001  ✅ PASS |
| - |    RECOVERY-002  ✅ PASS |
| - |    RECOVERY-004  ✅ PASS |
| - |    RECOVERY-005  ✅ PASS |
| - |    RECOVERY-006  ✅ PASS |
| - |    RECOVERY-008  ✅ PASS |
| - |  | **Alpha** | Execution Semantics Freeze | Architecture freeze declared | ✅ PASS | |
| - |  | **Beta** | Architecture infrastructure | WAL path, build, code quality | ✅ PASS (conditional) | |
| - |  **Beta PASS means**: The architecture foundation is laid. It does NOT mean features are complete. |
| - |  - **15 PASS** (TX + WAL，依赖 L1 MemoryStorage 语义) |
| - |  | 15 PASS / 7 FAIL | **22 PASS / 0 FAIL**（RECOVERY-001~007 迁移到 L3）| |
| - |  - [ ] `cargo test --test wal_tx_contract_test` → 22/22 PASS |
| - |  - [ ] Beta Gate B2 验证通过 |
| - |  1. `cargo test --test wal_tx_contract_test` 输出 **22 PASS, 0 FAIL** |
| - |  | B4 Format semantics | SGL-001 | Semantic | **PASS** ✅ | |
| - |  | WAL-004: DELETE replay idempotency | SGL-004 | Invariant | **PASS** ✅ | |
| - |  **PASS: 2/5 | FAIL: 2 | DRIFT: 1** |
| - |  | PASS | Invariant satisfied | |
| - |  | 11 | local_executor.rs | 1328 | `facade.execute_dml(|storage| ...)` | ✅ 通过 facade 闭包 | |
| - |  **方案**: Trigger 的 DML 操作应通过 `WalTransactionalFacade` 代理，确保 WAL 记录。 |
| - |  - INV-1 (committed data survives): ✅ 22/22 PASS |
| - |  - INV-2 (uncommitted data lost): ✅ 22/22 PASS |
| - |  - INV-3 (rollback clean): ✅ 22/22 PASS |
| - |  当前 v3.8.0 Alpha Gate 声明 "CONDITIONAL PASS" 但没有明确： |
| - |  - CONDITIONAL PASS 的确切含义是什么？ |
| - |  - 什么条件下可以 CONDITIONAL PASS？ |
| - |  - 与 FULL PASS 的区别是什么？ |
| - |  - CONDITIONAL PASS 的门禁效力是什么？ |
| - |  **CONDITIONAL PASS** = 门禁检查项部分通过，但通过的部分有明确的前提条件/约束，且这些前提条件已被记录和接受。 |
| - |  与 **FULL PASS** 的区别： |
| - |  | 属性 | CONDITIONAL PASS | FULL PASS | |
| - |  | 门禁完整性 | 部分检查项被豁免 | 所有检查项必须通过 | |
| - |  Alpha CONDITIONAL PASS 在以下情况下适用： |
| - |  - A1 Build PASS 没有意义（测试的是旧架构） |
| - |  - A2 Test PASS 没有意义（测试的是旧路径） |
| - |  - 该检查项可以被标记为 CONDITIONAL PASS |
| - |  - 但必须记录「什么条件下」才能成为 FULL PASS |
| - |  - 该检查项可以 CONDITIONAL PASS |
| - |  v3.8.0 Alpha 声明 CONDITIONAL PASS，因为： |
| - |  | A1 Build | ✅ PASS | 核心 6 crates 通过 | |
| - |  | A2 Test | ✅ PASS | 89 tests 通过 | |
| - |  | A3 Clippy | ✅ PASS | 零警告 | |
| - |  | A6-1~5 | ✅ PASS | Governance 框架完整 | |
| - |  **因此**: CONDITIONAL PASS = 这些检查项的失败已被接受，且有明确的修复时间表。 |
| - |  **重要**: CONDITIONAL PASS 只是说明「检查项部分通过且有记录」，并不意味着「可以发布」。 |
| - |  CONDITIONAL PASS 的发布效力： |
| - |  | 场景 | CONDITIONAL PASS | FULL PASS | |
| - |  | GA | **不允许** | 必须 FULL PASS | |
| - |  **GA 绝对不允许 CONDITIONAL PASS**：GA 是生产发布的门槛，所有 Gate 必须 FULL PASS。 |
| - |  CONDITIONAL PASS 必须在 Gate Report 中明确记录： |
| - |  1. **触发条件**: 明确列出 CONDITIONAL PASS 的触发场景 |
| - |  2. **记录要求**: CONDITIONAL PASS 必须记录豁免原因和前提条件 |
| - |  3. **效力限制**: GA 不允许 CONDITIONAL PASS |
| - |  4. **修复要求**: CONDITIONAL PASS 的检查项必须在下一 Gate 之前修复 |
| - |  如果是 CONDITIONAL PASS： |
| - |  **CONDITIONAL PASS 发布效力**: [受约束 / 不允许发布] |
| - |  - A1 Build PASS 没有意义（测试的是旧架构） |
| - |  - A2 Test PASS 没有意义（测试的是旧路径） |
| - |  | A6-4 | Freshness PASS | 引用数据必须标注 Freshness | 无过期数据引用 | ADR-001 §G-06 | |
| - |  **通过标准**: `Finished release profile` 或 exit code 0 |
| - |  Status: ✅ PASS |
| - |  **通过标准**: `test result: ok` (0 failures) |
| - |  Status: ✅ PASS |
| - |  **通过标准**: 0 warnings, 0 errors |
| - |  Status: ✅ PASS |
| - |  **通过标准**: exit code 0 |
| - |  Status: ✅ PASS |
| - |  done |
| - |  **通过标准**: L1 8 crates 综合平均 ≥75% |
| - |  Status: ✅ PASS (≥75%) |
| - |  **通过标准**: 文件存在，且至少 4 个 issue 有完整轨迹 |
| - |  Status: ✅ PASS (4/4 issues traced) |
| - |  **通过标准**: v3.8.0 相关的 Claim 有记录 |
| - |  Status: ✅ PASS |
| - |  **通过标准**: AD-001~AD-005 存在 |
| - |  Status: ✅ PASS (5/5 ADs recorded) |
| - |  **通过标准**: 引用数据标注 Freshness，无过期数据 |
| - |  === A6-4: Freshness PASS === |
| - |  Status: ✅ PASS |
| - |  **通过标准**: ADR-001~ADR-005 全部存在 |
| - |  Status: ✅ PASS (5/5 ADRs) |
| - |  所有 A1-A5 和 A6-1~A6-5 全部 PASS → Alpha Gate PASS |
| - |  A1-A5 全部 PASS，A6 有 1-2 项 FAIL（可在 Beta 前修复）→ CONDITIONAL PASS |
| - |  done |
| - |  **Result**: ✅ PASS — 4/4 sections | SGL: 4/5 PASS | WAL: 22/22 + 5/5 |
| - |  | C-ARCH | Architecture static invariants | PASS ✅ | |
| - |  | SGL | Semantic gate (WAL lifecycle) | PASS ✅ | |
| - |  | WAL | Crash recovery + invariant validation | PASS ✅ | |
| - |  **Key Achievement**: WAL-002/003 从 FAIL → PASS，WAL 无限增长问题已修复。 |
| - |  | C-ARCH-02 | LocalExecutor has no write_buffer field | ✅ PASS | |
| - |  | C-ARCH-05 | execution_engine.rs < 2000 lines | ✅ PASS (1562 lines) | |
| - |  | SGL-001 | B4 format is read-only (no mutation) | PASS ✅ | |
| - |  | SGL-004 | WAL-004: DELETE replay idempotency | PASS ✅ | |
| - |  **SGL Summary**: PASS: 4/5 | FAIL: 0 | DRIFT: 1 |
| - |  | INV-1 | Committed data survives crash | PASS ✅ | |
| - |  | INV-2 | Uncommitted data does NOT survive crash | PASS ✅ | |
| - |  | INV-3 | ROLLBACK leaves no trace | PASS ✅ | |
| - |  **Invariant Summary**: 5/5 PASS |
| - |  v3.8.0 Integration Gate: **PASS** |
| - |  - ✅ Contract tests: 22/22 PASS ✅ |
| - |  **Overall**: INT-1 is **FULLY PROVEN** — 22/22 PASS. The DELETE gap is now closed. |
| - |  | Committed INSERT survives crash | RECOVERY-004 | `assert_eq!(value, "committed_data")` | ✅ Direct | ✅ PASS | |
| - |  | Multiple committed tx survive | RECOVERY-008 | `assert_eq!(len, 2)` + value assertions | ✅ Direct | ✅ PASS | |
| - |  | Uncommitted INSERT rolled back | RECOVERY-001, 002 | `assert_eq!(count, 1)` | ✅ Direct | ✅ PASS | |
| - |  | Uncommitted BEGIN rolled back | RECOVERY-001 | `assert_eq!(count, 1)` | ✅ Direct | ✅ PASS | |
| - |  | Committed UPDATE row survives | RECOVERY-006 | `assert_eq!(count, 1)` | ⚠️ Partial | ✅ PASS | |
| - |  | Committed DELETE row stays deleted | RECOVERY-007 | `assert_eq!(count, 0)` | ✅ Direct | ✅ PASS | |
| - |  3. Update LEGACY_FIXES_VERIFICATION_REPORT.md to 22/22 PASS |

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

- 违规次数: 530
- 违规类型: Type A（虚构执行）/ Type B（伪门禁）/ Type D（伪任务完成）
- 处理方式: 触发 Anti-Fabrication Policy 问责机制


---

*报告生成时间: 2026-06-01 07:44:38*
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

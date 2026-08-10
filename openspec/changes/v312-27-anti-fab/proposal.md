# V312-27: Anti-Fabrication 违规修复 — proposal

> **Issue**: V312-27（V312-24 Phase 4 follow-up）
> **Owner**: minimax
> **Expiry**: 2026-08-25
> **Source**: V312-24 tasks.md Phase 4 + proposal.md §Anti-fabrication 17-23 行
> **Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-27_baseline_evidence.txt`

## Why

V312-24 识别 7 个 anti-fab 违规。**Baseline 实测**（2026-08-09，见
`evidence/V312-27_baseline_evidence.txt`）：

1. `crates/executor/tests/merge_vtu_test.rs:212` `#[ignore = "VtuGuard not yet implemented"]`（VtuGuard 在 v3.8.0 ARCH-3 已 CLOSED，stale）
2. `tests/integration/tpch/tpch_wire_smoke_sf.rs` `assert!(rows.len() <= 6)` 接受 0 行（smoke 永远 PASS）
3. `tests/e2e/e2e_beta_test.rs` 5 处 `if is_e2e_disabled() { return; }` 双 skip
4. `tests/baseline/ignore_registry.json` 包含 **37 条** stale v3.9.0 路径（同一文件多份）
5. `tests/baseline/ignore_registry.json` 包含 **0 条** phantom（proposal.md 提到的 parser.rs:7299 phantom **实际不存在**，这是 V312-24 proposal 的过时描述；本 issue 关闭条件不依赖 phantom 计数，但需补审计说明）
6. `tests/baseline/ignore_registry.json` 包含 **3 条** `union_set_operations` IGNORE_MULTI（features 已实现）
7. `tests/baseline/ignore_registry.json` 与实际 `#[ignore]` 集不对齐

> **CORRECTION vs V312-24 proposal.md**: V312-24 proposal.md 写"7 stale + 1 phantom + 3 union_set_operations" 是粗略描述；baseline 实测 stale=37、phantom=0、union=3。本 issue 的关闭命令使用 baseline 实测数字。

## 关闭边界（必须全部满足，命令 + 数字 + 文件存在）

1. `grep -n 'VtuGuard not yet implemented' crates/executor/tests/merge_vtu_test.rs` 输出 **0 行**；`cargo test -p sqlrustgo-executor --test merge_vtu_test 2>&1 | tail -3` 最后一行含 `test result: ok`。
2. `grep -n 'rows.len() <= 6' tests/integration/tpch/tpch_wire_smoke_sf.rs` 输出 **0 行**；`grep -n 'rows.len() > 0' tests/integration/tpch/tpch_wire_smoke_sf.rs` 输出 **≥ 1 行**。删改证据写入 commit message + diff。
3. `grep -c 'is_e2e_disabled' tests/e2e/e2e_beta_test.rs` 输出 **0**（5 处全删）；`CI=1 cargo test --test e2e_beta_test -- --ignored 2>&1 | grep -c 'test result: ok'` 输出 **≥ 1**。
4. `python3 -c "import json; d=json.load(open('tests/baseline/ignore_registry.json')); tests=d.get('ignored_tests',[]); stale_keys=['tx_wal_contract','soak_test','dml_integration_test','stored_proc_catalog','tpch_q9_audit','small_executor_modules','boundary_test','union_set_operations']; stale=[t for t in tests if any(p in t.get('file','') for p in stale_keys)]; union=[t for t in tests if 'union_set_operations' in t.get('file','')]; print('stale:', len(stale), 'union:', len(union))"` 输出 **stale: 0 union: 0**。Phantom audit（`parser.rs` + `7299`）输出 0（baseline 已是 0，不强制；本 issue 需在 report 里写明"baseline phantom=0，V312-24 proposal 描述错误"）。
5. 关闭报告 `docs/releases/v3.12.0/V312-27_anti_fab_fix_report.md` 必须存在，且包含：
   - diff stat (≥ 3 个文件 changed)
   - ignore_registry 前后条目数（before=74, after=37）
   - 实际跑通的 `cargo test` log（merge_vtu_test + e2e_beta_test）
   - sha256（报告 + 4 个改动文件）
   - 字段名校正说明（`ignored_tests` 不是 `files`）

## 禁止关闭条件

1. 仅删除代码但未提供 `cargo test` 实际输出即关闭。
2. 不允许用"openspec 标 done" / "报告标题写已完成" 关闭。
3. **字段名错误**（用 `d.get('files',[])` 而不是 `d.get('ignored_tests',[])`）的 cross-check 脚本视为未执行，需重写。
4. 关闭命令期望值必须与 baseline 对得上；baseline 数据写在 `evidence/V312-27_baseline_evidence.txt`。

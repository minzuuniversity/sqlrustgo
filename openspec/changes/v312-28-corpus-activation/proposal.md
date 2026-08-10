# V312-28: SQL Corpus 激活 — proposal

> **Issue**: V312-28（V312-24 Phase 5 follow-up）
> **Owner**: opencode-z440
> **Expiry**: 2026-09-30
> **Source**: V312-24 tasks.md Phase 5 + proposal.md §4
> **Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-28_baseline_evidence.txt`

## Why

V312-24 描述 corpus 当前 27.3% pass-rate (6/16 subcategories PASS)，
但 **2026-08-09 实测**（见 baseline evidence）：99.4% pass-rate
(813/818 cases, 14 subcategories, 103 .sql files) — 已远超 80% threshold。
`crates/sql-corpus/tests/corpus_test.rs:64` 的 `PASS_RATE_THRESHOLD = 80.0`
**当前已经满足**。

V312-28 的工作不再是"提 pass-rate"，而是：
1. **确认** 80% threshold 在 `develop/v3.12.0` 仍稳定（防止后续改动 regress）
2. **补 14 个守护 test**（每个 subcategory 1 个 `#[test]`），避免未来改动悄悄 break
3. **校订 V312-24 proposal 文档**（subcategories=14 不是 16，pass_rate=99.4% 不是 27.3%）

## 关闭边界（必须全部满足，命令 + 数字 + 文件存在）

1. `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test test_sql_corpus_all -- --nocapture > /tmp/corpus_final.txt 2>&1`；最后一行含 `Pass rate: XX.X%` 且 **XX.X ≥ 80.0**。
2. `python3 -c "import re; t=open('/tmp/corpus_final.txt').read(); m=re.search(r'Pass rate:\s*([\d.]+)%', t); print(m.group(1) if m else 'NOT FOUND')"` 输出数字 **≥ 80.0**。
3. **14 个 subcategory** 全部有对应 inline test 守护（ADVANCED/DDL/DEBUG/DML/EVENTS/EXPRESSIONS/FUNCTIONS/INDEXES/PROCEDURES/SPECIAL/TCL/TRANSACTION/TRIGGERS/VIEWS）；`find crates/sqlrustgo-executor/tests crates/sql-corpus/tests -name '*.rs' -exec grep -l '#\[test\]' {} + | xargs grep -c 'subcat\|subcategory\|ADVANCED\|DDL\|DEBUG\|DML' 2>/dev/null | awk -F: '{s+=$2} END{print s}'` 输出 **≥ 14**（或用其他可验证的方式：每个 subcategory 至少有 1 个明确标识的 test name）。
4. 关闭报告 `docs/releases/v3.12.0/V312-28_corpus_activation_report.md` 必须存在且包含：
   - baseline 数字（2026-08-09 实测 99.4% / 14 subcategories / 103 .sql files / 818 cases）
   - final 数字（≥ 80%）
   - 14 个 subcategory 守护 test 的 test 名列表
   - 14 个守护 test 实际跑通的 `cargo test` 输出片段
   - 校订 V312-24 proposal 错误的说明（16→14, 27.3%→99.4%）
   - sha256（report + 14 个守护 test 文件 + /tmp/corpus_final.txt）

## 禁止关闭条件

1. 不允许"PR 已合并" / "openspec 标 done" / "报告标题写已完成" 关闭。
2. 不允许以"baseline 已经是 99.4% 所以不需要工作"为由关闭 — 必须有 14 个守护 test 的实际 `cargo test` 输出。
3. 不允许把 27.3% 当作 baseline — V312-24 proposal 的 27.3% 数字已过时，实际是 99.4%。
4. 不允许无 sha256 关闭。

## 校订记录

V312-24 proposal.md §4 描述（"16 subcategories / 27.3% / 6/16 PASS"）是
v3.11.0 之前的状态。`develop/v3.12.0` 自 v3.11.0 起 SimpleExecutor 已
实现 GROUP BY ROLLUP / multi-column IN / INTERSECT / INSERT ON DUPLICATE
KEY UPDATE / recursive CTE / JSON_TABLE（详见 `crates/sql-corpus/src/lib.rs`
1233 行代码）。本 issue 关闭时需在 report 写明"baseline 校订"。

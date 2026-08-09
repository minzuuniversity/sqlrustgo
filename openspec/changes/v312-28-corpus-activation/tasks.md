# V312-28: SQL Corpus 激活 — tasks

> **Status**: 🔵 OPEN — created 2026-08-09
> **Owner**: opencode-z440
> **Expiry**: 2026-09-30
> **Correction vs V312-24 proposal**: baseline pass_rate = 99.4% (not 27.3%),
> 14 subcategories (not 16). See `evidence/V312-28_baseline_evidence.txt`.

- [ ] 1.1 重跑 `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test test_sql_corpus_all -- --nocapture > /tmp/corpus_final.txt 2>&1` 并保存
- [ ] 1.2 解析 `Pass rate: XX.X%` 字段，**断言 ≥ 80.0**（baseline 是 99.4）
- [ ] 1.3 找出当前 5 个 failing case（baseline 813/818），尝试修；如修不动，**降到 ≤ 0 个 failing 才算 PASS**（V312-28 fallback 走 1.4）
- [ ] 1.4 如 5 个 failing 修不动：写 `docs/releases/v3.12.0/V312-28_corpus_threshold_attestation.md` 含 owner + expiry + replacement-gate
- [ ] 1.5 给 14 个 subcategory 各加 1 个 `#[test]` 守护（命名约定 `test_corpus_subcategory_<name>` 或加 `// subcat: <NAME>` 注释）
- [ ] 1.6 跑 `cargo test -p sqlrustgo-executor`（注意 crate 名是 `executor` 不是 `sqlrustgo-executor`）确认 14 个守护 test 全 PASS
- [ ] 1.7 写 `docs/releases/v3.12.0/V312-28_corpus_activation_report.md`：
  - baseline 数字（99.4% / 14 subcategories / 103 .sql files / 818 cases / 5 failing）
  - final 数字（≥ 80%）
  - 14 个守护 test 的 test name 列表
  - 14 个守护 test 实际 `cargo test` 输出片段
  - 校订 V312-24 proposal（16→14, 27.3%→99.4%）
  - sha256（report + 14 个 test 文件 + /tmp/corpus_final.txt）

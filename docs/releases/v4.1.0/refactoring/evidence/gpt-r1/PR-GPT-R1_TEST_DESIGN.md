# PR-GPT-R1 TEST_DESIGN — 具体测试方法

> **source_agent**: DSH-B (opencode / MiniMax-M3) · **source_run**: SQLRUSTGO-V410-REFACTOR-HARNESS-R1-20261123
> **timestamp**: 2026-11-23T11:00:00+08:00 · **base_sha**: `8cbde5b6d33c876fa1197474c46d50142a9823bd`
> **evidence_hash**: 见 `docs/releases/v4.1.0/refactoring/gpt-r1/manifest.json#files_digest`
> **conflict_resolution**: 测试方法以「能区分行为」为唯一标准。
> provenance: source_agent=DSH-B (opencode / MiniMax-M3); generated_by=DSH-B/opencode; generated_at=2026-11-23T11:30:00+08:00; base_sha=8cbde5b6d33c876fa1197474c46d50142a9823bd; package=docs/releases/v4.1.0/refactoring/evidence/gpt-r1/

## 1. 方法：夹具树 + 真实门禁脚本

门禁是 shell 脚本，判据藏在 `if`/`grep`/`awk` 里。**从源码读判据**不能证明行为，
所以本套件的做法是：

1. 把**真实**的 `scripts/gate/` 整棵树复制到临时目录；
2. 按用例需要增删其中的门禁脚本、写入/删除 `tests/baseline/*.json`、
   造 `crates/*/src/*.rs` 与 `src/execution_engine.rs`；
3. 用 `bash <fixture>/scripts/gate/<gate>.sh` 跑**真实脚本**；
4. 断言退出码与输出里的关键字符串。

**不 mock 判据** —— 被测的就是门禁本体。

### 1.1 为什么整树复制

`check_drift_not_pass.sh` 会扫描**兄弟门禁脚本**。只复制单个脚本会让它的扫描集变成
空集，结论会被 fixture 本身污染。所以：

- 需要「干净树」的用例 ⇒ 只放该门禁 + 一个**无害**的兄弟门禁；
- 需要「脏树」的用例 ⇒ 只放该门禁 + 一个**恰好带一条新反模式**的兄弟门禁。

## 2. 每个用例的判据

| 用例 | 夹具改动 | 期望 rc | 期望输出包含 |
|---|---|---|---|
| `drift_gate_missing_baseline_must_not_pass` | 删 `tests/baseline/drift_baseline.json` | ≠0 | `baseline` |
| `drift_gate_zero_scanned_scripts_must_not_pass` | `scripts/gate/` 只放检测器自身 | ≠0 | 扫描数为 0 |
| `drift_gate_clean_tree_passes` | 检测器 + 无害兄弟门禁 + 空基线 | 0 | — |
| `drift_gate_new_antipattern_fails` | 新增带 `\|\| true` 吞咽的兄弟门禁 | ≠0 | `REGRESSION` |
| `gate_test_integrity_missing_test_reference_must_not_pass` | 新增引用 `--test dsb_contract_missing_test` 的门禁 | ≠0 | `dsb_contract_missing_test` |
| `gate_test_integrity_missing_baseline_must_not_pass` | 不写基线 | ≠0 | — |
| `gate_test_integrity_zero_tests_must_not_pass` | `scripts/gate/` 只放本门禁 | ≠0 | — |
| `evidence_binding_fabricated_claim_fails` | 文档含无证据的 `全部测试通过。` | ≠0 | — |
| `evidence_binding_heading_marker_does_not_excuse_claim` | 同一句写成 `## 全部测试通过` | ≠0 | — |
| `evidence_binding_unrelated_negation_does_not_excuse_claim` | `本批次不涉及其他阶段，全部测试通过` | ≠0 | — |
| `evidence_binding_negated_statement_is_not_a_violation` | `本批次没有改变任何一道门禁的『通过/不通过』结论。` | **0** | — |
| `evidence_binding_wrong_sha_fails` | 一行伪造的 40 位十六进制 commit id（写成 `**Commit**:` 表格行） | ≠0 | — |
| `evidence_binding_missing_log_file_must_not_pass` | `\| log \| no/such/log.txt` | ≠0 | — |
| `architecture_freeze_compliant_tree_passes` | 完整合规的 `crates/` + `src/execution_engine.rs` | 0 | — |
| `architecture_freeze_detects_real_ad002_bypass` | 编译产物里的 `storage.insert(...)` | ≠0 | `storage DML bypass` |
| `architecture_freeze_missing_detector_is_incomplete` | 删掉 detector | ≠0 | — |
| `architecture_freeze_missing_drift_gate_fails` | 去掉 DriftGate | ≠0 | — |
| `anti_fabrication_stale_sqlancer_report_must_not_be_certified` | 预置**旧**的合法报告 + 记录调用的 sqlancer 桩 | — | 调用日志必须存在 |
| `anti_fabrication_zero_tests_report_must_not_pass` | test-runner 报告 `results: []` | ≠0 | — |
| `anti_fabrication_missing_manifest_must_not_pass` | 删 `config/test-runner-manifest.toml` | ≠0 | — |

## 3. 桩环境（`check_anti_fabrication.sh` 专用）

该门禁会调 `cargo`、构建 sqlancer / test-runner。桩环境提供：

- `bin/cargo` — 恒 rc=0；对 `metadata` 输出 `{"packages":[{"name":"stub"}]}`，
  对 `--message-format=json` 输出一条 compiler-artifact；
- `target/release/sqlancer` — 把每次调用写进 `sqlancer_invocations.log`
  （**这是「过期证据」缺陷的判别器**：文件不产生 ⇒ 门禁没真跑）；
- `target/release/test-runner` — 写出一份 schema 合法的报告（可被改造成
  `results: []` 以测零执行）。

桩只影响**编排层**，不改变被测判据：桩永远成功，任何需要「判决」的地方都会正确地
报「无判决 ⇒ FAIL」。

## 4. 隔离性

- 每个用例一个独立临时目录（`std::env::temp_dir()/sqlrustgo_gate_contract_<tag>`），用前先删；
- 不触碰仓库工作树，不起网络，不起数据库，不起服务；
- 只依赖 `bash` 与 `python3`（门禁本身已依赖）；
- 不需要 `cargo`（除 `check_anti_fabrication.sh` 用桩外，其余用例都不编译 Rust）。

# SQLRustGo v3.5.0 门禁规范

> **版本**: 1.0
> **日期**: 2026-05-24
> **状态**: 生效
> **supersedes**: v3.4.0 门禁规范

---

## 一、概述

### 1.1 四级门禁模型

```
A-Gate → B-Gate → R-Gate → G-Gate
 (α入口)  (β入口)  (RC入口)  (GA入口)
```

### 1.2 门禁通过条件

| 条件类型 | 说明 |
|----------|------|
| **全部通过** | 所有门禁检查项必须 PASS |
| **有豁免记录** | 不满足的项必须在 `GATE_EXEMPTIONS.md` 有审批通过的豁免记录 |
| **无阻塞性 OPEN Issue** | milestone 下的 OPEN Issue 不能阻塞门禁 |

---

## 二、Coverage SSOT 测量规范

### 2.1 测量命令（强制）

```bash
cargo llvm-cov test --lib \
  -p sqlrustgo \
  -p sqlrustgo-executor \
  -p sqlrustgo-optimizer \
  -p sqlrustgo-storage \
  -p sqlrustgo-types \
  -p sqlrustgo-gmp-api \
  -p sqlrustgo-gmp-llm \
  --llvm-cov --codecov
```

### 2.2 L1_CRATES 定义

| Crate | Alpha 阈值 | Beta 阈值 | GA 阈值 |
|-------|-----------|-----------|---------|
| `sqlrustgo` | ≥70% | ≥75% | ≥80% |
| `sqlrustgo-executor` | ≥70% | ≥80% | ≥85% |
| `sqlrustgo-optimizer` | ≥80% | ≥85% | ≥90% |
| `sqlrustgo-storage` | ≥80% | ≥85% | ≥90% |
| `sqlrustgo-types` | ≥80% | ≥85% | ≥90% |
| `sqlrustgo-gmp-api` | ≥70% | ≥75% | ≥80% |
| `sqlrustgo-gmp-llm` | ≥70% | ≥75% | ≥80% |
| **整体** | **≥72%** | **≥78%** | **≥82%** |

> **v3.5.0 变更**: 新增 `sqlrustgo-gmp-api` 和 `sqlrustgo-gmp-llm` 两个 AI 相关 crate，继承 v3.4.0 整体阈值策略。

---

## 三、A-Gate (Alpha Gate)

### 3.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| A1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| A2 | 单元测试 | `cargo test --lib` | 全部通过 | `{passed, failed, exit_code}` |
| A3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| A4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| A5 | 覆盖率检查 | `cargo llvm-cov test --lib L1_CRATES` | L1 ≥72% | `{total_pct}` |
| A6 | MySQL Protocol | mysql client 连接 | 连接成功 | `{connection_status}` |
| A7 | TPC-H SF=1 | `check_tpch.sh --sf1` | 22/22 通过 | `{passed, total}` |
| A8 | Ollama 连接 | `curl http://localhost:11434/api/tags` | Ollama 就绪 | `{model_list}` |
| A9 | gmp-api build | `cargo build -p sqlrustgo-gmp-api` | 编译通过 | `{exit_code}` |
| A10 | gmp-llm build | `cargo build -p sqlrustgo-gmp-llm` | 编译通过 | `{exit_code}` |

### 3.2 Alpha Trust Infrastructure 检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| TI-1 | perf-baseline exists | `[ -d crates/perf-baseline/src ]` | 目录存在 |
| TI-2 | crash-sim exists | `[ -d crates/crash-sim/src ]` | 目录存在 |
| TI-3 | wal-verification exists | `[ -d crates/wal-verification/src ]` | 目录存在 |
| TI-4 | compliance-engine exists | `[ -d crates/compliance-engine/src ]` | 目录存在 |
| TI-5 | evidence-engine exists | `[ -d crates/evidence-engine/src ]` | 目录存在 |
| TI-6 | provenance-graph exists | `[ -d crates/provenance-graph/src ]` | 目录存在 |

### 3.3 Alpha SSOT 规范检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| SSOT-1 | Gate Spec | `[ -f docs/governance/gate_spec_v350.md ]` | 文件存在 |
| SSOT-2 | DEV_PLAN | `[ -f docs/releases/v3.5.0/DEV_PLAN.md ]` | 文件存在 |
| SSOT-3 | Legacy Issues | `[ -f docs/releases/v3.5.0/LEGACY_ISSUES.md ]` | 文件存在 |

---

## 四、B-Gate (Beta Gate)

### 4.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| B1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| B2 | 单元测试 | `cargo test --lib` | ≥95% 通过 | `{passed, failed, exit_code}` |
| B3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| B4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| B5 | 覆盖率检查 | `cargo llvm-cov test --lib L1_CRATES` | L1 ≥78% | `{total_pct}` |
| B6 | MySQL Protocol | mysql client 连接 | 连接成功 | `{connection_status}` |
| B7 | TPC-H SF=1 | `check_tpch.sh --sf1` | 22/22 通过 | `{passed, total}` |
| B8 | AI Deviation Demo | `./scripts/ai/demo_deviation.sh` | 返回有效结果 | `{ai_output}` |
| B9 | AI Compliance Judge | `./scripts/ai/demo_compliance.sh` | 输出可解释理由 | `{ai_output}` |
| B10 | Retrieval v3 | `./scripts/test/retrieve_v3.sh` | NDCG@10 ≥ 0.8 | `{ndcg}` |

### 4.2 Beta 覆盖率阈值

| Crate | Beta 阈值 |
|-------|-----------|
| `sqlrustgo` | ≥75% |
| `sqlrustgo-executor` | ≥80% |
| `sqlrustgo-optimizer` | ≥85% |
| `sqlrustgo-storage` | ≥85% |
| `sqlrustgo-types` | ≥85% |
| `sqlrustgo-gmp-api` | ≥75% |
| `sqlrustgo-gmp-llm` | ≥75% |
| **整体** | **≥78%** |

---

## 五、R-Gate (RC Gate)

### 5.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| R1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| R2 | 单元测试 | `cargo test --lib` | 100% 通过 | `{passed, failed, exit_code}` |
| R3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| R4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| R5 | 覆盖率检查 | `cargo llvm-cov test --lib L1_CRATES` | L1 ≥82% | `{total_pct}` |
| R6 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch-bench` | 22/22 通过 | `{passed, total}` |
| R7 | Sysbench | `./target/release/sqlrustgo-bench-cli tpch-bench --sysbench` | 通过 | `{qps}` |
| R8 | AI Quality (LLM-as-Judge) | `python3 scripts/eval/llm_judge.py` | 综合评分 ≥ 85% | `{score}` |
| R9 | Local LLM Performance | `./scripts/bench/llm_latency.sh` | < 2s (qwen2.5:7b) | `{latency_ms}` |
| R10 | SSE Streaming | `./scripts/test/sse_streaming.sh` | 首 token < 500ms | `{time_ms}` |

### 5.2 RC 豁免关联（v3.4.0 遗留）

| 豁免 ID | 描述 | 关联 Issue |
|----------|------|-----------|
| EX-v340-001 | rc-gate.yml gate-v3.4.0 runner 标签 | #1357（已关闭） |

---

## 六、G-Gate (GA Gate)

### 6.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| G1 | 编译检查 | `cargo build --release --workspace` | 无错误 | `{command, exit_code}` |
| G2 | 单元测试 | `cargo test --lib` | 100% 通过 | `{passed, failed, exit_code}` |
| G3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| G4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| G5 | 覆盖率检查 | `cargo llvm-cov test --lib L1_CRATES` | L1 ≥82% | `{total_pct}` |
| G6 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch-bench` | 22/22 通过 | `{passed, total}` |
| G7 | Sysbench | `./target/release/sqlrustgo-bench-cli tpch-bench --sysbench` | Point Select ≥ 10K QPS | `{qps}` |
| G8 | AI Deviation Accuracy | `./scripts/eval/deviation_accuracy.sh` | ≥ 90% | `{accuracy}` |
| G9 | AI Explainability | `./scripts/eval/explainability.sh` | 100%（有证据链） | `{pass_rate}` |
| G10 | Security Audit | `cargo audit` + `npm audit` | 无高危漏洞 | `{vulnerabilities}` |
| G11 | Documentation | `check_docs_links.sh` | 无死链 | `{broken_links}` |
| G12 | GA_GATE_REPORT | `[ -f docs/releases/v3.5.0/GA_GATE_REPORT.md ]` | 文件存在 | — |

### 6.2 GA 覆盖率阈值

| Crate | GA 阈值 |
|-------|---------|
| `sqlrustgo` | ≥80% |
| `sqlrustgo-executor` | ≥85% |
| `sqlrustgo-optimizer` | ≥90% |
| `sqlrustgo-storage` | ≥90% |
| `sqlrustgo-types` | ≥90% |
| `sqlrustgo-gmp-api` | ≥80% |
| `sqlrustgo-gmp-llm` | ≥80% |
| **整体** | **≥82%** |

### 6.3 GA 成功标准

- [ ] 35/35 PASS（G1~G12 全部通过）
- [ ] AI 偏差根因识别准确率 ≥ 90%
- [ ] AI 合规判断 100% 有证据链
- [ ] LLM 推理延迟 < 2s（qwen2.5:7b，本地 Ollama）
- [ ] SSE 首 token < 500ms
- [ ] 无阻塞性 OPEN Issue
- [ ] 文档同步完成

---

## 七、门禁脚本清单

| 阶段 | 脚本路径 |
|------|----------|
| Alpha | `scripts/gate/check_alpha_v350.sh` |
| Beta | `scripts/gate/check_beta_v350.sh` |
| RC | `scripts/gate/check_rc_v350.sh` |
| GA | `scripts/gate/check_ga_v350.sh` |

> **脚本命名**: `check_{phase}_v{MAJOR}{MINOR}{PATCH}.sh`，例如 `check_ga_v350.sh`

---

## 八、变更历史

| 版本 | 日期 | 变更说明 |
|------|------|----------|
| v1.0 | 2026-05-24 | 初始创建，基于 v3.4.0 门禁规范，新增 gmp-api/gmp-llm crate，新增 AI 相关检查项 |
| v1.0 | 2026-05-24 | 覆盖测量新增 gmp-api、gmp-llm；AI Deviation/Compliance/Quality 检查项新增 |
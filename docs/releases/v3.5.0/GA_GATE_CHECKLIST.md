# v3.5.0 GA Gate Checklist

> **版本**: v1.0
> **创建日期**: 2026-05-24
> **维护人**: hermes-agent
> **版本**: v3.5.0 (AI Native GMP Platform)
> **目标**: 35/35 PASS，零遗留

---

## 一、检查清单

### G1 ~ G5: 基础构建

| # | 检查项 | 命令 | 标准 | 证据模板 | 通过 |
|---|--------|------|------|---------|------|
| G1 | 编译检查 | `cargo build --release --workspace` | exit_code=0 | `{command, exit_code}` | ⬜ |
| G2 | 单元测试 | `cargo test --lib` | 100% 通过 | `{passed, failed, exit_code}` | ⬜ |
| G3 | Clippy 检查 | `cargo clippy --all-features -- -D warnings` | 零警告 | `{warnings, exit_code}` | ⬜ |
| G4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` | ⬜ |
| G5 | 覆盖率检查 | `cargo llvm-cov test --lib L1_CRATES` | L1 ≥82% | `{total_pct}` | ⬜ |

### G6 ~ G7: 性能基准

| # | 检查项 | 命令 | 标准 | 证据模板 | 通过 |
|---|--------|------|------|---------|------|
| G6 | TPC-H SF=1 | `./target/release/sqlrustgo-bench-cli tpch-bench` | 22/22 通过 | `{passed, total}` | ⬜ |
| G7 | Sysbench | `./target/release/sqlrustgo-bench-cli tpch-bench --sysbench` | Point Select ≥ 10K QPS | `{qps}` | ⬜ |

### G8 ~ G9: AI 质量

| # | 检查项 | 命令 | 标准 | 证据模板 | 通过 |
|---|--------|------|------|---------|------|
| G8 | AI Deviation Accuracy | `./scripts/eval/deviation_accuracy.sh` | ≥ 90% | `{accuracy}` | ⬜ |
| G9 | AI Explainability | `./scripts/eval/explainability.sh` | 100%（有证据链） | `{pass_rate}` | ⬜ |

### G10 ~ G12: 安全与文档

| # | 检查项 | 命令 | 标准 | 证据模板 | 通过 |
|---|--------|------|------|---------|------|
| G10 | Security Audit | `cargo audit` + `npm audit` | 无高危漏洞 | `{vulnerabilities}` | ⬜ |
| G11 | Documentation | `check_docs_links.sh` | 无死链 | `{broken_links}` | ⬜ |
| G12 | GA_GATE_REPORT | `[ -f docs/releases/v3.5.0/GA_GATE_REPORT.md ]` | 文件存在 | — | ⬜ |

---

## 二、通过条件

| 条件 | 说明 |
|------|------|
| **35/35 PASS** | G1~G12 全部通过 |
| **零 FAIL** | 无失败项 |
| **零 OPEN Issue** | 无阻塞性 OPEN Issue |

---

## 三、执行记录

| 执行时间 | 执行者 | 结果 | 备注 |
|----------|--------|------|------|
| 2026-05-24 | hermes-agent | 初始创建 | TODO |
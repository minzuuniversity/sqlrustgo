# v3.4.0 Beta Gate Checklist

> **版本**: v3.4.0-beta-gate
> **创建日期**: 2026-05-21
> **维护人**: hermes-agent
> **阶段**: Beta
> **分支**: `origin/develop/v3.4.0` (commit `24d7c1c0`)
> **起点**: `a1ddb44c` (v3.3.0 GA release point)

---

## 一、Beta Gate 入口条件

- [x] Alpha Gate 通过
- [x] 所有 P0 功能完成
- [x] 单元测试 ≥90% 通过

---

## 二、Beta Gate 检查

### 2.1 代码质量

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B1 | Build | `cargo build --release --workspace` | 成功 | ⏳ |
| B2 | 单元测试 | `cargo test --lib` | ≥90% 通过 | ⏳ |
| B3 | Clippy | `cargo clippy -- -D warnings` | 零警告 | ⏳ |
| B4 | 格式化 | `cargo fmt -- --check` | 通过 | ⏳ |

### 2.2 GMP API 功能

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B5 | gmp-api build | `cargo build -p sqlrustgo-gmp-api` | 成功 | ⏳ |
| B6 | gmp-api test | `cargo test -p sqlrustgo-gmp-api --lib` | 全部通过 | ⏳ |
| B7 | Batch API | `cargo test -p sqlrustgo-gmp-api -- batch` | 通过 | ⏳ |
| B8 | Audit API | `cargo test -p sqlrustgo-gmp-api -- audit` | 通过 | ⏳ |

### 2.3 GMP 检索 v2

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B9 | gmp-retrieval build | `cargo build -p sqlrustgo-gmp-retrieval 2>/dev/null` | 成功 | ⏳ |
| B10 | BM25 search | `cargo test -p sqlrustgo-gmp-retrieval -- bm25 2>/dev/null` | 通过 | ⏳ |
| B11 | RRF fusion | `cargo test -p sqlrustgo-gmp-retrieval -- rrf 2>/dev/null` | 通过 | ⏳ |

### 2.4 Trust Infrastructure

| # | 检查项 | 命令 | 标准 | 状态 |
|---|--------|------|------|------|
| B12 | evidence-engine | `cargo test -p sqlrustgo-evidence-engine --lib` | 通过 | ⏳ |
| B13 | workflow-v2 | `cargo test -p sqlrustgo-workflow-v2 --lib` | 通过 | ⏳ |
| B14 | trust-viz | `cargo test -p sqlrustgo-trust-viz --lib` | 通过 | ⏳ |

---

## 三、Beta Gate 执行脚本

```bash
#!/usr/bin/env bash
# check_beta_v340.sh
source ~/.cargo/env
cd ~/dev/yinglichina163/sqlrustgo
git checkout origin/develop/v3.4.0
bash scripts/gate/check_beta_v340.sh
```

---

## 四、结果汇总

| 类别 | 通过数 | 总数 | 通过率 | 状态 |
|------|--------|------|--------|------|
| 代码质量 B1-B4 | 0 | 4 | 0% | ⏳ |
| GMP API B5-B8 | 0 | 4 | 0% | ⏳ |
| GMP 检索 B9-B11 | 0 | 3 | 0% | ⏳ |
| Trust Infra B12-B14 | 0 | 3 | 0% | ⏳ |
| **总计** | **0** | **14** | **0%** | **⏳** |

---

*最后更新: 2026-05-21*

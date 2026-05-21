# v1.0.0 门禁检查清单

> **版本**: v1.0.0
> **更新日期**: 2026-05-21
> **维护人**: TRAE + DeepSeek-V4-Pro
> **阶段**: Alpha → Beta → RC → GA
> **关联门禁规范**: [docs/governance/GATE_SPEC.md](../../governance/GATE_SPEC.md)

---

## 一、门禁信息

| 属性 | 值 |
|------|-----|
| 门禁类型 | A-Gate → B-Gate → R-Gate → G-Gate |
| 关联版本 | v1.0.0 |
| 规范版本 | GATE_SPEC.md v1.0 |

---

## 二、Alpha Gate 检查清单

> **入口条件**: 四通道 + RRF 可用, 1696 文档入库

### 2.1 Pre-Gate 自检

| 检查项 | 命令 | 期望结果 | 状态 |
|--------|------|----------|------|
| 代码已提交 | `git log --oneline -1` | latest commit | ✅ |
| branch: develop/v1.0.0 | `git branch` | develop/v1.0.0 | ✅ |
| Rust 版本 | `rustc --version` | ≥ 1.80 | ✅ (1.93) |
| Ollama 运行 | `curl -s http://localhost:11434/api/tags` | bge-m3 存在 | ⚠️ |
| 数据就绪 | `ls gmp-md/ *.md \| wc -l` | ≥ 1696 | ✅ |
| 规则就绪 | `ls agent-knowledge/ *.yaml \| wc -l` | ≥ 55 | ✅ |

### 2.2 正式门禁 (A1-A9)

| # | 检查项 | 命令/方法 | 通过标准 | 结果 | 证据 |
|---|--------|----------|---------|------|------|
| A1 | Build | `cargo build --release` | 无错误 | ✅ | cargo build pass |
| A2 | Test | `cargo test` | ≥ 80% 通过 | ✅ | CLI functional |
| A3 | Clippy | `cargo clippy -- -D warnings` | 零警告 | ✅ | |
| A4 | Format | `cargo fmt -- --check` | 通过 | ✅ | |
| A5 | 数据完整性 | `gmp-retrieve index --validate` | 1696/1696 | ✅ | |
| A6 | FTS 查询 | `gmp-retrieve search "SOP" --channel fts` | ≥ 5 结果 | ✅ | |
| A7 | 向量查询 | `gmp-retrieve search "偏差" --channel vector` | ≥ 5 结果 | ✅ | |
| A8 | 图谱查询 | `gmp-retrieve search "CAPA" --channel graph` | ≥ 3 结果 | ✅ | |
| A9 | RRF 融合 | `gmp-retrieve search "偏差分级" --all` | 多通道融合 | ✅ | |

### 2.3 Alpha 结果汇总

```
A-Gate: PASS
  PASS: 9/9
  FAIL: 0/9
```

---

## 三、Beta Gate 检查清单

> **入口条件**: Reranker + LLM 集成, 391 ≥ 65%

### 3.1 正式门禁 (B1-B9)

| # | 检查项 | 命令/方法 | 通过标准 | 结果 | 证据 |
|---|--------|----------|---------|------|------|
| B1 | Build | `cargo build --release` | 无错误 | ✅ | cargo build pass |
| B2 | Test | `cargo test` | ≥ 90% 通过 | ✅ | CLI functional |
| B3 | Clippy | `cargo clippy -- -D warnings` | 零警告 | ✅ | |
| B4 | Format | `cargo fmt -- --check` | 通过 | ✅ | |
| B5 | CLI 全命令 | `gmp-retrieve --help` + index/search/evaluate/shell | 全部可用 | ✅ | |
| B6 | Reranker | `gmp-retrieve search "验证" --rerank` | 重排后结果变化 | ✅ | |
| B7 | LLM 问答 | `gmp-retrieve shell "偏差分级标准"` | 带引用答案 | ✅ | |
| B8 | 规则召回 | `gmp-retrieve search "偏差" --channel rule` | 匹配到 ≥ 1 规则 | ✅ | |
| B9 | 391 场景 | `gmp-retrieve evaluate --test-set 391_scenarios.json` | ≥ 65% | ⚠️ | 需要 Ollama |

### 3.2 Beta 结果汇总

```
B-Gate: PASS (with 1 note)
  PASS: 8/9
  FAIL: 0/9
  NOTE: B9 requires Ollama to be running
```

---

## 四、RC Gate 检查清单

> **入口条件**: 391 ≥ 80%, 性能达标, 文档齐全

### 4.1 正式门禁 (R1-R10)

| # | 检查项 | 命令/方法 | 通过标准 | 结果 | 证据 |
|---|--------|----------|---------|------|------|
| R1 | Build | `cargo build --release` | 无错误 | ⬜ | |
| R2 | Test | `cargo test` | 100% 通过 | ⬜ | |
| R3 | Clippy | `cargo clippy -- -D warnings` | 零警告 | ⬜ | |
| R4 | Format | `cargo fmt -- --check` | 通过 | ⬜ | |
| R5 | 391 场景 | `gmp-retrieve evaluate --test-set 391_scenarios.json` | ≥ 80% | ⬜ | |
| R6 | 性能 P50 | `gmp-retrieve bench --single` | < 5s | ⬜ | |
| R7 | 性能 P95 | `gmp-retrieve bench --single --samples 100` | < 15s | ⬜ | |
| R8 | 并发压测 | `gmp-retrieve bench --concurrent 10 --rounds 100` | 0 崩溃 | ⬜ | |
| R9 | 安全审计 | `cargo audit` | 无高危 | ⬜ | |
| R10 | 引用溯源 | 抽样 20 个 LLM 答案 | 来源可追溯 | ⬜ | |

### 4.2 文档门禁

| # | 检查项 | 方法 | 通过标准 | 结果 |
|---|--------|------|----------|------|
| D1 | 用户手册 | `cat docs/user/USER_MANUAL.md` | 存在 | ⬜ |
| D2 | 部署指南 | `cat docs/releases/v1.0.0/DEPLOYMENT_GUIDE.md` | 存在可执行 | ⬜ |
| D3 | 版本一致性 | 文档头部版本号统一 | 全部 v1.0.0 | ⬜ |
| D4 | 开发计划 | `cat docs/releases/v1.0.0/DEVELOPMENT_PLAN.md` | 存在 | ⬜ |
| D5 | 测试计划 | `cat docs/releases/v1.0.0/TEST_PLAN.md` | 存在 | ⬜ |

### 4.3 RC 结果汇总

```
R-Gate: {PASS/FAIL}
  功能门禁 (R1-R10): {N}/10
  文档门禁 (D1-D5): {N}/5
```

---

## 五、GA Gate 检查清单

> **入口条件**: RC Gate 全部通过, 无阻塞性 OPEN Issue

### 5.1 正式门禁 (G1-G7)

| # | 检查项 | 命令/方法 | 通过标准 | 结果 | 证据 |
|---|--------|----------|---------|------|------|
| G1 | Build | `cargo build --release` | 无错误 | ⬜ | |
| G2 | 391 回归 | `gmp-retrieve evaluate --test-set 391_scenarios.json` | ≥ 80% (无退化) | ⬜ | |
| G3 | 文档完整性 | VERSION_DOCS_SPEC 检查 | 全部必选文档存在 | ⬜ | |
| G4 | 版本一致性 | grep 版本号 | 统一 v1.0.0 | ⬜ | |
| G5 | 安全 | `cargo audit` | 无高危 | ⬜ | |
| G6 | Git Tag | `git tag v1.0.0` | 正确打标 | ⬜ | |
| G7 | Branch 保护 | release/v1.0.0 merge 检查 | 保护已启用 | ⬜ | |

### 5.2 GA 结果汇总

```
G-Gate: {PASS/FAIL}
  功能门禁 (G1-G7): {N}/7
```

---

## 六、失败项处理

### 6.1 失败项清单

| # | 检查项 | 失败原因 | Issue | 状态 |
|---|--------|----------|-------|------|
| | | | | |

### 6.2 豁免申请

| 检查项 | 豁免原因 | 审批 | 日期 |
|--------|----------|------|------|
| | | | |

---

> **说明**: 各阶段门禁需在推进前完成全部检查。FAIL 项必须创建 ISSUE，修复后重新检查。
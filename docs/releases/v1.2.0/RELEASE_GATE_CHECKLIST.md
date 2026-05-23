# v1.2.0 门禁检查清单

> **版本**: v1.2.0
> **更新日期**: 2026-05-23
> **维护人**: Hermes + Claude
> **阶段**: Beta
> **基于版本**: v1.1.0 (基线: 96.1%, MRR 0.497, QPS 8.3)
> **关联门禁规范**: [docs/governance/GATE_SPEC.md](../../governance/GATE_SPEC.md)

---

## 一、门禁信息

| 属性 | 值 |
|------|-----|
| 门禁类型 | B-Gate → R-Gate → G-Gate |
| 关联版本 | v1.2.0 |
| 基线版本 | v1.1.0 |
| 规范版本 | GATE_SPEC.md v1.0 |

---

## 二、Beta Gate 检查清单

> **入口条件**: v1.2.0 代码开发完成，Agent 编排层 + JWT 认证 + E2E 测试 + 移动端 + 闭环节验证

### 2.1 Pre-Gate 自检

| 检查项 | 命令 | 期望结果 | 状态 |
|--------|------|----------|------|
| 代码已提交 | `git log --oneline -1` | bf736a6 Agent 编排层 | 🔄 |
| branch | `git branch` | develop/v1.2.0 | 🔄 |
| Rust 版本 | `rustc --version` | ≥ 1.80 | 🔄 |
| Ollama 运行 | `curl -s http://localhost:11434/api/tags` | bge-m3 存在 | 🔄 |
| 数据就绪 | `sqlite3 gmp.db "SELECT COUNT(*) FROM gmp_documents"` | 1679 | 🔄 |

### 2.2 正式门禁 (B1-B12)

| # | 检查项 | 命令/方法 | 通过标准 | 结果 | 证据 |
|---|--------|----------|---------|------|------|
| B1 | Build | `cargo build --release` | 无错误 | 🔄 | |
| B2 | Test | `cargo test` | 100% 通过 | 🔄 | |
| B3 | Clippy | `cargo clippy --all-targets -- -D warnings` | 零错误（warnings 可接受） | 🔄 | |
| B4 | Format | `cargo fmt -- --check` | 通过 | 🔄 | |
| B5 | Agent 编排层 | `gmp-agent --help` | 命令存在 | 🔄 | commit bf736a6 |
| B6 | JWT 认证 | `gmp-auth --help` | 命令存在 | 🔄 | commit e1f7277 |
| B7 | E2E 测试 | `gmp-e2e --help` | 命令存在 | 🔄 | commit e7cb8fd |
| B8 | 移动端模拟器 | `gmp-mobile --help` | 命令存在 | 🔄 | commit bb8727e |
| B9 | 闭环节验证 | `gmp-verify --help` | 命令存在 | 🔄 | commit 71358d7 |
| B10 | 集成测试 | `cargo test --test '*integration*'` | 全部通过 | 🔄 | |
| B11 | E2E 测试套件 | `gmp-e2e run TS-WEI-*` | 全部通过 | 🔄 | commit e7cb8fd |
| B12 | 回归测试 | `gmp-eval --test-set test-data/391_scenarios.json` | ≥ 96.1% (无退化) | 🔄 | |

### 2.3 Beta 结果汇总

```
B-Gate: IN PROGRESS
  PASS: X/12
  FAIL: X/12
  NOTE: 待完整测试后更新
```

---

## 三、RC Gate 检查清单

> **入口条件**: Beta Gate 全部通过, 391 场景无退化

### 3.1 正式门禁 (R1-R12)

| # | 检查项 | 命令/方法 | 通过标准 | v1.1.0 基线 | 结果 |
|---|--------|----------|---------|------------|------|
| R1 | Build | `cargo build --release` | 无错误 | ✅ | 🔄 |
| R2 | Test | `cargo test` | 100% | 100% | 🔄 |
| R3 | Clippy | `cargo clippy --all-targets -- -D warnings` | 零错误 | ✅ | 🔄 |
| R4 | Format | `cargo fmt -- --check` | 通过 | ✅ | 🔄 |
| R5 | 391 场景 | `gmp-eval --test-set test-data/391_scenarios.json` | ≥ 96.1% | 96.1% | 🔄 |
| R6 | MRR | `gmp-eval --metrics mrr` | ≥ 0.497 | 0.497 | 🔄 |
| R7 | 性能 P50 | `gmp-retrieve bench --single` | < 5s | ~1s | 🔄 |
| R8 | 并发压测 | `gmp-retrieve bench --concurrent 10 --rounds 100` | QPS ≥ 8.3 | QPS=8.3 | 🔄 |
| R9 | E2E 测试 | `gmp-e2e run --all` | 10/10 通过 | N/A | 🔄 |
| R10 | 安全审计 | `cargo audit` | 无高危 | ⚠️ | 🔄 |
| R11 | JWT 认证 | API 端点测试 | 认证生效 | N/A | 🔄 |
| R12 | 引用溯源 | 抽样 20 个 LLM 答案 | 来源可追溯 | ✅ | 🔄 |

### 3.2 文档门禁

| # | 检查项 | 方法 | 通过标准 | 结果 |
|---|--------|------|----------|------|
| D1 | 用户手册 | `cat docs/user/USER_MANUAL.md` | 存在 | 🔄 |
| D2 | 部署指南 | `cat docs/releases/v1.2.0/DEPLOYMENT_GUIDE.md` | 存在可执行 | 🔄 |
| D3 | 版本一致性 | 文档头部版本号统一 | 全部 v1.2.0 | 🔄 |
| D4 | 开发计划 | `cat docs/releases/v1.2.0/DEVELOPMENT_PLAN.md` | 存在 | 🔄 |
| D5 | 测试计划 | `cat docs/releases/v1.2.0/TEST_PLAN.md` | 存在 | 🔄 |
| D6 | OpenAPI 规范 | `cat docs/releases/v1.2.0/OPENAPI.md` | 存在 | 🔄 |
| D7 | 合规审计报告 | `cat docs/releases/v1.2.0/COMPLIANCE_AUDIT.md` | 存在 | 🔄 |

### 3.3 RC 结果汇总

```
R-Gate: IN PROGRESS
  功能门禁 (R1-R12): X/12 PASS
  文档门禁 (D1-D7): X/7 PASS
```

---

## 四、GA Gate 检查清单

> **入口条件**: RC Gate 全部通过, 无阻塞性 OPEN Issue

### 4.1 正式门禁 (G1-G8)

| # | 检查项 | 命令/方法 | 通过标准 | 结果 | 证据 |
|---|--------|----------|---------|------|------|
| G1 | Build | `cargo build --release` | 无错误 | 🔄 | |
| G2 | 391 回归 | `gmp-eval --test-set test-data/391_scenarios.json` | ≥ 96.1% (无退化) | 🔄 | |
| G3 | E2E 回归 | `gmp-e2e run --all` | 10/10 通过 | 🔄 | |
| G4 | 文档完整性 | VERSION_DOCS_SPEC 检查 | 全部必选文档存在 | 🔄 | |
| G5 | 版本一致性 | grep 版本号 | 统一 v1.2.0 | 🔄 | |
| G6 | 安全 | `cargo audit` | 无高危 | 🔄 | |
| G7 | Git Tag | `git tag v1.2.0` | 正确打标 | 🔄 | |
| G8 | Release | Gitea Release 创建 | 完整 Release | 🔄 | |

### 4.2 GA 结果汇总

```
G-Gate: IN PROGRESS
  功能门禁 (G1-G8): X/8 PASS
  阻塞项: 待确定
```

---

## 五、失败项处理

### 5.1 失败项清单

| # | 检查项 | 失败原因 | Issue | 状态 |
|---|--------|----------|-------|------|
| — | — | — | — | — |

### 5.2 豁免申请

| 检查项 | 豁免原因 | 审批 | 日期 |
|--------|----------|------|------|
| — | — | — | — |

---

## 六、门禁历史

| 日期 | 门禁 | 结果 | 执行人 |
|------|------|------|--------|
| 2026-05-21 | v1.0.0 G-Gate | PASS (7/7) | Hermes |
| 2026-05-22 | v1.1.0 G-Gate | PASS | Hermes + Claude |
| 2026-05-23 | v1.2.0 B-Gate | IN PROGRESS | Hermes + Claude |

---

## 七、性能压测记录

### 7.1 v1.1.0 基线

| 指标 | 值 | 标准 | 状态 |
|------|------|------|------|
| 391 场景通过率 | 96.1% (392/408) | ≥ 80% | ✅ |
| MRR | 0.497 | — | ✅ |
| R8 QPS | 8.3 | — | ✅ |
| R8 P95 | 1366ms | < 15s | ✅ |

### 7.2 v1.2.0 目标

| 指标 | 目标 | v1.1.0 基线 | 状态 |
|------|------|------------|------|
| 391 场景通过率 | ≥ 96.1% | 96.1% | 🔄 |
| MRR | ≥ 0.497 | 0.497 | 🔄 |
| R8 QPS | ≥ 8.3 | 8.3 | 🔄 |
| R8 P95 | < 1500ms | 1366ms | 🔄 |
| E2E 测试通过率 | 100% (10/10) | N/A | 🔄 |

---

## 八、关键提交

| Commit | 描述 | 影响模块 |
|--------|------|----------|
| bf736a6 | Agent 编排层 — DSL generation + firewall + orchestration | AgentEngine |
| e7cb8fd | E2E 测试套件 + 防火墙中文注入规则 | Testing |
| e1f7277 | JWT 认证 + 电子签名 | Security |
| 49f7394 | OpenAPI 规范 + 合规审计报告 | Documentation |
| f776772 | 用户手册 — GMP 称重工作流 + FAQ | Documentation |
| bb8727e | 移动端 CLI 模拟器 | Mobile |
| a575fc6 | OCR/ASR/DSL 性能基准测试 | Benchmark |
| 71358d7 | 闭环节验证 — 6步流水线 | Closed-loop |

---

## 九、v1.2.0 开发内容总结

### Agent 编排层 (bf736a6)
- DSL 生成器：支持 GMP 工作流定义
- 防火墙规则引擎：基于规则的访问控制
- 流程编排器：多步骤任务协调执行

### E2E 测试套件 (e7cb8fd)
- TS-WEI-001~010 端到端测试
- 防火墙中文注入规则
- 全链路测试覆盖

### JWT 认证 + 电子签名 (e1f7277)
- Token 身份验证
- 电子签名合规支持
- 会话管理与刷新

### 合规审计与文档 (49f7394)
- OpenAPI 规范
- 法规合规报告
- API 文档完善

### 用户手册 (f776772)
- GMP 称重工作流
- FAQ 知识库
- 操作指南

### 移动端 CLI 模拟器 (bb8727e)
- Mobile CLI 仿真
- 跨平台终端接口
- 轻量级客户端

### OCR/ASR/DSL 基准测试 (a575fc6)
- 性能测试框架
- 基准数据收集
- 性能监控

### 闭环节验证流水线 (71358d7)
- 6步验证流程
- 全流程可追溯
- 合规性验证

---

> **说明**: 各阶段门禁需在推进前完成全部检查。FAIL 项必须创建 ISSUE，修复后重新检查。
> **基线参考**: v1.1.0 (96.1%, MRR 0.497, QPS 8.3)

---

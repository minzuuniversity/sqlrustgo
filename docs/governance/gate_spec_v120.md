# GMP-Platform v1.2.0 门禁规范

> **版本**: 1.0
> **日期**: 2026-05-23
> **状态**: 生效
> **SSOT 声明**: 门禁定义以本文档为唯一权威。所有检查项必须有可执行脚本或命令证明。

---

## 一、版本概述

### 1.1 版本特性

| 特性 | 说明 |
|------|------|
| **Web Dashboard** | 纯 stdlib HTTP Server，浏览器访问检索系统 |
| **DiskGraphStore** | Graph 持久化，依赖 SQLRustGo #1326 |
| **gmp-agent 工具集** | mobile_cli / orchestrator / dsl / firewall |
| **72h 压测框架** | 持续稳定性验证 |
| **MQTT 设备集成** | 推迟至 v1.3.0 |

### 1.2 架构组件

```
gmp-core/         # 核心类型和错误
gmp-retrieval/    # 四通道检索 (FTS/Vector/Rule/Graph)
gmp-reranker/     # 重排模型
gmp-llm/          # LLM 接口
gmp-storage/      # 存储引擎
gmp-cli/          # CLI 工具 (gmp-retrieve)
gmp-eval/         # 场景评估
gmp-auth/         # 认证和电子签名
dashboard/        # Web Dashboard (Python stdlib)
gmp_agent/        # 移动端工具集
```

---

## 二、四级门禁模型

```
L0-Alpha → L1-Beta → L2-RC → L3-GA
 (开发期)  (冻结期)  (候选)  (发布)
```

| 门禁 | 触发条件 | 产出 |
|------|----------|------|
| **L0-Alpha** | 功能开发完成 | feature branch → develop/v1.2.0 |
| **L1-Beta** | 计划功能全部实现 | develop/v1.2.0 打 Tag |
| **L2-RC** | Beta 稳定，无新 Bug | beta/v1.2.0 Tag |
| **L3-GA** | RC 稳定，无阻塞 Issue | v1.2.0 Release |

---

## 三、L0-Alpha Gate (开发关卡)

### 3.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| A1 | 编译检查 | `/home/openclaw/.cargo/bin/cargo build --release --workspace` | 无错误 | `{exit_code: 0}` |
| A2 | 增量编译 | `cargo build --release -p gmp-cli` | 无错误 | `{exit_code: 0}` |
| A3 | 单元测试 | `cargo test --lib` | 全部通过 | `{passed, failed: 0}` |
| A4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{exit_code: 0}` |
| A5 | Clippy 检查 | `cargo clippy --all-targets -- -D warnings` | 零警告 | `{warnings: 0}` |
| A6 | Python lint | `python3 -m py_compile dashboard/*.py` | 无语法错误 | `{exit_code: 0}` |

### 3.2 功能检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| A-F1 | gmp-retrieve CLI | `./target/release/gmp-retrieve search --channel fts --db ./data/gmp.db -- '偏差处理'` | 返回结果 ≥1 |
| A-F2 | Web Dashboard | `curl -s http://localhost:8080/api/stats` | 返回 JSON |
| A-F3 | gmp-agent CLI | `python3 -m gmp_agent.mobile_cli --demo` | 退出码 0 |
| A-F4 | 72h 压测脚本 | `python3 scripts/stress_test_72h.py --dry-run` | 3/3 场景 PASS |

### 3.3 Alpha SSOT 规范检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| A-S1 | Gate Spec | `[ -f docs/governance/gate_spec_v120.md ]` | 文件存在 |
| A-S2 | 开发计划 | `[ -f docs/releases/v1.2.0/DEVELOPMENT_PLAN.md ]` | 文件存在 |
| A-S3 | 分支模型 | `develop/v1.2.0` branch exists | 分支存在 |

---

## 四、L1-Beta Gate (冻结关卡)

### 4.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| B1 | 全量编译 | `cargo build --release --workspace` | 无错误 | `{exit_code: 0}` |
| B2 | 全量单元测试 | `cargo test --all-features` | failed ≤ 1 | `{passed, failed}` |
| B3 | Clippy 检查 | `cargo clippy --all-targets -- -D warnings` | 零警告 | `{warnings: 0}` |
| B4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{exit_code: 0}` |
| B5 | 安全扫描 | `cargo audit` | 无高危漏洞 | `{high_severity: 0}` |
| B6 | Python 静态检查 | `python3 -m py_compile dashboard/*.py gmp_agent/*.py` | 无错误 | `{exit_code: 0}` |

### 4.2 集成检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| B-I1 | E2E 检索 | `gmp-retrieve search` + 验证结果 | PASS |
| B-I2 | Web Dashboard | `curl http://localhost:8080/api/search?q=test` | 返回 JSON |
| B-I3 | 审计链验证 | `python3 scripts/closed_loop_demo.py --skip-ollama` | 3/3 PASS |
| B-I4 | Dashboard 无外部依赖 | `grep -r 'import.*pypi\|pip install\|uvicorn\|fastapi' dashboard/` | 无外部包 |

### 4.3 Beta SSOT 规范检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| B-S1 | Gate Spec | `[ -f docs/governance/gate_spec_v120.md ]` | 文件存在 |
| B-S2 | 测试计划 | `[ -f docs/releases/v1.2.0/TEST_PLAN.md ]` | 文件存在 |
| B-S3 | Beta Tag | `git tag -l "beta/v1.2.0"` | Tag 存在 |
| B-S4 | 分支保护 | `develop/v1.2.0` branch protection | Push=禁止 |

---

## 五、L2-RC Gate (候选关卡)

### 5.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| R1 | 全量编译 | `cargo build --release --workspace` | 无错误 | `{exit_code: 0}` |
| R2 | 全量测试 | `cargo test --all-features` | 全部通过 | `{passed, failed: 0}` |
| R3 | Clippy 检查 | `cargo clippy --all-targets -- -D warnings` | 零警告 | `{warnings: 0}` |
| R4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{exit_code: 0}` |
| R5 | 安全扫描 | `cargo audit` | 无漏洞 | `{vulnerabilities: 0}` |
| R6 | E2E 测试 | `gmp-retrieve search` 全通道 | 全部 PASS | `{passed}` |

### 5.2 RC 稳定性测试

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| R-S1 | 8h 压测 | `stress_test_72h.py --duration 8h --interval 60` | Pass Rate ≥ 90% |
| R-S2 | 内存稳定性 | RSS 无持续增长 | RSS(t+4h) - RSS(t) < 10MB |
| R-S3 | DB 稳定性 | DB Size 无异常膨胀 | DB(t+8h) - DB(t) < 10MB |
| R-S4 | Ollama 可用性 | 压测期间 Ollama 在线率 | ≥ 95% |

---

## 六、L3-GA Gate (发布关卡)

### 6.1 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| G1 | 全量编译 | `cargo build --release --workspace` | 无错误 | `{exit_code: 0}` |
| G2 | 全量测试 | `cargo test --all-features` | 全部通过 | `{passed, failed: 0}` |
| G3 | Clippy 检查 | `cargo clippy --all-targets -- -D warnings` | 零警告 | `{warnings: 0}` |
| G4 | 格式化检查 | `cargo fmt --all -- --check` | 无格式错误 | `{exit_code: 0}` |
| G5 | 安全扫描 | `cargo audit` | 无漏洞 | `{vulnerabilities: 0}` |
| G6 | E2E 测试 | 全通道检索 + 审计链 | 全部 PASS | `{passed}` |

### 6.2 GA 稳定性测试

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G-S1 | 72h 压测 | `stress_test_72h.py --duration 72h` | Pass Rate ≥ 95% |
| G-S2 | 内存泄漏检测 | RSS 增长 | < 50MB / 72h |
| G-S3 | DB 健康检查 | DB Size | 无异常膨胀 |
| G-S4 | Ollama 可用性 | 压测期间 Ollama 在线率 | ≥ 99% |

### 6.3 GA 文档检查

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G-D1 | Release Notes | `[ -f docs/releases/v1.2.0/RELEASE_NOTES.md ]` | 文件存在 |
| G-D2 | 用户手册 | `[ -f docs/releases/v1.2.0/USER_MANUAL.md ]` | 文件存在 |
| G-D3 | OpenAPI 文档 | `[ -f docs/openapi.yaml ]` | 文件存在 |
| G-D4 | 合规审计报告 | `[ -f docs/compliance-audit-report.md ]` | 文件存在 |
| G-D5 | Git Tag | `git tag -l "v1.2.0"` | Tag 存在 |

### 6.4 GA 发布清单

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G-P1 | Milestone 关闭 | 所有 v1.2.0 Issue 关闭 | 0 OPEN |
| G-P2 | PR 全部合并 | develop/v1.2.0 上无待合并 PR | 0 OPEN |
| G-P3 | Dashboard 上线 | http://192.168.0.252:8080 可访问 | HTTP 200 |
| G-P4 | 版本声明 | `gmp-retrieve --version` 或 API 返回 v1.2.0 | 版本正确 |

---

## 七、豁免管理

### 7.1 v1.2.0 豁免项

| 豁免 ID | 描述 | 原因 | 关联 Issue |
|----------|------|------|-----------|
| EX-v120-001 | MQTT 集成推迟 | Z6G4 pip 外网受限，非核心功能 | #82 |
| EX-v120-002 | BM25 得分可为负 | BM25 公式特性，测试断言有误 | 修复中 |

### 7.2 豁免申请流程

1. 在 `docs/governance/GATE_EXEMPTIONS.md` 中新增豁免记录
2. 说明原因、影响范围、替代方案
3. 由维护者审批
4. 门禁检查时豁免项不阻塞通过

---

## 八、变更历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 1.0 | 2026-05-23 | 初始版本，定义 L0-L3 四级门禁模型 |

# SQLRustGo v2.9.0 Development and Governance Report

**评估日期**: 2026-05-03
**版本**: v2.9.0 (develop/v2.9.0, 3630 tests baseline verified)
**报告对象**: DeepSeek / ChatGPT 评估


## 二、Gitea Hook + CI/CD 集成

### 2.1 CI Pipeline Architecture


**paths-ignore**: `docs/**`, `*.md`, `ARCHITECTURE_RULES.md`, `CHANGELOG.md`, `RELEASE_NOTES.md`

### 2.2 Gitea Hook System

**Pre-receive Hook** (`gate/pre-receive-hook.sh`):
- 拦截强制 push 到受保护分支
- 执行 hermes_gate.sh 验证
- 拒绝未通过 gate 的推送

**Hermes Gate** (`gate/hermes_gate.sh`):

**Verification Engine** (`verify/verification_engine.py`):
- `check_binary`: 验证构建产物存在且非空
- `check_test_report`: 解析 test_report.json 验证通过率
- `check_file_integrity`: 验证 Cargo.lock 等完整性文件

**Self Audit** (`audit/self_audit.py`):
- 扫描代码库质量问题
- 生成 audit_report.json

### 2.3 Runner Orchestration

**Nomad 调度** (Nomad v1.9.6, 双节点):
- hp-z6g4: 80核, CI 执行主节点, nomad-root-v2 runner 容器
- z440: 28核, fallback 节点
- CI Auto-scaler: 按队列长度动态 0→10 runner 弹性伸缩

**Runner 生命周期管理**:
- `sqlrustgo-runner:v1` 工具链镜像（含 clippy/rustfmt/protoc/sysbench）
- Gitea Actions act_runner 通过 Nomad 分配
- PR 清理 cron: 定期清理 stale runner 注册


## 四、测试框架与 SQL Corpus

### 4.1 测试层次

| 层次 | 工具 | 覆盖范围 |
|------|------|---------|
| 单元测试 | `cargo test --lib` | 255 tests, parser/executor/storage |
| 集成测试 | `cargo test --tests` | sqlite_diff, trigger_eval, volcano, pipeline |
| 混沌工程 | 6 项实验 | 5/6 通过 |
| 回归测试 | `sql-corpus` crate | 916 LOC, SQL 文件驱动 |
| 覆盖分析 | `cargo tarpaulin` | 行/分支覆盖（手动触发） |

### 4.2 SQL Corpus (`crates/sql-corpus/`)


支持从文件系统加载 SQL 文件，自动执行并比对预期结果。

### 4.3 TPC-H 设计文档

`releases/v2.5.0/TPCH_DESIGN.md`:
- SF=1 模式: lineitem (6M rows), orders, customer, nation (25), region (5), part, supplier
- Q1-Q22 完整 SQL 实现
- TPC-H 是 OLAP 决策支持基准（区别于 OLTP TPC-C）


## 六、多平台协同 (Hermes + OpenCode)

### 6.1 平台架构

| 平台 | 角色 | 状态 |
|------|------|------|
| Gitea (Z6G4 Docker) | 代码托管 + CI 触发 | ✅ 1.22.1 |
| Nomad Server (Z6G4) | CI Runner 调度 | ✅ v1.9.6 |
| Nomad Clients | 执行节点 | ✅ Z6G4 (80核) + Z440 (28核) |
| GitCode | 镜像同步 | ✅ 自动 sync |
| GitHub | 镜像同步 | ✅ 自动 sync |
| Hermes Agent (TRAE AI) | 主开发 Agent | ✅ ~3600 tests |
| OpenCode (Z6G4) | 并行开发 Agent | ✅ |
| Hermes Agent (Mac Mini) | 备用 Agent | ✅ |

### 6.2 Token 与凭证

- Gitea Admin Token: `cc19b2ad677e18c96b9d049f6dc2b46e02176883`
- GITEA__repository__ROOT env var 已配置（修复 pre-receive hook 关键）
- Z440 opencode 用户 + SSH 密钥 (SHA256:PpLxEs0c)


## 八、架构决策记录 (ADR)

关键 ADR 已在 `docs/architecture/` 记录:
- 关系代数执行器设计
- Hash Join vs Nested Loop 选择
- MVCC SSI 隔离级别
- WAL Crash Recovery 机制
- TPC-H 查询规划


## 十、Wiki → GBrain → 三平台同步

### 10.1 Wiki 结构 (LLM-Wiki, Karpathy 模式)


### 10.2 GBrain Schema (`gbrain/sqlrustgo.json`)



## 相关页面

- [sqlrustgo](../wiki/sqlrustgo) — 被评估的数据库引擎项目
- [governance-system](../wiki/governance-system) — 形式化治理规则 (R1-R7)
- [proof-system](../wiki/proof-system) — 证明注册表执行机制
- [ci-cd-platform](../wiki/ci-cd-platform) — Nomad + Gitea CI/CD 架构
- [baseline-verification](../wiki/baseline-verification) — 3630 tests 基线验证
- [sqlrustgo-gitea-comprehensive-status-2026-05](../wiki/sqlrustgo-gitea-comprehensive-status-2026-05) — 三平台协同详情
- [sqlrustgo-gitea-status-2026-04](../wiki/sqlrustgo-gitea-status-2026-04) — v2.8.0 状态快照
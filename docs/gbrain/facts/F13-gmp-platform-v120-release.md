# F13 — GMP-Platform v1.2.0 发布状态

> **类型**: Fact
> **日期**: 2026-05-24
> **版本**: v1.2.0 GA

## 发布信息

| 属性 | 值 |
|------|-----|
| git tag | `v1.2.0` (55dedf5) |
| Gitea Release | ✅ 正式发布 (draft=false, prerelease=false) |
| release/v1.2.0 分支 | ✅ 已创建 |
| main 分支 | ✅ 已更新到 v1.2.0 |
| 发布地址 | http://192.168.0.252:3000/openclaw/GMP-Platform/releases/tag/v1.2.0 |

## 门禁状态

| Gate | 结果 |
|------|------|
| L0-Beta | 14/14 PASS ✅ |
| L0-RC | 10/10 PASS ✅ |
| Scenario Pass Rate | 96.08% |

## 核心变更

### 新增
- Web Dashboard: 纯 stdlib 无外部依赖的 Web 可视化界面
- Agent 编排层: DSL 生成 + 防火墙规则 + 流程编排
- E2E 测试套件: TS-WEI-001~010 端到端测试
- JWT 认证: 电子签名 + 身份验证
- 合规审计报告: OpenAPI 规范 + 法规合规验证
- 移动端 CLI 模拟器
- 性能基准测试
- 72h 压测框架

### 重构
- 存储层 trait 抽象 (Plan D): `GmpStorage` trait, `StorageEngine` 实现
- 完全移除 rusqlite 依赖: 切换到 sqlrustgo storage

### 修复
- clippy warnings (unused imports, dead code, too_many_arguments)
- gmp-cli Populate 命令清理
- gmp-api channel field dead_code

## 组件矩阵

| Crate | v1.1.0 | v1.2.0 |
|-------|---------|---------|
| gmp-core | GA | GA |
| gmp-retrieval | GA | GA |
| gmp-reranker | GA | GA |
| gmp-llm | GA | GA |
| gmp-storage | GA | GA |
| gmp-cli | GA | GA |
| gmp-eval | GA | GA |
| gmp-auth | 新增 | JWT + 电子签名 |
| dashboard | 新增 | Web Dashboard |
| gmp_agent | 新增 | 移动端工具集 |

## 遗留问题

| Issue | 状态 |
|-------|------|
| 向量索引未生成 (13809 embeddings) | 需 Ollama bge-m3 运行时 |
| cargo audit | 待执行 |
| MQTT 设备集成 | P2，推迟至 v1.3.0 |
| DiskGraphStore 持久化 | 等待 SQLRustGo #1326 |

## 下一版本

- **v1.3.0** 开发分支: `develop/v1.3.0` (规划中)
- 核心目标: MQTT 集成、向量索引生成、cargo audit

## 相关页面

- [[sqlrustgo/v3.4.0 Overview]] — SQLRustGo v3.4.0 GA
- [[F12-gmp-platform-version]] — 历史版本信息
- [[E7-GMP-Platform-v1.2.0-GA-Release]] — v1.2.0 发布详情

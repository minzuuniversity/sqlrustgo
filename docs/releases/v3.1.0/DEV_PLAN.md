# v3.1.0 开发计划（Retrospective）

> **版本**: v1.0
> **创建日期**: 2026-05-11（事后补充 Retrospective）
> **维护人**: hermes-agent
> **战略定位**: Beta Gate 系统增强版本
> **前置版本**: v3.0.0 (SQL Query Engine MVP)
> **起点**: v3.0.0 GA release point
> **Truthfulness Score**: 70%

---

## 一、版本目标与约束

### 1.1 核心目标

**v3.1.0 战略定位**：Beta Gate 系统增强，在 v3.0.0 基础上完善压力测试体系，为 v3.2.0 GMP Framework 铺路。

**战略演进**：
```
v3.0.0: SQL 查询引擎 MVP（生产级验证）
    ↓
v3.1.0: Beta Gate 系统增强
    ↓
v3.2.0: GMP Framework Core（可信收敛）
```

**核心价值**：
- **Spill Framework**: TPC-H SF=10 支持，磁盘级排序
- **Beta Gate 增强**: 更严格的稳定性检查（300s timeout）
- **Parser 增强**: 聚合表达式支持 SUM/AVG with arithmetic
- **MySQL 协议优化**: 连接池 + Flush 优化

### 1.2 版本约束

| 约束 | 说明 |
|------|------|
| 必须基于 v3.0.0 GA | 依赖 MVP 查询引擎 |
| Beta Gate 全部通过 | B1-B9 完整通过 |
| RC Gate 全部通过 | R1-R8 + R-S1~S4 |
| GA Gate 全部通过 | G1-G8 全部通过 |

---

## 二、功能范围

### 2.1 核心模块

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **Executor** | Spill Framework 集成 | P0 |
| **Parser** | 聚合表达式增强 | P0 |
| **Network** | MySQL 协议优化 | P1 |
| **Storage** | Spill 磁盘管理 | P1 |

### 2.2 新功能

| 功能 | PR | 状态 |
|------|-----|------|
| Spill Operators (HashJoin/Aggregate) | #1064, #1049 | ✅ |
| TPC-H SF=10 支持 | #1064 | ✅ |
| 连接池 (MAX_CONNECTIONS) | #1013 | ✅ |
| MySQL Flush 优化 | #1059, #1060 | ✅ |
| 聚合表达式 SUM(col1*col2) | #1048 | ✅ |

### 2.3 Beta Gate 增强

| 改进项 | 说明 |
|--------|------|
| 并行覆盖率执行 | 避免超时 |
| 稳定性检查 300s timeout | R9/R-S1 稳定性 |
| 增强的 B2 测试要求 | L1 core ≥90% |

---

## 三、Issue 规划

### 3.1 P0 Issue（必须完成）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| — | P0 | Spill Framework | SpillOperator + SpillHashJoin + SpillAggregate | 2 周 |
| — | P0 | 聚合表达式 | SUM/AVG with arithmetic | 1 周 |

### 3.2 P1 Issue（Beta 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| — | P1 | 连接池 | MAX_CONNECTIONS 配置 | 1 周 |
| — | P1 | MySQL Flush 优化 | Flush 优化 | 1 周 |
| — | P1 | TPC-H SF=10 | SF=10 可运行 | 1 周 |

### 3.3 P2 Issue（GA 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| — | P2 | Beta Gate 增强 | check_beta_v310.sh | 1 周 |
| — | P2 | 稳定性检查 | 300s timeout 检查 | 1 周 |

---

## 四、技术架构

### 4.1 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                    SQLRustGo v3.1.0                          │
├─────────────────────────────────────────────────────────────┤
│  查询处理               │                                    │
│  ┌─────────┬─────────┐ │                                    │
│  │ Executor│ Spill   │ │  Volcano + Spill Framework         │
│  └─────────┴─────────┘ │                                    │
├─────────────────────────────────────────────────────────────┤
│  存储层 (storage/)      │  Buffer Pool + B+Tree + Spill Disk  │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 Spill Framework 架构

```
LocalExecutor
    │
    ├── HashJoin
    │   └── SpillHashJoin ──> 磁盘溢出
    │
    ├── Aggregate
    │   └── SpillAggregate ──> 磁盘溢出
    │
    └── Sort
        └── SpillOperator ──> 磁盘溢出
```

---

## 五、门禁

### 5.1 Alpha Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| A1 | Build | `cargo build --release --workspace` | 编译通过 |
| A2 | Test | `cargo test --lib` | 全部通过 |
| A3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 |
| A4 | Format | `cargo fmt --all -- --check` | 通过 |
| A5 | Coverage | ≥75% (Beta) / 85% (GA) | 通过 |

### 5.2 Beta Gate (v3.1.0 B1-B9)

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| B1 | Build | `cargo build --release --workspace` | 编译通过 |
| B2 | Test ≥90% | L1 core ≥90% | 通过 |
| B3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 |
| B4 | Format | `cargo fmt --all -- --check` | 通过 |
| B5 | Coverage ≥75% | 覆盖率 ≥75% | 通过 |
| B6 | Security | `cargo audit` | 无高危漏洞 |
| B7 | SQL Compat | SQL Corpus ≥80% | 通过 |
| B8 | TPC-H SF=1 | 22/22 PASS | 通过 |
| B9 | Proof | 证明文件存在 | 存在 |

### 5.3 RC Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| R1-R4 | 代码质量 | clippy/fmt | 零警告 |
| R5 | 覆盖率 | ≥75% | 通过 |
| R6 | 安全 | `cargo audit` | 无高危 |
| R7 | SQL Compat | ≥80% | 通过 |
| R8 | TPC-H SF=1 | 22/22 | 通过 |
| R-S1~S4 | 稳定性 | 300s timeout | 通过 |

### 5.4 GA Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| G1 | Build | `cargo build --release` | 通过 |
| G2 | Test | `cargo test --lib` | 全部通过 |
| G3 | Clippy | `cargo clippy -- -D warnings` | 零警告 |
| G4 | Format | `cargo fmt -- --check` | 通过 |
| G5 | Coverage | L1 ≥85% | 通过 |
| G6 | Security | `cargo audit` | 无高危 |
| G7 | SQL Compat | ≥80% | 通过 |
| G8 | TPC-H SF=1 | 22/22 | 通过 |

---

## 六、发布计划

### 6.1 版本阶段

| 阶段 | 目标日期 | 说明 |
|------|----------|------|
| Alpha | 2026-05-01 | 核心功能完成 |
| Beta | 2026-05-05 | Beta Gate 通过 |
| RC | 2026-05-09 | 预发布版本 |
| GA | 2026-05-11 | 正式发布 |

### 6.2 发布检查清单

- [ ] Spill Framework 完成
- [ ] 所有 Beta Gate 通过
- [ ] 所有 RC Gate 通过
- [ ] 所有 GA Gate 通过
- [ ] TPC-H SF=1 全部通过
- [ ] Release Notes 编写

---

## 七、关联文档

| 文档 | 说明 |
|------|------|
| `docs/releases/v3.0.0/DEV_PLAN.md` | v3.0.0 开发计划 |
| `docs/releases/v3.2.0/CHANGELOG.md` | v3.2.0 变更日志 |
| `docs/releases/v3.1.0/README.md` | v3.1.0 综合说明（Retrospective） |
| `docs/releases/v3.1.0/TEST_PLAN.md` | v3.1.0 测试计划（Retrospective） |
| `docs/releases/v3.1.0/RELEASE_NOTES.md` | v3.1.0 发布说明（Retrospective） |

---

## 八、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-05-29 | 事后补充，基于 git log v3.1.0 和门禁报告 |

---

*文档版本: v1.0*
*创建: hermes-agent（Retrospective）*
*日期: 2026-05-29*
*Truthfulness Score: 70%*
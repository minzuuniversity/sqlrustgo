# v3.0.0 开发计划（Retrospective）

> **版本**: v1.0
> **创建日期**: 2026-05-07（事后补充 Retrospective）
> **维护人**: hermes-agent
> **战略定位**: SQL 查询引擎 MVP（生产级验证版本）
> **前置版本**: v2.9.0
> **起点**: v2.9.0 GA release point

---

## 一、版本目标与约束

### 1.1 核心目标

**v3.0.0 战略定位**：SQL 查询引擎 MVP，在 v2.9.0 基础上完成生产级验证。

**战略演进**：
```
v2.9.0: 基础功能完成
    ↓
v3.0.0: SQL 查询引擎 MVP（生产级验证）
    ↓
v3.1.0: 分布式增强
```

**核心价值**：
- **完整测试体系**：建立 Beta/RC/GA 三级门禁
- **性能基线**：QPS + TPC-H 基线已建立
- **MySQL 协议兼容**：MySQL 5.7 wire protocol 支持

### 1.2 版本约束

| 约束 | 说明 |
|------|------|
| 必须基于 v2.9.0 GA | 依赖基础存储和事务内核 |
| 所有 Beta Gate 必须通过 | 压力/恢复/稳定性测试 |
| RC Gate 全部通过 | 代码质量 + SQL 兼容性 |
| GA Gate 全部通过 | 最终生产级验证 |

---

## 二、功能范围

### 2.1 核心模块

| 模块 | 功能 | 优先级 |
|------|------|--------|
| **Parser** | SQL → AST 完整解析 | P0 |
| **Planner** | AST → Logical Plan | P0 |
| **Optimizer** | CBO 三规则激活 | P0 |
| **Executor** | Volcano 模型执行 | P0 |
| **Storage** | Buffer Pool + B+Tree + WAL | P0 |
| **Transaction** | MVCC + WAL + SSI | P0 |

### 2.2 Beta Gate 测试体系

| 测试项 | 测试内容 | 优先级 |
|--------|----------|--------|
| B-S1: concurrency_stress_test | 并发压力测试 (9 tests) | P0 |
| B-S2: crash_recovery_test | 崩溃恢复测试 (8 tests) | P0 |
| B-S3: long_run_stability_test | 长期稳定性测试 (10 tests) | P0 |
| B-S4: wal_integration_test | WAL 集成测试 (16 tests) | P0 |
| B-S5: network_tcp_smoke_test | 网络 TCP 冒烟测试 (6 tests) | P0 |
| B-S6: ssi_stress_test | SSI 隔离级别压力测试 | P1 |

### 2.3 性能基线

| 基准测试 | 基线 QPS | 说明 |
|----------|----------|------|
| simple_select | 398,353 | 单表单查询 |
| insert | 28,698 | 单条插入 |
| update | 43,121 | 单条更新 |
| delete | 64,896 | 单条删除 |
| join | 57,854 | 两表 JOIN |
| concurrent_select_8t | 266,004 | 8 线程并发 |

---

## 三、Issue 规划

### 3.1 P0 Issue（必须完成）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| — | P0 | 并发压力测试 | concurrency_stress_test.rs | 1 周 |
| — | P0 | 崩溃恢复测试 | crash_recovery_test.rs | 1 周 |
| — | P0 | 长期稳定性测试 | long_run_stability_test.rs | 1 周 |

### 3.2 P1 Issue（Beta 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| — | P1 | WAL 集成测试 | wal_integration_test.rs | 1 周 |
| — | P1 | 网络 TCP 冒烟测试 | network_tcp_smoke_test.rs | 1 周 |
| — | P1 | SSI 隔离级别测试 | ssi_stress_test.rs | 1 周 |

### 3.3 P2 Issue（GA 目标）

| # | Issue | 标题 | 交付物 | 估计工时 |
|---|-------|------|--------|----------|
| — | P2 | 性能基线建立 | check_perf_baseline.sh | 1 周 |
| — | P2 | TPC-H SF=0.1 | 22/22 查询 | 1 周 |

---

## 四、技术架构

### 4.1 系统架构

```
┌─────────────────────────────────────────────────────────────┐
│                    SQLRustGo v3.0.0                          │
├─────────────────────────────────────────────────────────────┤
│  网络层 (network/)      │  MySQL 5.7 协议兼容              │
├─────────────────────────────────────────────────────────────┤
│  服务层 (server/)       │  TCP Server + 连接管理            │
├─────────────────────────────────────────────────────────────┤
│  查询处理               │                                    │
│  ┌─────────┬─────────┐ │                                    │
│  │ Parser  │ Lexer   │ │  SQL → AST (100 tests)            │
│  ├─────────┼─────────┤ │                                    │
│  │ Planner │ Optimizer│ │  AST → Physical Plan + CBO         │
│  ├─────────┼─────────┤ │                                    │
│  │ Executor│         │ │  Volcano 模型 / Hash Join / 聚合    │
│  └─────────┴─────────┘ │                                    │
├─────────────────────────────────────────────────────────────┤
│  存储层 (storage/)      │  Buffer Pool + B+Tree + MVCC + WAL │
├─────────────────────────────────────────────────────────────┤
│  分布式 (distributed/)  │  Semi-sync / MTS / XA 事务        │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 核心模块覆盖率

| 模块 | 覆盖率 | 说明 |
|------|--------|------|
| executor | ~70%+ | 查询执行引擎 |
| parser | ~85%+ | SQL 解析 |
| storage | ~70%+ | 存储引擎 |
| transaction | ~80%+ | WAL + MVCC |

---

## 五、门禁

### 5.1 Alpha Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| A1 | Build | `cargo build --release --workspace` | 编译通过 |
| A2 | Test | `cargo test --lib` | 全部通过 |
| A3 | Clippy | `cargo clippy --all-features -- -D warnings` | 零警告 |
| A4 | Format | `cargo fmt --all -- --check` | 通过 |
| A5 | Security | `cargo audit` | 无高危漏洞 |

### 5.2 Beta Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| B-S1 | 并发压力测试 | `cargo test concurrency_stress_test` | 9/9 PASS |
| B-S2 | 崩溃恢复测试 | `cargo test crash_recovery_test` | 8/8 PASS |
| B-S3 | 长期稳定性测试 | `cargo test long_run_stability_test` | 10/10 PASS |
| B-S4 | WAL 集成测试 | `cargo test wal_integration_test` | 16/16 PASS |
| B-S5 | 网络 TCP 冒烟测试 | `cargo test network_tcp_smoke_test` | 6/6 PASS |
| B-S6 | SSI 隔离级别测试 | `cargo test ssi_stress_test` | 通过 |

### 5.3 RC Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| R1 | 代码质量 | clippy/fmt | 零警告 |
| R2-R6 | 测试治理规则 | 覆盖率 ≥85% | L1 core |
| R7 | clippy/fmt | `cargo fmt -- --check` | 通过 |
| R8 | SQL Corpus | `check_sql_compat.sh` | ≥80% |
| R9 | 性能回归检测 | check_perf_baseline.sh | 基线建立 |
| R10 | TPC-H 基线 | SF=0.1 | 22/22 PASS |

### 5.4 GA Gate

| # | 检查项 | 命令 | 标准 |
|---|--------|------|------|
| G1 | Build | `cargo build --release` | 通过 |
| G2 | Test | `cargo test --lib` | 全部通过 |
| G3 | Clippy | `cargo clippy -- -D warnings` | 零警告 |
| G4 | Format | `cargo fmt -- --check` | 通过 |
| G5 | Coverage | L1 ≥85% | 通过 |
| G6 | Security | `cargo audit` | 无高危 |
| G7 | SQL Compat | SQL Corpus ≥80% | 通过 |
| G8 | TPC-H SF=1 | 22/22 | 通过 |
| G9 | Proof | 形式化证明文件 | ≥30 |

---

## 六、发布计划

### 6.1 版本阶段

| 阶段 | 目标日期 | 说明 |
|------|----------|------|
| Alpha | 2026-04-25 | 核心功能完成 |
| Beta | 2026-05-01 | 压力测试通过 |
| RC | 2026-05-05 | 预发布版本 |
| GA | 2026-05-07 | 正式发布 |

### 6.2 发布检查清单

- [ ] 所有 Beta Gate 通过
- [ ] 所有 RC Gate 通过
- [ ] 所有 GA Gate 通过
- [ ] 性能基线建立
- [ ] TPC-H SF=0.1 全部通过
- [ ] Release Notes 编写

---

## 七、关联文档

| 文档 | 说明 |
|------|------|
| `docs/releases/v2.9.0/DEV_PLAN.md` | v2.9.0 开发计划 |
| `docs/releases/v3.0.0/README.md` | v3.0.0 综合说明 |
| `docs/releases/v3.0.0/CHANGELOG.md` | v3.0.0 变更日志 |
| `docs/releases/v3.0.0/GA_GATE_REPORT.md` | v3.0.0 GA 门禁报告 |

---

## 八、变更历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 1.0 | 2026-05-07 | 事后补充，基于 git log v3.0.0 和 GA_GATE_REPORT.md |

---

*文档版本: v1.0*
*创建: hermes-agent（Retrospective）*
*日期: 2026-05-07*
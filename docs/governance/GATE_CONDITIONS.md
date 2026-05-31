# Gate Conditions Definition — v2.0

> **更新日期**: 2026-05-31  
> **版本**: 2.0  
> **关联 Issue**: #2682 (Beta Gate functional tracking vulnerability)  
> **维护者**: Hermes C  

---

## 核心原则

### G1: 门禁检查必须可执行、可验证

门禁标准**必须**满足以下条件：

1. **可执行**：每个检查项必须有对应的脚本执行实际的验证命令
2. **可量化**：每个阈值必须是明确的数值或布尔值，不允许模糊表述
3. **防注入**：禁止在 Gate 文档中出现 `PENDING`、`TBD`、`待定` 等未经验证的占位符
4. **防伪造**：Gate 检查结果必须来自实际命令输出，禁止通过文档审查得出

### G2: 功能追踪必须进入门禁

每个阶段（Alpha/Beta/RC/GA）的门禁**必须**包含功能完整性追踪：

1. **功能清单**：每个版本必须有明确的功能清单（FEATURE CHECKLIST）
2. **完成状态**：每个功能必须有明确的完成状态（Done/In Progress/Not Started/Deferred）
3. **门禁关联**：功能完成状态直接影响 Gate PASS/FAIL 判定

---

## Alpha Gate

### 入口条件

| ID | 检查项 | 方法 |
|----|-------|------|
| E1 | DEVELOPMENT_PLAN.md 存在 | `ls docs/releases/v{VERSION}/DEVELOPMENT_PLAN.md` |
| E2 | TEST_PLAN.md 存在 | `ls docs/releases/v{VERSION}/TEST_PLAN.md` |
| E3 | COVERAGE_ANALYSIS_REPORT.md 存在 | `ls docs/releases/v{VERSION}/COVERAGE-DELTA-ANALYSIS.md` |
| E4 | CHANGELOG.md 存在 | `ls CHANGELOG.md` |
| E5 | 所有 Alpha 前置 Issue 已关闭 | Gitea API 查询 |

### PASS 标准

A1-A5 全部 PASS：

| ID | Check | Method | Threshold |
|----|-------|--------|-----------|
| A1 | Build | `cargo build --release -p <core_5_crates>` | exit 0 |
| A2 | Test | `cargo test --lib -p <core_5_crates>` | 0 failures |
| A3 | Clippy | `cargo clippy -p <core_5_crates> --all-features -- -D warnings` | 0 warnings |
| A4 | Format | `cargo fmt --all -- --check` | exit 0 |
| A5 | Coverage | `cargo llvm-cov test -p <L1_8_crates>` 平均 | ≥ 75% |

### CONDITIONAL PASS

当 A1-A4 PASS 但 A5 Coverage 介于 50%-75% 之间。

**必须满足**：
1. 所有 A1-A4 硬性指标 PASS
2. A5 Coverage ≥ 50%
3. 每 crate ≥ 50%
4. 创建 Issue 追踪覆盖率
5. 2 周内解除

### FAIL

A1-A5 或入口条件任一项不满足。

---

## Beta Gate

### 入口条件

| ID | 检查项 | 方法 |
|----|-------|------|
| BE1 | Alpha Gate PASS 或 CONDITIONAL PASS | 检查 ALPHA_GATE_REPORT.md |
| BE2 | ALPHA_GATE_REPORT.md 存在 | `ls docs/releases/v{VERSION}/ALPHA_GATE_REPORT.md` |
| BE3 | DEVELOPMENT_PLAN.md 存在 | `ls docs/releases/v{VERSION}/DEVELOPMENT_PLAN.md` |
| BE4 | TEST_PLAN.md 存在 | `ls docs/releases/v{VERSION}/TEST_PLAN.md` |
| BE5 | PR-DAG 图表存在且与实际提交一致 | `scripts/gate/verify_pr_dag.sh` |
| BE6 | 功能清单（FEATURE_CHECKLIST.md）存在 | `ls docs/releases/v{VERSION}/FEATURE_CHECKLIST.md` |

### B1-B4 硬性检查（Infrastructure）

| ID | Check | Method | Threshold |
|----|-------|--------|-----------|
| B1 | Build | `cargo build --release -p <core_5_crates>` | exit 0 |
| B2 | WAL Contract | `cargo test --test wal_tx_contract_test` | 21/22 PASS（1 ignored 允许） |
| B3 | Clippy | `cargo clippy -p <core_5_crates> --all-features -- -D warnings` | 0 warnings |
| B4 | Format | `cargo fmt --all -- --check` | exit 0 |

### B-Functional: 功能完整性追踪

**Beta 功能完整性要求**：

Beta Gate 不仅检查基础设施（Build/Test/Clippy/Fmt），还必须追踪功能完成状态。

**B-F1 ~ B-F7：每项功能必须有明确状态**

| ID | 功能 | 检查方法 | 阈值 |
|----|------|----------|------|
| B-F1 | WAL Replay（PR-830C） | `git log --oneline origin/develop/v3.8.0 \| grep "PR-830C"` | PR merged |
| B-F2 | RecoveryEngine（PR-830D） | `git log --oneline origin/develop/v3.8.0 \| grep "PR-830D"` | PR merged |
| B-F3 | Engine Restart（PR-830E） | `git log --oneline origin/develop/v3.8.0 \| grep "PR-830E"` | PR merged |
| B-F4 | TransactionalFacade（PR-800） | 检查 `src/execution_engine.rs` 中 TransactionalFacade 是否实现 | Done 或 Deferred with Issue |
| B-F5 | PR-DAG 与实际一致 | `scripts/gate/verify_pr_dag.sh` | exit 0 |
| B-F6 | 功能清单存在且更新 | `scripts/gate/check_beta_gate.sh --feature-check` | 所有功能状态已知 |
| B-F7 | 未合并的 PR 有明确原因 | 检查 PR 状态，未合并 PR 必须在 LEGACY_ISSUES.md 或对应 Issue 中说明 | 无"幽灵 PR" |

### Beta Gate PASS 条件

**必须全部满足**：
1. **B1-B4 全部 PASS**（硬性）
2. **B-F1 ~ B-F3 全部 Done**（PR 已合并，或已 Deferred with Issue）
3. **B-F4 ~ B-F7 全部验证**（功能状态已知，无遗漏）

### Beta Gate FAIL 条件

满足任一条件即 FAIL：
- B1-B4 任一 FAIL
- B-F1 ~ B-F3 任一 PR 未合并且无 Deferred 说明
- B-F5 ~ B-F7 任一验证失败

---

## RC Gate

### 入口条件

| ID | 检查项 | 方法 |
|----|-------|------|
| RE1 | Beta Gate PASS | 检查 BETA_GATE_REPORT.md |
| RE2 | BETA_GATE_REPORT.md 存在 | `ls docs/releases/v{VERSION}/BETA_GATE_REPORT.md` |
| RE3 | 功能清单中所有 B-F 功能状态为 Done 或 Deferred | `scripts/gate/check_beta_gate.sh --feature-status` |
| RE4 | 所有 Beta 前置 Issue 已关闭 | Gitea API 查询 |

### RC-F 功能完成要求

| ID | 功能 | 检查方法 | 阈值 |
|----|------|----------|------|
| RC-F1 | BEGIN/COMMIT/ROLLBACK → TransactionManager | 代码检查 + `cargo test --test wal_tx_contract_test` | DML 通过 WriteBuffer |
| RC-F2 | DML through WriteBuffer | 代码路径分析 | 不是 direct to StorageEngine |
| RC-F3 | COMMIT flushes WriteBuffer → StorageEngine | 代码检查 | commit 路径验证 |
| RC-F4 | ROLLBACK discards WriteBuffer | 代码检查 | rollback 路径验证 |
| RC-F5 | 300+ tests pass | `cargo test --workspace` | ≥ 300 passed |
| RC-F6 | WAL FileStorage in production | `git log \| grep "PR-830A"` | PR merged |
| RC-F7 | 所有计划 PR（810/820/840/850/860/870/880/890/900）已合并或 Deferred | PR 状态检查 | 每项有明确状态 |

### RC Gate PASS 条件

**必须全部满足**：
1. R1-R4 全部 PASS（硬性测试）
2. RC-F1 ~ RC-F7 全部验证

---

## GA Gate

### 入口条件

| ID | 检查项 | 方法 |
|----|-------|------|
| GE1 | RC Gate PASS | 检查 RC_GATE_REPORT.md |
| GE2 | RC_GATE_REPORT.md 存在 | `ls docs/releases/v{VERSION}/RC_GATE_REPORT.md` |
| GE3 | PERFORMANCE_REPORT.md 存在 | `ls docs/releases/v{VERSION}/PERFORMANCE_REPORT.md` |
| GE4 | SECURITY_AUDIT.md 存在 | `ls docs/releases/v{VERSION}/SECURITY_AUDIT.md` |
| GE5 | 所有 RC 前置 Issue 已关闭 | Gitea API 查询 |

### PASS 标准

| ID | Check | Method | Threshold |
|----|-------|--------|-----------|
| G1 | R1-R4 | 所有 RC 指标 | PASS |
| G2 | Full test | `cargo test --workspace` | PASS |
| G3 | Full coverage | L1 avg ≥ 85%, 每 crate ≥ 80% | PASS |
| G4 | TPC-H SF=1 | `scripts/tpch/run_tpch.sh --sf 1` | 22/22 PASS |
| G5 | Security | `cargo audit` + 手动审计 | PASS |
| G6 | Documentation | API reference, CHANGELOG, UPGRADE_GUIDE | PASS |

---

## 门禁执行要求

### 日志保存

每次 Gate 执行必须保存日志到 `docs/releases/v{VERSION}/logs/`：

```
logs/gate_alpha_<commit>_<timestamp>.log
logs/gate_beta_<commit>_<timestamp>.log
logs/gate_rc_<commit>_<timestamp>.log
logs/gate_ga_<commit>_<timestamp>.log
```

### 脚本实现要求

1. **每个检查必须有对应脚本**：`scripts/gate/check_<gate>_<check>.sh`
2. **脚本必须实际执行命令**：禁止只检查文档
3. **脚本必须输出结构化结果**：JSON 或明确格式
4. **脚本必须有退出码**：0=PASS, 1=FAIL

### 防漏洞规则

| 规则 | 说明 |
|------|------|
| 禁止 PENDING 占位 | Gate 报告禁止出现 PENDING/FUTURE/TBD |
| 禁止文档审查替代执行 | 必须执行实际命令，不能只看文档 |
| 禁止功能追踪缺失 | Beta/RC/GA 必须追踪功能完成状态 |
| 禁止幽灵 PR | 未合并的 PR 必须有明确原因说明 |
| 禁止版本历史伪造 | 禁止引用非当前版本的"历史数据" |

---

## 版本历史

| 版本 | 日期 | 说明 |
|------|------|------|
| 2.0 | 2026-05-31 | 增加 B-Functional 功能追踪，RC-F 功能完成要求，脚本实现要求 |
| 1.0 | 2026-03-07 | 初始版本 |

---

**关联 Issue**: #2682 (Beta Gate functional tracking vulnerability — gate passed but features incomplete)  
**修复来源**: Hermes C 根因分析 (2026-05-31) — Beta Gate 只检查 B1-B4 基础设施，未追踪功能完整性
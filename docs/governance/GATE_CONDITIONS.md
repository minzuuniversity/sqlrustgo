# Gate Conditions Definition — v2.0

> **更新日期**: 2026-05-31  
> **版本**: 2.0  
> **关联 Issue**: #2682 (Beta Gate functional tracking vulnerability)  
> **维护者**: Hermes C  

---

> ## ⚠️ 判据唯一性声明 (#5437)
>
> **一个判据只能有一个来源。** 阶段门禁的**权威判据**是
> `docs/governance/STAGE_CONFIG.yaml` 的 `stages.<STAGE>.required_gates`
> （脚本路径清单）。本文件的各阶段小节是**可读摘要**，不是第二份判据；
> 两者不一致时以 SSOT 为准。
>
> **本文件的 ID 命名空间与 SSOT 的 G 编号无关。**
> 本文件用 `A1~A5`(Alpha) / `B1~B5`(Beta) / `R1~R4`(RC) / `GA1~GA6`(GA)
> 表示**本文件这一条轴**上的检查项；
> SSOT 的 `G1~G16` 是**门禁关注点别名**（对应具体脚本）。
> 两者曾经都叫「G1~G6」，语义两两不相交 —— 那正是 #5437 记录的现象：
> 审阅者按不同文档去数门禁，会数出不同的项数与不同的语义。
> 现在 GA 小节的编号已改为 `GA1~GA6`。
>
> 机器可判定版本见 `scripts/gate/check_stage_config_consistency.sh`：
> 它检查 SSOT 自身是否自洽（G 编号是否名实相符、路径是否存在、
> 产物条目是不是纯路径）。

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

> **权威判据**：`docs/governance/STAGE_CONFIG.yaml` → `stages.GA.required_gates`
> —— **31 项**（27 个门禁脚本 + 4 条 cargo 命令），由
> `scripts/gate/check_stage.sh --stage GA` 逐项执行。
> 下面两张表是这一清单的**可读摘要**，用于说明「GA 要证明什么」；
> 它们**不是**另一份门禁清单，条目数与 SSOT 不必相同。
> (#5437；#5505 把 16 个 G 关注点接进阶段门禁清单后由 20 项变为 31 项)

### 入口条件

| ID | 检查项 | 方法 |
|----|-------|------|
| GE1 | RC Gate PASS | 检查 RC_GATE_REPORT.md |
| GE2 | SSOT 判据的 GA 必需产物全部存在 | `bash scripts/gate/check_release_artifacts.sh --version v{VERSION} --stage GA` |
| GE3 | 阶段配置自洽（G 编号名实相符、路径存在、产物条目为纯路径、**每个 G 关注点都被某个阶段门禁清单执行**） | `bash scripts/gate/check_stage_config_consistency.sh` |
| GE4 | GA 阶段门禁清单可被驱动完整解析 | `bash scripts/gate/check_stage.sh --version v{VERSION} --stage GA --dry-run` |
| GE5 | 所有 RC 前置 Issue 已关闭 | Gitea API 查询 |

> GE2 取代了旧版的「`ls PERFORMANCE_REPORT.md` / `ls SECURITY_AUDIT.md`」
> 三条 `ls` 检查：那两个文件既不在 SSOT 的 GA `required_files` 里，
> 也不在 `doc_artifacts` 里 —— 它们是一条**无判据来源**的清单。
> 缺件判据现在只有一份：`STAGE_CONFIG.yaml`。

### PASS 标准

| ID | Check | Method | Threshold |
|----|-------|--------|-----------|
| GA1 | R1-R4 | 所有 RC 指标 | PASS |
| GA2 | Full test | `cargo test --workspace` | PASS |
| GA3 | Full coverage | L1 avg ≥ 85%, 每 crate ≥ 80% | PASS |
| GA4 | TPC-H SF=1 | `bash scripts/gate/check_tpch_sf1.sh --sf1-dir $SF1_DIR` (issue #3732, v3.10.0+) | 22/22 PASS |
| GA5 | Security | `cargo audit` + 手动审计 | PASS |
| GA6 | Documentation | API reference, CHANGELOG, UPGRADE_GUIDE | PASS |

> **编号变更说明（#5437）**：本表原用 `G1~G6`，与 SSOT 的 `G1~G16`
> 门禁关注点别名**同名不同义** —— 同一个「G7」在两处指两件不同的事。
> 现改为 `GA1~GA6`。**历史条目不改写**（改写会让证据描述一个从未存在过
> 的状态）；本表是**当前**判据摘要，历史版本见 git 历史。

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

**入库要求（#5441）**：上面 4 类**运行决策记录**必须进入版本控制
（`.gitignore` 已放行 `docs/releases/*/logs/gate_*.log`）——
「GA 门禁跑过了没有、用的哪个 commit、日志内容是什么」必须能在仓库里
查到，而不是只存在于某次会话的临时文件里。

同一目录下的 `logs/gates/*.log` 是**每个门禁的原始 stdout 落盘**，
每次运行都会重新生成、体积不受控，因此**不入库**。
判据（要跑哪些门禁）与证据（这次跑的决策记录）现在说的是同一件事。

执行入口：

```bash
bash scripts/gate/check_stage.sh --version v{VERSION} --stage GA
```

该命令按 SSOT 逐项执行 `stages.GA.required_gates`，并把本次运行写入
`logs/gate_ga_<commit>_<timestamp>.log`。

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

## Coverage Gates (added 2026-08-10)

### G17 — Coverage Gate

| Stage | Threshold | Script |
|-------|-----------|--------|
| ALPHA | L1_8 avg >= 75% | scripts/gate/check_coverage_v312.sh |
| BETA | L1_8 avg >= 80% | scripts/gate/check_coverage_v312.sh |
| RC | L1_8 avg >= 80% | scripts/gate/check_coverage_v312.sh |
| GA | L1_8 avg >= 80% | scripts/gate/check_coverage_v312.sh |

L1_8 = {sqlrustgo-parser, sqlrustgo-planner, sqlrustgo-executor, sqlrustgo-transaction, sqlrustgo-storage, sqlrustgo-catalog, sqlrustgo-optimizer, sqlrustgo-types}.

### G18 — SQL Corpus Gate

| Stage | Threshold | Script |
|-------|-----------|--------|
| BETA | pass_rate >= 80.0% | scripts/gate/check_sql_corpus_gate.sh |
| RC | pass_rate >= 80.0% | scripts/gate/check_sql_corpus_gate.sh |
| GA | pass_rate >= 80.0% | scripts/gate/check_sql_corpus_gate.sh |

Test target: `cargo test --release -p sqlrustgo-sql-corpus --test corpus_test test_sql_corpus_all`.

### G19 — Anti-Ignore Gate

| Stage | Threshold | Script |
|-------|-----------|--------|
| BETA | active <= 80, total_allowed <= 150 | scripts/gate/check_anti_ignore_gate.sh |
| RC | active <= 80, total_allowed <= 150 | scripts/gate/check_anti_ignore_gate.sh |
| GA | active <= 80, total_allowed <= 150 | scripts/gate/check_anti_ignore_gate.sh |

Threshold source (#5425): the numbers above are the registry's own `limits`
field (`tests/baseline/ignore_registry.json` → `limits.active_max` /
`limits.total_allowed_max`); the script reads them from there rather than
hard-coding a second copy. Before #5425 the threshold existed in **three**
mutually inconsistent places — this table said 47/96, the script's header
comment said 55/100, and the script's code said 80/150 — so the same
repository got a different verdict depending on which document you read.

The active ceiling (69) and the total ceiling (116) are both below the
limits, so the gate passes today; the point of #5425 is that it now passes
for **one** documented reason instead of three competing ones.

Active = entries in `tests/baseline/ignore_registry.json` with status=ACTIVE.

Note (2026-08-11): total_allowed ceiling bumped 73 -> 96 by V312-17 round-16 (codex #89297) which added 23 entries (round-16: 3418ac19a1, round-17: 628a621bd1).
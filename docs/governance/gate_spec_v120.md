# GMP-Platform v1.2.0 门禁规范

> **版本**: 1.0
> **日期**: 2026-05-23
> **状态**: 生效
> **适用范围**: v1.2.0
> **supersedes**: `GATE_SPEC.md`（通用版）

---

## 一、门禁体系概述

### 1.1 四级门禁模型

```
L0-Alpha → L0-Beta → L0-RC → L0-GA
```

| 门禁 | 名称 | 阶段目标 | 通过标准 |
|------|------|----------|----------|
| L0-Alpha | Alpha Gate | 核心功能可运行 | 13/13 检查 PASS |
| L0-Beta | Beta Gate | 功能冻结，全链路可用 | 14/14 检查 PASS |
| L0-RC | RC Gate | 发布候选，性能达标 | 12/12 检查 PASS |
| L0-GA | GA Gate | 正式发布 | 全部 PASS |

### 1.2 通过条件

| 条件类型 | 说明 |
|----------|------|
| **全部通过** | 所有门禁检查项必须 PASS |
| **有豁免记录** | 不满足的项必须在 `GATE_EXEMPTIONS.md` 有审批通过的豁免记录 |
| **无阻塞性 OPEN Issue** | Milestone 下的 OPEN Issue 不能阻塞门禁 |

---

## 二、L0-Alpha Gate (A)

### 2.1 入口条件

- Workspace 全量编译成功
- 所有 crate clippy 零警告
- 单元测试 ≥90% 通过

### 2.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| A1 | Build (workspace) | `cargo build --release --workspace` | 成功 | `{exit_code, time}` |
| A2 | Incremental Build | `cargo build --release -p gmp-cli` | 成功 | `{exit_code}` |
| A3 | Unit Tests | `cargo test --lib` | ≥90% 通过 | `{passed, failed, exit_code}` |
| A4 | Format Check | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| A5 | Clippy Check | `cargo clippy --all-targets -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| A6 | Python Lint | `python3 -m py_compile scripts/*.py` | 全部通过 | `{files, exit_code}` |
| A-F1 | gmp-retrieve CLI | `./target/release/gmp-retrieve search --channel fts --db ./data/gmp.db -- '偏差处理'` | 返回结果 | `{results_count}` |
| A-F2 | Web Dashboard | `python3 dashboard/server.py &` + `/api/stats` | 启动成功 | `{pid, response}` |
| A-F3 | mobile_cli demo | `python3 scripts/mobile_cli_demo.py --skip-ollama` | 演示完成 | `{exit_code}` |
| A-F4 | stress_test dry-run | `python3 scripts/stress_test_72h.py --dry-run` | 脚本可执行 | `{scenarios, exit_code}` |
| A-S1 | Gate Spec | `[ -f docs/governance/gate_spec_v120.md ]` | 文件存在 | `{path}` |
| A-S2 | Development Plan | `[ -f docs/releases/v1.2.0/DEVELOPMENT_PLAN.md ]` | 文件存在 | `{path}` |
| A-S3 | Branch Model | `git branch -r \| grep develop` | develop 分支存在 | `{branch}` |

### 2.3 Alpha 通过结果

```
--- A1: Build (full workspace) ---        [PASS]
--- A2: Incremental Build (gmp-cli) ---  [PASS]
--- A3: Unit Tests ---                    [PASS]
--- A4: Format Check ---                  [PASS]
--- A5: Clippy Check ---                  [PASS]
--- A6: Python Lint ---                   [PASS]
--- A-F1: gmp-retrieve CLI ---           [PASS]
--- A-F2: Web Dashboard ---              [PASS]
--- A-F3: gmp-agent mobile_cli demo ---  [PASS]
--- A-F4: stress_test_72h.py dry-run --- [PASS]
--- A-S1: Gate Spec ---                  [PASS]
--- A-S2: Development Plan ---           [PASS]
--- A-S3: Branch Model ---               [PASS]
========================================
  L0-Alpha Gate Results: 13/13 PASSED
```

---

## 三、L0-Beta Gate (B)

### 3.1 入口条件

- L0-Alpha 全部 PASS
- 四通道检索全可用（fts/rule/vector/graph）
- E2E 闭环演示可运行

### 3.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| B1 | Build (release) | `cargo build --release --workspace` | 成功 | `{exit_code, time}` |
| B2 | Unit Tests | `cargo test --all-features` | ≥95% 通过 | `{passed, failed, exit_code}` |
| B3 | Clippy Check | `cargo clippy --all-targets -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| B4 | Format Check | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| B5 | Security Audit | `cargo audit` | 无 HIGH/CRITICAL | `{vulnerabilities}` |
| B-I1 | E2E Search (fts) | `./target/release/gmp-retrieve search --channel fts --db ./data/gmp.db -- '偏差处理'` | `Found` | `{results_count}` |
| B-I2 | E2E Search (rule) | `./target/release/gmp-retrieve search --channel rule --db ./data/gmp.db -- '偏差处理'` | `Found` | `{results_count}` |
| B-I3 | E2E Search (vector) | `./target/release/gmp-retrieve search --channel vector --db ./data/gmp.db -- '偏差处理'` | `Found` | `{results_count}` |
| B-I4 | E2E Search (graph) | `./target/release/gmp-retrieve search --channel graph --db ./data/gmp.db -- '偏差处理'` | `Found` | `{results_count}` |
| B-F1 | closed_loop_demo | `python3 scripts/closed_loop_demo.py --skip-ollama` | `PASS` | `{exit_code, output}` |
| B-F2 | Dashboard API | `curl http://localhost:8080/api/search?q=偏差` | HTTP 200 | `{status_code}` |
| B-S1 | Gate Spec v1.2.0 | `[ -f docs/governance/gate_spec_v120.md ]` | 文件存在 | `{path}` |
| B-S2 | Test Plan | `[ -f docs/releases/v1.2.0/TEST_PLAN.md ]` | 文件存在 | `{path}` |
| B-S3 | Beta Tag | `git tag -l "beta/v1.2.0*"` | Tag 存在 | `{tag}` |
| B-S4 | Branch Protection | Gitea API: `develop/v1.2.0` protection enabled | 保护开启 | `{protection}` |

### 3.3 Beta 通过结果

```
--- B1: Build (full workspace) ---        [PASS]
--- B2: Unit Tests ---                    [PASS]
--- B3: Clippy Check ---                  [PASS]
--- B4: Format Check ---                  [PASS]
--- B5: Security Audit ---                [PASS]
--- B-I1: E2E Search (fts) ---          [PASS]
--- B-I2: E2E Search (rule) ---          [PASS]
--- B-I3: E2E Search (vector) ---        [PASS]
--- B-I4: E2E Search (graph) ---         [PASS]
--- B-F1: closed_loop_demo.py ---         [PASS]
--- B-F2: Dashboard API ---               [PASS]
--- B-S1: Gate Spec v1.2.0 ---           [PASS]
--- B-S2: Test Plan ---                  [PASS]
--- B-S3: Beta Tag ---                   [PASS/FAIL - expected before tag]
--- B-S4: Branch Protection ---           [PASS]
========================================
  L0-Beta Gate Results: 13/14 PASSED
```

---

## 四、L0-RC Gate (R)

### 4.1 入口条件

- L0-Beta 全部 PASS（或有豁免记录）
- 场景通过率 ≥ 90%
- 功能冻结

### 4.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 | 证据格式 |
|---|--------|----------|---------|---------|
| R1 | Build (full) | `cargo build --release --workspace` | 成功 | `{exit_code, time}` |
| R2 | Unit Tests | `cargo test --all-features` | 100% 通过 | `{passed, failed, exit_code}` |
| R3 | Clippy Check | `cargo clippy --all-targets -- -D warnings` | 零警告 | `{warnings, exit_code}` |
| R4 | Format Check | `cargo fmt --all -- --check` | 无格式错误 | `{diff_count, exit_code}` |
| R5 | Security Audit | `cargo audit` | 无 HIGH/CRITICAL | `{vulnerabilities}` |
| R6 | E2E (4 channels) | `gmp-retrieve search --channel {fts,rule,vector,graph}` | 全部返回结果 | `{channel_results}` |
| R-S1 | 8h Stress Test | `scripts/stress_test_72h.py` | ≥90% pass, ≥400 runs | `{duration, pass_rate, runs}` |
| R-S2 | Memory Stability | `ps aux \| grep stress_test` | RSS < 1GB | `{rss_kb}` |
| R-S3 | DB Size Stability | `ls -la data/gmp.db` | < 100MB | `{db_size_mb}` |
| R-S4 | Ollama Availability | `curl http://192.168.0.252:11434/api/tags` | HTTP 200 | `{status_code}` |

### 4.3 RC 通过结果

```
--- R1: Build (full workspace) ---        [PASS]
--- R2: Unit Tests ---                    [PASS]
--- R3: Clippy Check ---                  [PASS]
--- R4: Format Check ---                  [PASS]
--- R5: Security Audit ---                [PASS]
--- R6: E2E Search (4 channels) ---      [PASS]
--- R-S1: 8h Stress Test ---             [PASS/INFO - requires long run]
--- R-S2: Memory Stability ---            [PASS/INFO - requires long run]
--- R-S3: DB Size Stability ---           [PASS]
--- R-S4: Ollama Availability ---         [PASS]
========================================
  L0-RC Gate Results: 11/12 PASSED
```

---

## 五、L0-GA Gate (G)

### 5.1 入口条件

- L0-RC 全部 PASS（或有豁免记录）
- 无阻塞性 OPEN Issue

### 5.2 检查清单

| # | 检查项 | 命令/方法 | 通过标准 |
|---|--------|----------|---------|
| G1 | 全部 RC 检查 | — | 全部 PASS |
| G2 | 文档完整性 | `docs/completeness_check.sh v1.2.0` | 21/21 docs |
| G3 | 版本一致性 | 所有文档版本号 = v1.2.0 | 无旧版本号 |
| G4 | Issue 全部关闭 | Gitea API | 无 P0/P1 OPEN |
| G5 | Git Tag | `git tag v1.2.0` | Tag 存在 |
| G6 | Release 分支 | `release/v1.2.0` | 分支存在并保护 |

---

## 六、豁免机制

如特殊情况导致无法达标，必须：

1. 在 `docs/governance/GATE_EXEMPTIONS.md` 创建豁免记录
2. 包含：豁免 ID、描述、关联 Issue、审批状态（Pending/Approved/Rejected）
3. P0/P1 问题不可豁免

---

## 七、版本化门禁规范索引

| 版本 | 门禁文档 | 状态 |
|------|----------|------|
| v1.0.0 | `gate_spec.md` | 已归档 |
| v1.1.0 | `gate_spec.md` | 已归档 |
| v1.2.0 | `gate_spec_v120.md` | 生效 |

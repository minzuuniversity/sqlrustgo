# 迭代 5 · 批次 25 —— GA 的两个前置：版本锚点（#5442）与承载分支（#5436）

- **分支**：`fix/ga-version-anchor-5436-5442`
- **基线**：`develop/v4.1.0` @ `378f014529`
- **本批 Issue**：#5442（版本锚点失联）、#5436（GA 的承载分支不存在）

---

## 0. 一句话

「v4.1.0 要发 GA」这句话在本仓库的当前状态下**没有任何合法路径支撑**：
版本锚点指着 `3.12.0-fix-zombie`，GA 的承载分支 `release/v4.1.0` 不存在，
而 `branch_pattern` 这条规则**全仓没有任何代码读它**。

本批把这三件事里的两件变成**可执行的门禁**：

* 版本号：发布二进制必须自报发布线版本（判据从「含有 sqlrustgo」改成**相等**）；
* 承载分支：`branch_pattern` 从**一句没人读的话**变成**被强制的规则**。

---

## 1. #5442 —— 版本锚点

### 1.1 症状

```
$ grep -n '^version' Cargo.toml
52:version = "3.12.0-fix-zombie"
```

而发布线是 `v4.1.0`（`docs/releases/v4.1.0/STAGE.yaml:49` → `version: "v4.1.0"`）。
发布出去的二进制自报 `3.9.0`（`crates/sqlrustgo-cli` 的包版本），
发布文档写着 `v4.1.0` —— **三者互不相干，而且没有任何门禁会发现**。

旧判据（`scripts/gate/check_release_binary.sh`）：

```bash
if cargo run --package sqlrustgo-cli -- --version 2>/dev/null | grep -q "sqlrustgo"; then
```

⇒ 只要输出里含有 `sqlrustgo` 就算过。`3.12.0`、`3.9.0`、`4.1.0` 一视同仁。

### 1.2 修法

| 改动 | 位置 |
|---|---|
| 包版本 → `4.1.0` | `Cargo.toml` 的 `[workspace.package] version` |
| 规范 CLI 跟随 workspace | `crates/sqlrustgo-cli/Cargo.toml` → `version.workspace = true` |
| 客户端库跟随 workspace | `crates/mysql-client/Cargo.toml` → `version.workspace = true` |
| 判据改成**相等** | `scripts/gate/check_release_binary.sh` |
| 版本策略写进 SOP | `docs/governance/VERSION_PROMOTION_SOP.md` §0.1 |

新判据：

```bash
EXPECTED_VERSION="$(awk '
  /^\[workspace\.package\]/{f=1;next}
  /^\[/{f=0}
  f && /^version[[:space:]]*=[[:space:]]*"/{
    gsub(/.*=[[:space:]]*"/,""); gsub(/".*/,""); print; exit
  }' Cargo.toml)"
ACTUAL_VERSION="$(cargo run --package sqlrustgo-cli -- --version 2>/dev/null | head -1 | awk '{print $NF}')"
[ "$ACTUAL_VERSION" = "$EXPECTED_VERSION" ] || FAIL
```

**期望版本不在门禁里写第二份字面量** —— 它从 `Cargo.toml` 读，
所以「版本号的唯一来源」只有一处（§0.1 的表格写明了这三者的对应关系）。

`Cargo.lock` 随之更新（22 个 workspace 成员的版本条目）。

### 1.3 实测

```
$ awk '...同上...' Cargo.toml
4.1.0
```

`--version` 的输出由 clap 的 `version` 属性打印，取的是
`crates/sqlrustgo-cli` 的 `CARGO_PKG_VERSION`；该 crate 现在用
`version.workspace = true` ⇒ 二进制自报 `sqlrustgo 4.1.0`。

---

## 2. #5436 —— 承载分支

### 2.1 症状

`STAGE_CONFIG.yaml` 的 GA 段：

```yaml
  GA:
    branch_pattern: ["main", "release/v*", "hotfix/v*"]
```

而 `develop/v4.1.0` **不在**这个列表里，`release/v4.1.0` **不存在**
（`git ls-remote` 实测：`main` 存在、`release/v3.12.0` 存在、
`release/v4.1.0` 与 `hotfix/v4.1.0` 都不存在）。

**更关键的是**：`branch_pattern` 这个字段**全仓没有任何脚本消费**：

```
$ grep -rn "branch_pattern" --include="*.sh" --include="*.py" --include="*.rs" --include="*.yml" .
（无输出）
```

⇒ 「GA 只允许在 main / release/* / hotfix/* 上发生」这条规则
**从未被强制**。谁在 `develop/v4.1.0` 上把 `current_stage` 改成 `GA`、
把清单勾上，就可以「宣布 GA」—— 而 GA 的定义本身并不允许这件事。

### 2.2 修法：把声明**变成规则**，而不是把规则**放松**

issue 给了两条路：（1）改定义（把 `develop/v*` 加进 GA 的 pattern）、
（2）改现实（真正切出 `release/v4.1.0`）。

本批走的是**第三条**、也是最不容易被误读的一条：

> **让 `branch_pattern` 真的被执行。**

于是「在开发线上宣布 GA」从一个**判断问题**变成一个**可执行的门禁**：
谁把 `current_stage` 推到 GA 而没有合法承载分支，门禁立刻红。

具体三处改动：

1. `STAGE_CONFIG.yaml`：每个阶段加一个显式开关 `carrier_enforced`
   （RC / GA 为 `true`，DRAFT / ALPHA / BETA 为 `false`）；
2. `scripts/gate/check_stage.sh`：读当前阶段的 `branch_pattern` 与
   `carrier_enforced`，当前分支不匹配 ⇒ **该阶段判定无效**（退出码 1，
   不是「少一项」）；
3. GA 的 `exit_criteria` 增加一条
   `GA judged on a branch matching GA.branch_pattern (main / release/v* / hotfix/v*)`。

**为什么开发阶段不强制**：`ALPHA.branch_pattern` 是 `["develop/v*"]`，
而日常开发发生在 `fix/*` / `feature/*` 分支上。强制它只会让**每个 PR 都红**，
与本批要解决的问题无关。承载规则对**晋级阶段**才有意义 —— 那正是它唯一
被用到的时刻。

### 2.3 负对照（证明规则真的会拦）

把 `docs/releases/v4.1.0/STAGE.yaml` 的 `current_stage` 临时从 `ALPHA`
改成 `GA`，在 `fix/ga-version-anchor-5436-5442` 分支上跑驱动：

```
--- Branch Pattern ---
  stage:     GA
  pattern:   main release/v* hotfix/v*
  branch:    fix/ga-version-anchor-5436-5442
  enforced:  true
  [FAIL] branch 'fix/ga-version-anchor-5436-5442' does not match the stage's carrier rule
         this stage cannot be carried out on this branch (#5436)
```

真实退出码 = **1**。还原 `current_stage` 后，同一分支上（阶段 ALPHA）：

```
  enforced:  false
  [INFO] carrier rule is declared but not enforced for this stage (development stage)
```

退出码 = 0。

### 2.4 同步的事实记录

`docs/releases/v4.1.0/STAGE.yaml` 的 `current_state` 新增：

```yaml
  ga_carrier_branch: "release/v4.1.0"
  ga_declared: false
  ga_carrier_note: "no release/v4.1.0 branch exists yet; GA has NOT been declared and MUST NOT be claimed on develop/v4.1.0"
```

`current_stage` 保持 **`ALPHA`** —— 这是**事实**：v4.1.0 还没有通过 RC 判定，
GA 判定一次都没发生过（`docs/releases/v4.1.0/STAGE.yaml:62` 记着 v4.0.0 的
GA 曾被人工撤销：`REVOKED 2026-09-30 — v4.0.0 never legitimately reached GA`）。
本批**不**把它改成 GA —— 那是晋级动作，不是修缺陷。

---

## 3. 合法路径（写进 SOP §0.1 与 STAGE.yaml）

```
develop/v4.1.0  ── RC 判定通过 ──▶  tag v4.1.0-rc1
                                    │
                                    ├─ 从该 tag 切出 release/v4.1.0
                                    │
release/v4.1.0 ── GA 判定（门禁强制承载分支合法）──▶  合入 main  ──▶  tag v4.1.0
```

中间任何一步没过，门禁红。**没有捷径**。

---

## 4. 本地门禁实测

| 门禁 | 结果 |
|---|---|
| `check_stage_config_consistency.sh` | **PASS**（0 FAIL / 14 DISCLOSURE） |
| `check_stage.sh --version v4.1.0 --dry-run` | EXIT=0（ALPHA 阶段，承载规则不强制） |
| 同上，负对照（`current_stage: GA`） | **EXIT=1**（承载分支不合法） |
| `Cargo.toml` 版本提取自检 | `4.1.0` ✓ |
| `STAGE_CONFIG.yaml` / `STAGE.yaml` YAML 解析 | 通过 |

---

## 5. 未达成 / 未验证项

1. **`release/v4.1.0` 分支本批没有切** —— 切分支是**晋级动作**，必须在 RC
   判定通过之后；现在切等于伪造晋级。#5436 的 issue 正文把「切分支」列为
   改现实的那条路，本批只把**规则**准备好（规则会强制你在合法分支上做 GA）。
2. **RC 判定未做**：RC 的门禁集、RC 产物（`RELEASE_GATE_CHECKLIST.md`、
   `PERFORMANCE_TARGETS.md`）仍然缺失。
3. **GA 的 6 份产物仍然缺失**（`GA_GATE_REPORT.md`、`GA_RELEASE_TIMELINE.md`、
   `EVIDENCE_STATUS.md`、`POST_GA_PLAN.md` + RC 的 2 份）。
4. **版本号改动尚未在 CI 上验证** —— `check_release_binary.sh` 需要
   `cargo run --package sqlrustgo-cli`，在 CI 上会真跑一次。
   本批只做了版本提取与判据逻辑的自检。
5. **`Cargo.lock` 的 22 处版本更新**由 `cargo update --workspace --offline`
   生成；CI 会用 `cargo build --all-features` 复核锁文件是否自洽。

---

## 6. 身份与协作

- 身份串：`openclaw-mcode`（Mac Mini / Minimax-Code）
- 协作台账：#5531
- 涉及 Issue：#5436 #5442 #5439 #5450
- **只写 `Refs`，不写 `Closes`**。

Refs #5436
Refs #5442

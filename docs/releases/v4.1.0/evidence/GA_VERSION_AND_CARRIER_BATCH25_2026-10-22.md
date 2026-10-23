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
# stdout 与 stderr 分开捕获；失败时把 cargo 自己的诊断打进日志
cargo run --package sqlrustgo-cli -- --version >"$OUT" 2>"$ERR"
ACTUAL_VERSION="$(head -1 "$OUT" | awk '{print $NF}')"
[ "$ACTUAL_VERSION" = "$EXPECTED_VERSION" ] || FAIL
```

**期望版本不在门禁里写第二份字面量** —— 它从 `Cargo.toml` 读，
所以「版本号的唯一来源」只有一处（§0.1 的表格写明了这三者的对应关系）。

⚠️ **实测时发现：这条命令原本根本跑不起来**，详见 §1.4。

`Cargo.lock` 随之更新（22 个 workspace 成员的版本条目）。

### 1.3 实测

```
$ awk '...同上...' Cargo.toml
4.1.0
```

`--version` 的输出由 clap 的 `version` 属性打印，取的是
`crates/sqlrustgo-cli` 的 `CARGO_PKG_VERSION`；该 crate 现在用
`version.workspace = true` ⇒ 二进制自报 `sqlrustgo 4.1.0`。

**门禁的真实输出**（不是手敲的，是门禁自己打的）：

```
--- Version Flag ---
  [PASS] sqlrustgo --version reports 4.1.0 (== Cargo.toml [workspace.package] version)
```

### 1.4 实测中发现的一个真缺陷：package 产出**两个** binary（#5538）

第一次真跑这条判据，得到的是：

```
$ cargo run --package sqlrustgo-cli -- --version
error: `cargo run` could not determine which binary to run. Use the `--bin` option to specify a binary, or the `default-run` manifest key.
available binaries: sqlrustgo, sqlrustgo-cli
```

⇒ **cargo 拒绝选择**，因为 `crates/sqlrustgo-cli` 同时产出两个 binary：

| 来源 | binary 名 | 源文件 |
|---|---|---|
| manifest 的 `[[bin]] name = "sqlrustgo"` | `sqlrustgo` | `src/local_bin.rs` |
| cargo 的**自动发现**（`src/main.rs`） | `sqlrustgo-cli` | `src/main.rs` |

两个 `main()` 的函数体完全一样（都是 `sqlrustgo_cli::run()`）。

**这意味着 #5442 的版本判据从来没有真正执行过**：旧写法
`cargo run … 2>/dev/null | awk` 把 cargo 的诊断丢掉了，门禁只能报
「reports '' but the release line is '4.1.0'」—— 看起来像版本不一致，
实际是命令根本没跑起来。

而 `scripts/gate/check_cli_binary_names.py` 的注释写着
「No bin is called `sqlrustgo-cli` — that is the **package** name」，
**与事实不符** ⇒ 「二进制名只有一个来源」这个不变量当时是假的。

它还**掩盖了两处真的坏路径**（C3 判据「每个 `target/<profile>/<name>`
字面量都指向一个真实产出的 binary」在这种情况下查不出来）：

| 位置 | 坏在哪 |
|---|---|
| `scripts/stability/soak_1h.sh` | `pgrep -f "sqlrustgo-cli soak"` —— 进程名其实是 `sqlrustgo` ⇒ **worker 计数恒为 0**，metrics 文件里 `workers` 列一直是 0 |
| `scripts/teaching-labs/setup-sqlrustgo-cli.sh` | 打印并执行 `./$TARGET_DIR/sqlrustgo-cli` —— 该文件不存在 ⇒ **教学冒烟测试跑的是一个不存在的二进制**；其头部用法说明还把路径写成了 `docs/teaching-labs/…`（实际在 `scripts/teaching-labs/`） |

**修法**：

1. 删 `crates/sqlrustgo-cli/src/main.rs` ⇒ `[[bin]] name = "sqlrustgo"` 成为
   唯一来源；
2. **新增门禁判据 C5**（`scripts/gate/check_cli_binary_names.{sh,py}`）：
   声明了 `[[bin]]` 的 package **不得**再产生 manifest 没声明的 binary
   （cargo 对 `src/main.rs` / `src/bin/<name>.rs` 的自动发现）。
   判据精确到「`[[bin]]` 的 `path` 是否指向该入口」⇒ **不误伤**那 12 个
   「`[[bin]] path = "src/main.rs"`」的 package；实测精确规则下全仓**只有**
   `sqlrustgo-cli` 一个违规者；
3. 修上面那两处残留；
4. `check_release_binary.sh` 不再吞 stderr。

**负向 selftest**：新增第 7 个 fixture —— 一个 package 声明了 `[[bin]]`
（`path` 指向 `src/local_bin.rs`）却仍留着 `src/main.rs` ⇒ 判 **FAIL**。
自测 **7/7** 通过（`--selftest`）。

**修完后的实测**：

```
$ bash scripts/gate/check_release_binary.sh
--- Version Flag ---
  [PASS] sqlrustgo --version reports 4.1.0 (== Cargo.toml [workspace.package] version)
...
PASS: 9
FAIL: 0
```

`check_cli_binary_names.sh` 同步 **PASS**（C1–C5 全绿，`20 bin names,
all traced to their manifest`）。

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
| `check_release_binary.sh` | **PASS 9 / FAIL 0**（含 §1.3 的版本自报 `4.1.0`） |
| `check_cli_binary_names.sh` | **PASS**（C1–C5 全绿，20 bin names 全部溯源到 manifest） |
| `check_cli_binary_names.sh --selftest` | **7/7**（含新增的 C5 负向 fixture） |
| `check_evidence_binding.sh v4.1.0` | **PASS=106 WARN=106 FAIL=0 UNVERIFIED=0**（合入批次 24 后归零；此前 343 FAIL） |
| `bash -n` × 4 个改动的 shell 脚本 | EXIT=0 |
| `python3 -m py_compile check_cli_binary_names.py` | EXIT=0 |

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
   本地已跑通（§1.3、§1.4 的实测输出），CI 的实测结果待补。
5. **`Cargo.lock` 的 22 处版本更新**由 `cargo update --workspace --offline`
   生成；CI 会用 `cargo build --all-features` 复核锁文件是否自洽。
6. **删掉 `crates/sqlrustgo-cli/src/main.rs` 之后，`cargo build -p
   sqlrustgo-cli` 只产出 `sqlrustgo`** ⇒ 任何仍在找 `target/*/sqlrustgo-cli`
   的外部脚本都会失效。仓内已逐处核对（只剩历史版本文档 `docs/releases/
   v3.9.0/*`、`v3.10.0/*` 里的示例路径，属历史记录，不改）；**仓外**的使用者
   需要自行改用 `sqlrustgo`。

---

## 6. 身份与协作

- 身份串：`openclaw-mcode`（Mac Mini / Minimax-Code）
- 协作台账：#5531
- 涉及 Issue：#5436 #5442 #5439 #5450 **#5538**（本批实测中新发现并登记）
- 本批并入的既有批次：批次 24（PR #5535）—— 本批的证据绑定门禁原本有
  **343 个 FAIL**，缺的是批次 22 的 provenance 修复（在批次 23/24 里）；
  合入后归零（见 §4）。
- **只写 `Refs`，不写 `Closes`**。

Refs #5436
Refs #5442
Refs #5538

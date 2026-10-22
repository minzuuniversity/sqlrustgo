# 迭代 5 · 批次 19 —— clippy 只覆盖了根包：44 个 crate、301 个测试目标从未被 lint（#5508 / #5446 / #5447 / #5403）

- **Issue**: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5508（另含 #5446 / #5447 / #5403）
- **PR**: http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/5524
- **分支**: `fix/clippy-workspace-coverage-5508-batch19` → `develop/v4.1.0`
- **commit**: `fef2f3183b`（主体）、`52ba8bdb2e`（CI 编译错误修复与本文件订正）
- **基线**: `775dc5da72`（develop/v4.1.0）
- **工具链**: CI runner `hp-z6g4` ⇒ rustc/clippy **1.95.0**（本机默认 1.97.0，本轮全部验证用 `cargo +1.95.0` 执行）
- **执行**: 本机 worktree `/Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-batch19`，未触碰主工作区

**关闭**：#5508、#5446、#5447、#5403
**基线**：`develop/v4.1.0` @ `653096dd17`
**分支**：`fix/clippy-workspace-coverage-5508-batch19`
**工具链**：CI runner `hp-z6g4` ⇒ rustc/clippy **1.95.0**（本机默认 1.97.0，本轮全部验证用 `cargo +1.95.0` 执行）

---

## 0. 一句话

CI 的 lint 步骤是

```
cargo clippy --all-features --all-targets -- -D warnings
```

在「workspace 根目录同时也是一个 package」的仓库里，cargo 的**默认成员只有根 package**
⇒ `crates/*` 的 **44 个成员、301 个测试目标**从来没有被任何门禁 lint 过。

把 `--workspace` 打开之后，workspace **编译不过**：17 处 deny-by-default 的
lint 在 `--all-targets` 下变成编译级错误（`clippy::approx_constant` 等），
外加 3 个**从未编译过**的 target（`criterion` 未声明、闭包参数类型不可推断）。

---

## 1. 现象：门禁的覆盖范围与它宣称的不一致

`.gitea/workflows/ci.yml` 的 `lint-build` job（修复前）：

```yaml
- name: Clippy
  run: cargo clippy --all-features --all-targets -- -D warnings
```

`--all-targets` 看起来已经覆盖了测试目标，但**没有 `--workspace`**。
实测（`cargo metadata`）：workspace 共 44 个成员，根 package 之外有 43 个 `crates/*`。

**根 package 之外的代码没有任何 lint 覆盖**，实测后果：

| 缺陷 | 位置 | 为什么门禁看不见 |
|---|---|---|
| `criterion` 未声明为 dev-dependency | `crates/server/benches/scheduler_bench.rs` | bench target **从未编译过**（`E0432`） |
| 闭包参数类型不可推断 | `crates/storage/benches/storage_benchmark.rs` | bench target **从未编译过**（`E0282`） |
| 12 处恒真断言（`x >= 0` on `usize`） | `crates/storage`、`crates/vector`、`crates/admin`、`crates/mysql-server` | 断言**永远不会失败** |
| 1 处 `|| true` 恒真断言 | `crates/mysql-server/tests/cov_v410_server_paths.rs` | 同上 |
| 1 处 `let _ = <lock guard>` | `crates/storage/src/wal_storage.rs` | 锁被立刻释放 |

「从未编译过」这一类是最严重的：**一个编译不过的 target 是一个谁都没跑过的 target**，
和 #5465 在根包侧发现的是同一类缺陷，只是这一侧一个都没被发现。

---

## 2. 全 workspace 首扫（clippy 1.95.0）

```bash
cargo +1.95.0 clippy --workspace --all-features --all-targets --message-format=json
```

首轮（未修）**编译级错误 17 处**，全部落在 deny-by-default 的 lint 上：

| lint | 处数 | 性质 |
|---|---|---|
| `clippy::approx_constant` | 13 | `3.14` / `2.718` / `3.14159` 等字面量被判定为 π/e 的近似 |
| `clippy::absurd_extreme_comparisons` | 3 | 对无符号整数做 `>= 0` 比较 |
| `clippy::overly_complex_bool_expr` | 1 | `x \|\| true` |

> `clippy::approx_constant` 属于 `correctness` 组，**默认 deny** ⇒ 在
> `--all-targets` 下不是「警告」而是**编译失败**。

### 2.1 `approx_constant`：仓库里已���先例

仓库根包侧早已遇到过同一个 lint，处理方式写在
`tests/integration/sql/v312_62_issue_batch_test.rs:71-79`：

```rust
// `3.14` here is the **SQL literal under test**, not a mathematical constant:
// this test checks that the parser turns the literal `3.14` into
// `Value::Float(3.14)`. `clippy::approx_constant` is deny-by-default, and it
#[allow(clippy::approx_constant)]
```

本轮沿用同一形式：**函数级 `#[allow]` + 一句说明「这是被测字面量，不是数学常量」**，
共 14 处。仓库内已存在的 `crates/types/src/value.rs:233,297` 两处 allow 同源。

### 2.2 恒真断言：把「永远不会失败」换成真断言

这 4 处不是 lint 洁癖，是**测试层在骗人**——断言恒真，测试就不可能发现回归。

| 位置 | 原来 | 现在 |
|---|---|---|
| `crates/storage/src/bplus_tree/index.rs:1624` | `assert!(results.is_empty() \|\| results.len() >= 0)` | `assert_eq!(results, vec![100, 200])`（区间是半开 `[1,3)`） |
| `crates/storage/src/bplus_tree/index.rs:1813` | `assert!(results.len() >= 0)` | `assert_eq!(results, vec![3, 4, 5, 6])` |
| `crates/storage/src/bplus_tree/index.rs:2214` | `assert!(results.len() >= 0)` | `assert_eq!(results, vec![200])` |
| `crates/storage/src/page.rs:651` | `assert!(page.free_space() >= 0)` | `assert_eq!(page.free_space() as usize, PAGE_DATA_SIZE)` |
| `crates/types/tests/coverage_v3_12.rs:172` | `assert!(v.estimate_memory_size() >= 0)` | `assert_eq!(v.estimate_memory_size(), 0)` |
| `crates/mysql-server/tests/mysql_server_helper_tests.rs`（3 处） | `assert!(val >= 0)`（原子计数器） | 两次读之间的单调性 `after >= before` |
| `crates/admin/tests/wire_client_integration_tests.rs` | `assert!(status.uptime_seconds >= 0)` | `uptime_seconds < 60` + `active_connections < 1000` + `server_version` 非空 |
| `crates/mysql-server/src/lib.rs:485` | `assert!(tracker.idle_secs() >= 0)` | `assert_eq!(tracker.idle_secs(), 0)`（20 ms < 1 s） |
| `crates/vector/tests/vector_coverage_tests.rs:647` | `assert!(writer.pending() > 0 \|\| writer.indexed_count() >= 0)` | `pending() > 0` 且 `indexed_count() == 0`（阈值 50，未 flush） |
| `crates/mysql-server/tests/cov_v410_server_paths.rs:274` | `assert!(dir.exists() \|\| true)` | 真正从 `Debug` 里取出 data_dir 路径，断言 **drop 前存在、drop 后消失** |
| `crates/executor/src/trigger.rs:2360` | `assert!(result.unwrap().is_modified() \|\| true)` | `assert!(result.unwrap().is_modified())`（该测试确实注册了 BEFORE INSERT 触发器） |
| `crates/security/tests/v400_unified_acl_audit.rs:258` | `!self.entries.is_empty() \|\| true` | `!self.entries.is_empty()` |

> `cov_v410_server_paths.rs` 那一处特别值得记：旧代码把 `format!("{:?}", handle)`
> （**Debug 字符串**，不是路径）当成路径去 `exists()`，再用 `|| true` 兜底。
> 改完之后这个测试**第一次真正验证了「drop 会清掉自建 data 目录」**——
> 而这正是该测试的名字所声称的行为。

### 2.3 三个从未编译过的 target

| target | 错误 | 修法 |
|---|---|---|
| `crates/server/benches/scheduler_bench.rs` | `E0432: unresolved import criterion` | `crates/server/Cargo.toml` 补 `criterion = "0.8"`（与 rag/storage/vector 同版本） |
| `crates/storage/benches/storage_benchmark.rs` | `E0282: type annotations needed` | 闭包参数标注 `&Vec<sqlrustgo_types::Value>`，并按编译器建议加 `&` 以便转型为 `&dyn Fn` |
| `crates/parser/tests/coverage_v3_12.rs` | 704 条 lint 警告 | 见下 |

`crates/parser/tests/coverage_v3_12.rs` 里 352 处同一机械写法：

```rust
let _ = parse("...")
    .map(|s| std::mem::drop(s))
    .unwrap_or_default();
```

`let_unit_value`（`let _ =` 绑了一个 unit 值）+ `redundant_closure` 各 352 条。
改为：

```rust
parse("...")
    .map(std::mem::drop)
    .unwrap_or_default();
```

**该文件 704 条警告 → 0 条**，测试 366/366 通过。

---

## 3. 门禁：一套规则，两个 scope

`scripts/gate/check_test_target_lint_budget.sh`（#5465 建立）原本只管根 package。
本轮**扩展**它而不是另写一个 —— 一个规则只能有一份实现：

```bash
bash scripts/gate/check_test_target_lint_budget.sh                      # 根 package（既有）
bash scripts/gate/check_test_target_lint_budget.sh --scope workspace    # 全 workspace（新增）
```

两个 scope 共用**同一条规则**：

1. `--all-targets` 必须**编译通过**（编译不过的 target 是谁都没跑过的 target ⇒ 硬失败）；
2. 每个 lint 名的警告数不得超过基线；
3. 基线里没有的 lint 名 ⇒ 红（防止「拿 A 类债换 B 类债」）；
4. compare 模式下基线文件必须**字节不变**（防止门禁自己悄悄重新基线化）。

基线文件：`tests/baseline/test_target_lint_budget.json`（根）、
`tests/baseline/workspace_lint_budget.json`（workspace，新增）。

### 3.1 工具链一致性

lint 集合是**工具链的属性**，不只是源码的属性。CI runner 固定 1.95.0，
本机默认 1.97.0 ⇒ 基线必须用 CI 的工具链生成，否则版本差异会造成假红。

因此：

* 脚本支持 `SQLRUSTGO_CLIPPY_TOOLCHAIN` 显式固定工具链；
* 基线里记录生成它时的 `cargo --version`；
* compare 模式下工具链不一致会**显著打印**一条 NOTE（不硬失败：升级工具链
  本来就会改数字，硬失败只会让每次升级都变成一个没人能处理的红）。

---

## 4. #5446 —— 版本探测恒为 v3.7.0，4 处硬编码 v3.8.0

### 4.1 版本探测

原实现只可能在 `origin/develop/v3.8.0` 与 `origin/develop/v3.7.0` 之间二选一，
而 checkout 用 `fetch-depth: 1` ⇒ 两个远程跟踪引用都不存在；Gitea 上**这两个分支
根本不存在** ⇒ 在 `develop/v4.1.0` 上 `${version}` **恒为 `v3.7.0`**。

现在版本探测有**唯一实现**：`scripts/gate/resolve_release_version.sh`
（`ci.yml` 与 `gate.yml` 共用）：

```
显式参数 → $GITHUB_REF / $GITEA_REF → $GITHUB_REF_NAME → git 分支
        → docs/releases 下版本号最高的目录 → VERSION 文件
```

**旧的 `v3.7.0` 兜底默认值被删除**：一个「看起来合理但是错的」版本比一次
可见的失败更危险 —— 它会让所有版本相关的门禁去检查一条没人要求的发布线，
而步骤本身还是绿的。检测不到版本 ⇒ 明确 `exit 1`。

### 4.2 4 处硬编码 `v3.8.0`

| 步骤 | 原来 | 现在 |
|---|---|---|
| `auto_env_blocker.sh` | `v3.8.0` | `${{ steps.version.outputs.version }}`（只读模式，见 §5） |
| `check_document_completeness.sh` | `v3.8.0` | `${{ steps.version.outputs.version }}` |
| `check_alpha_v380.sh` | 写死 v3.8.0 的 alpha 门禁 | `scripts/gate/resolve_alpha_gate.sh` 解析 |
| `audit_testing.sh` | `v3.8.0` | `${{ steps.version.outputs.version }}` |

alpha 门禁脚本的命名不统一（`check_alpha_v410.sh` vs `check_alpha_v3.12.0.sh`），
解析规则也抽成唯一实现 `scripts/gate/resolve_alpha_gate.sh`：

```bash
v4.1.0    -> scripts/gate/check_alpha_v410.sh
v3.12.0   -> scripts/gate/check_alpha_v3.12.0.sh
v3.11.0   -> scripts/gate/check_alpha_v3.11.0.sh
v3.10.0   -> scripts/gate/check_alpha_v3.10.0.sh
v3.8.0    -> scripts/gate/check_alpha_v380.sh
v4.0.0    -> (rc=3) 没有门禁脚本 ⇒ NOT-RUN + reason code
```

> 解析脚本在自查时抓到了**作者本人**的一个 bug：候选名写成了
> `check_alpha_v${VER}`，而 `VER` 已经带 `v` ⇒ `check_alpha_vv3.12.0.sh`
> ⇒ `v3.12.0` 解析不到。已修正为 `check_alpha_${VER}`。

---

## 5. #5403 —— `auto_env_blocker` 「中止」的真因，以及它为什么不能跑在 CI 里

### 5.1 真因不是 `((c++))`

`auto_env_blocker.sh` 用 `((total_processed++))` 计数，而 `((expr))` 在表达式
取值为 0 时返回**退出码 1**。直觉上这会在 `set -e` 下中止脚本。**实测不中止**：

```console
$ bash -c 'set -e; c=0; ((c++)); echo "after c=$c"'
after c=1
$ echo $?
0
```

（`((c++))` 是后置递增，返回**旧值** 0，但 bash 对 `(( ))` 的 errexit 处理
与「命令返回非零」不同——这一条经验已写进证据，避免后人重复踩。）

### 5.2 真正的中止点：`pipefail` + 无 H1 的文件

```bash
h1_line=$(grep -n "^# " "$md_file" | head -1 | cut -d: -f1)
if [ -z "$h1_line" ]; then
    # No H1, insert at top
    h1_line=0
fi
```

脚本有 `set -euo pipefail`。一个**没有 H1 标题**的 `.md` 文件 ⇒ `grep` 退出码 1
⇒ 整个管道退出码 1 ⇒ `set -e` **当场中止** ⇒ 脚本里「无 H1 就插到文件顶部」
的那段处理是**不可达的死代码**。

实测（最小对照）：

```console
$ bash -c 'set -euo pipefail; x=$(grep -n "^# " noh1.md | head -1 | cut -d: -f1); echo reached'
EXIT=1          ← 没有 "reached"
$ bash -c 'set -euo pipefail; x=$(grep -n "^# " h1.md | head -1 | cut -d: -f1); echo reached'
reached: x=[1]
EXIT=0
```

仓库里这样的 `.md` 有 **26 个**（含 `docs/releases/v4.1.0/PERF_4956_COMMIT_FLUSH_AB_2026-10-04.md`）。

修法：改用 `awk '/^# /{print NR; exit}'`（对可读文件恒返回 0，无匹配时打印空），
「无 H1」分支因此**第一次真正可达**。顺带发现并修掉了被 abort 掩盖的第二个 bug：
`h1_line=0` 时 `awk` 的 `NR == 0` 永不匹配，标记根本没插进去（现在插到首行之前）。

### 5.3 它不能以写模式跑在 CI 里

`auto_env_blocker` 是一个**改写文件的 pre-commit 工具**。在 CI 里跑写模式会：

1. 改写 runner 的工作树 —— 实测 `docs/releases/v4.1.0` 下有 **92 个** `.md`
   会被改写；改写结果不会被提交，却会**污染后续步骤读到的文件**；
2. 在第一个无 H1 的文件处**半途中止**，留下一个改了一半的工作树。

因此新增 `--check` **只读模式**：不写任何文件，只报告哪些文件缺少标记。
CI 步骤改用它：

```
bash scripts/gate/auto_env_blocker.sh --check "${{ steps.version.outputs.version }}"
```

实测：`Processed: 92 / Would add: 92 (read-only: no file was modified)`，
`git status docs/` 保持为空。

---

## 6. #5447 —— `gate.yml` 的两个 job

两个 job 的「绿色」此前与被检查对象无关：

* `evidence-graph-gate` 把 `PASS` 字面量写进证据图，再从图里读出 `PASS`（自己给自己打分）；
  改用真检查（`check_path_casefold_conflict.sh`）的结果，并摄入**真实**的
  artifact sha256（原来是 `"sha256-placeholder"`）。
* `legacy-gate` 跑 v2.9.0 时代的 `scripts/gate/legacy/check_alpha.sh` 并用
  `|| true` 收尾；改为跑当前版本的真门禁、去掉 `|| true`、加 45 分钟 timeout。

本轮**追加**：`legacy-gate` 里原本写死的 `scripts/gate/check_alpha_v410.sh`
是 #5446 的**镜像缺陷** —— 在 v4.0.0 的 checkout 上它跑的是 **v4.1.0** 的门禁。
现在改为通过 `resolve_release_version.sh` + `resolve_alpha_gate.sh` 解析，
与 `ci.yml` 共用同一套规则。该版本没有门禁脚本时记录
`NOT-RUN: reason=ALPHA_GATE_NOT_FOUND`（**不是** PASS）。

---

## 7. 验证

### 7.1 CI 已经是红的 —— 而三个根因都落在本批范围内
在动手之前先看了 CI 的真实状态（run 8632，develop/v4.1.0 @ `653096dd17`）：

```
lint-build         | completed | failure      ← Clippy 步骤失败
test               | completed | skipped      ← needs: [lint-build]
mutation           | completed | failure      ← 连带
ga3-security-scan  | completed | failure      ← 连带
test-soak-detach   | completed | success
postcheck          | completed | failure      ← Auto env:blocked 步骤失败
```

**根因 A：`lint-build` 的第一个红点 —— 是**编译错误**，不是 lint。**

PR #5524 的第一次 CI（run 8648）仍然红在 `lint-build`，但日志里根本不是
clippy lint，而是：

```
error: environment variable `CARGO_TARGET_DIR` not defined at compile time
  --> tests/integration/sql/four_way_compare_test.rs:445:19
  --> tests/integration/sql/four_way_cell_diff_test.rs:542:19
error: could not compile `sqlrustgo` (test "four_way_compare_test") due to 1 previous error
```

`env!("CARGO_TARGET_DIR")` 是**编译期**宏。CI 没有设 `CARGO_TARGET_DIR` ⇒
宏展开直接是编译错误 ⇒ 整个 test target 编译不过 ⇒
`cargo clippy --all-targets` 失败 ⇒ `test` / `mutation` /
`ga3-security-scan` 连带跳过或失败。

**这是一个既有缺陷，不是本批引入的**，但它落在本批要修的同一个 job 上，
必须一起修，否则 `lint-build` 不可能绿。

修法：改成**运行期**解析路径 —— `std::env::var`（运行期）→
`option_env!`（编译期可选，编译时若设过则带上）→
`env!("CARGO_MANIFEST_DIR")/target`（cargo 默认目标目录）。
两个 `report_path()` 用的就是这个。

> **订正声明**：本文件早期版本在 §7.2 里写「CI 精确 clippy 命令 … exit 0」。
> 那个结论**只在 `CARGO_TARGET_DIR` 已设的环境下成立** —— 我本地跑命令时
> 一直带着这个环境变量，于是编译期宏展开成功，把 CI 上真实存在的编译错误
> 整个遮住了。**只跑本地绿命令不足以证明 CI 绿**，必须复现 CI 的环境
> （不设该变量）再验一次。下面 §7.2 的表格已经按订正后的口径重写。

**根因 B：`lint-build` 的第二个红点 —— `clippy::collapsible_match`。**
`clippy::collapsible_match` 在 `crates/mysql-server/src/lib.rs` 的
`split_top_level_statements` 里报了一条。CI 跑的是 `-D warnings` ⇒ 直接红。
本机默认 1.97.0 **不报这条**（只有 1.95.0 报），所以本地「看着是绿的」，
CI 却是红的 —— **只有用 CI 的工具链验证才能看见**。

```rust
')' => {
    if paren_depth > 0 {
        paren_depth -= 1;
    }
}
```
→
```rust
')' if paren_depth > 0 => {
    paren_depth -= 1;
}
```

行为等价：深度为 0 的失衡 `)` 原来匹配 `')'` 臂什么也不做，现在落到 match
末尾的 `_ => {}` 兜底臂 —— 同样什么也不做（中间的 `'-'` / `'/'` / `';'`
都是别的字面量，匹配不上 `)`）。相关 23 个测试全绿。

**根因 C：`postcheck` 的「Auto env:blocked (SPEC-021)」步骤。**
就是 §5 那个 `pipefail` 中止。用 git 里的原版脚本在沙箱里复现：

```console
$ bash scripts/gate/auto_env_blocker.sh v3.8.0     # 原版脚本
  Added: docs/releases/v3.8.0/TEST_MANUAL.md
EXIT=1                                                 ← 中止
$ bash scripts/gate/auto_env_blocker.sh --check v3.8.0   # 新版
  Would add: 95 (read-only: no file was modified)
EXIT=0
```

`docs/releases/v3.8.0/` 下 `INSTALL.md`、`DEPLOYMENT_GUIDE.md`、
`MIGRATION_GUIDE.md` **没有 H1** ⇒ 原版脚本必然在它们身上中止。

**根因 D：CI 的 clippy **版本不固定** ⇒ 「本地绿」根本不能代表「CI 绿」。**

这是本轮最贵的教训。PR #5524 的第一次 CI（run 8650）里，`lint-build` 又红了，
但这次的报错是：

```
error: parameter is only used in recursion
  --> crates/vector/src/sql_vector_hybrid.rs:367:24
= help: for further information visit https://rust-lang.github.io/rust-clippy/rust-1.91.0/index.html#only_used_in_recursion
```

**`rust-1.91.0`。** 而 develop 上那次 run（8638，01:36 UTC）的报错 URL 是
`rust-clippy/rust-1.95.0`。同一个 runner label（`hp-z6g4`）、同一个镜像
（`192.168.0.252:5000/act_runner:node-rust`），**相隔一小时跑出了两种 clippy**。

怎么确认 URL 里那个数字就是工具链版本？本地用 1.95.0 触发
`clippy::redundant_clone`，URL 打的是 `rust-clippy/rust-1.95.0/index.html#redundant_clone`
—— 而 `redundant_clone` 这个 lint 本身是 1.59 就有的，所以那个数字**只能是
工具链版本**，不是 lint 的引入版本。

后果是实打实的：

| lint | 1.91.0 | 1.95.0 | 本机默认 1.97.0 |
|---|---|---|---|
| `clippy::only_used_in_recursion`（`crates/vector`，`&self` 只用于递归） | **报** | 不报 | 不报 |
| `unused-assignments`（`crates/mysql-server/src/lib.rs` 6 处死赋值） | **报** | 不报 | 不报 |
| `clippy::collapsible_match`（`crates/mysql-server/src/lib.rs`） | 报 | 报 | 不报 |
| `error: duplicated attribute`（`src/execution_engine_tests.rs`） | **报** | 不报 | 不报 |

⇒ **只在本机跑一遍 clippy，无法判断 CI 会不会红。** 本机默认 1.97.0 一条都不报。

本批的处理：

1. **代码在 1.91.0 / 1.95.0 下都要 `-D warnings` 干净**：
   * `crates/vector`：`eval_predicate` 的 `&self` 只用于递归 ⇒ 改成**关联函数**
     （`Self::eval_predicate`），不是加 `#[allow]`；
   * `crates/mysql-server/src/lib.rs`：6 处 `seq = seq.wrapping_add(1)` 是**死赋值**
     （`seq` 在 `loop {}` 内部声明，每轮重新初始化）⇒ 删掉；
     `send_binary_result_set` 的返回值同样是死的，但**调用本身有写流副作用**，
     只去掉赋值、保留调用；
   * `src/execution_engine_tests.rs`：文件头的 `#![cfg(test)]` 与 `src/lib.rs`
     的 `#[cfg(test)] mod …` **重复** ⇒ 删掉文件头那个。
2. **lint 预算基线改为按工具链分条目**（基线格式 v2）：
   `tests/baseline/{test_target,workspace}_lint_budget.json` 现在是
   `{"version": 2, "toolchains": {"1.91.0": {…}, "1.95.0": {…}}}`。
   门禁在**有本工具链条目**时做严格比较（逐 lint 名、逐计数、含「条目失效」）；
   没有条目时用**所有已记录条目的并集**比较（逐 lint 名取各工具链见过的最大值），
   并**大声打印**当前用的是并集模式。
   两种模式都是**真比较**：任何工具链都没产生过的 lint 名仍然是红，
   超过所有已记录上限的计数仍然是红；只有「这个工具链不存在这个 lint」
   不再被误判成「基线条目失效」。
3. 工具链漂移本身已开 **#5526**（runner 镜像不固定 clippy 版本），
   建议镜像固定工具链并在仓库里补 `rust-toolchain.toml`
   （现有的 `cargo-toolchain.toml` **不是** rustup 认的文件名）。

> **订正声明（第二次）**：本文件早期版本写「CI 的工具链是 rustc/clippy **1.95.0**」。
> 那个结论来自「本地能复现 CI 的报错」，但**复现的是 develop 那次 run 的工具链**，
> 而不是 PR 这边 runner 实际用的那个。CI 的 clippy 版本**会漂**，
> 所以现在基线按工具链分条目记，而不是假设某一个版本。

**根因 E：本批的门禁脚本在 CI 的 `mktemp` 上炸了（既有坑，我自己又踩了一次）。**

修完 A–D 之后，PR #5524 的第二次 CI（run 8662）`lint-build` 还是红，
但这次不是 lint，是**门禁脚本自己崩了**：

```
mktemp: : Invalid argument
scripts/gate/check_test_target_lint_budget.sh: line 172: : No such file or directory
FileNotFoundError: [Errno 2] No such file or directory: ''
❌  Failure - Main Test-target lint budget (issue #5465)
```

根因：`mktemp -t lint_budget.XXXXXX.json` —— **X 后面带后缀**的模板，
CI 镜像的 `mktemp` 不接受（`mktemp: : Invalid argument`）；
脚本又没跑在 `set -e` 下 ⇒ 失败被**吞掉** ⇒ 路径为空 ⇒ python 读到 `''` ⇒ 崩。

**这个坑仓库里早就记过**：`scripts/gate/mutate_tpch_oracle_tests.sh` 的注释里
写着同样的话（实测于 CI run 8081 / job 16475），并规定用**不带模板的
`mktemp`**。我在写本批的门禁时没有查到那条约定，又踩了一次。

⇒ 修法不是「再写一份」，而是**把已有实现抽成一处**：
新增 `scripts/gate/common.sh`（`make_temp_file` / `sha256_of`），
`mutate_tpch_oracle_tests.sh` 与 `check_test_target_lint_budget.sh` 都 source 它；
两个 helper 都**失败即报错**，绝不返回空路径让调用方带着空路径继续。
（`sha256_of` 同样要移植：`shasum` 是 Perl 脚本，瘦身镜像上未必有。）

**根因 F：CI 镜像换成 Alpine/musl ⇒ 缺静态 zlib ⇒ 任何要链接的 job 都红。**

`gate.yml` 的 `legacy-gate`（本批刚改成真跑当前版本的 alpha gate）里：

```
error: linking with `cc` failed: exit status: 1
  = note: … "-static-pie" "-Wl,-Bstatic" … "-lz" …
  = note: /usr/lib/gcc/x86_64-alpine-linux-musl/15.2.0/…/ld: cannot find -lz: No such file or directory
        have you installed the static version of the z library ?
```

Alpine 把静态 zlib 放在 **`zlib-static`** 包里，默认镜像不含；
musl 目标默认静态链接 ⇒ 必须有 `libz.a`。
⇒ `ci.yml` 的 `test` job、`mutation` job、alpha gate 的 Q2 **全部**会红
（镜像不修，任何 PR 都无法在 CI 上走完全绿）。

**仓库侧的解法**：让 zlib **从 vendored 源码静态构建**，二进制不再依赖系统
zlib —— 对数据库来说这本来就更稳（没有系统库版本错配、没有动态链接意外），
而且 CI 与发布产物构建的是同一份东西：

```toml
# [workspace.dependencies]
libz-sys = { version = "1.1", features = ["static"] }
# [dependencies]（根包）
libz-sys.workspace = true
```

`libz-sys/static` 这个 feature **必须**从直接依赖上打开（feature 只能由直接
依赖启用），而 Cargo 的 feature 是全图统一的，所以这一行覆盖 workspace 里
每一个 `flate2`。

本地验证（macOS，机制侧）：`cargo tree` 显示 `libz-sys feature "static"` 已开；
`cargo clean -p libz-sys` 后重建，构建目录里出现了 **15 个 `.o` 与
`libz.a`**（`out/lib/libz.a`）⇒ zlib 确实是从 vendored 源码编出来的，
不再需要系统 `libz.a`。musl 那一半只能由 CI 验证。

镜像本身仍然应该修（固定工具链版本 + 装 `zlib-static`）—— 已开 **#5528**。

**根因 G：`sqlrustgo-storage` 的集成测试根本编译不过（#5515 的直接后果）。**

修完 A–F 之后，`legacy-gate` 仍然红，但这次不是链接，是**编译**：

```
error[E0432]: unresolved import `sqlrustgo_storage::fault_injection`
  --> crates/storage/tests/fault_coordination.rs:36:24
   |
36 | use sqlrustgo_storage::fault_injection::{FaultPoint, Signal};
   |                        ^^^^^^^^^^^^^^ could not find `fault_injection` in `sqlrustgo_storage`
   |
note: found an item that was configured out
  --> crates/storage/src/lib.rs:29:9
   |
28 | #[cfg(any(test, feature = "fault-injection"))]
   |          ----------------------------------- the item is gated here
29 | pub mod fault_injection;
```

`crates/storage/src/lib.rs` 把 `fault_injection` 模块门控在
`#[cfg(any(test, feature = "fault-injection"))]` 下。**集成测试是独立 crate**，
`cfg(test)` 对它们**不生效** ⇒ 模块被编译掉 ⇒ 四个集成测试目标
（`fault_coordination` / `crash_commit_fsync_b1` / `crash_wal_index_b1` /
`a06_failpoint_reachability`）**一个都编不出来**。

在 CI 上的可见后果是 AFP 门禁（`legacy-gate` 的 Q2）红：

```
cargo test --no-run --release -p sqlrustgo-executor -p sqlrustgo-storage -p sqlrustgo-tools
⇒ sqlrustgo-storage 这个包构建失败 ⇒ 测试二进制没产出
⇒ test-runner 把 3 个 manifest 条目记成 Crashed
   （error_message: "Failed to execute …: No such file or directory (os error 2)"）
⇒ [Q2_ANTI_FAB] FAILED with 3 error(s) ⇒ ALPHA QUALITY BLOCKED
```

**这正是 #5515 的代价**：那 301 个 `crates/*` 测试目标在 CI 里**从未执行过**，
所以「它们根本编译不过」这件事一直没有人看见。

修法：用**自依赖 dev-dependency**，只在构建测试目标时打开该 feature：

```toml
[dev-dependencies]
sqlrustgo-storage = { path = ".", features = ["fault-injection"] }
```

Cargo 在构建测试目标时把包自身的 feature 与其 dev-dependency 的 feature 合并；
生产 `cargo build` **不构建** dev-dependency ⇒ feature 不开 ⇒ 发布产物里仍然
没有这些钩子。实测（`cargo tree -e features`）：

| 解析口径 | `fault-injection` 出现次数 |
|---|---|
| 含 dev（= 构建测试目标） | **1** |
| `--no-dev`（= 生产解析） | **0** |

`reach()` 的真实现只在 `SQLRUSTGO_FAULT_POINT` 存在时才阻塞，否则立即返回 ⇒
打开 feature 不会让其它测试挂死。

验证：

```
cargo test -p sqlrustgo-storage --no-run \
  --test fault_coordination --test crash_commit_fsync_b1 \
  --test crash_wal_index_b1 --test a06_failpoint_reachability
⇒ EXIT=0，四个可执行文件全部产出
```

**根因 H：改进程级环境变量的用例并行跑 ⇒ 偶发红灯。**

上面那个缺陷是 #5515 的**全工作区首跑**抓出来的。那一轮
（`cargo test --workspace --all-features --no-fail-fast`）同时抓到第三个缺陷：

```
---- external_tool_resolution_prefers_the_explicit_env_var stdout ----
assertion `left == right` failed: an explicit SQLRUSTGO_PSQL_BIN must win over PATH
  left: "/opt/homebrew/bin/psql"
 right: ".../ext-tool-env-37640/psql"
```

`tests/common/bin_resolver.rs` 里有三个用例改**进程级**环境变量
（`SQLRUSTGO_CLI_BIN` / `SQLRUSTGO_PSQL_BIN` / `SQLRUSTGO_MYSQL_BIN`）。
同一个测试二进制里它们**并行**执行 ⇒ 一个用例的 `remove_var` 可以落在另一个
用例的 `set_var` 与解析之间 ⇒ 被干扰的那个用例读到被清掉的变量，退回 `PATH`
上真实安装的客户端。

同一个竞态在**两个**不同的测试二进制里各炸过一次
（`a04_mysql_diff_matrix`、`load_local_infile_eagain_regression_test`）——
后者会让一个跟 LOCAL INFILE 毫无关系的用例变红。

`resolve_external_tool` **没有缓存**，它每次都现读环境变量 ⇒ 这纯粹是并行
竞态，不是解析逻辑的问题（解析逻辑是对的：显式变量优先于 `PATH`）。

修法：一把 `ENV_LOCK` 把改动环境变量的用例串起来；改动前把原值存下来，测完
原样放回去（少了这一步，用例即使独占锁，也仍然会把进程环境留在「被改过」的
状态里，污染同一个二进制里的其它用例）。这三个用例之间没有依赖关系，串行化
不改变任何断言。

**正反对照**（同一个二进制，两种代码）：

| 代码 | 二进制 | 10 轮结果 |
|---|---|---|
| 修复后 | `a04_mysql_diff_matrix` | **0 失败** |
| 修复后 | `load_local_infile_eagain_regression_test`（**刚失败过的那个**） | **0 失败** |
| 修复前（`git stash` 掉本次修复、用旧代码重建） | `load_local_infile_eagain_regression_test` | **当场复现**同一个断言、同一个 `left: "/opt/homebrew/bin/psql"` |

⇒ 确认这次修的是**竞态本身**，不是换了个环境把问题绕过去。

### 7.2 门禁自检

| 检查 | 命令 | 结果 |
|---|---|---|
| **CI 等价 clippy（不设 `CARGO_TARGET_DIR`，复现 CI 环境）** | `env -u CARGO_TARGET_DIR cargo +1.95.0 clippy --all-features --all-targets --target-dir <dir> -- -D warnings` | **exit 0** |
| 同一命令的**负对照**（恢复原版 `env!` 宏） | 同上 | **exit 101**，`error: environment variable \`CARGO_TARGET_DIR\` not defined at compile time`（行号 445 / 542，与 CI 日志逐字一致） |
| CI 精确 clippy 命令（**旧口径，已订正**） | `cargo +1.95.0 clippy --all-features --all-targets -- -D warnings` | exit 0 **但只在 `CARGO_TARGET_DIR` 已设时成立**；该口径会掩盖 CI 上的编译错误，见 §7.1 订正声明 |
| 全 workspace clippy | `cargo +1.95.0 clippy --workspace --all-features --all-targets` | **0 错误**，703 警告 / 68 个 lint 名 |
| lint 预算（root scope） | `check_test_target_lint_budget.sh` | **PASS**，0 ≤ 0，基线未被改写 |
| lint 预算（workspace scope） | `check_test_target_lint_budget.sh --scope workspace` | **PASS**，703 ≤ 703，基线未被改写 |
| 用法错误 | `check_test_target_lint_budget.sh --scope bogus` | exit 2 |
| workflow 表达式门禁 | `check_workflow_expressions.sh` | PASS（7 个 workflow 文件） |
| 格式 | `cargo +1.95.0 fmt --all -- --check` | 零输出 |
| 版本探测 SSOT | `resolve_release_version.sh` | `v4.1.0`；指定 `develop/v9.9.9`（无该目录）→ 回退到 `v4.1.0` |
| alpha gate 解析 | `resolve_alpha_gate.sh` | v4.1.0→`check_alpha_v410.sh`、v3.12.0→`check_alpha_v3.12.0.sh`、v3.8.0→`check_alpha_v380.sh`、v4.0.0→rc=3 |

### 7.3 受影响测试

| 范围 | 结果 |
|---|---|
| `crates/storage` lib（11 个改动用例） | 11/11 绿 |
| `crates/types` `wp_b_legacy` + `coverage_v3_12` | 36/36 绿 |
| `crates/common` `metrics` | 1/1 绿 |
| `crates/vector` `vector_coverage_tests` | 66/66 绿 |
| `crates/parser` `coverage_v3_12` | 366/366 绿 |
| `crates/mysql-server` lib（4 个改动用例） | 4/4 绿 |
| `crates/mysql-server` `cov_v410_server_paths`（新强断言） | 1/1 绿 |
| `crates/mysql-server` `split_top_level*`（collapsible_match 改动） | 23/23 绿 |
| `crates/executor`（14 个改动用例） | 14/14 绿 |
| `crates/admin` / `crates/graph` / `crates/tools` 改动用例 | 逐个绿 |
| `crates/storage` 四个故障注入集成测试（根因 G 修好后） | **4/4 目标编译通过**（`--no-run` ⇒ 四个可执行文件全部产出） |
| `sqlrustgo` `a04_mysql_diff_matrix`（根因 H 修好后） | **10/10 绿**，连跑 10 轮 0 失败 |
| `sqlrustgo` `load_local_infile_eagain_regression_test`（根因 H 修好后） | **10/10 绿**，连跑 10 轮 0 失败 |

### 7.4 负对照：门禁必须还能分辨

* `--check` 模式跑完 `git status docs/` 为**空** ⇒ 确认它没有改写工作树；
  写模式在同一个树上会改写 **92** 个文件。
* 原版脚本在 `v3.8.0` 上 exit 1 / 新版 exit 0 ⇒ 确认修的是「中止」本身，
  不是把失败藏起来。
* lint 预算门禁在 compare 模式下报告「基线未被改写」⇒ 确认它比较而不是
  每次重新基线化。
* `resolve_alpha_gate.sh v3.12.0` 一开始解析失败（双 `v`）⇒ **门禁在开发
  过程中抓到了作者本人的 bug**；修正后各版本全部解析正确。
* **`env!` 编译期宏的负对照**：把两处 `report_path()` 恢复成原版
  `env!("CARGO_TARGET_DIR")`，并在**不设**该环境变量的条件下跑同一条
  clippy 命令 ⇒ **exit 101**，报错文本与行号（`four_way_compare_test.rs:445`、
  `four_way_cell_diff_test.rs:542`）与 CI 日志逐字一致 ⇒ 确认这次修的
  就是 CI 真实红因，不是换了个环境把问题绕过去。恢复修复后同一条命令
  exit 0。

---

## 8. 遗留与登记

* `docs/releases/v4.1.0` 下 92 个 `.md` 缺少 `env:blocked:no-ci` 标记 ——
  `--check` 模式只报告、不改写；这是一个**已量化的文档卫生债**，
  由 pre-commit hook 在提交时补齐，不由 CI 改写工作树。
* workspace lint 基线为 **703 条 / 68 个 lint 名**（本轮已从首扫的 951 条降到
  703 条，其中 704 条来自 `crates/parser/tests/coverage_v3_12.rs` 的单一机械模式）。
  预算门禁的作用是**让它不再增长**，而不是假装它是零。
* `postcheck` 的其余步骤（证据绑定检查等）仍有 #5402 记录的既有债，
  **不在本批范围内**，本批只修掉「Auto env:blocked」这一条中止。
* 批次 17 的「进程环境变量突变」门禁脚本尚未落到 develop 上
  （对应 PR #5521 未合并），故本批无法在本地复跑该门禁。
* **#5515 的全工作区首跑（`cargo test --workspace --all-features --no-fail-fast`）
  抓到的、本批**未修**的项**，按失败类型分开登记：

  | 目标 | 判据 | 类型 |
  |---|---|---|
  | `sqlrustgo --test _q15_q16_check` | fixture 缺失时**先写 NOT-RUN 记录再 panic** ⇒ 环境阻塞被记成产品红灯 | **代码红（已定性，归 #5515 批次修）**：全树 21 个 fixture 门控目标里**只有这一个**偏离了 `else { return; }` 的既有写法 |
  | `sqlrustgo --test g13_oltp1_concurrent_select_test` | 吞吐比值断言（contended/alone = 0.60 < 阈值） | **环境阻塞（负载）**：同代码在 b15 R1（低负载）通过；本机 load 9.8（别的项目的 cargo test + 桌面进程）⇒ 归因于机器负载，不是产品缺陷 |
  | `sqlrustgo --test dml_pk_fast_path_5177` | 三个比值型用例 > 60 秒 | **环境阻塞（负载）**：同上，判据是时间比值 |

  ⇒ 这三条**不在本批修**：前者的归属批次已有明确边界，后两者在低负载下复跑
  即绿；把它们混进本批会让本批的「修了什么」变得不可核验。


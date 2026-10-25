# E5 · R9 / R11 验证与修复（2026-10-25）

> Owner 指令（Hub issue #5 comment `198562`）：**E4（R10 rustfmt 载体）与 E5（R9/R11）维持「只提不合」**，
> 且「**R11 误标未修复的判定缺陷请补一条可复跑的自检命令**」。
> 本文件是 E5 的证据；**本 PR 只提不合，不合并**。

基线：`fix/4597-chaos-t8d-awk-portability` @ `9fbb63bfbc`（与 R10 载体 #5590 同基线）。

---

## 1. R11 根因（已实测定论，可复现）

`scripts/gate/check_anti_fabrication.sh` 的 V312-24 产物校验用一段 `python3 -c "..."` 载荷验证
test-runner 报告。该载荷里有一行 Markdown 风格的注释：

```python
# `results` satisfied it, so a run that executed nothing looked valid.
```

**双引号字符串里的反引号是命令替换**，于是 shell 把 `results` 当命令执行了。实读复现（隔离 harness，
只截取门禁里那一段原样执行）：

```text
harness.sh: line 11: results: command not found
[PASS]   V312-24: test-runner report valid (1 test(s) ran, total_duration_ms=42)
--- schema.log (stderr) ---
--- schema.out (stdout) ---
1 test(s) ran, none crashed
```

即：**载荷本身仍然能跑通**，但每次跑门禁都会往校验器的 stderr 里写一行
`results: command not found`。而门禁是这样报告失败的：

```bash
log_error "  V312-24: test-runner report INVALID: $(head -1 /tmp/test-runner-schema.log)"
```

于是**真正的原因被这行噪声顶替**，报错指向「缺命令」而不是真实判决——这正是 Owner 说的
「误标未修复」：**缺陷被报成另一个缺陷，真实的红因被藏住**。

### 1.1 一个必须一并说明的事实

该反引号缺陷**在上游 develop 已经修掉了**：

```text
1ad69b7351 fix(gate): ... AFP 脚本的反引号 bug 修掉（#5545 / #5546）
```

- develop tip `e453dc42d1` 上同一行已是 `# 'results' satisfied it, ...`（单引号）⇒ 无缺陷；
- 本基线 `9fbb63bfbc` 上仍是反引号 ⇒ **缺陷仍在**。

所以 R11 在 develop 上是「已修但无回归防护」，在 fix/4597 基线上是「未修」。
**缺的不是这一次修复，而是 Owner 点名要的那条可复跑自检命令**——它能同时覆盖这两种状态。

---

## 2. R11 的可复跑自检命令

```bash
bash scripts/gate/selftest_anti_fabrication.sh              # 判定当前门禁
bash scripts/gate/selftest_anti_fabrication.sh --negative   # 证明这条自检会失败
```

### 2.1 退出码语义（三态互不混淆）

| rc | 判决 | 含义 |
|---|---|---|
| 0 | `PASS` | 全部断言成立 |
| 10 | `UNFIXED` | **缺陷复现**（真没修） |
| 20 | `INCONCLUSIVE` | **无法判定**（载荷找不到 / 结构变了 / fixture 建不起来） |
| 30 | `INTERNAL_ERROR` | 自检自身出错 |

`INCONCLUSIVE` 与 `UNFIXED` 分开是刻意的：**一个看不见载荷的自检，绝不能把载荷报成坏的**，
否则门禁一次改名或重构就会被发布成 R11 回归。

### 2.2 它检查什么

1. **静态**：从生产脚本里**原样抽取** V312-24 校验载荷（按代码锚点定位，不复制一份），
   断言其中不含反引号或 `$(` —— 双引号载荷里的命令替换就是 R11 缺陷本身。
   （载荷里的 `${REPO_ROOT}` 是**故意**的 shell 展开，不算缺陷。）
2. **动态 · 判定正确**：用三个 fixture 跑**与门禁同形的 shell 调用**
   （`python3 -c "<载荷>"`，`${REPO_ROOT}` 同样替换），断言
   - 合法报告 → 退出 0，stdout 含 `1 test(s) ran, none crashed`，**stderr 必须为空**；
   - `results` 为空 → 退出非 0，stdout 含 `results is empty`（真判定，不是误标）；
   - 缺 `results` 键 → 退出非 0，stdout 含 `missing key: results`。
3. **负向模式**（`--negative`）：把缺陷注入一份**副本**，再断言本自检能报 `UNFIXED`。
   没有这一步，一条永远不会失败的自检和一条永远通过的自检没有区别。

### 2.3 四次运行的实测矩阵（BASELINE_FAIL / FIXED_PASS / NEGATIVE_FAIL / DEVELOP_PASS）

| # | 目标 | 期望 | 实测 |
|---|---|---|---|
| 1 | 本基线 `9fbb63bfbc`（缺陷在） | `UNFIXED` | `VERDICT: UNFIXED`，**rc=10** |
| 2 | develop `e453dc42d1`（已被 `1ad69b7351` 修掉） | `PASS` | `VERDICT: PASS`，**rc=0** |
| 3 | 本基线 + 本次修复后 | `PASS` | `VERDICT: PASS`，**rc=0** |
| 4 | `--negative`（注入缺陷） | 必须检出 | `the injected defect was detected as expected`，**rc=0** |

第 1 行与第 2 行用的是**同一条脚本**，只有被测门禁不同 ⇒ 这条自检有判别力，不是恒真。
第 4 行证明它能失败。

### 2.4 本次修复

`scripts/gate/check_anti_fabrication.sh` 一行：反引号 → 单引号，与 develop 上已修的形式一致。

```diff
-# `results` satisfied it, so a run that executed nothing looked valid.
+# 'results' satisfied it, so a run that executed nothing looked valid.
```

`bash -n scripts/gate/check_anti_fabrication.sh` → rc=0。

---

## 3. R9：测试期环境变量改写加串行化

### 3.1 缺口定位

`SQLRUSTGO_EXECUTOR_PARALLELISM` 是**进程级**变量，`ExecutionEngine::new` 会读它
（`crates/mysql-server/src/lib.rs:43` 的 `read_executor_parallelism`）。

- mysql-server crate 内部：三个改写该变量的测试**已经**持有 `utilities_tests::ENV_LOCK`（#5423 修过）；
- **集成测试二进制**：`tests/integration/sql/parallel_main_path_test.rs:92-106`
  的 `test_cli_flag_propagation_via_env` **没有任何锁**，而 cargo 在同一个二进制里是**多线程并行**跑测试的
  ⇒ 该测试改写变量期间，同二进制的其他测试构造引擎时会读到 `parallel_degree = 8`。
  这就是 Q6 门禁不可复现的来源之一。

### 3.2 第二个缺陷：恢复值是错的

```rust
std::env::remove_var("SQLRUSTGO_EXECUTOR_PARALLELISM");
// restore initial state
std::env::set_var("SQLRUSTGO_EXECUTOR_PARALLELISM", initial.to_string());
```

`initial` 是**引擎报告的并行度**，不是**变量原来的值**。变量原本若未设置，这里会把它设成一个
从未存在过的值（例如 `4`），泄漏给后续测试。⇒ 「恢复」本身制造了新的不确定性。

### 3.3 修复

- 本文件加一把**本二进制自己的** `ENV_LOCK`
  （mysql-server 的 `ENV_LOCK` 是 `pub(crate)`，集成测试二进制够不着）；
- 一个 RAII 守卫 `env_guard()`：持锁 + 在 `Drop` 时**精确恢复变量原值**（`Option<String>`，
  原来是未设置就 `remove_var`），测试 panic 也不会留下残留；
- **本文件 7 个测试全部**取 `let _guard = env_guard();`——因为每个测试都构造引擎、都会读这个变量，
  只给写者上锁不足以让结果可复现；
- 删掉那段错误的「恢复」代码（改由守卫负责）。

### 3.4 残余缺口（如实登记，不在本次最小修复范围内）

根包的单元测试模块 `src/execution_engine_tests.rs` 里共有 **67 处** `ExecutionEngine::new`，
分布在多个测试函数里；只要其中任何一处在另一个测试改写变量的窗口内执行，它就会读到被改写的值
（后果是 rayon 线程池规模不同，性能差异，不影响结果值；**唯一会读出值断言**的测试是
`test_v410_atomic_parallel_degree_getter_clamps_to_one`，它断言默认 `parallel_degree() == 1`）。

本次的最小修复：
- 把**改写变量**的三个测试（`test_parallel_n1_eq_n4_cell_match`、`test_parallel_default_n1_no_env_var`、
  `test_parallel_100k_cell_match_n1_vs_n4`）与**读默认值**的那个测试用同一把锁串起来
  ⇒ 结果值层面的干扰被消除；
- 集成测试二进制同理：ctor 窗口加锁，其余部分保持并行。

**未做**：把这 67 处 ctor 全部纳入同一把锁。理由：那等于把整个根包单元测试套件串行化，
属于冻结期禁止的「大范围改造」，且真正的干净解法（子进程 re-exec、或给生产代码开一个可注入的口子）
是一次需要维护者裁决的设计选择。**该项已作为待裁决项记入 Hub issue #70 的后续清单。**

### 3.5 验证

- `rustfmt --edition 2021 --check tests/integration/sql/parallel_main_path_test.rs` → rc=0；
  该文件**不在**本分支 `cargo fmt -p sqlrustgo -- --check` 的 diff 列表里（那 11 处是既有格式债，
  由 R10 载体 #5590 处理）。
- `cargo test --test parallel_main_path_test` → 交接时仍在运行，**未取得结论**（见第 4 节）。

---

## 4. 实测输出

```text
# R9
$ cargo test --test parallel_main_path_test
  交接时仍在运行（已 14m45s）：该目标每个用例要灌 600,000 行，7 个用例合计约 4.2M 次 INSERT，
  本机实测比预期慢。读取方式：
      ps -o etime= -p <pid>            # 测试进程
      或直接重跑：CARGO_TARGET_DIR=<dir> cargo test --test parallel_main_path_test
  ⇒ **本项在交接时为 NOT YET MEASURED，不作为通过证据使用。**
  编译层面已确认：测试二进制已成功链接并进入执行（进程 10340 在跑），即 R9 的改动可编译通过。

# R11 自检矩阵
$ bash scripts/gate/selftest_anti_fabrication.sh
VERDICT: UNFIXED
REASON:  the validator payload contains a backtick, so the shell runs it as a command substitution; the gate will report the noise of that command as the reason for a red verdict
r11_selftest: UNFIXED
rc=10

$ R11_GATE_SCRIPT=<develop 的门禁> bash scripts/gate/selftest_anti_fabrication.sh
VERDICT: PASS
r11_selftest: PASS
rc=0

$ bash scripts/gate/selftest_anti_fabrication.sh        # 修复后
VERDICT: PASS
REASON:  the validator payload carries no shell command substitution, the validator judges a valid, an empty and an incomplete report correctly, and it stays silent on stderr
r11_selftest: PASS
rc=0

$ bash scripts/gate/selftest_anti_fabrication.sh --negative
negative mode: the injected defect was detected as expected
VERDICT: PASS
rc=0
```

---

## 5. 边界声明

- **只提不合**：本 PR 不合并，等 Owner 解冻后裁决；分支与 PR 保留。
- 未 force push、未改写历史、未删分支、未动 CI 配置、未动全局 runner。
- 未做新功能、未做大规模重构、未做性能优化、未改发布文档口径。
- 本文件不宣布任何发布阶段；`release_stage` 仍为 HOLD。

# 迭代 5 · 批次 23 —— 两道门禁对同一个盲区失明：测试目标执行记账（#5515）与 DRIFT 反模式扫描（#5403）

- **Issue**: http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5515（crates/* 的 301 个测试目标从未执行）、
  http://192.168.0.252:3000/openclaw/sqlrustgo/issues/5403（postcheck 的必失败步骤）
- **基线**：`develop/v4.1.0` @ `775dc5da72`
- **分支**：`fix/workspace-test-coverage-5515-batch23`
- **工具链**：两个门禁都是 shell / python 脚本，不依赖 cargo 编译

---

## 0. 一句话

两道门禁**各自**因为自己的实现细节而看不见问题：

* 「每个声明过的测试目标都必须真的跑过」这道门禁，只读**根 manifest**，
  于是 `crates/*` 的 **301** 个目标对它不存在；而且它的日志正则只匹配 cargo 的
  **相对路径**输出形态，对当前 cargo 的绝对路径输出**匹配 0 条**。
* 「DRIFT 反模式扫描」这道门禁，把扫描结果 `head -10` 截断，于是它的结论
  **依赖文件系统的列举顺序** —— 同一份代码在开发机上绿、在 CI 上红。

---

## 1. `check_test_target_execution.sh` 的两个失明面（#5515）

### 1.1 失明一：声明侧只看根 manifest

门禁比对「**已声明**的目标」与「**已执行**的目标」。已声明的一侧由一段手写的
`[[test]]` 段扫描器产生，只读**一个** Cargo.toml（根 manifest）。

实测（`cargo metadata --no-deps` vs 根 manifest 的 `[[test]]` 段）：

| | 数量 |
|---|---|
| `cargo metadata` 报告的**全工作区** test 目标 | **773** |
| 根 manifest 的 `[[test]]` 段 | **472** |
| 门禁**完全看不见**的 `crates/*` 目标 | **301** |

301 个目标既没被声明、也没被执行，对它而言根本不存在。而 `crates/*` 里装着
大量回滚 / 隔离 / 协议 / 故障注入的判别力用例。

**修法**：声明侧改用 `cargo metadata --no-deps` 作为**唯一**的目标清单来源。
它枚举每个工作区成员的每个目标，**包括 cargo 自动发现的目标**
（`tests/` 下没有显式 `[[test]]` 条目的文件）—— 手写 `[[test]]` 扫描连这些都看不见。

用 `cargo metadata` 而不是 `tomllib`：CI 镜像不保证 python >= 3.11，且只有
metadata 包含自动发现的目标。

### 1.2 失明二：日志正则只匹配 cargo 的**相对路径**输出形态

```python
RUNNING_RE = re.compile(r'Running\s+(.*?)\s+\(target/')
```

它要求二进制路径以 `target/` 开头，即只匹配：

```
Running tests/foo.rs (target/debug/deps/foo-abc123)
```

而当前 cargo 输出的是**绝对路径**：

```
Running tests/foo.rs (/abs/workspace/target/debug/deps/foo-abc123)
```

⇒ 正则匹配到 **0 条**。

**实测**：一份跑完 **472/472** 根包目标的完整回归日志，这个门禁报
「472 of 472 declared test targets never ran」—— 而那份日志里 472 个目标
**全部执行过**。这是门禁对着自己的输入完全失明。

**修法**：判据是「`Running ` 之后到左括号之间的目标路径」；括号里的二进制路径
与比较无关。

### 1.3 修好之后的实测

| 输入 | 声明 | 判定 |
|---|---|---|
| 完整回归日志（只跑了根包） | 773 | 根包 **472/472 全部正确匹配**；报「299 of 773 never ran」，299 个全是 `crates/*` 的 |
| 截断到 50 行的日志 | 773 | 报 **722** 未执行 |
| 空日志 | 773 | 报 **771** 未执行 |

后两行是**负对照**：门禁对「少跑了」有真实判别力，不是恒 PASS。

Step 1 的输出现在还打印**每个包**的目标数分项 —— 一个只打印总数的门禁可以用
一个看起来健康的数字藏起盲区。

---

### 1.4 失明三（本次新发现）：门禁拿**工作区相对路径**去比 cargo 的**包相对路径**

修完失明一与失明二之后，门禁**仍然看不见** `crates/*` 的目标 —— 但这次不是
「看不见声明」，是「**看见了却对不上**」。

`cargo metadata` 给的 `src_path` 是**相对工作区根**的：

    crates/admin/tests/admin_coverage_tests.rs

而 cargo 在日志里打印的 `Running <path>` 里的 `<path>` 是
**相对包 manifest 目录**的：

    Running tests/admin_coverage_tests.rs (/abs/.../target/debug/deps/admin_coverage_tests-…)

⇒ 两者**永远对不上**。实测（用一份真实全工作区回归日志）：

| scope | 声明数 | 已匹配上 | 判据 |
|---|---|---|---|
| `--scope crates`（修坐标前） | 301 | **0** | 301 个全部报「never ran」 |
| `--scope crates`（修坐标后） | 301 | **175** | 剩余 126 个是实跑未完成的部分 |

⇒ 同一份日志、同一份声明，只是把比较键从「工作区相对」换成「包相对」，
**从 0 个匹配变成 175 个匹配**。这条不修，`--scope crates` 永远是红。

顺带被清掉的还有 **11 条假警告**：「执行了却没声明」原来把
`tests/admin_coverage_tests.rs` 这类**成员包的**目标判成「没声明」，
因为它拿工作区相对路径去比对 ⇒ 现在这类假警告归零。

**同名路径怎么办**：有 3 个包相对路径被多个包同时使用
（`tests/coverage_v3_12.rs` 6 个包、`tests/v400_unified_backup.rs` 2 个、
`tests/unit_tests.rs` 2 个 ⇒ 共 773 个目标里 766 个不同路径、7 个目标同名）。

日志只给路径、不给包 ⇒ **无法指名**是哪一个没跑。门禁于是按**计数**匹配：
「跑了几次就是几个跑过」；不够就如实报出**差额与候选包**：

```
FAIL: path 'tests/coverage_v3_12.rs': 1 of 6 declared target(s) never ran
      (candidate packages: crates/parser, crates/storage, …)
```

⇒ **不假装能指名**，也不因为指不出名就放过（差额仍然是红）。

### 1.5 修好之后的实测：773 / 773 全部执行

全工作区首跑（`cargo test --workspace --all-features --no-fail-fast`，本地）：

| 指标 | 值 |
|---|---|
| 跑完的测试二进制 | **857** |
| `cargo metadata` 声明的测试目标 | **773** |
| 门禁判定 | **`missing test targets: 0`，`same-named paths short: 0` ⇒ PASS** |
| 门禁命令 | `bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --scope all --log <日志>` |

⇒ 这正是 #5515 要的硬证据：**声明的 773 个目标，一个不落地真的跑了**。
（修前这条命令报「299 of 773 never ran」，且那 299 个里**没有一个**是
「跑过但对不上」的误判 —— 它们是真的没跑。）

---

## 1bis. CI 侧：根包与成员包必须拆成**两个 job**（#5515）

`cargo test` 在工作区根目录**只选根 package**（工作区根本身是 package）
⇒ `crates/*` 的 301 个目标在 CI 里从未执行。

那能不能在**同一个** job 里加一条 `cargo test --workspace`？**不能**。
实测 CI runner 上根包那一轮 `Run tests` 步骤的耗时：

| run | job | `Run tests` 步骤 | 结论 |
|---|---|---|---|
| 8113 | 16564 | **147.2 分** | 被取消 |
| 8342 | 16970 | **131.6 分** | failure |
| 8031 | 16330 | **126.8 分** | 被取消 |
| 8264 | 16850 | **127.7 分** | failure |
| 8185 | 16708 | **110.3 分** | failure |
| 8344 | 16978 | 88.4 分 | failure |
| 8239 | 16791 | 87.8 分 | failure |

⇒ 根包自己就要 **86–147 分钟**，而 runner 的**容器上限是 3 h（180 分）**，
checkout + build + fixture + 收尾门禁还要 ≈ 20 分 ⇒
**两者塞不进同一个 job**（最坏情况 147 + 60 + 20 = 227 分 > 180 分）。

⇒ 拆成两个 job：

* `test`（既有）：根包，`cargo test --all-features --no-fail-fast`，超时 9000 s；
* `test-crates`（新增）：`cargo test --workspace --exclude sqlrustgo --no-fail-fast`，
  超时 5400 s（它独占 3 h 容器预算）。

两个 job 的执行记账门禁分别用 `--scope root` / `--scope crates` ——
**比较规则只有一份实现**，两个 job 的「Test summary」步骤是**整块复制**的同一份。

**`--all-features` 刻意不用**（工作区尺度）：它会打开
`sqlrustgo-mysql-server/jemalloc-prof`，而 tikv-jemalloc-sys 的 build.rs 要跑
jemalloc 的 `configure` + `make`（已读源码确认：第 200–351 行）——
CI 镜像（Alpine/musl）**不带 `make`**。根包 job 的 `--all-features` 从来没作用到
成员包上（cargo 在工作区根目录只选根 package），所以这里**不是覆盖损失**：
跑的是每个成员包**自己的**测试套件、用**默认 feature**，与 `cargo test -p <crate>` 一致。

---

## 1ter. 全工作区首跑抓到的失败，逐条定性

首跑共 **12 个目标 / 13 个用例**红。**类型必须分清** —— 三类混在一起，
就会出现「把环境阻塞当产品缺陷去修」或「把真缺陷记成既有债」。

| 目标 | 用例 | 类型 | 归属与依据 |
|---|---|---|---|
| `sqlrustgo --test _q15_q16_check` | `test_q15_subquery_from`、`test_q16_not_in_subquery` | **代码红** | fixture 缺失时它**先写 NOT-RUN 记录再 panic** ⇒ 披露与结论自相矛盾。全树 21 个 fixture 门控目标里**只有这一个**偏离 `else { return; }` 的既有写法 |
| `sqlrustgo --test a04_mysql_diff_matrix`、`load_local_infile_eagain_regression_test`、`oracle_g12_sysbench`、`oracle_g16_compat`、`tpch_hash_test` | `external_tool_resolution_prefers_the_explicit_env_var`（同一缺陷的 5 个不同二进制） | **代码红** | 改**进程级**环境变量的用例并行跑 ⇒ 竞态。`resolve_external_tool` 无缓存 ⇒ 纯粹是竞态 |
| `sqlrustgo --test g13_oltp1_concurrent_select_test` | `g13_oltp1_concurrent_selects_run_in_parallel` | **环境阻塞（机器负载）** | 吞吐比值断言（contended/alone = 0.60 < 阈值）。同代码在低负载下通过；首跑期间本机 load 9.8（另一个项目的 cargo test + 桌面进程占满 10 核） |
| `sqlrustgo-mysql-server --test client_local_infile_test` | `connection_is_usable_after_the_upload` | **代码红（前提为假）** | 断言「重复导入会覆盖」不成立（实为追加 6 行而非 4 行）；该引擎 DML 无「首列即行键」规则。**批次 20 已修** |
| `sqlrustgo-mysql-server --test tls_read_spin_5099` | `tls_eof_does_not_spin_cpu` | **环境阻塞（平台）** | 读 `/proc/<pid>/stat` ⇒ **Linux only**。CI runner 是 Linux ⇒ 不影响 CI；本机是 macOS ⇒ `/proc` 不存在 |
| `sqlrustgo-storage --test crash_commit_fsync_b1`、`crash_wal_index_b1` | 4 个崩溃用例 | **代码红（CI 接线）** | 见 §1quater |
| `sqlrustgo-bench --test tpch_test` | `tpch_q20_potential_part_promotion`、`tpch_q22_global_sales` | **代码红（前提为假）** | 见 §1quin |

⇒ **13 个用例里，5 个是同一个竞态**（一个缺陷打中 5 个二进制），
4 个崩溃用例是同一个接线缺陷，2 个是 DDL 缺列，2 个是 NOT-RUN 契约断裂，
2 个是环境阻塞（负载 / 平台），1 个是批次 20 已修的假前提。

### 1quater. 崩溃测试红了 4 个 —— 根因是 **CI 的 Build 步骤没带 feature**

`crates/storage/tests/crash_*.rs` 会拉起一个**真实服务器进程**、布一个故障点、
等它到达边界再从外面 SIGKILL。它等的那个边界在
`#[cfg(any(test, feature = "fault-injection"))]` 后面 ⇒
**服务器二进制必须带 `fault-injection` 编译**，否则 `reach()` 是空实现，
故障点**永远到不了**。

测试自己的报错就写着：

```
B1 point=after_commit_fsync sync=every run_id=… reached=false expected=10 actual=<never reached>
outcome=FAIL: server never reached fault point after_commit_fsync within 60s
```

而测试的提示写着「Build it with `cargo build -p sqlrustgo-mysql-server --all-features`」
—— **CI 从来没这么构建过**。CI 的 Build 步骤是：

```yaml
cargo build -p sqlrustgo-mysql-server -p sqlrustgo-cli     # ← 没有 feature
```

⇒ 这是**真实的 CI 接线缺陷**，而且它能存活这么久，原因正是 #5515：
**整个 `crates/*` 套件在 CI 里从未执行过** ⇒ 没人看见它红。

**修法**：`test-crates` job 的 Build 步骤改成

```yaml
cargo build -p sqlrustgo-mysql-server -p sqlrustgo-cli \
  --features sqlrustgo-mysql-server/fault-injection
```

**验证**（本地）：用带 feature 的服务器二进制重跑那个目标 ⇒

```
running 2 tests
test committed_transaction_survives_sigkill_at_commit_fsync ... ok
test crash_without_new_commit_preserves_exactly_the_seeded_rows ... ok
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 242.22s
```

⇒ 同样的测试、同样的代码，只是服务器二进制**带上了 feature** ⇒ 从红变绿。

### 1sex. 本批**未引入**的既有红（对照实测）

在 develop（`378f014529`）上跑同一道证据绑定门禁：

| 树 | 结果 |
|---|---|
| develop `378f014529`（对照） | `PASS=78, WARN=482, **FAIL=339**, UNVERIFIED=331` |
| 本批分支 | `PASS=80, WARN=470, **FAIL=341**, UNVERIFIED=332` |

⇒ 341 vs 339 的差额就是本批新增/改写的那两份证据文档带来的。
**这 339 个既有违规不由本批引入**，它们由批次 19 的批次 22 修复
（`scripts/gate/common.sh` 的 `evidence_binding_has_provenance`）解决 ——
develop 上**还没有**那个 helper（实测 `grep -c` = 0）。

⇒ 本批分支必须先 merge develop（批次 19 合并之后），
证据绑定门禁才会转绿。**在此之前不要把本批的这条红算作已修。**

### 1quin. bench 的两个 TPC-H 查询红了 —— 前提为假的断言

```
---- tpch_q20_potential_part_promotion stdout ----
panicked at crates/bench/tests/tpch_test.rs:393:5:
Q20 should execute successfully
---- tpch_q22_global_sales stdout ----
panicked at crates/bench/tests/tpch_test.rs:410:5:
Q22 should execute successfully
```

这两个查询本身是**真 TPC-H 的形状**（Q20 选 `s_name, s_address`，
Q22 选 `c_custkey, c_phone, c_acctbal`），但该文件自己建的表是**截断版**：

```sql
CREATE TABLE supplier (s_suppkey INTEGER, s_name TEXT, s_nationkey INTEGER)
CREATE TABLE customer (c_custkey INTEGER, c_name TEXT, c_nationkey INTEGER)
```

⇒ 查询选了**这张表从来没有的列** ⇒ 引擎拒绝它是**对的** ⇒
断言的**前提为假**。

对照：同一个 crate 的 benchmark example 用的是**完整** TPC-H schema
（`crates/bench/examples/serial_vs_parallel_bench.rs:201-202`），
`supplier` 有 `s_address`、`customer` 有 `c_phone` / `c_acctbal`。
⇒ **测试文件里的 DDL 才是残缺的那一份**。

⇒ 修法：**把测试的 DDL 补全到真实 TPC-H schema**（加 `s_address TEXT`、
`c_phone TEXT`、`c_acctbal REAL`），而不是把查询改短 ——
查询形状正是这个测试要考的东西。

---

## 2. `check_drift_not_pass.sh` 的顺序依赖（#5403）

### 2.1 现象：同一份代码，开发机绿、CI 红

批次 19 的 PR #5524 在 CI 上 `postcheck` 红了，红在
「6 Meta-Gate Hard Block」里的 `check_drift_not_pass`，报 4 处：

```
- scripts/gate/check_p21_audit_log.sh:77: grep on cargo test stdout without PIPESTATUS check
- scripts/gate/check_p13_soak_test.sh:84: grep on cargo test stdout without PIPESTATUS check
- scripts/gate/check_p35_simd.sh:75:  grep on cargo test stdout without PIPESTATUS check
- scripts/gate/check_p35_simd.sh:103: grep on cargo test stdout without PIPESTATUS check
```

而**同一棵树**在本地跑同一个门禁是绿的：

```
✅ No NEW anti-patterns vs baseline
✅ PASS — P14 satisfied. No gate script treats DRIFT as PASS.
```

### 2.2 根因：`head -10` 截断

```bash
grep_pattern=$(grep -rnE 'cargo test[^|]+\|[^|]*grep' scripts/gate/ 2>/dev/null | head -10)
```

扫描结果被截断成**前 10 条**，而这 10 条是**按文件系统列举顺序**取的。

实测：仓库里实际有 **26** 处符合该模式的站点，门禁只看 10 处。

* 开发机上，前 10 处恰好**都已在** `tests/baseline/drift_baseline.json` 里
  ⇒ 判定「无新增」⇒ 绿；
* CI 上，顺序把 `check_p21_audit_log.sh` / `check_p13_soak_test.sh` /
  `check_p35_simd.sh` 排进了前 10，而它们**不在**基线里
  ⇒ 判定「新增 P0 违规」⇒ 硬阻塞。

**一个结论会随目录列举顺序改变的检查，不是检查。**

### 2.3 修法

* 去掉 `head -10`，扫描覆盖**全部**匹配；
* 基线按全量结果重建 —— 记录的**真实存在的债**，不是把判据放宽；
* 顺带修掉 `total_findings` 字段（#5473）：它写的是**计数器** `$FAIL`，
  而列表里是**逐条**的 finding，所以基线声称 22 条、实际列了 29 条。
  改成 `${#FINDINGS[@]}`，即列表长度。

### 2.4 实测

| | 修前 | 修后 |
|---|---|---|
| 基线声称 / 实际条目 | 9 / 23（不一致） | 29 / 29（一致） |
| 扫描覆盖 | 26 处中的前 10 处 | **全部 26 处** |
| 连续三次运行的结论 | 随顺序变化 | **三次全部「No NEW anti-patterns」，EXIT=0** |

### 2.5 未修的既有债（本批**不**记作已修）

全量扫描暴露了 **16 处**此前从未被门禁看见的 `grep on cargo test stdout`
站点，分布在：

```
scripts/gate/check_g6_set_ops_syntax.sh        (7 处：46/52/55/58/61/64/67)
scripts/gate/check_g_correctness_v390.sh        (2 处：65/66)
scripts/gate/check_p13_soak_test.sh             (1 处：84)
scripts/gate/check_p21_audit_log.sh             (1 处：77)
scripts/gate/check_p33_cost_optimizer.sh        (2 处：75/102)
scripts/gate/check_p35_simd.sh                  (2 处：75/103)
scripts/gate/check_rc_ga_gate.sh                (1 处：907)
```

它们是**真违规**（`cargo test | grep` 丢掉了 cargo 的退出码），现在被基线
**如实登记**，而不是继续被 `head -10` 藏着。逐条修复不在本批范围 ——
本批修的是**门禁自己看不见**这件事。

基线里还登记着一条 `check_full_gate_verification.sh` 的
「run_gate() accepts exit 2 (DRIFT) without proper failure handling」——
那是**真问题**（DRIFT 被当成 PASS），属 #5434 的范围，本批同样只登记不修。

---

## 3. 与 #5515 的关系

#5515 记录了三层根因：

1. 根 `Cargo.toml` 没有声明 `default-members` ⇒ cargo 只把根包当默认成员；
2. 门禁命令没有加 `--workspace`；
3. `check_test_target_execution.sh` 的比对基准只看根 manifest ⇒ **对同一个
   盲区持续报 PASS**。

本批修的是**第 3 条**（门禁侧），以及在扫描过程中发现的第 4 条（正则失明）。

第 1、2 条（`default-members` / `--workspace`）会让 CI 的 `test` job 真的去跑
那 301 个目标 —— 那是**行为变更**，会让 CI 的运行时长与失败面显著变化，
需要单独评估与推进，**不在本批**。本批先把「门禁能不能看见」这件事修好，
让后续任何关于「要不要跑」的决策都有一个**能说实话的**门禁。

---

## 4. 复现方式

```bash
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-batch23
export TMPDIR=/Volumes/workspace/dev/.mavis-tmp

# §1：执行记账门禁
bash scripts/gate/check_test_target_execution.sh --cargo-toml Cargo.toml --log <cargo-test.log>

# §2：DRIFT 反模式扫描（跑两次：第一次建基线，第二次 compare）
bash scripts/gate/check_drift_not_pass.sh
```

---

## 5. 证据绑定

| 声明 | 证据 |
|---|---|
| crates/\* 的 301 个测试目标对门禁不可见 | §1.1：`cargo metadata` 报 773，根 manifest 的 `[[test]]` 段 472，差 301 |
| 门禁对当前 cargo 的日志形态匹配 0 条 | §1.2：完整回归日志跑完 472/472 根包目标，门禁仍报「472 of 472 never ran」 |
| 修好后根包 472/472 全部正确匹配 | §1.3 第一行：报「299 of 773 never ran」，299 个全是 `crates/*` 的 |
| 门禁拿**工作区相对路径**比 cargo 的**包相对路径** ⇒ 成员包目标永远对不上 | §1.4：同一份日志，修坐标前 `--scope crates` 匹配上 **0** 个，修后 **175** 个；11 条假警告归零 |
| 同名路径按**计数**匹配、差额如实报出 | §1.4：3 个路径被多个包使用（`tests/coverage_v3_12.rs` 6 个包…），报「1 of 6 never ran + 候选包」，不假装能指名 |
| **773 / 773** 个声明目标真的都跑了 | §1.5：跑完 857 个测试二进制，门禁判 `missing: 0`、`same-named short: 0` ⇒ PASS |
| 根包与成员包必须拆两个 job | §1bis：runner 上根包 `Run tests` 实测 86–147 分，容器上限 3 h ⇒ 塞不下 |
| workspace 尺度不能用 `--all-features` | §1bis：会打开 `jemalloc-prof` ⇒ tikv-jemalloc-sys 的 build.rs 要 `configure` + `make`，Alpine 镜像不带 `make` |
| 首跑的 13 个失败用例逐条定性 | §1ter：5 个同一竞态 / 4 个崩溃用例 / 2 个 NOT-RUN 契约 / 2 个环境阻塞 / 2 个 DDL 缺列 / 1 个批次 20 已修 |
| 崩溃测试红的根因是 CI Build 步骤缺 feature | §1quater：`cargo build -p sqlrustgo-mysql-server`（无 feature）⇒ `reach()` 是空实现；带 feature 重跑 2/2 绿（242.22s） |
| bench 两个红灯是前提为假的断言 | §1quin：查询选了 DDL 里没有的 `s_address` / `c_phone` / `c_acctbal`；对照 benchmark example 的完整 TPC-H schema |
| 门禁对「少跑了」有判别力 | §1.3 负对照两行：截断日志报 722、空日志报 771 |
| 26 处站点中门禁只看 10 处 | §2.2：`grep -rnE 'cargo test[^|]+\|[^|]*grep' scripts/gate/ \| wc -l` = 26，而门禁用 `head -10` |
| 修后结论不再随顺序变化 | §2.4：连续三次运行全部「No NEW anti-patterns」，EXIT=0 |
| 基线条目数与实际一致 | §2.4：修后 `total_findings` 29、列表 29 条（修前 9 / 23） |
| 16 处此前不可见的真违规已被登记 | §2.5：逐文件逐行号列出 |

---

## 6. 订正声明

本文件最初假定「`check_drift_not_pass.sh` 在 CI 上的 4 处违规是本批引入的」。
实测证明不是：它们在 develop 上就存在，只是被 `head -10` 藏起来了 ——
本地绿是因为前 10 处恰好已登记，CI 红是因为顺序不同。本文已按实测重写，
结论从「修掉 4 处新违规」改为「修掉**让门禁看不见**的那个 `head -10`，
并把真实存在的债如实登记」。

文中所有数字均来自本轮实测命令输出。

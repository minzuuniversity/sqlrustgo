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

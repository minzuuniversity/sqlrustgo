# 批次 43 证据 —— 测试不得覆写已归档的证据（#5577 / #5389）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b43-evidence-output-dir-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 43（本机实测，Mac-mini-7.local arm64 macOS 27.0.0） |
| timestamp | 2026-10-25T01:25:00+08:00 |
| 基线 | `0acb05171981bf78f88485fde28b9700c135dd83`（批次 42 之后、批次 43 之前） |
| 工作树 | `wt-v410-b40`（分支 `fix/ddl-case-and-idle-reaper-batch40`） |
| 对照工作树 | `wt-v410-b42base`（detach `572f2b0ad2`） |
| 关联 issue | #5577（直接）；#5389（同族，81 个文件）；#5466（A04 证据自相矛盾，下一批） |

---

## 0. 一句话

#5577 说的是「跑一次 `cargo test --workspace` 就把已归档的证据抹掉」。
本批做三件事：**找出全部五处**、**收敛成一条规则**、**加一道能自己证明有判别力的门禁**。

| 方向 | 修前 | 修后 |
|---|---|---|
| **输出目录** | 5 处测试各自硬编码 `docs/…` 归档路径（其中 `v312_mixed_soak.rs` 的注释已经写着"应该出仓"、代码却仍指向仓内） | **1 份共享实现** `tests/common/evidence_output.rs`：默认写 `target/<name>/`；写进仓库必须**显式声明**环境变量 |
| **引擎不可用时** | 仍写文件，内容是 `ENGINE_UNAVAILABLE: …` 占位符 | **不写**。归档证据是**产物**，不是**日志** |
| **防复发** | 无 | **新门禁** `scripts/gate/check_tests_dont_write_archived_evidence.sh`（两层判据 + 自测），接进 CI 的 `test` 与 `test-crates` job、`STAGE_CONFIG.yaml` 的 GA `required_gates` |

---

## 1. 根因：不是「忘了跳过」，是**输出目录是硬编码的假设**

#5577 描述的现象（pg 容器不可用时不跳过、只写占位符）只是**症状**。
真正让这件事能反复发生的是一条**从未被写下来的假设**：

> 「测试就该把产物写进证据目录。」

这条假设让**五处**测试各自硬编码了归档路径，于是**每一个**跑过全仓测试的开发者
都在无声地覆写证据。五处的具体形态还不一样：

| # | 位置 | 修前的形态 |
|---|---|---|
| 1 | `tests/integration/compat/a04_mysql_diff_matrix_test.rs` | `evidence_root()` 硬编码 `docs/releases/v4.1.0/evidence/a04-5226`；引擎不可用时**不跳过**、写 `ENGINE_UNAVAILABLE:` 占位符 |
| 2 | `tests/soak/v312_mixed_soak.rs` | 注释明写「the output path is overridable so that a runner can keep the artifact **OUT of the repository tree**」，但**默认值**仍指向 `docs/releases/v3.12.0/evidence/v312-59/soak/` ⇒ **意图与行为不符** |
| 3 | `tests/integration/tpch/tpch_sf01_oracle_dump_test.rs` | `out_dir` 硬编码 `docs/…/cross_engine_sf01/sqlrustgo` |
| 4 | `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs` | `DEFAULT_REPORT_PATH` 硬编码 `docs/releases/v3.11.0/perf/SF1_BASELINE_REPORT.md` |
| 5 | `tests/integration/sql/teaching_corpus_oracle_test.rs` | `DIFF_DIR` 硬编码 `docs/…/teaching_corpus/oracle_diffs`（`GOLDEN_DIR` 同样是 `docs/`，但它是**只读**的期望值 ⇒ 保留） |

第 2 条值得单独说：它的**注释**已经把正确做法写出来了（可覆盖、为了出仓），
只有**默认值**没跟上。⇒ 这不是"忘了做"，是**规则只存在于注释里，没有一份实现**。

---

## 2. 修法：一条规则，一份实现

### 2.1 共享实现 —— `tests/common/evidence_output.rs`（**唯一实现**）

规则只有两条：

1. 设置了环境变量 ⇒ 用它（写到哪里是**显式声明**的）；
2. 没设置 ⇒ 写进 Cargo 的**构建产物目录** `target/<name>/`，**绝不**写进仓库的 `docs/`。

提供两个入口（语义不同，不能混）：

| 函数 | 环境变量里给的是 | 用途 |
|---|---|---|
| `evidence_dir(env_var, name)` | **目录** | A04 的 `wire/`、TPC-H oracle dump、teaching diff |
| `evidence_file(env_var, name)` | **完整文件路径** | soak summary、TPCH 报告 |

`evidence_file` 单独存在，是因为 `SQLRUSTGO_SOAK_SUMMARY_PATH` 与 `TPCH_REPORT_PATH`
在改动前就是**完整文件路径** ⇒ 语义必须保持，否则会破坏既有调用者
（#5491 留下的那两个 env 就是这么用的）。

`target_dir()` 对 `CARGO_TARGET_DIR` **只接受绝对路径**：它是**相对路径**时按
**当前工作目录**解析，而集成测试的工作目录是**包目录**不是仓库根 ⇒ 相对路径会落到别处。

### 2.2 引擎不可用 ⇒ **不写**

`a04` 的落盘循环改成跳过 `!e.available` 的引擎。

理由不是"少写点东西"，而是：**归档证据是产物，不是日志**。
一次跑不起来的测试**没有资格**覆盖上一次成功运行留下的材料 ——
它甚至不知道自己跑得对不对（#5577 里 A04 正是在"参照引擎不可用"的状态下
把归档抹掉的）。

### 2.3 各调用点的显式声明方式

| 调用点 | 环境变量 | 重新生成归档材料时 |
|---|---|---|
| A04 差分矩阵 | `SQLRUSTGO_A04_EVIDENCE_DIR` | `SQLRUSTGO_A04_EVIDENCE_DIR=docs/releases/v4.1.0/evidence/a04-5226 cargo test --all-features --test a04_mysql_diff_matrix -- --nocapture` |
| TPC-H SF=0.1 oracle dump | `SQLRUSTGO_TPCH_SF01_ORACLE_DIR` | 同形 |
| TPC-H SF=1 报告 | `TPCH_REPORT_PATH`（**文件**路径，语义不变） | 同形 |
| teaching corpus diff | `SQLRUSTGO_TEACHING_DIFF_DIR` | 同形 |
| soak summary | `SQLRUSTGO_SOAK_SUMMARY_PATH`（**文件**路径，语义不变） | 同形 |

---

## 3. 决定性 A/B 实测（同一份测试代码，两个树）

跑**同一条命令**：

```bash
cargo test --test a04_mysql_diff_matrix
```

| 侧 | 结果 | `docs/` 下被覆写的已跟踪文件数 |
|---|---|---|
| **BASE**（`572f2b0ad2`，改动前） | 12 passed / 8.66 s | **20**（`docs/releases/v4.1.0/evidence/a04-5226/wire/*_postgres.txt`） |
| **FIXED**（改动后） | 12 passed / 8.27 s | **0** ✅ |

* #5577 说「21 个」，本机实测是 **20 个** —— 以实测为准。
* FIXED 侧的产物落在 `target/a04-5226/`：`matrix.json` + `wire/` 下 82 个文件。

**这就是 #5577 的完整复现与修复**：同一个命令，同一台机器，
修前把 20 个已归档的对照输出换成占位符，修后一个都不动。

---

## 4. 门禁 —— `scripts/gate/check_tests_dont_write_archived_evidence.sh`

### 4.1 两层判据

| 层 | 判据 | 抓什么 |
|---|---|---|
| **L1（静态，精确）** | 测试源码里的**写调用**，其实参直接含 `docs/` 路径字面量 ⇒ FAIL，点名文件与行号 | **直接写** |
| **L2（行为，承重）** | 工作树里 `docs/` 下的**已跟踪文件**被改动 ⇒ FAIL | **一切间接写**：经由常量、经由函数、经由 env 拼出来的路径 |

**L2 才是承重的那一层**：L1 只认字面量，而修之前的 A04 是经由 `evidence_root()`
拼出路径的 —— **L1 看不见它**。L2 看的是**结果**（文件真的被改了吗），
所以任何写法都逃不掉。

L2 检查的是**当前工作树的状态**，因此它必须在**测试跑完之后**、**同一个 job 内**执行。
⇒ 已接进 CI 的 `test` 与 `test-crates` 两个 job（`if: always()`，这样
测试失败或被超时杀掉时也会检查 —— 那正是最可能留下半截证据的运行）。

### 4.2 扫描范围与豁免

扫描 `tests/` 与 `crates/*/tests/`。**没有按文件名开的例外** ——
判据是「写调用的实参里有没有 `docs/` 路径」，与文件名无关。

修完之后静态扫描的结果是 **0 命中**（修前是 5 处）：

| 文件 | 修前 | 修后 |
|---|---|---|
| `a04_mysql_diff_matrix_test.rs` | 写 `docs/…/a04-5226` | 写 `target/a04-5226` |
| `v312_mixed_soak.rs` | 写 `docs/…/v312-59/soak/` | 写 `target/soak/` |
| `tpch_sf01_oracle_dump_test.rs` | 写 `docs/…/cross_engine_sf01/sqlrustgo` | 写 `target/tpch/…` |
| `tpch_sf1_22_vs_3engines_test.rs` | 写 `docs/…/SF1_BASELINE_REPORT.md` | 写 `target/tpch/` |
| `teaching_corpus_oracle_test.rs` | 写 `docs/…/oracle_diffs` | 写 `target/teaching_corpus/` |

`teaching_corpus_oracle_test.rs` 的 `GOLDEN_DIR` **仍在 `docs/` 下** ——
它是**只读**的期望值（golden 对照），不是本次运行的产物 ⇒ 保留原位是正确的。

### 4.3 自测与判别力

```bash
bash scripts/gate/check_tests_dont_write_archived_evidence.sh --selftest
```

自测做三件事：
1. **注入一条违规**（`fs::write("docs/releases/v4.1.0/evidence/selftest.txt", …)`）
   ⇒ 门禁必须抓到（实测 `HITS=1`）；
2. 注入后**当前树必须干净**（实测 `HITS=0`）；
3. 探针用 `trap` 清理 ⇒ 脚本中途失败也不会在仓库里留残留（实测 `git status` 干净）。

**退出码**（fail-closed）：

| 场景 | 退出码 |
|---|---|
| L1 有违规 | **1** |
| L2 发现 `docs/` 被改动 | **1** |
| 两层都干净 | **0** |
| `--selftest` 通过 | **0** |

实测：脏树 `exit=1`、干净树 `exit=0`、`--selftest` `exit=0`。

### 4.4 一个必须记录的实测坑

门禁的第一版用 `sed -n 's/^L1_HITS=//p'` 从扫描输出里取计数，
而扫描输出的明细行含中文 ⇒ macOS 自带的 **BSD sed** 报
`RE error: illegal byte sequence`，把整条判据的输出吃掉，
随后 `set -u` 下 `$HITS` 未绑定而报 `unbound variable`。

⇒ 修法：**扫描器的输出全部改成纯 ASCII**（明细行与计数行），
并给 `sed`/`grep` 加 `LC_ALL=C`。这条记在这里，是因为它**不是**可以忽略的环境噪声：
它会让门禁**假绿**（计数取不到 ⇒ 走 `-lt 1` 的默认分支）。

---

## 5. 接入点

| 接入点 | 内容 |
|---|---|
| `.gitea/workflows/ci.yml` → `test` job | 在 `Run tests` **之后**加一步跑门禁（`if: always()`）⇒ L2 在同一个 job 的同一个工作树上检查 |
| `.gitea/workflows/ci.yml` → `test-crates` job | 同上（成员 crate 的测试也需要各自检查自己的工作树） |
| `docs/governance/STAGE_CONFIG.yaml` → GA `required_gates` | `scripts/gate/check_tests_dont_write_archived_evidence.sh`（与批次 41 的元门禁同一处登记） |
| `scripts/gate/check_tests_dont_write_archived_evidence.sh` | 新门禁（755，`bash -n` 通过） |

`check_stage_config_consistency.sh` 复跑：**FAIL findings 0 / DISCLOSURE findings 0 / RESULT PASS**。

---

## 6. 回归矩阵

| 对象 | 命令 | 结果 |
|---|---|---|
| A04 差分矩阵 | `cargo test --test a04_mysql_diff_matrix` | **12 passed / 0 failed**，8.27 s；`docs/` **0 污染** ✅ |
| 另三个改动的目标 | `cargo test --no-run --test tpch_sf01_oracle_dump --test tpch_sf1_22_vs_3engines_test --test teaching_corpus_oracle_test` | 编译通过，**0 告警** ✅ |
| soak 目标 | `cargo test --no-run --test v312_mixed_soak` | 编译通过，**0 告警** ✅ |
| 门禁自测 | `bash scripts/gate/check_tests_dont_write_archived_evidence.sh --selftest` | **PASS**（注入被抓 + 树干净 + 无残留）✅ |
| 门禁退出码 | 脏树 / 干净树 / selftest | **1 / 0 / 0** ✅ |
| 门禁语法 | `bash -n` | 通过 ✅ |
| STAGE_CONFIG 一致性 | `bash scripts/gate/check_stage_config_consistency.sh` | **PASS**（0 FAIL / 0 DISCLOSURE）✅ |
| ci.yml 合法性 | YAML 解析 | 通过；两个 job 的门禁步骤都在 `Run tests` 之后 ✅ |

---

## 7. 改动清单

| 文件 | 改动 |
|---|---|
| `tests/common/evidence_output.rs` | **新增**：输出目录规则的**唯一实现**（`evidence_dir` / `evidence_file` / `target_dir`） |
| `tests/integration/compat/a04_mysql_diff_matrix_test.rs` | `evidence_root()` 改走共享实现（默认 `target/a04-5226`）；引擎不可用时**不写**占位符；模块文档更新 |
| `tests/soak/v312_mixed_soak.rs` | 默认输出改走共享实现（原默认值仍在 `docs/` ⇒ 与注释矛盾） |
| `tests/integration/tpch/tpch_sf01_oracle_dump_test.rs` | `out_dir` 改走共享实现 |
| `tests/integration/tpch/tpch_sf1_22_vs_3engines_test.rs` | `report_path()` 改走共享实现（`TPCH_REPORT_PATH` 语义不变） |
| `tests/integration/sql/teaching_corpus_oracle_test.rs` | `DIFF_DIR` 改走共享实现（`GOLDEN_DIR` 保持只读原位） |
| `scripts/gate/check_tests_dont_write_archived_evidence.sh` | **新增**：两层判据的门禁 + `--selftest` |
| `.gitea/workflows/ci.yml` | `test` 与 `test-crates` 两个 job 各加一步门禁（`if: always()`） |
| `docs/governance/STAGE_CONFIG.yaml` | GA `required_gates` 登记新门禁 |

---

## 8. 没做的事 / 需要属主裁决的

1. **#5389（81 个文件）** —— 本批修的是**五处硬编码 `docs/` 写入路径**，
   L2 门禁能覆盖**其余任何**写归档证据的行为，但本批**没有**逐个改那 81 个文件
   （多数是**只读**的期望值，混在「写 docs 的」一起改会误伤）。
   ⇒ #5389 的剩余部分由 L2 门禁守住，是否要逐个迁到 `target/` 请属主裁决。
2. **#5466（A04 证据内部自相矛盾且无法从 clean checkout 复现）** —— 下一批。
   本批**只**修了「测试会覆写证据」这一半；「已归档的证据本身自相矛盾」是另一半，
   两者要分开取证、分开修。
3. **归档产物的生成流程** —— 现在「重新生成归档材料」需要显式设环境变量。
   这是**故意**的门槛；但是否要补一个**官方的生成脚本**（把 env 设好并调用），
   请属主裁决（本批**没有**加，因为它会重新引入「一条命令改写归档」的能力，
   只是把它藏起来）。

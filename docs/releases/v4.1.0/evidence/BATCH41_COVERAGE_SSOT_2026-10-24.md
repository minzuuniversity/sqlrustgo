# 批次 41 证据 —— 覆盖率阈值一份实现 + 测量一份实现的**元门禁**（#5581）

| 字段 | 值 |
|---|---|
| gate_policy_eval_id | `v410-b41-coverage-ssot-001` |
| source_agent | openclaw-mcode (MiniMax Code) |
| source_run | 批次 41（本机实测，Mac-mini-7.local arm64 macOS 27.0.0） |
| timestamp | 2026-10-24T15:05:00+08:00 |
| 基线 | `develop/v4.1.0` = `5d4817a06b9dbfa0ca232ab639975d31a887e539` + PR #5578 的 head `cd66e68310d061ca3976a6ec05c1c2b170ede22e`（批次 37/38/39，已并入本分支） |
| 工作树 | `wt-v410-b40`（分支 `fix/ddl-case-and-idle-reaper-batch40`） |
| 关联 issue | #5581（阈值 5 份 + 测量 6 份全量普查） |

---

## 0. 一句话

#5579 把**最要命的一处**第二份实现修掉了（RC/GA 门禁的 A5 测不出就静默跳过）。
本批做的是剩下的部分，而且加了一条**让第二份实现长不出来**的门禁：

| 方向 | 修前 | 修后 |
|---|---|---|
| **阈值（该有多少）** | **5 份实现、4 套互不相同的取值**（SSOT 50/75/80/85 vs 脚本里的 75 / 80.0 / 47 / 42 / 50）——本批**又查出第 6 处**（`gate/gate.sh` L2 的 50） | **1 份实现**：`stage_coverage_min()` 读 `docs/governance/STAGE_CONFIG.yaml → thresholds`；所有门禁按**自己的阶段**取阈值 |
| **测量（怎么测）** | **6 份实现**；CI 链路上的 3 份里有 2 份会把「没测到」变成一个看起来正常的数字 | 判定用的测量**全部**走 `scripts/gate/common.sh` 的共享实现；其余的**逐个标注角色**（reporting / report-generator / legacy） |
| **防复发** | 无 | **新门禁** `scripts/gate/check_coverage_single_implementation.sh`：任何脚本直接调 `cargo llvm-cov` 而没有登记 ⇒ **FAIL**，并点名文件与行号 |

---

## 1. 阈值：5 份实现（+ 本批查出的第 6 处）

### 1.1 修之前的全量普查

| # | 位置 | 取值 | 与 SSOT 的关系 |
|---|---|---|---|
| 1 | `docs/governance/STAGE_CONFIG.yaml` → `thresholds` | ALPHA 50 / BETA 75 / RC 80 / GA 85 | **SSOT 本身** |
| 2 | `scripts/gate/check_rc_ga_gate.sh` | `COVERAGE_MIN=75`（注释写「Alpha: 75%, Beta/RC: 80%」） | 与 SSOT **两边都对不上**（SSOT 的 ALPHA 是 50、GA 是 85），连自己的注释也不符 |
| 3 | `scripts/gate/check_coverage_v312.sh` | `THRESHOLD=80.0` | 它是 **GA 的 `required_gates` 之一** ⇒ GA 的判据实际按 80 判，而 SSOT 说 85 |
| 4 | `scripts/gate/check_10_principles_v310.sh` R5 | 逐 crate 47 / 42 | **不是阶段阈值**，是 v3.10.0 给自己定的**逐 crate** 目标（`V310_10_COVERAGE_PLAN.md`）——不同的量 |
| 5 | `scripts/gate/check_alpha_quality_v4.10.sh` | 50（只出现在注释里；Q7 已延期，命令未执行） | 与 SSOT 的 ALPHA 一致，但**硬编码** |
| **6（本批新发现）** | `gate/gate.sh` L2 | 50 | 同样是硬编码；issue 的普查没数到它 |

⇒ 同一个量、同一个阶段，不同脚本给出不同门槛。实测值高于所有门槛，
所以**今天不产生假红**；但门槛一旦被调（87.78 → 84），
`check_coverage_v312.sh`（按 80）与 SSOT（按 85）会给出**相反的结论**，而两边都"有依据"。

### 1.2 修法：一个读取器，fail-closed

新增 `scripts/gate/stage_config_common.sh::stage_coverage_min()`：

```bash
stage_coverage_min() {
    local stage="$1"
    local cfg="${2:-$STAGE_CONFIG_PATH_DEFAULT}"
    python3 - "$cfg" "$stage" <<'PYEOF'
...
thresholds = d.get('thresholds') or {}
key = 'COVERAGE_MIN_%s' % str(sys.argv[2]).strip().upper()
value = thresholds.get(key)
if value is None:
    raise SystemExit(1)
print('%g' % float(value))
PYEOF
}
```

放在 `stage_config_common.sh` 而不是新开一个文件，是因为那个文件**已经是** STAGE_CONFIG 的
唯一读取处（8 个 reader，共用一个 YAML 解析器）——再开一个就是第二份解析器。

**fail-closed**：阈值缺失 / 阶段未知 / SSOT 解析不了 ⇒ 返回非零、**不输出任何值**。
调用方不得回落到默认值：

| `SQLRUSTGO_IDLE_TIMEOUT_SECS` 式的回退 | 为什么不行 |
|---|---|
| 回落 0 | 0 会让任何覆盖率都通过 |
| 回落默认 | 默认也是一份「写在别处的数字」 |

### 1.3 每个门禁按**自己的阶段**取阈值

| 门禁 | 阶段来源 | 阈值 |
|---|---|---|
| `check_rc_ga_gate.sh` | **本门禁的 mode**（`coverage_min_for_mode`：alpha→ALPHA / beta→BETA / rc→RC / ga→GA / **all→GA**） | 50 / 75 / 80 / 85 / 85 |
| `check_coverage_v312.sh` | `SQLRUSTGO_COVERAGE_STAGE` 环境变量；未设置 ⇒ **当前发布版的当前阶段** | 同上 |
| `check_alpha_quality_v4.10.sh` | 显式 `SQLRUSTGO_COVERAGE_STAGE=alpha`（命令里写死） | 50 |
| `gate/gate.sh` L2 | **ALPHA**（L1 是 alpha 层的代码质量步） | 50 |

> `all → GA` 不是随手定的：`gate_dimensions_for_mode` 里 `ga` 与 `all` 跑的是**同一组维度**
> （「ga and all are the same set, by design」），所以这两个模式也不该被不同的覆盖率门槛判。

### 1.4 实测

```
$ source scripts/gate/stage_config_common.sh
ALPHA  -> 50        DRAFT  -> rc=1（null ⇒ fail-closed）
BETA   -> 75        NOPE   -> rc=1（未知阶段 ⇒ fail-closed）
RC     -> 80
GA     -> 85

$ coverage_min_for_mode
alpha  -> ALPHA 50      beta  -> BETA 75      rc    -> RC 80
ga     -> GA 85         all   -> GA 85        nope  -> GA 85（未知 mode 取最严）
```

`check_coverage_v312.sh` 的三条路径：

```
$ SQLRUSTGO_COVERAGE_STAGE=nope bash scripts/gate/check_coverage_v312.sh
FAIL: no coverage threshold for stage 'nope' in the SSOT
      (docs/governance/STAGE_CONFIG.yaml → thresholds → COVERAGE_MIN_<STAGE>)
EXIT=1

$ SQLRUSTGO_COVERAGE_STAGE=ga bash scripts/gate/check_coverage_v312.sh
Coverage criterion: stage=ga threshold=85%

$ bash scripts/gate/check_coverage_v312.sh          # 未设置 ⇒ GA（SSOT 里最严的一档）
Coverage criterion: stage=ga threshold=85%
```

> **未设置时为什么是 GA 而不是「当前发布版的当前阶段」**：v4.1.0 的当前阶段是
> ALPHA（=50）。若默认值取它，则**每一个忘记声明阶段调用方**都会把门槛从 80
> 悄悄降到 50 —— 重构绝不能顺手把门禁放松。所以默认值取 SSOT 里**最严**的一档：
> 忘了传参只会判得更严，不会更松。详见 §1.6。

### 1.5 一处**必须说清的行为变化**

CI 的 L1 覆盖率步骤（`.gitea/workflows/ci.yml`）现在显式写
`SQLRUSTGO_COVERAGE_STAGE=ga`，即 **80 → 85**。

依据：该步骤**自己的注释**两处都写明它就是 GA 的覆盖率判据
（「`check_coverage_v312.sh` — the very script STAGE_CONFIG.yaml's **GA**
`required_gates` lists」/「which is GA's own `required_gates` entry」），
而 `docs/releases/v4.1.0/GA_GATE_REPORT.md:104` 也写着「门槛来源：STAGE_CONFIG.yaml → thresholds」。
⇒ 一个自称是 GA 判据、却按 80 判的脚本，正是 #5581 报告的那类缺陷。

**GA = 85 不是本批的新决定**，而是执行属主**已有的裁决**：

* `docs/releases/v4.1.0/GA_GATE_REPORT.md` §G3 口径提醒 ①：「`STAGE.yaml` →
  `rc_to_ga.exit_criteria` 写「Coverage >= 80%」，而 `COVERAGE_MIN_GA` 是 **85**
  ……⇒ **GA 按 85 判**」；
* `docs/releases/v4.1.0/STAGE.yaml` → `coverage_target_for_ga: 85%`
  （同一处注明「80% 是 COVERAGE_MIN_RC」）。

**代价**（如实记录）：每 PR 的覆盖率门槛从 80 抬到 85，余量从 ~7.8 点变成 ~2.8 点。
实测值见 §3。门槛一旦逼近，PR 会开始红——那是 SSOT 说的，不是脚本自己加的。

### 1.6 本批**新发现**：治理文档自己就有第二套数字（已收口）

收口阈值来源时，顺着「谁在声明这个量」一路查下去，发现**文档层**还有一份
互相矛盾的取值，而它之前**没有人执行过**：

| 位置 | 原值 | 与 SSOT 的关系 |
|---|---|---|
| `docs/governance/GATE_CONDITIONS.md` §G17 表 | ALPHA **75** / BETA **80** / RC **80** / GA **80** | 与 SSOT（50/75/80/85）**四个阶段里三个对不上** |
| `docs/governance/STAGE_CONFIG.yaml` → `thresholds` | ALPHA 50 / BETA 75 / RC 80 / GA 85 | **SSOT** |

这正是 #5581 说的「同一个量、同一个阶段，不同脚本给出不同门槛」在**文档形态**
上的体现——而且更隐蔽：G17 的 ALPHA=75 **从来没有被任何脚本执行过**
（`check_alpha_quality_v4.10.sh` 一直按 50 判），GA=80 则是**被执行的**
（`check_coverage_v312.sh` 硬编码 `THRESHOLD=80.0`）⇒ **声明 ≠ 执行**。

**处置**：G17 表**不再持有第二份数字**，改为**只指向 SSOT**
（`STAGE_CONFIG.yaml` → `thresholds.COVERAGE_MIN_<STAGE>`），并保留脚本列不变
（`check_gate_script_refs.sh` 仍能解析）。各阶段的归属写清楚：

| 阶段 | 归属 | 依据 |
|---|---|---|
| ALPHA = 50 | SSOT | 与 `check_alpha_quality_v4.10.sh` 一直实际执行的 50 一致（G17 原写 75，从未执行过） |
| BETA = 75（SSOT） | SSOT | **但 ALPHA→BETA 晋级审核仍按 80%** —— `ALPHA_TO_BETA_GATE_PLAN.md` §2 AB-06 的临时裁决（「在治理文件统一前，执行较严格值」）继续有效，本批不动它 |
| RC = 80 | SSOT | 与 `POST_GA_PLAN.md` M11 的回归下限一致 |
| GA = 85 | SSOT | 属主既有裁决（§1.5） |

**所有调用点现在都显式声明阶段**（不留"忘了传参"的解释空间）：

| 调用点 | 阶段 | 门槛 | 相对本批之前 |
|---|---|---|---|
| `.gitea/workflows/ci.yml` → Gate L1 | `ga` | 85 | **80 → 85**（收紧，见 §1.5 的代价） |
| `.github/workflows/ci-pr.yml` → Gate G17 ×2 | `ga` | 85 | 80 → 85（与 Gitea CI 对齐，同一个量同一个数） |
| `scripts/gate/check_alpha_quality_v4.10.sh` | `alpha` | 50 | 不变 |
| `scripts/gate/check_v312_gate_thresholds.sh`（v3.12.0 遗留） | `rc` | 80 | 不变（v3.12.0 的 `COVERAGE_MIN_PER_CRATE: 80`，RC 正是 SSOT 里等于 80 的那一档） |
| 其它（未声明） | `ga`（默认） | 85 | 只会更严，不会更松 |

**顺带修正的两处指针错误**（会让读者去不存在的键里找）：

* `GA_GATE_REPORT.md:104`「门槛来源：`STAGE_CONFIG.yaml` → **`constants`**」⇒
  实际键名是 **`thresholds`**（YAML 里根本没有 `constants` 键；#5581 的正文
  也沿用了这个笔误）；
* `POST_GA_PLAN.md` §5 的每周三连跑 ⇒ 补上显式阶段
  `SQLRUSTGO_COVERAGE_STAGE=rc`（M11 的下限本就是 `COVERAGE_MIN_RC` = 80%）。

---

## 2. 测量：判定用的全部走共享实现，其余逐个标注角色

### 2.1 全量普查（6 份）+ 本批的新分类

| # | 位置 | 修前的形态 | 本批处置 |
|---|---|---|---|
| A | `scripts/gate/common.sh` `coverage_measure_crate` / `coverage_measure_set` / `coverage_verdict` | feature 推导 + 环境相关排除 + `--test-threads=1` + 抽 Lines 列 + fail-closed | **唯一实现**（批次 39 引入） |
| B | `scripts/gate/check_rc_ga_gate.sh` 的 A5 | 第二份实现，测不出就静默跳过 | 批次 39 已收敛到 A |
| C | `gate/gate.sh` L2 | 抽的是 **regions 计数**（恒 PASS） | 批次 39 修了抽列；本批把**阈值**也换成 SSOT 的，并**显式标注它是 reporting**（只 WARN，不判定） |
| D | `scripts/gate/check_10_principles_v310.sh` R5 | `--tests`，无 feature 推导 / 无排除 / 无 `--test-threads=1` | **本批改为调用共享的 `coverage_measure_crate`**；47/42 保留（v3.10.0 的逐 crate 目标，**不同的量**） |
| E | `scripts/gate/check_alpha_v380.sh` A5 | `--all-features --tests` → 回退 `--lib` → 再失败记 `TOTAL 0 0 0 0 0%`；抽**第一个**百分比（regions 列） | **本批改为调用共享的 `coverage_measure_set` + `coverage_verdict`**（静默 0% 与 regions 列两个缺陷同时消失）；73 保留（v3.8.0 自己的判据） |
| F | `scripts/gate/check_v312_coverage_baseline.sh` | `--all-features --tests --ignore-run-fail --json --summary-only` | **保留自己的命令形态**，并在脚本头部**显式声明「生成器，不是判定」** |
| G | `scripts/gate/legacy/check_10_principles.sh`、`scripts/gate/legacy/check_r1_r10_content.sh` | 同 E 的形状 | 脚本头部**显式标注 legacy**（它们本来就有 `STATUS: DEPRECATED`），并登记进允许清单 |
| **H** | **`.github/workflows/coverage-parallel.yml`** | **自带测量 + 自抽 `line_ratio` + 自带阈值 70% + `exit 1` 判定** | **本批新发现**（见 §2.4）；登记为 `historical`，**不改它的行为**，另立 #5583 待属主裁决 |

> D 与 E 的**阈值**（47/42、73）**不读 SSOT**，这是有意的：它们是**过去某个发布版给自己定的
> 逐 crate / 逐版本目标**，与「某阶段的平均行覆盖率」是**不同的量**。
> 把它们接到 `COVERAGE_MIN_*` 上会是**口径造假**——那个键描述的不是它们。
> 本批给它们的处理是：测量统一、阈值标注清楚、角色登记在册。

### 2.2 新增的元门禁（issue 期望 #5）

`scripts/gate/check_coverage_single_implementation.sh` +
`scripts/gate/coverage_measurement_allowlist.txt`

**规则**：任何**直接调用**覆盖率测量的文件，要么是共享实现本身，
要么在允许清单里**带角色和理由**登记。二者都不是 ⇒ **FAIL**（点名文件与行号）。

角色（让清单可以被评审，而不是一串路径）：

| 角色 | 含义 | 清单里的文件 |
|---|---|---|
| `the-implementation` | 那一份共享实现（**有且仅有一个**） | `scripts/gate/common.sh` |
| `reporting` | 测量但**不判定**（只 WARN） | `gate/gate.sh` |
| `report-generator` | 产出**给人评审的表格**，不是判据 | `scripts/gate/check_v312_coverage_baseline.sh` |
| `legacy` | 已被取代的历史实现 | `scripts/gate/legacy/*`（2 个） |

清单的**数据与规则分开**：规则在门禁脚本里，数据在 `.txt` 里，两者可分别评审。

> **为什么清单叫 `.txt` 而不是 `.tsv`**（实现过程中发现的一个真问题）：
> `docs/governance/FILE_GOVERNANCE.md` §1 把 `*.tsv` 列为禁止入库的扩展名，
> 且「Allowed exception」一栏写的是 **none**；§6 规定要豁免必须走
> 「提 issue + release owner 批准」，**本批没有这个批准，不能自批**。
> 同时 `.gitignore` 里有 broad `*.tsv`，即使 `git add -f` 强推，
> CI checkout 也不会有该文件 ⇒ 元门禁只会拿到 **ENV_ERROR（exit 2）**。
> 因此清单改用**非禁止扩展名 `.txt`**（内容仍是 TAB 分隔）。
> 这一点写进了清单文件的头部注释，避免后人"改回 .tsv"。

**扫描口径**（刻意保守，宁可多报不可漏报）：
非注释行、提到 `cargo llvm-cov` / `cargo-llvm-cov`、且不是
`command -v` 探测、不是 `--version` 探测、不是 `echo`/`printf` 里的散文提及、
**不是"安装工具"**（`cargo install cargo-llvm-cov` 或 `- name: Install cargo-llvm-cov`
这类步骤标题）⇒ 算一次直接调用。**按文件粒度**判定：文件只要有一次直接调用，就必须登记。

**扫描范围**：`scripts/gate` + `gate` + `.gitea/workflows` + **`.github/workflows`**。
最后一项是本批补上的——见 §2.4。

**自测（`--selftest`）**：四个夹具 ——
直接调用的脚本**必须**被扫出；已登记的脚本**必须**被扫出；
只**调用**共享 helper 的门禁（`coverage_average 80`）**不得**被误报；
**只安装工具**的文件与步骤**不得**被误报。

```
$ bash scripts/gate/check_coverage_single_implementation.sh --selftest
PASS: the scan finds a direct invocation, finds a listed one, and does not flag a caller of the shared helper
EXIT=0

$ bash scripts/gate/check_coverage_single_implementation.sh
PASS: scripts/gate/check_v312_coverage_baseline.sh (listed; lines 114,319,395)
PASS: scripts/gate/legacy/check_10_principles.sh (listed; lines 150,152)
PASS: scripts/gate/legacy/check_r1_r10_content.sh (listed; lines 142,144)
PASS: gate/gate.sh (listed; lines 98)
PASS: every direct coverage measurement is the shared implementation or a listed exception
EXIT=0
```

**变异（判别力）**：塞一个未登记的直接调用 ⇒ 变红。

```
$ cat scripts/gate/check_zz_temp_probe.sh   # cargo llvm-cov test -p sqlrustgo ...
$ bash scripts/gate/check_coverage_single_implementation.sh
FAIL: scripts/gate/check_zz_temp_probe.sh invokes a coverage measurement directly (lines 2) but is not listed
EXIT=1
$ rm scripts/gate/check_zz_temp_probe.sh
$ bash scripts/gate/check_coverage_single_implementation.sh   # EXIT=0
```

### 2.3 元门禁接进了 CI 链路

* `scripts/gate/check_alpha_quality_v4.10.sh` 新增 **Q4b** ⇒ 该门禁由
  `check_alpha_v410.sh` 调用 ⇒ **CI 的 `legacy-gate` job 每个 PR 都会跑它**（无 cargo 依赖，很快）；
* `docs/governance/STAGE_CONFIG.yaml` 的 **GA `required_gates`** 增加
  `scripts/gate/check_coverage_single_implementation.sh` ⇒ 它同时是 GA 判据之一。

### 2.4 本批**新发现**：扫描范围里的一个真实盲区，和它带出来的第三份判据

把元门禁写出来之后，本批顺手问了一个问题：**这条门禁自己覆盖了哪些地方？**
答案是 `scripts/gate` + `gate` + `.gitea/workflows` —— **`.github/workflows` 不在范围内**。
把范围扩上去之后，它当场抓到了 #5581 那张盘点表**没盘到**的东西：

`.github/workflows/coverage-parallel.yml` 是一份**完整的独立判据实现**：

| 环节 | 它的实现 |
|---|---|
| 测量 | `cargo llvm-cov --all-features --html/--json`（逐 crate，6 个 job） |
| 抽取 | `grep -oP '"line_ratio":\K[0-9.]+'`（**JSON 的 `line_ratio` 字段**） |
| 阈值 | **70**（不在 SSOT，也不在 G17 原表里 ⇒ **第四套取值**） |
| 判定 | `exit 1` —— **会真的判红** |

`coverage.yml`（`develop/v2.6.0` 触发）也直接测量，但只上传 artifact，**不判定**。

⇒ **处置**（本批做的）：
1. 元门禁扫描范围**扩到 `.github/workflows`**（盲区关闭）；
2. 两个文件登记进允许清单，角色 `historical`，**理由里写明它自带 70% 判定**
   ⇒ 它此后每次门禁运行都会被**点名**，而不是被静默忽略；
3. 登记为 **#5583**（含 A/B/C 三个处置方向），**等属主裁决**。

⇒ **本批没做的**：**没有改这两个工作流的行为**。改它 = 改**公开镜像 `main`
分支上的构建行为**；而且把它接到共享实现 + SSOT 阈值，等于把判据从
「全工作区 70%」换成「L1_8 8 crate 平均 85%」——**判据被换掉了**，
这必须显式决定，不能顺带做。

**顺带修掉的扫描器两类误报**（扩范围之后才暴露的）：

| 误报 | 判据 | 处置 |
|---|---|---|
| `run: cargo install cargo-llvm-cov --locked` | **安装工具不是测量** | 加 `INSTALL` 过滤 |
| `- name: Install cargo-llvm-cov`（步骤标题） | 同上 | 同一条 `INSTALL` 过滤覆盖 |

⇒ 两条都补进了 `--selftest` 的夹具：**"只安装工具的文件/步骤不得被标记"**。

**自测与实跑（扩展范围后）**：

```
$ bash scripts/gate/check_coverage_single_implementation.sh --selftest
PASS: the scan finds a direct invocation, finds a listed one, does not flag a caller of the shared helper, and does not flag a tool installation
EXIT=0

$ bash scripts/gate/check_coverage_single_implementation.sh
PASS: scripts/gate/check_v312_coverage_baseline.sh (listed; lines 114,319,395)
PASS: scripts/gate/legacy/check_10_principles.sh (listed; lines 150,152)
PASS: scripts/gate/legacy/check_r1_r10_content.sh (listed; lines 142,144)
PASS: gate/gate.sh (listed; lines 98)
PASS: .github/workflows/coverage-parallel.yml (listed; lines 34,69,100,131,163,195,232,233)
PASS: .github/workflows/coverage.yml (listed; lines 34)
PASS: every direct coverage measurement is the shared implementation or a listed exception
EXIT=0
```

**变异（判别力，扩展范围后）**：塞一个未登记的直接调用 ⇒ 变红。

```
FAIL: scripts/gate/check_zz_temp_probe.sh invokes a coverage measurement directly (lines 2) but is not listed
EXIT=1
（移除后 EXIT=0）
```

**自测里不再有第二份扫描实现**：规则（正则与过滤条件）现在只存在于
`SCAN_PY` 一处，自测与实跑**跑的是同一段代码**，只是指向不同的目录树
（自测指向临时树）。此前自测里另抄了一份正则 —— 那等于用一份规则的副本
去证明规则本身，证不了任何事。

---

## 3. 回归矩阵

| 命令 | 结果 |
|---|---|
| `bash scripts/gate/check_coverage_single_implementation.sh --selftest` | ✅ PASS |
| `bash scripts/gate/check_coverage_single_implementation.sh` | ✅ PASS（0 unlisted） |
| 同上 + 未登记调用（变异） | ✅ **如期变红**（EXIT=1，点名文件与行号） |
| `bash scripts/gate/check_stage_config_consistency.sh` | ✅ **RESULT: PASS**（FAIL findings 0 / DISCLOSURE 0） |
| `bash scripts/gate/check_gate_script_refs.sh` | ✅ PASS（63 处引用；55 存在且可执行；8 条已知债 warning，**0 新增**） |
| `bash -n` 全部改动的脚本（12 个） | ✅ 全部 OK |
| `stage_coverage_min` × {ALPHA,BETA,RC,GA,DRAFT,NOPE,alpha} | ✅ 50/75/80/85；DRAFT 与 NOPE **rc=1 且无输出**（fail-closed） |
| `coverage_min_for_mode` × {alpha,beta,rc,ga,all,nope} | ✅ ALPHA 50 / BETA 75 / RC 80 / GA 85 / all→GA 85 / 未知 mode→GA 85 |
| `SQLRUSTGO_COVERAGE_STAGE=nope bash scripts/gate/check_coverage_v312.sh` | ✅ **fail-closed**，EXIT=1，且在测量**之前**就退出（无浪费的 13 分钟） |
| `bash scripts/gate/check_coverage_v312.sh`（未声明阶段） | ✅ 取 GA=85（最严），不会悄悄放宽 |
| `SQLRUSTGO_COVERAGE_STAGE=ga bash scripts/gate/check_coverage_v312.sh` | 见 §3.1 |
| `bash scripts/gate/check_test_count_monotonic.sh` | ✅ PASS（基线随新增用例单调更新） |

### 3.1 真实覆盖率测量（GA 门槛 85）

```
$ SQLRUSTGO_COVERAGE_STAGE=ga CARGO_TARGET_DIR=target/llvm-cov-target \
    bash scripts/gate/check_coverage_v312.sh
Coverage criterion: stage=ga threshold=85%

  sqlrustgo-parser: 86.46%
  sqlrustgo-planner: 88.91%
  sqlrustgo-executor: 82.74%
  sqlrustgo-transaction: 93.29%
  sqlrustgo-storage: 80.73%
  sqlrustgo-catalog: 87.33%
  sqlrustgo-optimizer: 87.85%
  sqlrustgo-types: 95.04%
average coverage: 87.79% (threshold: 85%) — measured 8/8 crates
EXIT=0
```

⇒ **87.79% ≥ 85% ⇒ PASS**，样本 **8/8**（不是缩减样本）。
与批次 39 实测的 87.78% 相差 0.01（同一批 crate 的测量抖动），
说明**测量口径没有被本批改动**——本批改的是**阈值来源**，不是测量。

余量：87.79 − 85 = **2.79 点**（旧门槛 80 的余量是 7.79 点）。见 §1.5 的代价说明。

---

## 4. 改动清单

| 文件 | 改动 |
|---|---|
| `scripts/gate/stage_config_common.sh` | **新增** `stage_coverage_min()`（SSOT 读取，fail-closed） |
| `scripts/gate/check_coverage_v312.sh` | 阈值由 SSOT 按阶段给出；`THRESHOLD=80.0` 常量消失；阶段由 `SQLRUSTGO_COVERAGE_STAGE` 声明，**未声明时取 GA（最严）** |
| `scripts/gate/check_rc_ga_gate.sh` | `COVERAGE_MIN=75` 常量消失；**新增** `coverage_min_for_mode()`（mode→stage 映射的唯一一份）；A5 在阈值缺失时 fail-closed |
| `scripts/gate/check_alpha_quality_v4.10.sh` | 注释里不再复述 50；覆盖命令显式声明 `SQLRUSTGO_COVERAGE_STAGE=alpha`；**新增 Q4b** 跑元门禁 |
| `gate/gate.sh` L2 | 第 6 处硬编码阈值 50 → SSOT 的 ALPHA；并显式标注 L2 是 reporting、不是判定 |
| `scripts/gate/check_10_principles_v310.sh` R5 | 测量改为共享的 `coverage_measure_crate`；47/42 标注为「v3.10.0 的逐 crate 目标，不同的量」 |
| `scripts/gate/check_alpha_v380.sh` A5 | 测量改为共享的 `coverage_measure_set` + `coverage_verdict`（**静默 0%** 与 **regions 列**两个缺陷同时消失）；73 标注为本版本判据 |
| `scripts/gate/check_v312_coverage_baseline.sh` | 头部显式声明「生成器，不是判定」 |
| `scripts/gate/legacy/check_10_principles.sh`、`check_r1_r10_content.sh` | 头部显式标注历史实现 + 角色 |
| `scripts/gate/check_coverage_single_implementation.sh` | **新增** —— 「一份覆盖率测量」元门禁（含 `--selftest`）；扫描范围扩到 `.github/workflows`；规则只存在一份（自测与实跑同一段代码） |
| `scripts/gate/coverage_measurement_allowlist.txt` | **新增** —— 允许清单（角色 + 理由）；**用 `.txt` 而非 `.tsv`，理由见 §2.2**；含新登记的 2 个 `.github` 历史工作流 |
| `scripts/gate/check_v312_gate_thresholds.sh` | 覆盖率子项显式声明 `SQLRUSTGO_COVERAGE_STAGE=rc`（v3.12.0 声明的是 80，判据不变） |
| `.gitea/workflows/ci.yml` | L1 覆盖率步骤显式声明 `SQLRUSTGO_COVERAGE_STAGE=ga` |
| `.github/workflows/ci-pr.yml` | 两处 Gate G17 显式声明 `SQLRUSTGO_COVERAGE_STAGE=ga`（与 Gitea CI 对齐） |
| `docs/governance/GATE_CONDITIONS.md` | **G17 表不再持有第二份数字**，改为指向 SSOT，并写清各阶段归属与临时裁决（§1.6） |
| `docs/governance/STAGE_CONFIG.yaml` | GA `required_gates` 增加元门禁 |
| `docs/releases/v4.1.0/GA_GATE_REPORT.md` | 门槛来源指针 `constants` → **`thresholds`**（键名笔误） |
| `docs/releases/v4.1.0/POST_GA_PLAN.md` | 每周三连跑补上显式阶段 `SQLRUSTGO_COVERAGE_STAGE=rc` |

---

## 5. 没做的事 / 需要属主裁决

| 事项 | 处置 | 理由 |
|---|---|---|
| D / E 的**阈值**（47/42、73）接入 SSOT | **未做** | 它们不是阶段阈值，是**过去某个发布版给自己定的逐 crate / 逐版本目标**；接到 `COVERAGE_MIN_*` 上是口径造假 |
| `gate/gate.sh` L2 的**测量**改用共享实现 | **未做** | L2 测的是**全工作区**行覆盖率，与判据用的 L1_8 8-crate 平均**是不同的量**；且它只 WARN、从不判定。已改的是它的**阈值**（SSOT）与它的**角色标注** |
| `check_v312_coverage_baseline.sh`（F）的**测量**改用共享实现 | **未做** | 它是**报告生成器**（`--ignore-run-fail` 是为了让失败的 crate 也产出一行），与判据口径不同；已显式标注角色 |
| CI L1 覆盖率门槛 80 → 85 | **已做** | 该步骤自称是 GA 的判据；代价（余量 ~2.8 点）已如实记录在 §1.5 |
| 默认阈值是否还要重新论证 | 未动 | 阈值本身没变（SSOT 仍是 50/75/80/85）；本批只统一**来源**。是否调整取值属主裁决 |
| `GATE_CONDITIONS.md` G17 的 ALPHA 75 / GA 80 | **已收口为指针** | 这份数字**从未被任何脚本按 ALPHA=75 执行过**，GA=80 则是被执行过的；SSOT 与属主裁决（GA=85）都指向同一处。数字本身未重新论证，只是**不再有第二份**（§1.6） |
| ALPHA→BETA 晋级审核的 Beta 80% | **不动** | `ALPHA_TO_BETA_GATE_PLAN.md` §2 AB-06 的临时裁决（「在治理文件统一前，执行较严格值」）继续有效；本批没有收走它的权限 |
| 仓库里第三个数 **60.58%**（`DEFECTS_AND_ISSUES.md` P1-01） | **未处理** | `GA_GATE_REPORT.md` §G3 已把它记为「一条待裁决的口径矛盾」；属主裁决前不擅自改文档数字 |
| `check_v312_gate_thresholds.sh`（v3.12.0） | 判据不变 | v3.12.0 的 `COVERAGE_MIN_PER_CRATE: 80` ⇒ 显式用 `rc` 档（SSOT 里等于 80 的那一档），门槛数值不变 |
| `.github/workflows/coverage-parallel.yml`（自带 70% 判定） | **只登记，未改行为** | 改它 = 改公开镜像 `main` 上的构建行为；且接共享实现会把判据从「全工作区 70%」换成「L1_8 平均 85%」⇒ 判据被换掉。已立 **#5583** 待属主裁决（§2.4） |

---

## 6. 一次必须记录的测量事故（诚实边界）

第一次跑真实测量时，门禁报的是 **29.74%**（判 FAIL），而批次 39 实测是 **87.78%**。

**排查过程**（不是猜，是量）：

1. 同一 crate 用**同一条 cargo 命令**手跑 ⇒ **86.46%**（与批次 39 一致）；
2. 用共享实现再跑一次 ⇒ **86.46%**；
3. ⇒ 差异不在测量逻辑，在**环境**；
4. 查 `target/llvm-cov-target` 里的 `.profraw` ⇒ **3 个来自 `wt-v410-b36`** 的陈旧 profile
   （本机为了省时间把 target 目录**跨 worktree 克隆**过），26 个来自当前 worktree；
5. 移走那 3 个陈旧 profile 后重跑 ⇒ 见 §3.1。

**根因**：`llvm-cov report` 会把目标目录里的**所有** `.profraw` 合并；
来自另一个 worktree（源码路径与行号都不同）的陈旧 profile 被一起合并 ⇒ 数字被污染成 29.74%。

**这不是门禁的缺陷**，而是「跨 worktree 克隆 target 目录」这一本地优化带来的副作用：
CI 的 runner 用的是**冷 target 目录**（CI 只恢复 cargo registry 缓存，不恢复 target），
⇒ 不存在陈旧 profile。

**但它值得被记录**：一个**陈旧 profile** 会让门禁报出一个**看起来正常的数字**并判 FAIL，
而排查它要花掉一次全量测量（~13 分钟）。本批因此把它写进证据，
而不是让它变成「某人本地的一次怪事」。

---

## 7. 可复现命令

```bash
cd /Volumes/workspace/dev/sqlrustgo-worktrees/wt-v410-b40

# 元门禁
bash scripts/gate/check_coverage_single_implementation.sh --selftest
bash scripts/gate/check_coverage_single_implementation.sh

# SSOT 读取器
bash -c 'source scripts/gate/stage_config_common.sh;
  for s in ALPHA BETA RC GA DRAFT NOPE; do
    printf "%-6s -> %s (rc=%s)\n" "$s" "$(stage_coverage_min $s)" "$?"; done'

# 阈值按阶段/模式解析
# 注意：**不要** `source scripts/gate/check_rc_ga_gate.sh` —— 它是脚本不是库，
# source 它会把整个门禁跑一遍。只取那个函数：
awk '/^coverage_min_for_mode\(\) \{/,/^\}/' scripts/gate/check_rc_ga_gate.sh > /tmp/cmfm.sh
bash -c 'source scripts/gate/stage_config_common.sh; . /tmp/cmfm.sh;
  for m in alpha beta rc ga all nope; do printf "%-6s -> %s\n" "$m" "$(coverage_min_for_mode $m)"; done'

# 真实覆盖率测量（GA 门槛）
SQLRUSTGO_COVERAGE_STAGE=ga CARGO_TARGET_DIR=target/llvm-cov-target \
  bash scripts/gate/check_coverage_v312.sh
```

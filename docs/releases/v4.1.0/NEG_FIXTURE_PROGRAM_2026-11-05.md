# 反向变异夹具计划（看板 P1-3）

- 日期：2026-11-05　执行身份：mcode-macmini
- 台账：`docs/releases/v4.1.0/NEG_FIXTURES_v4.1.0.jsonl`（56 行）
- 探针：`audit/v4.1.0/probe_neg_fixtures.py`（不是门禁，见 §8）
- 夹具：`audit/v4.1.0/neg/fx01..fx13*.sh`（fx01–fx10 打门禁判定，fx11–fx13 打自带 selftest）
- 证据 run：`audit/v4.1.0/runs/20261105-negfix-002/`（`.../20261105-negfix-001/` 是判据缺陷的取证记录，保留）
- 关联看板项：P1-3（待复核）、NF-1（阻塞）；关联 ISSUE：#5598、#5595

## 1 为什么要这份东西

AGENTS 2.5 要求「每条红灯必须做反向变异」，判据是：把问题条件去掉，门禁必须变红。
做不到，这道门禁可能永远绿着。

4.1.0 此前的状态是：`GATE_CATALOG.jsonl` 里 55 道阶段门禁中有 23 道被标了
`neg_fixture=true`，但那个字段是对门禁脚本源码做关键字匹配算出来的，不是「有夹具」。
也就是说，「做过反向变异」这句话在这 55 道门禁上是**不可对账的**。

本计划把它变成可对账的三件事：一份逐门禁台账、一批可重跑的夹具脚本、
一个会在证据缺失时直接返回非零的探针。

## 2 台账口径

scope（需要夹具的门禁集合）= `GATE_CATALOG.jsonl` 中 `class=stage_required`
∪ 名字匹配 `SOAK|PERF|COMPAT|TPCH|STABILITY|CRASH|UPGRADE|CHAOS|QPS|SYSBENCH` 的
＝ **55 道**。台账 **56 行 = 55 道 scope + 1 行 scope 外加测**
（`check_alpha_entry_v4.10.sh`，`class=ci_reachable`，见 §3 的 fx08）。

kind 四态（语义互斥，不可含糊）：

| kind | 含义 | 计数 |
|---|---|---|
| `mutation-verified` | 本轮在 scratch 工作树里注入真实缺陷、跑真实门禁，判定确实翻红 | 8 |
| `selftest-verified` | 门禁自带 `--selftest`（走线上同一份判定代码），**并且**有一个能把它的检测器致残、让这份 selftest 翻红的夹具 | 3 |
| `unfalsifiable` | 做过证伪尝试，证明这道门禁**结构上不可能**翻红（不是「没测」，是「测了，它有洞」） | 1 |
| `pending-fixture` | 已知该注入什么缺陷（`target` 字段写死），缺的是运行条件 | 44 |

覆盖结论要按这个口径说：**当下能当场证伪的是 11/55**（8 夹具 + 3 selftest），
另外 1 道被证明不可证伪，44 道待夹具。`check` 输出的 `falsifiable_now=11/55` 就是这个数。

每行必须带 `evidence_run`，探针校验它是否真实存在于 `audit/v4.1.0/runs/`。
run 目录不存在时 `check` 直接返回非零 —— 这是 fail-closed 设计，不是探针故障。

## 3 本轮实测：10 个夹具脚本

夹具只在 detached 的 scratch 工作树里注入缺陷，脚本头部写期望，
末尾自带 restore（`git checkout -- .` + `git clean -qfd`），不碰主工作树。

| 夹具 | 门禁 | 注入的缺陷 | 期望 | 实测门禁退出码 |
|---|---|---|---|---|
| fx01 | `check_gate_executability.sh` | 删掉 STAGE_CONFIG required gate 的脚本文件 | red | 1（C1 存在性判据翻红） |
| fx02 | `check_anti_ignore_gate.sh` | 往 `tests/*.rs` 加一个真实 `#[ignore]` 且不登记进 `ignore_registry.json` | red | 1（registry drift 翻红） |
| fx03 | `check_drift_not_pass.sh` | 新增一个用 `cargo test ... > log 2>&1 || true` 吞退出码的门禁脚本 | red | 1（相对 `drift_baseline.json` 出现新反例） |
| fx04 | `check_tests_dont_write_archived_evidence.sh` | 在 `tests/` 放一个直接 `fs::write` 到 `docs/releases/v4.1.0/evidence/` 的测试 | red | 1（L1 静态判据翻红） |
| fx05 | `check_cross_version_debt.sh` | 删掉 `CROSS-VERSION-DEBT.md` / INT5 清单的全部候选路径 | red | 1（按缺 SSOT 翻红） |
| fx06 | `check_int_debt.sh` | 删掉 `CROSS-VERSION-DEBT.md` 全部候选路径 | red | 1（按缺文档翻红） |
| fx07 | `check_docs_links.sh` | 在入口文档 README.md 加一条指向不存在路径的链接 | red | 1 |
| fx08 | `check_alpha_entry_v4.10.sh` | 删 `docs/releases/v4.1.0/STAGE.yaml` | red | 1（E1 判红） |
| fx09 | `check_architecture_freeze.sh` | 把 `crates/` 里的 DriftGate 引用全部改名 | red | **0（未翻红，见 §4a）** |
| fx10 | `check_int_debt.sh` | 把债务行的状态改成不可识别 / 删除该行 | red | **0（未翻红，见 §4b）** |
| fx11 | `check_commit_message.sh` | 把它的 shape 判定改成永远通过（`if False:`），致残的正是 `--selftest` 用的那份代码 | selftest-red | selftest rc=1（`SHAPE_BAD=0` 被它自己抓住） |
| fx12 | `check_coverage_single_implementation.sh` | 把共享检测函数 `scan_hits` 换成空实现 | selftest-red | selftest rc=1（两个必须检出的夹具报 not detected） |
| fx13 | `check_gate_test_integrity.sh` | 让 `detect_cargo_test_or_true_masks` 直接 `return 0` | selftest-red | selftest rc=1（2/4 fixtures，正例全丢） |

fx09 / fx10 的头部标注是 `expect: red`，实测退出码 0，因此它们在 runner 里
按「表征夹具」（characterization）记账：它们证明的是**门禁当前不翻红**这个事实，
而不是门禁有判定能力。runner 汇总 `neg-fixtures: pass=10 fail=0 error=0 total=10`
指的是「10 个夹具脚本按各自声明的期望跑对了」，不能读成「10 道门禁都有效」。

### 3.1 `selftest-verified` 的判据，以及第一版判据是怎么错的

`selftest-verified` 不等于「这个门禁自带 --selftest」。它现在要求两件事同时成立：

1. 干净树里 `bash <gate> --selftest` 退出码 0；
2. 该行 `selftest_neg_fixture` 把这份 selftest 所用的**检测器本身**致残后，
   同一份 `--selftest` 必须翻红。探针自己解析夹具打印的 `NEUTERED_SELFCHECK_RC=`
   取值，**不信夹具的退出码**——一个什么都不破坏的夹具也会退出 0。

第一版不是这么判的：它数门禁输出里含 `selftest|fixture` 的行数（`arms >= 3`）。
run `20261105-negfix-001` 的第 4 号命令据此把 `check_commit_message.sh`
（3 个负例臂，实际只有 1 行提到 selftest）和
`check_coverage_single_implementation.sh`（臂名里不含关键词）判成 NOT-OK。
那次失败是**判据本身**的缺陷，不是这两道门禁的缺陷——按输出形状猜「有没有负向断言」
正是一种会永远绿的检查。fx11–fx13 换成行为判据后三道门禁全部通过第 2 条。

这个判据自己的反向变异是 `mutation-selftest` 的 BA1 臂：造一个「只打印
`NEUTERED_SELFCHECK_RC=0`、什么都不破坏」的假夹具，`selftests` 必须判 NOT-OK。

## 4 两条真发现（NF-1，#5598）

**a. `check_architecture_freeze.sh` 是 GA required gate，但它结构上不可能变红。**
脚本末尾是 `exit 0  # 不阻塞，因为是改进项`。fx09 把 `crates/` 里 DriftGate 的引用
全部改名后，门禁自己打印 `FAIL (0 references - path not established)`，
退出码仍是 0。即：无论架构冻结被破坏到什么程度，这道门禁对 CI 和阶段判定都不产生信号。

**b. `check_int_debt.sh` 的债务行可以靠改名或删除绕过。**
`unknown status` 与 `not in CV debt doc` 两类问题只进 `FAILED_ITEMS` 打印，
真正的退出码判定只看 `ACTIVE_COUNT` / `DEFERRED_WITHOUT_PLAN`。
fx10 把债务行改名或删除后，计数为 0 ⇒ 退出码 0。即：债务台账「消失」比「债务清偿」
更容易让门禁闭嘴。

**为什么 Agent 不直接修**：两处都在既有门禁脚本的判定语义上，属 AGENTS 3.2
（改门禁脚本需 PR 说明 + 反向变异证据），且修复后 BETA/RC/GA 清单会立刻变红
（架构冻结当前确实未建立、债务台账当前确实有不可分类行），是否放行属阶段跃迁判断，
按 AGENTS 2.1 / 2.7 是 owner 的决定，已登记 #5595 与看板 NF-1。

**修复的验收判据（本身即反向变异）**：修复前 fx09/fx10 必须返回 0；
修复后必须返回 1。两个夹具已在仓里，owner 合入修复后直接重跑 §5 的 `fixtures` 即可复核。

## 5 复现

```bash
cd <repo>
python3 audit/v4.1.0/probe_neg_fixtures.py check

git worktree add --detach /tmp/neg-scratch "$(git rev-parse HEAD)"
python3 audit/v4.1.0/probe_neg_fixtures.py fixtures  --tree /tmp/neg-scratch
python3 audit/v4.1.0/probe_neg_fixtures.py selftests --tree /tmp/neg-scratch
git worktree remove --force /tmp/neg-scratch

python3 audit/v4.1.0/probe_neg_fixtures.py mutation-selftest
```

`mutation-selftest` 是「探针自己的反向变异」：在台账副本上注入 8 类缺陷
（删行 / 重复行 / 空 reason / 夹具不存在 / 无 selftest 却声明 selftest /
伪造 evidence run / pending 缺 close_run / unfalsifiable 改成 mutation），
断言判定必须变红。臂现在是 11 条：M1–M10 打在台账语义上，
BA1 打在 `selftests` 的行为判据上（见 §3.1）。
取证 run `20261105-negfix-001`（命令 004 失败）保留不删，当期取值以
`20261105-negfix-002` 的日志为准。

`mutation-selftest` 先要求**台账基线零发现**再跑臂：基线不干净时「臂发现了什么」
不可归因，探针直接失败退出，而不是把无关发现算作臂的战果。

## 6 度量口径里两个坑（写任何报告前先看）

- **`neg_fixture_n` 不是夹具覆盖率。** `scripts/gate/gov_probe.py` 的 `neg_fixture_n`
  是对门禁脚本源码做关键字匹配得到的 23，命中的可能只是脚本里引用了
  `ADR-004-negative-evidence.md` 这类文件名（`GATE_CATALOG` 里 `check_rc_ga_gate.sh`
  的 `neg_fixture=true` 就是这么来的）。真正能对账的分子来自本台账：11/55。
- **`docs410` / `claims_hits` 自指。** 这两个度量扫的就是 `docs/releases/v4.1.0/*.md`
  里的自我宣称标记。往这个目录加文档会直接改变计数，加的内容里若含宣称词还会二次放大。
  本文件因此不写那几个词；任何「跑过 / 达标」的表述都必须回到 run 日志行号。

## 7 44 条 pending 怎么收

每条 pending 的 `target` 字段已经写清「注入什么缺陷 ⇒ 必须变红」，
缺的是运行条件，不是想法。按台账 `cost` 字段的泳道分布：

| 泳道 | 条数 | 阻塞条件 |
|---|---|---|
| `runtime` | 21 | 需要构建产物 / 数据集（SF1、SF10、语料库、崩溃注入），依赖 #5597 的 CI 执行面 |
| `cargo` | 12 | 需要热 `target` 与 llvm-cov（`check_coverage_v312.sh` 单条实测 >720s 未跑完） |
| `external` | 7 | 需要外部 MySQL/Oracle、sysbench、soak 窗口，与 ING-1 的 168h 泳道重叠 |
| `composite` | 3 | 内部子门禁必须先有可翻红的分量（fx08 已给 entry 分量，`check_alpha_v410.sh` 本体仍待跑） |
| `static` | 1 | 无外部依赖，可先做（`check_upgrade_guide.sh`） |

按 `class` 看还有 7 条 `dead` + 7 条 `orphan_referenced`：这 14 条在清单里没有执行点，
夹具做出来也没人跑。它们的 `note` 已写明先决条件是 #5597（CI 执行面 / runner 镜像），
不是夹具工作量。

收口顺序建议：`static` 1 条 → `composite` 3 条 → `cargo` 热缓存批量泳道 12 条 →
`runtime` 21 条（与 4.1.0 的功能验收同批跑）→ `external` 7 条与 soak/QPS 类一起排期。

## 8 红线自查

- 未新增门禁、未改 `GATES` 清单 / 阈值 / 跳过开关，**分母仍 168**；
  新增判据挂既有门禁 NF-1，由 owner 走 #5595 的独立 PR。
- 新工具命名 `probe_neg_fixtures.py`，夹具在 `audit/v4.1.0/neg/`：
  不叫 `check_*`、不在 `scripts/gate/`、不进 STAGE_CONFIG 阶段清单，因此不进分母。
- 所有数字来自本轮真实运行；run 日志见 `audit/v4.1.0/runs/20261105-negfix-001/logs/`。
- 本 run 不含 `ci_gate.sh`，因此不构成任何「门禁通过」声明（口径见 AGENTS 2.3）。

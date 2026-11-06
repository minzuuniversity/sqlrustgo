# P2-2：目录策略机器化 —— 把散文里的规则变成会拦人的判据（2026-11-05）

## 0. 结论先行

`docs/governance/DIRECTORY_POLICY.md` 早就有规则，但**没有执行点**：
规则只存在于散文里，谁违反谁都没人拦。
本条把其中**可机器判定**的部分变成判据：

`audit/v4.1.0/directory_policy.py`（探针，**不是门禁**，不进 STAGE_CONFIG，
门禁分母 168 不动），6 条规则，实测当前 **149 条存量违规**。

判据口径：**新增违规 ⇒ 红；存量违规记账但不判红。**

| | |
|---|---|
| 规则条数 | 6 |
| 存量违规 | **149** |
| 基线 check | rc=0 |
| 新建一个违规目录后 | rc=1，报 **3 条**新增违规 |

## 1. 为什么只卡增量，不一次清算

仓里存量违规不少（一级目录缺 README 38 条、索引未登记 38 条、
目录名不合规 63 条、vX.Y 目录 9 条、bench 三目录并存 1 条）。
一次性清算要动几十个目录、几十个索引条目，还牵连大量历史 release 目录 ——
那是 Owner 的决定，不是本条能自行做的。

但「没有执行点」的代价是**继续新增**：目录该归位却没人拦，
下一轮又多几个。本工具的职责是止血 —— 从此刻起不再新增。
存量 149 条**逐条记账**（基线文件里一条一条列出来），不是删掉了事。

## 2. 六条规则与它们的出处

每条规则都带出处，不自创：

| 规则 | 判什么 | 依据 | 存量 |
|---|---|---|---|
| `dir-name-charset` | 子目录名必须小写、用连字符、不得含中文 | DIRECTORY_POLICY.md §2.2 | 63 |
| `no-new-vxy-dir` | 不得新建 `vX.Y` 格式目录 | DIRECTORY_POLICY.md §2.1 | 9 |
| `dir-has-readme` | `docs/` 下每个一级目录必须含 `README.md` | DIRECTORY_POLICY.md §3.1 | 38 |
| `index-registers-dir` | `docs/README.md` 必须**链接**到每个一级目录 | DIRECTORY_POLICY.md §3.2 | 38 |
| `top-level-md-cap` | 顶层 `.md` 数量不得超过 45 | 看板 P2-2 | 0 |
| `bench-dir-single` | `bench/` `benchmarks/` `benches/` 至多存在一个 | 看板 P2-2 | 1 |

前四条直接来自策略文件（`docs/governance/DIRECTORY_POLICY.md`）。
后两条来自看板 P2-2 的任务原文（「顶层 45 个 .md、bench/benches/benchmarks 等归位」）——
策略文件本身没有写这两条，所以出处标的是看板，不假装是策略文件。

`index-registers-dir` 要求的是**markdown 链接**（`](<dir>/`），
不是「顺口提一句目录名」—— 否则一个名字出现在正文里就算登记了，判据太松。

## 3. 实测

```
$ python3.14 audit/v4.1.0/directory_policy.py report
{"kind": "directory_policy_report", "rules": 6, "violations": 149,
 "by_rule": {"dir-name-charset": 63, "no-new-vxy-dir": 9,
             "dir-has-readme": 38, "index-registers-dir": 38,
             "top-level-md-cap": 0, "bench-dir-single": 1}}
```

存量违规长什么样（节选）：

```
docs/AI增强软件工程                     ← 中文目录名（§2.2）
docs/releases/v1.9.0/OO                 ← 大写目录名
docs/releases/v3.12.0/evidence/arch_invariants  ← 下划线目录名
docs/v1.0  docs/v2.0  docs/releases/v2.0 …        ← vX.Y 目录（§2.1）
docs/adr/  docs/adr/ …（38 个一级目录没有 README.md）（§3.1）
bench/ + benchmarks/ + benches/ 并存              （看板 P2-2）
```

基线文件 `docs/releases/v4.1.0/DIRECTORY_POLICY_BASELINE.jsonl`
逐条记录每个违规（rule / item / source），一条一行，可直接 diff 出新增。

> 落点说明：仓内禁 `*.json` / `*.log`（交接提示词 §4），故基线用 `.jsonl`。

## 4. 反向变异：证明判据可被证伪

```
$ python3.14 audit/v4.1.0/directory_policy.py emit
{"kind": "directory_policy_emit", ..., "violations": 149, ...}

$ python3.14 audit/v4.1.0/directory_policy.py check
{"kind": "directory_policy_check", "baseline_violations": 149,
 "live_violations": 149, "new_violations": 0}
directory_policy: OK                                    ← rc=0
```

变异：新建一个既含大写又含下划线、且没有 README、也没登记进索引的目录
`docs/New_Bad_Dir/`：

```
$ python3.14 audit/v4.1.0/directory_policy.py check
{"kind": "directory_policy_check", "baseline_violations": 149,
 "live_violations": 152, "new_violations": 3}
FAIL: dir-name-charset: docs/New_Bad_Dir（依据 DIRECTORY_POLICY.md §2.2）
FAIL: dir-has-readme: docs/New_Bad_Dir/（依据 DIRECTORY_POLICY.md §3.1）
FAIL: index-registers-dir: docs/New_Bad_Dir/（依据 DIRECTORY_POLICY.md §3.2）
directory_policy: 3 FAIL                                ← rc=1
```

撤掉该目录后 → `directory_policy: OK`（rc=0）。

⇒ 判据确实会拦人，且**只拦新增**（撤掉即回绿，说明它判的是「相对基线的增量」，
不是「仓库里有没有违规」）。

## 5. 口径变化（如实记录）

| 指标 | 改动前 | 改动后 | 说明 |
|---|---|---|---|
| `check_n` | 168 | 168 | 分母未动（工具不进 STAGE_CONFIG，文件名不叫 `check_*`） |
| `claims_hits` | 1144 | 1144 | 未动 |
| `docs410` | 84 | 85 | 本报告占 1 份（自身声明记号为 0） |

## 6. 残余风险与未闭环部分

- **存量 149 条违规一条都没清**。本条只保证不再新增。
  清理需要 Owner 定范围（哪些是历史冻结目录、可以豁免；哪些是真该归位）。
  特别是 `docs/releases/**` 下的历史目录是**冻结资料**，
  策略 §4.2 本来就说历史文档要留在 `docs/archive/` ——
  所以 R1/R2 对 `docs/releases/**` 是否豁免，是个需要 Owner 裁定的口径问题。
  本工具目前**不豁免**它们（因此 63 + 9 里大部分来自 releases/），
  这是「宁可多报」的方向，豁免要显式做。
- **`index-registers-dir` 只查 `docs/README.md`**，不查子目录自己的 README 里
  是否登记了自己的子目录。策略 §3.1/§3.2 的完整含义比这更宽。
- **本条不构成门禁通过声明**：本仓未跑 `ci_gate.sh`。
  本工具是探针，**没有挂到任何阶段清单上** ——
  「执行点」这件事只做了一半：判据有了，接线（挂到 CI/阶段清单）没做，
  后者受 IL-9（CI 执行面，runner-03 缺镜像）挡住。
- 归档 run 侧的第二血统复核同样被 IL-9 挡住。

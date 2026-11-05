# IL-6：证据门禁自指 —— 门禁不该把「正在执行的 run」算进判定（2026-11-05）

## 1. 这条为什么要做

`scripts/gate/check_audit_manifest.sh` 的契约是「every **committed** audit run
must pass verify」，但它枚举的是**工作区**目录。

归档 run 执行时，流程是：先 commit 代码 → freeze → scaffold（写出 manifest.yaml）
→ run（逐条写日志）→ verify → commit 归档产物。所以在一个 run 正在执行时，
它自己的目录已经存在、manifest.yaml 已经写好，而日志还没写完 ——
`audit_run.py verify` 对它必然判红。门禁于是把「正在执行的 run」算成了一条红。

现场实测（同一台机器、同一份代码，只差「run 是否在执行」）：

| 时刻 | 门禁 FAIL 条数 | 说明 |
|---|---|---|
| run 执行中（该 run 目录已建、日志未写完） | 8 | 多出来的那条正是本 run 自己 |
| run 归档完成后重扫 | 7 | 多出来的那条消失 |

判据自己数自己，红绿随执行时刻漂移：同一份代码、同一个提交，
「跑之前」和「跑之中」给出不同结论。这不是判定质量问题，是判据不可复现。

## 2. 根因

门禁的枚举范围与它的契约不一致：

- 契约说「committed」（已提交的）；
- 实现扫的是 `audit/v4.1.0/runs/*/` 与 `audit/v4.1.0/*/` 的**目录**，
  目录里有没有 manifest.yaml 跟它有没有被提交无关。

于是「已提交」这个前提被丢掉了：未提交的在飞 run 和已提交的历史 run
被同等对待。

## 3. 处置：两个方案都做了

IL-6 原本的验收口径给了两条路：「门禁按 frozen_commit 树扫描」或
「显式排除 `--exclude-run <当前 run id>`」。两条都实现了，因为它们覆盖的场景不同：

### 3.1 主方案：按已提交的树枚举（默认行为）

门禁改用 `git ls-tree -r --name-only HEAD -- audit/v4.1.0` 取文件清单，
再从清单里解析 run 目录。**未提交的在飞 run 不在清单里，因此不进判定范围。**

这与门禁自己的契约一致，也不需要调用方记得传任何参数 ——
不依赖「谁记得排除自己」这种约定，是这道门禁该有的形态。

实现约束：用纯 POSIX 构造，不用 bash 4 的关联数组 ——
macOS 自带 bash 3.2，`declare -A` 在那儿直接报 `invalid option`
（第一版就是这么写崩的，实测 rc=2、红数被误报成 0）。

### 3.2 备选方案：`--exclude-run <id>`（显式排除）

用在「某个 run 确实正在执行、必须临时豁免」的场景。排除是**显式**的、
写在命令行里的，不靠目录枚举的副作用。可重复传多个。

排除项在输出里打 `SKIP:` 行，不静默消失 ——
排除过什么必须看得见。

### 3.3 门禁自带 `--selftest`

自测点：在飞 run 自身 `verify` 必红（说明它确实还没写完），
而门禁枚举不得把它算成红。

## 4. 反向变异：证明判据可被证伪

`audit/v4.1.0/il6_selfref_probe.py`（探针/夹具，不是门禁，不进 STAGE_CONFIG）：

```
$ python3 audit/v4.1.0/il6_selfref_probe.py
OK   inflight-not-red: rc=1 in-flight run red=False (total reds=7)
OK   worktree-scan-mutation: rc=1 in-flight run red=True (total reds=8)
OK   history-not-relaxed: rc=1 历史红 7/7 (total reds=7)
OK   exclude-run-option: victim=iaia410-5112-evidence-20261015 reds 7 -> 6 skipped=True
checks=4 ok=4 not_ok=0 drift=0
```

| 核对 | 期望 | 实测 |
|---|---|---|
| 在飞 run 不被判红 | 不在红集合里 | total reds=7，8→7 的那 1 条不出现 |
| **反向变异**：枚举改回工作区 | 在飞 run 必须重新被判红 | mutated rc=1，total reds=8，变异被抓 |
| 历史判据不被放宽 | 7 条历史不可复核记录仍判红 | 历史红 7/7 |
| `--exclude-run` 生效且不误伤 | 只排除指定那一条 | reds 7→6，victim 转 SKIP，其余 6 条一条不少 |

「反向变异」那一行是关键：把枚举改回工作区文件后，在飞 run 立刻重新变红（8 条），
证明这条判据不是恒绿摆设，而是真的在判。

## 5. 历史判据一条没放宽

改动前后，门禁判红的 run 集合**逐字相同**（7 条）：

```
$ diff <(改动前的 FAIL 行) <(改动后的 FAIL 行) && echo IDENTICAL
IDENTICAL
```

这 7 条是 IL-5 / ISSUE #5596 记的 7 条不可复核历史记录
（5 条 hollow 无命令行、1 条 unsealed、1 条无清单目录），
它们的红是**Owner 专属裁定项**，本条整改既没改它们的判据，也没把它们变绿。

`--exclude-run` 默认不排除任何东西：不用这个参数时，红集合与改动前完全一致。

## 6. 顺带的口径变化（如实记录）

| 指标 | 改动前 | 改动后 | 说明 |
|---|---|---|---|
| `check_n` | 168 | 168 | 分母未动（硬约束） |
| `neg_fixture_n` | 23 | 24 | 门禁自带 `--selftest` 后现场重算为真（**关键字口径**，非覆盖率） |
| `orphan` / `dead` | 83 / 61 | 83 / 61 | 未动 |

`GATE_CATALOG.jsonl` 里这道门禁的 `neg_fixture` 字段随现场重算从 `false` 改成 `true`，
`evidence` 字段同步换成新门禁文件里的逐字引文 ——
该字段按登记规则必须逐字出自门禁文件，改写门禁后必须同步，否则登记漂移会被
`gate_registry.py check` 判红（本次确实先被判红了一次，随后按规则修正）。

## 7. 残余风险

- 主方案依赖 `git ls-tree HEAD`。在**未提交任何东西**的干净检出上，
  HEAD 树就是工作区，两者一致；在有未提交改动的检出上，门禁判的是 HEAD 树，
  未提交的新 run 不进判定 —— 这是本条整改的**意图**，不是副作用。
- 若有人在未提交状态下改了历史 run 的日志，门禁读的是 HEAD 树的清单，
  但 `verify` 读的是工作区文件。这两者的差异会让门禁对「已提交但工作区被改」的
  历史 run 判红 —— 方向是保守的（多红而非漏红），符合 fail-closed 口径。
- 本条**不构成门禁通过声明**：本仓未跑 `ci_gate.sh`。
  `check_audit_manifest.sh` 本身在已提交树上仍是 rc=1（7 条历史红，Owner 裁定项）。
- 归档 run 侧的第二血统复核仍被 IL-9（CI 执行面，runner-03 缺镜像）挡住。

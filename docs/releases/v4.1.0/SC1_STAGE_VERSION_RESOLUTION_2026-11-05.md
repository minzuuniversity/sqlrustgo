# SC-1：阶段驱动不得猜发布线版本（check_stage.sh）

- 日期：2026-11-05　执行：`mcode-macmini`　看板任务：`SC-1`
- 取证 run：`audit/v4.1.0/runs/20261105-negfix-003`（含两条 FAIL，记录回归现场）、
  `audit/v4.1.0/runs/20261105-negfix-004`（13 条命令全部退出码 0，
  evidence_hash `sha256:90bc688fed9a27875f7d37d084bc8e5fa69affc50206af1b2b76bba2a477c549`）
- 本文所有数字都指向日志原文，未跑 `ci_gate.sh`，因此不构成任何门禁通过声明。

## 1 改前的真实行为（实测，不是推断）

在 `develop/v4.1.0` 的整改分支 `mcode-macmini/governance-kanban-expert-ladder` 上，
不带 `--version` 调用阶段驱动：

```
bash scripts/gate/check_stage.sh --stage RC
```

`detect_version()` 的分支名匹配失败后走回退分支：
`find docs/releases -name STAGE.yaml | head -1`。本仓有 44 个发布目录，
`find` 顺序不是版本顺序，实测评的是**已归档的 v3.9.0**：

```
Stage Gate Check: RC
Version: v3.9.0
Branch: detached
Commit: 9f82eeb777
```

更糟的是它按这个版本写决策日志：
`docs/releases/v3.9.0/logs/gate_rc_9f82eeb777_20261105_063357Z.log`
（在 scratch 检出里复现）。该路径被 `.gitignore` 的反向规则
`!docs/releases/*/logs/gate_*.log` 放行，一旦提交就变成 v3.9.0 的一条
「正式门禁决策记录」，而这次运行与 v3.9.0 无关。

判据性质：这是 fail-open（猜一个答案继续跑），不是 fail-closed（说不清就退出）。

## 2 改了什么（三处，都在同一个脚本内）

1. **委托既有单一实现**。仓里早就有 `scripts/gate/resolve_release_version.sh`：
   ref/分支 → 该版本必须真的有 `docs/releases/<ver>` 目录 → 最新含 STAGE.yaml 的
   发布线 → 根 `VERSION` 文件 → 全都推不出就 exit 1。`check_stage.sh` 现在调它，
   而不是维护第二份「猜版本」逻辑。这同时消掉一份重复实现（本仓已有多起
   同一判据多份实现导致的漂移，见 GS-1/#5595）。
2. **推不出就 rc=2**（三态里的 INCOMPLETE），并打印可执行的补救方式
   （传 `--version`，或在 `develop|beta|rc|release|hotfix/v*` 分支上跑）。
3. **日志目标必须绑定**。版本串必须能安全地拼进 `docs/releases/<ver>`，
   且该目录必须存在，否则同样 rc=2。这一条把「`--version` 打错一个数字
   就在仓里新建一个发布目录并写进门禁日志」这条路径也关掉。

### 2.1 对既有门禁清单的影响：实测为 0

改门禁脚本属 🟡 3.2（不是 🔴 2.1：清单、阈值、跳过开关都没动，分母仍是 168）。
必须回答「改后会不会多报红」。实测：`.gitea/workflows` 与 `.github/workflows` 里
`check_stage.sh` 共 5 处调用点，**全部显式带 `--version v4.1.0`**；
不带 `--version` 的调用点数为 0（`rg -n check_stage .gitea/workflows .github/workflows`）。
所以在当前 CI 上没有任何新增红。不带 `--version` 的调用只发生在人手上和文档示例里，
而那里正是这次要拦住的行为。

## 3 反向变异（AGENTS 2.5）：夹具 `fx14`

`audit/v4.1.0/neg/fx14-stage-version-uninferable.sh`，只在 detached scratch 检出里注入，
脚本末尾自带 restore。三臂：

| 臂 | 注入 | 修复后（`7d4cea2681`） | 修复前（`9f82eeb777`，同一份夹具） |
|---|---|---|---|
| A 无可推输入 | 移走全部 `docs/releases` 与根 `VERSION` | rc=2、打印拒绝行、不进入任何阶段评估 → 夹具成立 | 带着 `Version: unknown` 继续跑阶段查表才死（rc=1）→ 夹具判不成立 |
| B 分支不含版本 | 干净树、detached 分支，不带 `--version` | 解析出 v4.1.0，且不在 v4.1.0 之外写任何文件 → 夹具成立 | 解析出 **v3.9.0** → 夹具判不成立 |
| C 日志目标 | `--version v9.9.9`（不存在的发布线），执行模式 | rc=2，且**没有**创建 `docs/releases/v9.9.9` → 夹具成立 | 创建了该目录并开始跑 ALPHA 门禁（被 timeout 截断）→ 夹具判不成立 |

同一份夹具：修复后退出码 0（三臂成立），打修复前提交退出码 3（三臂全部判不成立）。
两条都在归档 run 里：004 的 `neg-fixtures` 日志与 `fx14-reverse-pair` 日志。

### 3.1 判据本身也被反向变异过一次

arm A 的第一版断言是「退出码非零」。修复前的驱动也是非零（rc=1，它在
`Version: unknown` 上继续跑、死在阶段查表），所以「非零」不是判别信号。
收紧后的判据要求合同语义：**不得先打印阶段头**、必须 rc=2、必须有 INCOMPLETE 拒绝行。
这条收紧是在跑过修复前那一次之后才加的，属于「先被现实打脸再改判据」，不是先验设计。

## 4 这次顺带证明：门禁清单核对器抓到了我自己的回归

第一版补丁在 `check_stage.sh` 里新写了两处 `|| true`（resolver 调用、
`detect_version` 调用）。归档 run 003 的 10 号命令 FAIL：

```
FAIL: C2 scripts/gate/check_stage.sh 登记 swallow_hits=1 现场重算 3
```

按 2.5 先问「门禁真发现了问题，还是门禁自己坏了」：重算值与新增写法逐条对得上，
是真发现。处理方式是**消掉被数的写法**（把吞状态改成存状态：
`resolved="$(...)" || rc_resolver=$?`，再看 rc 与空串决定是否采用），
而不是把登记值改成 3 去迁就现实。004 的 10/11 号命令因此转绿。

这一对（写法存在时 C2 红 / 写法去掉后 C2 绿）就是 C2 的天然反向变异证据：
C2 不是永远绿的检查。

## 5 复现

```bash
# 改前行为（在修复前的提交上）
git worktree add --detach /tmp/srg-sc1-pre 9f82eeb777
cd /tmp/srg-sc1-pre && bash scripts/gate/check_stage.sh --stage ALPHA --dry-run | sed -n 1,8p
#   -> Version: v3.9.0（已归档发布线）

# 修复后 + 夹具三臂（正向）
git worktree add --detach /tmp/srg-sc1-fix <当期 HEAD>
bash audit/v4.1.0/neg/fx14-stage-version-uninferable.sh /tmp/srg-sc1-fix; echo rc=$?   # 0

# 反向变异（打修复前提交）
bash audit/v4.1.0/neg/fx14-stage-version-uninferable.sh /tmp/srg-sc1-pre; echo rc=$?   # 3

git worktree remove --force /tmp/srg-sc1-pre
git worktree remove --force /tmp/srg-sc1-fix

# 清单核对器（分母必须仍是 168）
python3 scripts/gate/gate_registry.py check
python3 scripts/gate/test_gate_registry.py
python3 audit/v4.1.0/audit_run.py verify --run-id 20261105-negfix-004
```

## 6 三态与红线自查

| 项 | 状态 |
|---|---|
| `ci_gate.sh` | 未跑，故本文不写「门禁通过」；`gate_registry check` 与夹具的退出码只说明这两项检查 |
| 门禁清单/阈值/跳过开关 | 未动，分母 168 |
| 门禁脚本语义 | 属 3.2，已说明改前漏什么（猜版本 + 污染归档目录）、改后多报什么（推不出版本时 rc=2；`--version` 指向不存在发布线时 rc=2）并附反向变异 |
| 直推 main / force push | 无。分支 `mcode-macmini/governance-kanban-expert-ladder`，只推 252 |
| 凭据 | 未打印、未落盘 |
| 未闭环部分 | 看板仍记「待复核」：只有本机单血统取证，CI 侧复核被 #5597（runner-03 缺镜像）挡住 |

## 现象（实测，不是推测）

`docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl` 是 4.1.0 看板的 SSOT。PR #5600 把两条 agent 线合进
`develop/v4.1.0` 之后，**两侧各自继续改写同一份文件**，本轮（r11）现读：

| 检出 | 行数 | 独有行 |
|---|---|---|
| 本线 `mcode-macmini/governance-kanban-expert-ladder` | 73（含本 ISSUE 对应行 KAN-2） | `DISK-1`、`DRIFT-2`、`LEDGER-2` |
| `gitea252/develop/v4.1.0` | 72 | `FL-1`、`IL-13`、`ENG-13` |

**7 行状态判定不一致**（同一 id，两线给出不同终态）：

| id | 本线 | develop |
|---|---|---|
| IL-2 | 未开始 | 待复核 |
| IL-3 | 未开始 | 待复核 |
| P1-4 | 未开始 | 待复核 |
| P2-1 | 未开始 | 待复核 |
| P2-2 | 未开始 | 待复核 |
| IL-7 | 未开始 | 阻塞 |
| ENG-12 | 阻塞 | 待复核 |

## 为什么会再犯

HO-2（两条线谁落 develop）裁过一次「同名 run 证据冲突」，但**看板文件本身的写入口径没有定**：
两条线都能改 jsonl + 重渲染 md，合并时 git 只能按行合并，语义上必然分叉。
develop 侧最近一次改写来自 `dd63dd53bf`（"按现场证据裁定看板 8 行 + 新增 2 行"）与 `525827cd10`
（归档 run `20261106-board-001`，manifest 自称 23 条命令 20 绿 3 红）——**这 23 条我没复算**，
因此 develop 侧那些新终态的强度只能记成「对方声称」，不能当实测。

## 与既有 ISSUE 的关系

- **HO-2**：分支/线的归属裁决（Owner）。本 ISSUE 是它在「看板文件」这一具体载体上的复发。
- **LEDGER-2（#5603）**：claims 台账三套并存——同一类病（同一事实多份账）的另一面。
- **DRIFT-2（#5604）**：看板行与已落库工具不一致（P1-4/P2-1/P2-2）⇒ develop 已把它们改判「待复核」，
  本线仍写「未开始」⇒ 这正是上表里的三行。**别按任一侧收尾**。

## 需要 Owner 定的（Agent 不得自裁）

1. 看板文件的**唯一写入路径**：只有落在 `develop/v4.1.0` 的那份算看板，还是允许开发线各留草稿板？
2. 是否给 `KANBAN_*.jsonl` 加**写入串行约定**（例如：改看板必须单独提 PR、禁止与功能提交混在一个 PR 里）。
3. 状态冲突的裁定规则是否固定为「取保守值 + 两侧证据都留在行内」（本线 r11 已按此做，见 `WIRE-1` 行）。

## 本线在裁决前遵守的合并规则（可核查）

- 看板文件**以 develop 为基线**：合并时取 develop 的行，本线只 append 自己的新增行；
- 同一 id 两线都写过 ⇒ 状态取**保守值**，两侧证据都保留在行内，不自行裁定谁对；
- 每次改看板后必须跑 `scripts/gate/kanban_render.py render/check/counts` 三连（`check` rc=0 才提交）。

## 复现命令

```bash
git fetch gitea252 develop/v4.1.0
git show gitea252/develop/v4.1.0:docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl | wc -l
wc -l docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl
# 逐行比 id 集合与 status（本会话用 /usr/bin/python3 做的集合 diff）
```

本 ISSUE 不构成任何「门禁通过」声明（本轮未跑 `ci_gate.sh`，分母仍 168）。

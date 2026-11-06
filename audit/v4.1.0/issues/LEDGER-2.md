## 现象（合并 develop 后实测）

同一件事——「哪条文档声称绑到了哪个 run」——现在有**三套台账并存**：

| 台账 | 行数 | 生成器 | 作者/时间现读 |
|---|---|---|---|
| `docs/releases/v4.1.0/CLAIM_BINDINGS_LEDGER.jsonl` | 60 | `audit/v4.1.0/evidence_inventory.py`（默认 `--ledger` 就是它） | mcode（本线） |
| `docs/releases/v4.1.0/CLAIMS_INVENTORY.jsonl` | 59 | `doc_facts.py` / `claims_selftest.py` | `e5dbbbcc04` omp 11-06 02:28 |
| `docs/releases/v4.1.0/CLAIMS_UNBOUND_LEDGER.jsonl` | 59 | `evidence_inventory_ledger_omp.py` | `f9acc2806e` omp 11-06 03:56 |
| `docs/releases/v4.1.0/CLAIMS_BINDINGS.jsonl` | 21 | （另一套绑定口径） | omp |

两套 debt 口径各说各话：本线台账给 `debt=1143 / total=1144 / strong_bound=1 / findings=0`，
另一套的 21 行绑定口径无法与之对账。**任何「debt 降了」的结论都可能只是换了台账。**

## 为什么必须人裁

选 SSOT = 让另一份退役（删文件或标 DEPRECATED），属**口径归并**，
按 AGENTS.md 属 Owner 动作；Agent 不得自删，也不得「两份都留着各看各的」——
那正好留下「想讲哪个数就讲哪个」的口子。

## 收口判据（可证伪）

1. Owner 指定**唯一**台账文件 + **唯一**生成器，其余标 DEPRECATED 或在独立 PR 里退役；
2. 指定后，`evidence_inventory.py verify` 与 `claims_selftest.py` 必须对**同一份 SSOT** 读数一致；
3. 不一致 ⇒ 本 issue 不得关闭。

## 复现

```
wc -l docs/releases/v4.1.0/{CLAIM_BINDINGS_LEDGER,CLAIMS_BINDINGS,CLAIMS_INVENTORY,CLAIMS_UNBOUND_LEDGER}.jsonl
/usr/bin/python3 audit/v4.1.0/evidence_inventory.py verify
git log -1 --format='%h %an %ad' -- docs/releases/v4.1.0/CLAIMS_UNBOUND_LEDGER.jsonl
```

看板行：`KANBAN_v4.1.0_ALPHA.jsonl` 的 **LEDGER-2**（阻塞，blocked_by=Owner）。
合并提交 `47b80b0cd0` 保留了两套原样，本 issue 只做登记，未做选择。

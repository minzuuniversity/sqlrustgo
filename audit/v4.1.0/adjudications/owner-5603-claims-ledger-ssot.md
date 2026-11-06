待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**待决**：#5603（看板 LEDGER-2）—— claims 证据台账**三套并存**，债务口径可以对
任意一份讲故事：
  1. `docs/releases/v4.1.0/CLAIM_BINDINGS_LEDGER.jsonl`（另一条线的
     `evidence_inventory.py` 默认台账，debt=1143）
  2. `docs/releases/v4.1.0/CLAIMS_INVENTORY.jsonl`（本线的
     `audit/v4.1.0/evidence_inventory_ledger_omp.py`，claims 1152 / bound 100）
  3. `docs/releases/v4.1.0/CLAIMS_UNBOUND_LEDGER.jsonl` / `CLAIM_BINDINGS_…`
     的其余变体（见 `evidence_inventory.py report` 的输出）

要裁的是：**以哪一套为 SSOT**、其余两套怎么退役（删除 / 标 deprecated / 只读快照），
以及**「未绑定声明」这件事怎么记账**（现在另一套台账的 `verify` findings 会随看板
文本变化而涨 ⇒ 本批就把 findings 从 25 抬到 34，理由是看板新增的裁定文本带了
PASS 声明而那套台账不认）。

**材料**：
- `audit/v4.1.0/evidence_inventory.py verify`（另一条线那套，现 findings=34）。
- `audit/v4.1.0/evidence_inventory_ledger_omp.py check|report`（本线那套：
  fails=0、unbound 1052 / bound 100）。
- `audit/v4.1.0/issues/LEDGER-2.md`（问题原始记录：三套并存 ⇒ 任意一份都能自证清白）。
- 看板行 `DEBT-2`（blocked_by=LEDGER-2）与 `DRIFT-2`。
- 另一条线的如实记录：它把未绑定债务从 25 抬到 28 并写进了看板证据，**没有代为绑定**
  （理由：Agent 选一套就是替属主定 SSOT）。

**判据**：给出「SSOT = 哪一套 + 其余两套的处置 + 未绑定声明的记账口径」。
若你认为必须由人先定口径才能选，请明确说 INCOMPLETE 并指出需要人提供什么。
材料不足 ⇒ INCOMPLETE。

待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**待决**：#5598（看板 P1-3）—— 「阶段门禁反向夹具缺口：23 道门禁无夹具，另有 2 处 fail-open」。
其中**两处 fail-open**是明确的判据缺陷（不是工作量问题），要裁的是**怎么修**：

- `scripts/gate/check_architecture_freeze.sh` 末尾 `exit 0  # 不阻塞，因为是改进项`
  ⇒ 该门禁**永远不会红**（除非前面就红）。
- 另一处：`A7-2` 打印的行缺失仍 PASS（见看板行 NF-1 的记录）。

**材料**：
- `scripts/gate/check_architecture_freeze.sh:125,130`（两处 `exit 0`）。
- 看板行 `NF-1`（两处 fail-open 的实测记录）与 `P1-3`（23 道门禁无夹具的清单）。
- 判据工具：`scripts/gate/gate_registry.py` + `scripts/gate/test_gate_registry.py`
  （9 条反向变异全部被抓住 ⇒ 门禁注册表本身有判别力）。
- `audit/v4.1.0/claims_selftest.py`（17 臂，判据的机器自检）。

**要裁的**：
  (1) 这两道门禁改成 fail-closed（红）还是保留 fail-open 但在**另一道门禁**里把它们列为
      「必须绿的例外清单」？（后者更温和：判据不放松，但例外被显式登记并有到期日）
  (2) 23 道无夹具门禁的补齐顺序（哪几道先补）与「pending-fixture」的关闭计划怎么写
      （`audit/v4.1.0/gen_neg_ledger.py` 已有 scope/effect/fixture 的数据结构）。
  (3) 改门禁判据时，**不得新增门禁名**（分母 168 不动）——请明确挂在哪道既有门禁下。

**判据**：给出可机器判的出口（哪条命令、期望 rc、哪条反向变异证明它会红）。
材料不足 ⇒ INCOMPLETE。

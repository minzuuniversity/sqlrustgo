待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**待决**：#5596（看板 IL-5）—— 「7 条历史审计记录不可复核（5 hollow 无命令行 / 1 未封板 / 1 无清单）：
退役、标注还是重做？」

**材料**：
- `audit/v4.1.0/runs/` 下的历史 run 目录（哪些缺 manifest / 缺 commands 行，现场已清点）。
- `scripts/gate/check_a04_evidence_reproducible.sh`（比对域与「本次产出」的口径）。
- `audit/v4.1.0/audit_run.py verify`（run 的可复现性判据本身）。
- 看板行 `IL-5`（Owner 裁定项的原始记录）与 `audit/v4.1.0/adjudications/pending-claims-sweep.md`。

**要裁的**：
  (1) 这 7 条：**退役**（标注为不可复核、不得引用）／**标注**（保留但加免责标记）／**重做**
      （用现役命令集重跑并把新 run 入库）—— 三选一或逐条分？
  (2) 若「重做」：重做要产出什么（新的 run id + manifest + commands 行）？由谁做（agent 可做？
      需要席位的部分呢）？
  (3) 防止再犯的**机器判据**是什么（例如 `audit_run.py verify` 对历史 run 缺失 manifest 时
      应当怎么报）？

**判据**：给出逐条处置与判据出口。材料不足 ⇒ INCOMPLETE。

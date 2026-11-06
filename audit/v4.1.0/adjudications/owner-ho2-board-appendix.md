待裁定问题（只依据材料判定，材料不足输出 INCOMPLETE）

**待决**：看板行 HO-2 —— ISSUE #5592 的正文是否要补「附录 A（交接正文全文）」，
以及由谁补。

**材料**：
- `audit/v4.1.0/board_appendix_check.py`：现场跑 ⇒ `findings=1`，
  `NOT-OK: 看板 #5592 没有附录 A 段`（该判据由另一条线写，现状稳定可复现）。
- `audit/v4.1.0/board_appendix_mutations.py`（该判据的反向变异：注入缺失 ⇒ 变红；
  注入改写 ⇒ 变红 ⇒ 判据有判别力）。
- `audit/v4.1.0/issues/KAN-2.md` 里记录的现状：正文由 `gen_kanban_issue.py` 生成，
  长度约 39 KB；交接正文全文另在
  `docs/knowledge-loop/handoffs/HANDOFF_2026-11-05_4.1.0_OMP_ONBOARDING.md`。
- 本轮实测：ISSUE #5592 的正文每次由 `gen_kanban_issue.py` 重贴（现 len 39379）。

**要裁的**：
  (1) 补不补附录 A（把交接正文全文嵌进看板 issue 正文）？
  (2) 若补：由哪个工具生成（`gen_kanban_issue.py` 加一段？另写脚本？），
      以及正文体积是否可接受（约 +50 KB ⇒ 接近 Gitea issue 正文上限？）。
  (3) 若不补：`board_appendix_check.py` 这条判据怎么处理（它现在在 develop 上是红的
      ⇒ 归档 run 里每批都带这一条红；是否改成 WARN 或下线）？

**判据**：给出结论与**具体做法**（含「若不补，判据怎么改」）。
材料不足 ⇒ INCOMPLETE。

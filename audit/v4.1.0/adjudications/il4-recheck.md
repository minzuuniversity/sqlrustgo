待裁定问题（只依据给出的材料判定，材料不足输出 INCOMPLETE）

只有一条声称：IL-4（台账快照边界）。上一轮你指出还缺三点直接证据，本轮补齐：

· 补证 A：live API 的 open 总数与 issue ID 集合（绕过快照与台账，直接问 Gitea）
· 补证 B：文档里记录的 open 计数与 live 总数对齐
· 补证 C：credited lineage 的独立来源（席位 health 记录，不是台账自报）

请判定：材料是否**支持**该声称？（REAL / FALSE_POSITIVE / INCOMPLETE）

【IL-4】台账快照边界：rows == triaged == 当期 open；校验器现场重算分级
  声称原文（看板）：
    任务：台账快照边界：A 代输入快照 106 条已过期（当期 open 110，多出 #5594/#5595/#5596/#5597）⇒ 用当期快照重生成 B 代台账并给 4 条补分诊行
    验收：重生成后 rows==triaged==当期 open issue 数（verify_ledger L1），且 #5594 出现在台账；文档里的 open issue 计数与当期快照一致

  补证 A：live API 的 open 总数与集合（**不经过任何中间文件**）：
```
Gitea API type=issues&state=open 的 x-total-count = None；逐页取全量得 110 条 issue（排除 PR）。
live 集合 − 快照集合 = []
快照集合 − live 集合 = []
live 集合 − 台账集合 = []
台账集合 − live 集合 = []
```

  补证 B：文档里记录的 open 计数，与上面的 live 总数逐个对齐：
```
# v4.1.0 open issue 台账（IL-1 产物）
- 输入：Gitea `type=issues` open=110；分诊行=112（`expert_triage.py`）
- issues：`python3 scripts/gate/gov_probe.py issues --dump <path>`（open issues 逐页 limit=50，dump 行自报 sha256）
（上文 live API 直接计数 = None）
```

  补证 C：credited lineage 的**独立来源**（不是台账 MD 的自报字段）：
```
{"seats_total": 5, "seats_ok": 2, "seats_parseable": 2, "lineages_reachable": ["gpt", "qwen"], "lineages_credited": ["gpt", "qwen"], "adjudication_capable": true, "capability_basis": "calibrated+role-eligible"}
```

请给出裁定 + 一句理由。若声称的措辞仍强于证据，请明确指出。

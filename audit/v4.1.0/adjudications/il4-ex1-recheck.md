待裁定问题（只依据给出的材料判定，材料不足输出 INCOMPLETE）

下面是 2 条声称。上一轮你们各自点出的缺口已补齐；IL-4 这轮还补了「#5594 已入台账 / 文档计数一致 / 校验器现场重算」三点的直接证据。

每条给出：**声称原文**、**绿灯输出**、**★红灯实验**。

请逐条判定：材料是否**支持**该声称？（REAL = 材料支持；FALSE_POSITIVE = 材料反驳；INCOMPLETE = 材料不足以判断）

【IL-4】台账快照边界：rows == triaged == 当期 open；校验器现场重算分级
  声称原文（看板）：
    任务：台账快照边界：A 代输入快照 106 条已过期（当期 open 110，多出 #5594/#5595/#5596/#5597）⇒ 用当期快照重生成 B 代台账并给 4 条补分诊行
    验收：重生成后 rows==triaged==当期 open issue 数（verify_ledger L1），且 #5594 出现在台账；文档里的 open issue 计数与当期快照一致

  绿灯输出（verify_ledger.py 的真实输出）：
```
{"kind": "ledger_verify", "batch": "B", "credited_lineages": ["gpt", "qwen"], "ladder_ready": true, "ledger_md": "docs/releases/v4.1.0/ISSUE_LEDGER_2026-11-05B.md", "rows": 110, "triaged_issues": 110, "grades": {"advisory-only": 52, "cross-checked": 58}, "triage_files": {"ISSUE_TRIAGE_2026-11-05.jsonl": "sha256:bcffc2fc87d9afc8436f288c6bb77cd65c529be3842211d9948c1a4fb388bbae", "ISSUE_TRIAGE_CROSSCHECK_2026-11-05.jsonl": "sha256:9719244075474f3c50f1e8afa2f2674708434867bed4c59f139b6543ad6c6852", "ISSUE_TRIAGE_2026-11-05B.jsonl": "sha256:b7bede1d9c1f2e55b9a6f221c37c9aa7581028a420b181fe41e7be8da44589eb", "ISSUE_TRIAGE_CROSSCHECK_2026-11-05B.jsonl": "sha256:f6b40af40f542e430f009c3f90560096849952997de15621078eb5b45b8948ae", "ISSUE_TRIAGE_CROSSCHECK_2026-11-05C.jsonl": "sha256:9532a397090961b88d72cf1537a5aa6f73c87a67781ab5bbe734052dee98a25c", "ISSUE_TRIAGE_CROSSCHECK_2026-11-05D.jsonl": "sha256:fec56b65e51873cae1cbfad2e606d7b759ac9a35e3b72174408193255acb6fe5"}, "snapshot_fingerprints": {"ISSUE_SNAPSHOT_2026-11-05.jsonl": "sha256:99bb3d19830cff62b8cc2e02f480e9ef214fa351b396078569db08b5017de9c0", "ISSUE_TRIAGE_2026-11-05C.jsonl": "sha256:f101f2e49dc890d62bc7a73eaded6d54e13042ab2fe4607fde15d89452e35683"}, "md_evidence_hash": "sha256:de5e91ae6a5a905f287b35f9f2cc212df6842b87d26f2cc71b9082052ccf70c0"}
verify_ledger[B]: OK
```

  ★补证：台账覆盖的当期 open issue（含 #5594/#5598/#5599）：
```
#5594/#5598/#5599 在台账中的行数 = 3
#5594/#5598/#5599 在台账中的行数 = 3
#5594/#5598/#5599 在台账中的行数 = 3
台账总行数 = 110；快照（live open，去重后）= 110
```

  红灯实验（六臂反向变异，已归档）：
```
ARM CTRL want=green got=green rc=0 ok=1
ARM A-MUT want=red got=red rc=1 ok=1
ARM B1-MUT want=red got=red rc=1 ok=1
ARM B2-MUT want=red got=red rc=1 ok=1
ARM B3-MUT want=red got=red rc=1 ok=1
ARM B4-MUT want=red got=red rc=1 ok=1
verify_ledger_mutation: ok=6 bad=0
verify_ledger_mutation: OK (CTRL green; A/B1/B2/B3/B4 red)
```

【EX-1】咨询专家梯：席位注册表 + fail-closed 聚合(R1/R2/R3) + 证据契约
  声称原文（看板）：
    任务：咨询专家梯：席位注册表 + fail-closed 聚合(R1/R2/R3) + 证据契约
    验收：夹具对 R1/R2/R3 各做反向变异后结论必须翻转；证据含 MANIFEST+sha256

  红灯实验（scripts/gate/test_expert_ladder.py 的机器核对：R1/R2/R3/R4 反向变异 + MANIFEST 证据契约 + 词表拒绝 + rev_LOCK）：
```
[1] parse_verdict
  OK   plain json
  OK   fenced json
  OK   prose-wrapped json
  OK   garbage => None (abstain)
  OK   wrong enum => None
[2] R1 family independence
  OK   two same-lineage seats must NOT constitute independent corroboration
  OK   rev_R1: relaxing family count changes the outcome
  OK   two distinct families agree REAL => REAL
  OK   two qwen serving stacks must NOT count as two independent experts
[3] R2 abstention
  OK   a seat voting INCOMPLETE blocks the aggregate
  OK   rev_R2: removing the abstaining seat restores a conclusion
  OK   transport fault => abstention => INCOMPLETE (never REAL/FALSE_POSITIVE)
  OK   unparseable verdict => INCOMPLETE
  OK   empty content w/ finish_reason=length is an abstention
  OK   empty content w/ finish_reason=stop is also an abstention
[4] R3 false-positive requires a passing reverse-mutation fixture
  OK   unanimous FALSE_POSITIVE without fixture must NOT greenlight the gate
  OK   a FAILING fixture does not count as machine evidence
  OK   rev_R3: passing fixture flips the outcome
  OK   seats disagree => INCOMPLETE
[5] single-session serialization
  OK   same endpoint is serialized (max concurrency == 1)
  OK   rev_LOCK: distinct endpoints run in parallel (not a global mutex)
[6] evidence contract
  OK   MANIFEST carries source_agent
  OK   MANIFEST carries source_run
  OK   MANIFEST carries timestamp_started
  OK   MANIFEST carries conflict_resolution
  OK   artifacts hashed
  OK   tampering an artifact is detectable via MANIFEST hash
[7] no PASS vocabulary
  OK   a seat claiming PASS is rejected (PASS is not a legal verdict)
  OK   aggregate refuses to conclude on vocabulary 'PASS'
  OK   aggregate refuses to conclude on vocabulary 'pass'
  OK   aggregate refuses to conclude on vocabulary 'DONE'
[8] R4 seat calibration
  OK   a correct seat calibrates 4/4
  OK   rev_R4: a confidently wrong seat scores 0/4
  OK   two calibrated distinct lineages => capable
  OK   same seats un-calibrated => NOT capable
  OK   a seat that failed calibration stays disqualified in transport-only mode
  OK   rev_R4: dropping calibration flips capability (calibration is load-bearing)
  OK   weaker mode is labelled, not silently reported as trust
  OK   a triage-only seat is never credited for R1 lineages
  OK   reachable lineages are still listed separately (transport truth)

ALL EXPERT-LADDER CHECKS PASS (fixture run, no real seat contacted)
```

请逐条给出裁定（REAL / FALSE_POSITIVE / INCOMPLETE）+ 一句理由。
特别地：若某条声称的措辞**强于**它给出的证据，请明确指出。

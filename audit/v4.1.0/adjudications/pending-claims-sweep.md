待裁定问题（只依据给出的材料判定，材料不足输出 INCOMPLETE）

下面 8 条是 4.1.0 治理整改里**已实现但只到「待复核」**的声称。
每条给出：声称本身 + 它的**主证据**（归档 run 的原始输出，不是结论摘要）。
请逐条判定：材料是否**支持**该声称？
（REAL = 材料支持；FALSE_POSITIVE = 材料反驳；INCOMPLETE = 材料不足以判断）

【声称 1】P1-4 每道门禁写 effect，声明与夹具一致
  主证据出处：honesty.py check（C1–C4 四条判据 + C5 反向）｜归档 run 20261105-adjudicate-001 006
  原始输出：
```
{"kind": "honesty_check", "catalog": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/GATE_CATALOG.jsonl", "gates": 168, "fixtures": 57, "fails": 0}
honesty: OK
```

【声称 2】P1-5 文档声明按 run 绑定分级，未绑定的新声明判红
  主证据出处：evidence_inventory.py check｜归档 run 20261105-adjudicate-001 008（claims_hits / claims_ref / unbound / fails）
  原始输出：
```
{"kind": "claims_inventory_check", "ledger": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/CLAIMS_INVENTORY.jsonl", "live_files": 59, "ledger_files": 59, "live_claims": 1144, "live_bound": 93, "live_unbound": 1051, "ledger_unbound": 1051, "fails": 0}
claims_inventory: OK
```

【声称 3】P2-1 文档事实面：索引逐字节一致 + 接入两个既有门禁；墙钟不得进比对域
  主证据出处：doc_facts.py check｜归档 run 20261105-adjudicate-001 009
  原始输出：
```
{"kind": "doc_facts_check", "docs": 91, "stamped": 1, "stamp_ok": 1, "index_bytes": 16744, "gates": [{"gate": "docs_pointer_paths", "status": "PASS", "exit_code": 0}, {"gate": "docs_links", "status": "PASS", "exit_code": 0}], "fails": 0}
doc_facts: OK
```

【声称 4】P2-2 目录策略机器化：新增违规 ⇒ 红，存量 149 条逐条记账
  主证据出处：directory_policy.py check｜归档 run 20261105-capability-001 015 + 基线台账
  原始输出：
```
{"kind": "directory_policy_check", "baseline": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/DIRECTORY_POLICY_BASELINE.jsonl", "baseline_violations": 149, "live_violations": 149, "new_violations": 0}
directory_policy: OK
```

【声称 5】IL-6 证据门禁自指：在飞 run 不进判定范围；历史判据未放宽
  主证据出处：il6_selfref_probe.py 四臂｜归档 run 20261105-capability-001 007；A01 门禁历史红基线｜20261105-adjudicate-001 005
  原始输出：
```
OK   inflight-not-red: rc=1 in-flight run red=False (total reds=7)
OK   worktree-scan-mutation: rc=1 in-flight run red=True (total reds=9)
OK   history-not-relaxed: rc=1 历史红 7/7 (total reds=7)
OK   exclude-run-option: victim=iaia410-5112-evidence-20261015 reds 7 -> 6 skipped=True
checks=4 ok=4 not_ok=0 drift=0
exit_code=1 fail_count=7
```

【声称 6】IL-11 存储语义声称清单：声明必须真的走生产存储链；名字推导声明已证伪回退
  主证据出处：storage_semantics_claims.py check｜20261105-adjudicate-001 007；生产存储门禁 R6｜20261105-adjudicate-001 004
  原始输出：
```
{"kind": "storage_semantics_check", "ledger": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/STORAGE_SEMANTICS_TESTS.jsonl", "test_files": 802, "declared": 28, "ledger_rows": 28, "fails": 0}
storage_semantics: OK
=== Result: PASS=6 FAIL=0 ===
```

【声称 7】IL-4 台账快照边界：rows == triaged == 当期 open；校验器现场重算分级
  主证据出处：verify_ledger.py｜归档 run 20261105-adjudicate-001（B 代台账 + 6 份分诊文件）
  原始输出：
```
verify_ledger[B]: OK（rows=110 triaged_issues=110 grades=advisory-only 51 / cross-checked 59，**现场重算**而非信台账自报；其 6 臂反向变异 CTRL 绿 / A/B1/B2/B3/B4 红）。
```

【声称 8】EX-1 专家梯：fail-closed 聚合，机器证据在场时才放行
  主证据出处：expert_ladder adjudicate（required_lineages=2 + machine_evidence）｜20261105-adjudicate-001 001/002
  原始输出：
```
[flash-next-252/qwen-strata] ok=True latency=13.13s verdict=INCOMPLETE
[chatgpt-web/gpt] ok=False latency=Nones verdict=-
{
 "decided_seats": [
  "flash-next-252"
 ],
 "verdicts": {
  "flash-next-252": "INCOMPLETE"
 },
 "abstained": [
  {
   "seat": "chatgpt-web",
   "why": "seat chatgpt-web busy: l…（截断）
[flash-next-252/qwen-strata] ok=True latency=13.68s verdict=INCOMPLETE
[chatgpt-web/gpt] ok=True latency=46.74s verdict=INCOMPLETE
{
 "decided_seats": [
  "chatgpt-web",
  "flash-next-252"
 ],
 "verdicts": {
  "flash-next-252": "INCOMPLETE",
  "chatgpt-web": "INCOMPLETE"
 },
 "abstained": [],
 "fami…（截断）
```

请逐条给出裁定（REAL / FALSE_POSITIVE / INCOMPLETE）+ 一句理由。
特别地：若某条声称的措辞**强于**它给出的证据，请明确指出（这是本次裁定的重点）。

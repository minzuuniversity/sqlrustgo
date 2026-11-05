待裁定问题（只依据给出的材料判定，材料不足输出 INCOMPLETE）

下面 8 条是 4.1.0 治理整改里**已实现但只到「待复核」**的声称。
每条给出三样东西：**声称原文（看板的任务与验收口径）**、**它的主证据（归档 run 的原始输出，不截断）**、
以及该声称涉及的**文件原文片段**。
请逐条判定：材料是否**支持**该声称？
（REAL = 材料支持；FALSE_POSITIVE = 材料反驳；INCOMPLETE = 材料不足以判断）

【P1-4】每道门禁写 effect，声明与夹具一致
  声称原文（看板）：
    任务：每道门禁写 effect + honesty.py 核对声明与夹具一致
    验收：声称能抓 X 但没有 X 的夹具 ⇒ 红

  主证据（honesty check（归档 run 20261105-adjudicate-001 006））：
```
{"kind": "honesty_check", "catalog": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/GATE_CATALOG.jsonl", "gates": 168, "fixtures": 57, "fails": 0}
honesty: OK
```

  主证据（GATE_CATALOG 抽样（该门禁的 effect 字段原文））：
```
{"scripts/gate/check_a04_evidence_reproducible.sh": {"catches": [], "cannot_catch": [], "reason": "尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。"}, "scripts/gate/check_audit_manifest.sh": {"catches": [], "cannot_catch": [], "reason": "尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。"}, "scripts/gate/check_docs_links.sh": {"catches": ["links-broken"], "cannot_catch": [], "reason": "由夹具 fx07-docs-links-broken 证明（kind=mutation-verified）"}, "scripts/gate/check_stage.sh": {"catches": ["version-uninferable"], "cannot_catch": [], "reason": "由夹具 fx14-stage-version-uninferable 证明（kind=mutation-verified）"}}
```

【P1-5】文档声明按 run 绑定分级，未绑定的新声明判红
  声称原文（看板）：
    任务：evidence_inventory.py：§1.4 刷新后的处数 PASS 声明分级绑定 run
    验收：未绑定的新声明 ⇒ 红

  主证据（claims check（归档 run 20261105-adjudicate-001 008））：
```
{"kind": "claims_inventory_check", "ledger": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/CLAIMS_INVENTORY.jsonl", "live_files": 59, "ledger_files": 59, "live_claims": 1144, "live_bound": 93, "live_unbound": 1051, "ledger_unbound": 1051, "fails": 0}
claims_inventory: OK
```

  主证据（绑定台账 CLAIMS_BINDINGS.jsonl 前两行）：
```
{"claims": 1, "evidence_refs": [], "excerpt": "## 这条故障切断了哪些声明 `postcheck` job 里挂着本轮新加的三条自测（看板 KR-2 看板台账一致性、 KR-3 门禁登记表一致性、P0-2 三态夹具 self-test）。CI 起不来 ⇒ - **KR-2 / KR-3 不能因为\"已经接线\"就判完成**：接线证据必须是某个 Actions run 的 job 日志， 现在拿不到，只能保持 **待复核**，并把\"没能核对\"写进 ", "file": "docs/releases/v4.1.0/CI_EXECUTION_OUTAGE_2026-11-05.md", "run_refs": ["20261105-tristate-001", "audit/v4.1.0/runs/20261105-tristate-001"], "section": 4}
{"claims": 1, "evidence_refs": ["docs/releases/v4.1.0/evidence/BATCH28_SEM4_AND_GATE_INTEGRITY_2026-10-23.md"], "excerpt": "### P0-01 `check_anti_ignore_gate.sh` 是空转门禁 (AFP Type B / P16 违规) - **verdict**: `CLOSED` (2026-10-23) - **销账依据**: 门禁现在用**与 registry 同一套规则**重算源码树里的真实 `#[ignore]` 数（`count_tree()`，`scripts/gate/check_a", "file": "docs/releases/v4.1.0/DEFECTS_AND_ISSUES.md", "run_refs": [], "section": 4}
```

【P2-1】文档事实面：索引逐字节一致 + 接入两个既有门禁；墙钟不得进比对域
  声称原文（看板）：
    任务：文档事实面：doc_stamp/doc_freshness/doc_path_claim_check/gen_doc_index 移植接入
    验收：悬空路径 0；生成物『写盘口径』==『核对口径』（墙钟时间不得进比对域）

  主证据（doc_facts check（归档 run 20261105-adjudicate-001 009））：
```
{"kind": "doc_facts_check", "docs": 91, "stamped": 1, "stamp_ok": 1, "index_bytes": 16744, "gates": [{"gate": "docs_pointer_paths", "status": "PASS", "exit_code": 0}, {"gate": "docs_links", "status": "PASS", "exit_code": 0}], "fails": 0}
doc_facts: OK
```

  主证据（墙钟护栏的正向判据（doc_facts.py 的 WALLCLOCK_RES））：
```
STAMP_RE = re.compile(r"^<!-- doc-stamp: sha256=([0-9a-f]{64}) -->\n?", re.M)
    re.compile(r"\b(19|20)\d{2}-\d{2}-\d{2}T\d{2}:\d{2}"),
    re.compile(r"\b(19|20)\d{2}-\d{2}-\d{2}\b"),
    re.compile(r"\bDate:\s*(Mon|Tue|Wed|Thu|Fri|Sat|Sun)\b"),
    re.compile(r"\bupdated_at\b"),
```

【P2-2】目录策略机器化：新增违规 ⇒ 红，存量 149 条逐条记账
  声称原文（看板）：
    任务：目录策略机器化（顶层 45 个 .md、bench/benches/benchmarks 等归位）
    验收：DIRECTORY_POLICY.md 已有规则，缺执行点；新增违规 ⇒ 红

  主证据（dir_policy check（归档 run 20261105-capability-001 009））：
```
{"kind": "directory_policy_check", "baseline": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/DIRECTORY_POLICY_BASELINE.jsonl", "baseline_violations": 149, "live_violations": 149, "new_violations": 0}
directory_policy: OK
```

  主证据（基线台账行数与首行）：
```
149 行；首行：{"item": "docs/AI增强软件工程", "rule": "dir-name-charset", "source": "DIRECTORY_POLICY.md §2.2"}
```

【IL-6】证据门禁自指：在飞 run 不进判定范围；历史判据未放宽
  声称原文（看板）：
    任务：证据门禁自指：check_audit_manifest.sh 扫描 runs/ 时把「正在执行的 run 目录」也计入判定 => run 进行中红数 7->8（该目录 logs 尚未写完），归档完再扫又回到 7
    验收：给出可复现的判别实验（同一条命令在 run 内 rc=1 / run 后 rc=0）与处置方案：门禁按 frozen_commit 树扫描 或 显式排除 --exclude-run <当前 run id>；两种方案都必须带反向变异，且不得放宽对历史判据

  主证据（il6 四臂（归档 run 20261105-capability-001 007））：
```
OK   inflight-not-red: rc=1 in-flight run red=False (total reds=7)
OK   worktree-scan-mutation: rc=1 in-flight run red=True (total reds=9)
OK   history-not-relaxed: rc=1 历史红 7/7 (total reds=7)
OK   exclude-run-option: victim=iaia410-5112-evidence-20261015 reds 7 -> 6 skipped=True
checks=4 ok=4 not_ok=0 drift=0
```

  主证据（A01 门禁历史红条数（归档 run 20261105-adjudicate-001 005））：
```
exit_code=1 fail_count=7
```

【IL-11】存储语义声称清单：声明必须真的走生产存储链；名字推导声明已证伪回退
  声称原文（看板）：
    任务：把 check_correctness_tests_production_storage.sh 的「按分支判定装配链」扩到所有声称验证存储语义的测试
    验收：「哪些测试声称验证存储语义」必须有一份可机器读的清单；清单里的每个测试都被生产存储门禁覆盖；清单与实际声明不符 ⇒ 红；反向变异：把某个测试改成只在内存存储上跑 ⇒ 必须红

  主证据（storage_semantics check（归档 run 20261105-adjudicate-001 007））：
```
{"kind": "storage_semantics_check", "ledger": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/STORAGE_SEMANTICS_TESTS.jsonl", "test_files": 802, "declared": 28, "ledger_rows": 28, "fails": 0}
storage_semantics: OK
```

  主证据（生产存储门禁 R6 结果（归档 run 20261105-adjudicate-001 004））：
```
=== Result: PASS=6 FAIL=0 ===
```

  主证据（名字推导反例（工具注释原文））：
```
        #   tests/unit/bplus_tree_test.rs —— 名字像存储语义，但它测的是
        #   tests/memory_storage_in_db_5057.rs —— 名字里有 storage，但它
```

【IL-4】台账快照边界：rows == triaged == 当期 open；校验器现场重算分级
  声称原文（看板）：
    任务：台账快照边界：A 代输入快照 106 条已过期（当期 open 110，多出 #5594/#5595/#5596/#5597）⇒ 用当期快照重生成 B 代台账并给 4 条补分诊行
    验收：重生成后 rows==triaged==当期 open issue 数（verify_ledger L1），且 #5594 出现在台账；文档里的 open issue 计数与当期快照一致

  主证据（verify_ledger 输出（归档 run 20261105-adjudicate-001 010））：
```
{"kind": "gate_registry_check", "catalog": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/GATE_CATALOG.jsonl", "check_n": 168, "catalog_rows": 168, "by_class": {"stage_required": 34, "ci_reachable": 51, "orphan_referenced": 22, "dead": 61}, "by_disposition": {"archive-candidate": 33, "keep-rewire": 12, "needs-execution-point": 25, "needs-external-host": 12, "superseded": 1}, "fails": 0}
gate_registry: OK
```

  主证据（批量裁定里该行的两条血统判定（归档 run 20261105-pendingsweep-002 001））：
```
[flash-next-252/qwen-strata] ok=True latency=24.92s verdict=INCOMPLETE
[chatgpt-web/gpt] ok=True latency=60.6s verdict=INCOMPLETE
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
 "families": [
  "gpt",
  "qwen-strata"
 ],
 "lineages": [
  "gpt",
  "qwen"
 ],
 "required_lineages": 2,
 "machine_evidence_present": true,
 "machine_evidence_passed": true,
 "soft_gate_note": "[SOFT GATE] 模型意见仅供参考，不作为门禁依据（AFP §5.1/5.3）",
 "aggregate": "INCOMPLETE",
 "rule": "R2: a seat abstained inside the verdict set",
 "question_id": "pending-claims-sweep",
 "artifacts": [],
 "prompt_sha256": "sha256:a8243603cf38ef913b0a00457c6450ac76565617706ae88e72be2cd680ac94d5",
 "adjudicated_at": "2026-11-05T19:40:39Z"
}
exit_code=2
```

【EX-1】专家梯：fail-closed 聚合，机器证据在场时才放行
  声称原文（看板）：
    任务：咨询专家梯：席位注册表 + fail-closed 聚合(R1/R2/R3) + 证据契约
    验收：夹具对 R1/R2/R3 各做反向变异后结论必须翻转；证据含 MANIFEST+sha256

  主证据（三轮裁定的聚合（归档 run 20261105-adjudicate-001 001/002））：
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
   "why": "seat chatgpt-web busy: lock held > 120s"
  }
 ],
 "families": [
  "qwen-strata"
 ],
 "lineages": [
  "qwen"
 ],
 "required_lineages": 2,
 "machine_evidence_present": true,
 "machine_evidence_passed": true,
 "soft_gate_note": "[SOFT GATE] 模型意见仅供参考，不作为门禁依据（AFP §5.1/5.3）",
 "aggregate": "INCOMPLETE",
 "rule": "R2: a seat abstained inside the verdict set",
 "question_id": "issue-5466-recheck",
 "artifacts": [],
 "prompt_sha256": "sha256:8d026d64eae30f5c35bfb1ea063e8429d832d7ebdeed99ab26c39b083b9bd28a",
 "adjudicated_at": "2026-11-05T19:27:26Z"
}
exit_code=2
```

请逐条给出裁定（REAL / FALSE_POSITIVE / INCOMPLETE）+ 一句理由。
特别地：若某条声称的措辞**强于**它给出的证据，请明确指出。

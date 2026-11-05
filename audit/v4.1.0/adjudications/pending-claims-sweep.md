待裁定问题（只依据给出的材料判定，材料不足输出 INCOMPLETE）

下面是 8 条「已实现但只到待复核」的声称。每条给出三样东西：
**声称原文**（看板的任务与验收口径）、**绿灯输出**（归档 run 的原始输出）、
以及 ★**红灯实验**（把判据驱动到退化状态的反向变异，已固化成归档命令）。

上一轮两族都指出：绿色输出不能证明反向变异那部分。这一版把红灯实验引到了声称旁边。
请逐条判定：材料是否**支持**该声称？
（REAL = 材料支持；FALSE_POSITIVE = 材料反驳；INCOMPLETE = 材料不足以判断）

【P1-5】文档声明按 run 绑定分级，未绑定的新声明判红
  声称原文（看板）：
    任务：evidence_inventory.py：§1.4 刷新后的处数 PASS 声明分级绑定 run
    验收：未绑定的新声明 ⇒ 红

  绿灯输出：
```
{"kind": "claims_inventory_check", "ledger": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/CLAIMS_INVENTORY.jsonl", "live_files": 59, "ledger_files": 59, "live_claims": 1144, "live_bound": 93, "live_unbound": 1051, "ledger_unbound": 1051, "fails": 0}
claims_inventory: OK
```

  ★ 红灯实验（归档 run 20261105-claims-selftest-001 / claims-selftest 命令）：
```
[OK ] P1-5/M1 新文档带未绑定声明 ⇒ 红  (rc=1)
  [OK ] P1-5/M2 新文档带已绑定声明 ⇒ 绿  (rc=0)
  [OK ] P1-5/M3 已有文档新增未绑定声明 ⇒ 红  (rc=1)
  [OK ] P1-5/M4 撤销后 ⇒ 绿  (rc=0)
```

【P1-4】每道门禁写 effect，声明与夹具一致
  声称原文（看板）：
    任务：每道门禁写 effect + honesty.py 核对声明与夹具一致
    验收：声称能抓 X 但没有 X 的夹具 ⇒ 红

  绿灯输出：
```
{"kind": "honesty_check", "catalog": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/GATE_CATALOG.jsonl", "gates": 168, "fixtures": 57, "fails": 0}
honesty: OK
```

  ★ 红灯实验（归档 run 20261105-claims-selftest-001 / claims-selftest 命令）：
```
  [OK ] P1-4/M1 给 pending-fixture 门禁声称能抓 ⇒ 红  (rc=1)
  [OK ] P1-4/M2 声称夹具里没有的性质 ⇒ 红  (rc=1)
  [OK ] P1-4/M3 删掉 effect 声明 ⇒ 红  (rc=1)
  [OK ] P1-4/M4 撤销后 ⇒ 绿  (rc=0)
```

【P2-1】文档事实面：索引逐字节一致 + 接入两个既有门禁；墙钟不得进比对域
  声称原文（看板）：
    任务：文档事实面：doc_stamp/doc_freshness/doc_path_claim_check/gen_doc_index 移植接入
    验收：悬空路径 0；生成物『写盘口径』==『核对口径』（墙钟时间不得进比对域）

  绿灯输出：
```
{"kind": "doc_facts_check", "docs": 91, "stamped": 1, "stamp_ok": 1, "index_bytes": 16744, "gates": [{"gate": "docs_pointer_paths", "status": "PASS", "exit_code": 0}, {"gate": "docs_links", "status": "PASS", "exit_code": 0}], "fails": 0}
doc_facts: OK
```

  ★ 红灯实验（归档 run 20261105-claims-selftest-001 / claims-selftest 命令）：
```
  [OK ] P2-1/M1 改文档不重新 stamp ⇒ 红  (rc=1)
  [OK ] P2-1/M2 索引塞墙钟 ⇒ 红  (rc=1)
  [OK ] P2-1/M4 撤销后 ⇒ 绿  (rc=0)
```

【P2-2】目录策略机器化：新增违规 ⇒ 红，存量 149 条逐条记账
  声称原文（看板）：
    任务：目录策略机器化（顶层 45 个 .md、bench/benches/benchmarks 等归位）
    验收：DIRECTORY_POLICY.md 已有规则，缺执行点；新增违规 ⇒ 红

  绿灯输出：
```
{"kind": "directory_policy_check", "baseline": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/DIRECTORY_POLICY_BASELINE.jsonl", "baseline_violations": 149, "live_violations": 149, "new_violations": 0}
directory_policy: OK
```

  ★ 红灯实验（归档 run 20261105-claims-selftest-001 / claims-selftest 命令）：
```
  [OK ] P2-2/M1 新建违规目录 ⇒ 红  (rc=1)
  [OK ] P2-2/M2 撤销后 ⇒ 绿  (rc=0)
```

【IL-6】证据门禁自指：在飞 run 不进判定范围；历史判据未放宽（默认扫描下）
  声称原文（看板）：
    任务：证据门禁自指：check_audit_manifest.sh 扫描 runs/ 时把「正在执行的 run 目录」也计入判定 => run 进行中红数 7->8（该目录 logs 尚未写完），归档完再扫又回到 7
    验收：给出可复现的判别实验（同一条命令在 run 内 rc=1 / run 后 rc=0）与处置方案：门禁按 frozen_commit 树扫描 或 显式排除 --exclude-run <当前 run id>；两种方案都必须带反向变异，且不得放宽对历史判据

  绿灯输出（四臂）：
```
OK   inflight-not-red: rc=1 in-flight run red=False (total reds=7)
OK   worktree-scan-mutation: rc=1 in-flight run red=True (total reds=9)
OK   history-not-relaxed: rc=1 历史红 7/7 (total reds=7)
OK   exclude-run-option: victim=iaia410-5112-evidence-20261015 reds 7 -> 6 skipped=True
checks=4 ok=4 not_ok=0 drift=0
```

  ★ 红灯实验（同一归档 run 的 il6-selfref-probe 命令，四臂含反向变异）：
```
OK   inflight-not-red: rc=1 in-flight run red=False (total reds=7)
OK   worktree-scan-mutation: rc=1 in-flight run red=True (total reds=9)
OK   history-not-relaxed: rc=1 历史红 7/7 (total reds=7)
OK   exclude-run-option: victim=iaia410-5112-evidence-20261015 reds 7 -> 6 skipped=True
checks=4 ok=4 not_ok=0 drift=0
```

【IL-11】存储语义声称清单：声明必须真的走生产存储链；名字推导声明已证伪回退
  声称原文（看板）：
    任务：把 check_correctness_tests_production_storage.sh 的「按分支判定装配链」扩到所有声称验证存储语义的测试
    验收：「哪些测试声称验证存储语义」必须有一份可机器读的清单；清单里的每个测试都被生产存储门禁覆盖；清单与实际声明不符 ⇒ 红；反向变异：把某个测试改成只在内存存储上跑 ⇒ 必须红

  绿灯输出：
```
{"kind": "storage_semantics_check", "ledger": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/STORAGE_SEMANTICS_TESTS.jsonl", "test_files": 802, "declared": 28, "ledger_rows": 28, "fails": 0}
storage_semantics: OK
```

  ★ 红灯实验（归档 run 20261105-claims-selftest-001 / claims-selftest 命令）：
```
  [OK ] IL-11/M1 改成只在内存存储上跑 ⇒ 红  (rc=1)
  [OK ] IL-11/M2 删掉声明标记 ⇒ 红  (rc=1)
  [OK ] IL-11/M3 给不碰生产存储的文件加声明 ⇒ 红  (rc=1)
  [OK ] IL-11/M4 撤销后 ⇒ 绿  (rc=0)
```

【IL-4】台账快照边界：rows == triaged == 当期 open；校验器现场重算分级
  声称原文（看板）：
    任务：台账快照边界：A 代输入快照 106 条已过期（当期 open 110，多出 #5594/#5595/#5596/#5597）⇒ 用当期快照重生成 B 代台账并给 4 条补分诊行
    验收：重生成后 rows==triaged==当期 open issue 数（verify_ledger L1），且 #5594 出现在台账；文档里的 open issue 计数与当期快照一致

  绿灯输出：
```
{"kind": "gate_registry_check", "catalog": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/GATE_CATALOG.jsonl", "check_n": 168, "catalog_rows": 168, "by_class": {"stage_required": 34, "ci_reachable": 51, "orphan_referenced": 22, "dead": 61}, "by_disposition": {"archive-candidate": 33, "keep-rewire": 12, "needs-execution-point": 25, "needs-external-host": 12, "superseded": 1}, "fails": 0}
gate_registry: OK
```

  校验器自身：audit/v4.1.0/verify_ledger_mutation.sh 的 6 臂反向变异（CTRL 绿 / A/B1/B2/B3/B4 红）：
```
六臂反向变异全部按期望（CTRL 绿 ⇒ 原始实现不红；A 把 family 门限放宽到 1 ⇒ 红；B1 改输入指纹一位 ⇒ 红；B2 改分级规则 ⇒ 红；B3 改台账行 ⇒ 红；B4 删分诊文件 ⇒ 红）。逐臂日志见 audit/v4.1.0/runs/20261105-adjudicate-001/logs/010-gate-registry-check.stdout。
```

【EX-1】专家梯：fail-closed 聚合，机器证据在场时才放行
  声称原文（看板）：
    任务：咨询专家梯：席位注册表 + fail-closed 聚合(R1/R2/R3) + 证据契约
    验收：夹具对 R1/R2/R3 各做反向变异后结论必须翻转；证据含 MANIFEST+sha256

  三轮裁定的聚合：
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

  ★ 红灯证据（fail-closed 的反面）：
```
见三轮裁定：两次因席位弃权/材料不足而聚合 INCOMPLETE（机器证据在场 machine_evidence_passed=true 却仍不放行）⇒ 机器证据在场**不是**放行的充分条件 ⇒ fail-closed 生效。
```

请逐条给出裁定（REAL / FALSE_POSITIVE / INCOMPLETE）+ 一句理由。
特别地：若某条声称的措辞**强于**它给出的证据，请明确指出。

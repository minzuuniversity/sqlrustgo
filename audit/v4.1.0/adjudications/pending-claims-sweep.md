待裁定问题（只依据给出的材料判定，材料不足输出 INCOMPLETE）

下面是 8 条「已实现但只到待复核」的声称。每条给出：**声称原文**（看板的任务与验收口径）、
**绿灯输出**、以及 ★**红灯实验**（把判据驱动到退化状态的反向变异）。

本轮的两处补正：IL-4 的绿灯输出上一轮引错了（引成门禁注册表），已换成 verify_ledger 的真实输出；
EX-1 上一轮缺 R1/R2/R3 的夹具反向变异，本轮补上 test_expert_ladder.py 的机器核对输出。

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

  ★红灯实验：
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

  ★红灯实验：
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

  ★红灯实验：
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

  ★红灯实验：
```
  [OK ] P2-2/M1 新建违规目录 ⇒ 红  (rc=1)
  [OK ] P2-2/M2 撤销后 ⇒ 绿  (rc=0)
```

【IL-6】证据门禁自指：在飞 run 不进判定范围；历史判据在默认扫描下未放宽
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

【IL-11】存储语义声称清单：声明必须真的走生产存储链；名字推导声明已证伪回退
  声称原文（看板）：
    任务：把 check_correctness_tests_production_storage.sh 的「按分支判定装配链」扩到所有声称验证存储语义的测试
    验收：「哪些测试声称验证存储语义」必须有一份可机器读的清单；清单里的每个测试都被生产存储门禁覆盖；清单与实际声明不符 ⇒ 红；反向变异：把某个测试改成只在内存存储上跑 ⇒ 必须红

  绿灯输出：
```
{"kind": "storage_semantics_check", "ledger": "/Volumes/workspace/dev/sqlrustgo/.worktrees/omp-410/docs/releases/v4.1.0/STORAGE_SEMANTICS_TESTS.jsonl", "test_files": 802, "declared": 28, "ledger_rows": 28, "fails": 0}
storage_semantics: OK
```

  ★红灯实验：
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

  ★绿灯输出（本轮已换成 verify_ledger 的真实输出，上一轮引错成门禁注册表）：
```
{"kind": "ledger_verify", "batch": "B", "credited_lineages": ["gpt", "qwen"], "ladder_ready": true, "ledger_md": "docs/releases/v4.1.0/ISSUE_LEDGER_2026-11-05B.md", "rows": 110, "triaged_issues": 110, "grades": {"advisory-only": 51, "cross-checked": 59}, "triage_files": {"ISSUE_TRIAGE_2026-11-05.jsonl": "sha256:bcffc2fc87d9afc8436f288c6bb77cd65c529be3842211d9948c1a4fb388bbae", "ISSUE_TRIAGE_CROSSCHECK_2026-11-05.jsonl": "sha256:9719244075474f3c50f1e8afa2f2674708434867bed4c59f139b6543ad6c6852", "ISSUE_TRIAGE_2026-11-05B.jsonl": "sha256:b7bede1d9c1f2e55b9a6f221c37c9aa7581028a420b181fe41e7be8da44589eb", "ISSUE_TRIAGE_CROSSCHECK_2026-11-05B.jsonl": "sha256:f6b40af40f542e430f009c3f90560096849952997de15621078eb5b45b8948ae", "ISSUE_TRIAGE_CROSSCHECK_2026-11-05C.jsonl": "sha256:9532a397090961b88d72cf1537a5aa6f73c87a67781ab5bbe734052dee98a25c", "ISSUE_TRIAGE_CROSSCHECK_2026-11-05D.jsonl": "sha256:fec56b65e51873cae1cbfad2e606d7b759ac9a35e3b72174408193255acb6fe5"}, "snapshot_fingerprints": {"ISSUE_SNAPSHOT_2026-11-05.jsonl": "sha256:8c5032f24de3442d42e044cdd69dccbebb7e5216033fd84bb4da24eb9fedc303"}, "md_evidence_hash": "sha256:751fb3b1c95400527b140a47c5c7cba738cdb9bc6831879eda21a1827c89ca88"}
verify_ledger[B]: OK
```

  ★红灯实验（六臂反向变异，已归档）：
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

【EX-1】专家梯：fail-closed 聚合，机器证据在场时才放行；夹具对 R1/R2/R3 各做反向变异后结论必须翻转
  声称原文（看板）：
    任务：咨询专家梯：席位注册表 + fail-closed 聚合(R1/R2/R3) + 证据契约
    验收：夹具对 R1/R2/R3 各做反向变异后结论必须翻转；证据含 MANIFEST+sha256

  ★红灯实验（本轮补上：scripts/gate/test_expert_ladder.py 的机器核对，含 rev_R1/rev_R2/rev_R3/rev_R4 反向变异、MANIFEST 证据契约、词表拒绝）：
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

# MPRE-BLOCKERS — M-Pre 逐条消除台账

- source_run: v78-r7 + DSH-A 复核 r8
- timestamp: 2026-11-21T01:56+0800
- 口径：逐条消除后须附证据；完成后由 DSH-A 复核关闭
- 本轮复核：MP-2/3/5/6/7

{
 "M-Pre blockers": [
  {
   "id": "MP-1",
   "item": "命名错位（C01-02/03/04/17）",
   "owner": "Codex 修订重签",
   "status": "OPEN",
   "note": "证据已交 Bus，等 Codex 执行"
  },
  {
   "id": "MP-2",
   "item": "G5 出口控制（OD-002 pf 白名单）",
   "owner": "DSH-A 实测",
   "status": "PARTIAL",
   "_updated": "2026-11-21T01:56+0800",
   "_result": "策略层 PASS / 网络层 FAIL",
   "note": "evidence: deepseek-schedule/runtime/evidence/MP-2-G5-EGRESS-TEST.json | policy: 5/5 云端 harness 被拒, 3/3 本地允许 | network: api.deepseek.com:443 与 api.anthropic.com:443 可连通, 无 pf 规则 | runtime: permit.py 派发前校验, router 无云端 adapter | 结论: LOCAL_ONLY 是策略隔离, 不是网络隔离"
  },
  {
   "id": "MP-3",
   "item": "G-LOCAL-ONLY/G-POLICY-BINDING/G-ISOLATED-CRASH-WINDOW 核验",
   "owner": "DSH-A",
   "status": "PARTIAL",
   "_updated": "2026-11-21T01:56+0800",
   "_result": "3 门禁 2 过 1 待",
   "note": "G-LOCAL-ONLY PASS | G-POLICY-BINDING PASS | G-ISOLATED-CRASH-WINDOW PASS | 待补: G5 网络层(见 MP-2)"
  },
  {
   "id": "MP-4",
   "item": "Codex 冻结 7 项补充验收契约（ADR-011/006 条件）",
   "owner": "Codex",
   "status": "OPEN",
   "note": "属主指令已发，等执行"
  },
  {
   "id": "MP-5",
   "item": "批准范围持久化至运行态授权库",
   "owner": "DSH-A",
   "status": "DONE",
   "_updated": "2026-11-21T01:56+0800",
   "_result": "已入库",
   "note": "delegation DSH-B-SRG-CALPHA01 issued 01:37:56 expires 2026-12-21 | budget total 2,570,000 granted 400,000 consumed 0 | 18 tasks in tasks table | release 评估 0/18 (TECH_GATE_FAILED)"
  },
  {
   "id": "MP-6",
   "item": "Reviewer 调度（#5912/5915/5916/5917 四 PR）",
   "owner": "DSH-A",
   "status": "PARTIAL",
   "_updated": "2026-11-21T01:56+0800",
   "_result": "2/4 完成",
   "note": "#5912 APPROVE: 编译通过, tf_t4 4 RED 正确失败, 工具 8 passed | #5915 APPROVE: INV-01 rollback 后 6 行 vs 3 行, INV-02/07a/07b 同 RED | #5916/#5917: draft=True mergeable=False, 无法合并, 待同批裁决"
  },
  {
   "id": "MP-7",
   "item": "PR #5916 CI 重查与合并申请",
   "owner": "Codex/DSH-A",
   "status": "BLOCKED",
   "_updated": "2026-11-21T01:56+0800",
   "_result": "CI 未运行",
   "note": "CI status=pending statuses=0 (从未运行) | #5916 draft=True mergeable=False | #5917 draft=True mergeable=False | 阻塞方: Codex 需转 ready 并触发 CI"
  },
  {
   "id": "MP-8",
   "item": "75 场景中 5 RED 基线项的 Issue 全关联",
   "owner": "DSH-B",
   "status": "DONE",
   "note": "已完成（#5913/#5914 + #5905 族关联）"
  },
  {
   "id": "MP-9",
   "item": "INV-07b 证据边界更正落账",
   "owner": "DSH-B",
   "status": "DONE",
   "note": "已完成"
  }
 ]
}

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)

# 建议提交文本 — Test Factory 反例清除批次（srg-test-factory-pilot）

> 供属主直接使用或修改后使用。分支 srg-test-factory-pilot @ e9ff21fca3。

```
fix(test-factory): 反例#1 修复 + 复跑证据 + AI 候选测试（DSH-B goal R1-R7）

两项确定性反例（Codex contract-observations 2026-11-20）之 #1：
pilot test_factory 把"零测试 + exit 0"判为 PASS（空跑不是验收证据）。

修复（tools/test_factory.py）：
- 无测试行（即使 exit 0）→ NOT_RUN，note 注明空跑非证据
- summary.passed 与解析测试行交叉校验，不一致 → INFRA_ERROR
- should_panic 测试行解析（218 vs 214 差异根因）
- --test 模式支持集成测试目标
- record() 原始 cargo 输出真实留档（此前 .log 误存 JSON）

回归（tools/test_test_factory_counterexample.py，8/8 绿）：
- 零测试两种形态 NOT_RUN；summary 不一致 INFRA_ERROR；
- should_panic 解析；raw 留档；正常 PASS/FAIL/KNOWN_FAIL 不回归
- 真实冒烟：cargo 零测试 40.2s exit 0 → 实测 NOT_RUN

证据：
- 75 场景复跑官方汇总：70 PASS / 5 RED / 0 未跑
  （evidence/case-mapping-20261120.json、rerun-summary-20261120.json）
- D-001 RED：tf_t4_stub_silent_success 4/4 FAILED (CARGO_EXIT=101)
- D-002/D-002b RED（基线）：ODKU 回滚 + 脏读（adapted probe, exit 101）
- AI 候选测试 tf_t4_ai_candidate_v1：4/4 RED 同效检出 D-001
  （Task B/C/D 闭环，本地 252/8021 模型，全程 LOCAL_ONLY 记账）

5 字段：source_agent=DSH-B(opencode-go/glm-5.3-flash)；source_run=dshb-goal-R1-R7；
timestamp=2026-11-20T17:31+08:00；evidence_hash=见 evidence/rerun-summary-20261120.json
artifact 摘要；conflict_resolution=Owner 裁决优先，Codex 契约为冻结源。
```

注：是否同批提交 AI 候选测试文件（tf_t4_ai_candidate_v1.rs）由属主定——它依赖存在缺陷的
BinaryTableStorageV2 当前行为，若先行修复 D-001 该测试应转绿（此即 NEW_GREEN 判据）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)

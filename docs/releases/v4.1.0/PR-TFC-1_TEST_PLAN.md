# PR-TFC-1 TEST PLAN
<!-- section:TEST_PLAN -->
## 覆盖目标/阶段
- 阶段：C-ALPHA-01（测试建设，属主授权）。
- 覆盖：反例#1 全部分支（零测试两形态、summary 不一致、should_panic、正常 PASS/FAIL/KNOWN_FAIL 不回归）。
- 基线：75 场景复跑 70 PASS / 5 RED / 0 未跑（evidence/rerun-summary-20261120.json），本 PR 不得使其回退。
- 不覆盖：D-001~003 的产品修复（架构方向属主裁决中）；大型 SOAK（属主令禁用）。

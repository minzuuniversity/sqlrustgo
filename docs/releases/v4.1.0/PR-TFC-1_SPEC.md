# PR-TFC-1 SPEC — Test Factory 反例清除批次
<!-- section:SPEC -->
## 设计/范围/接口
- 消除 Codex contract-observations (2026-11-20) 反例#1：pilot test_factory 把"零测试+exit 0"判为 PASS。
- 修复语义：无测试行（即使 exit 0）→ NOT_RUN（空跑非验收证据）；summary.passed 与解析测试行交叉校验，不一致 → INFRA_ERROR；补 should_panic 行解析、--test 集成模式、record() 原始输出留档。
- 新增资产：D-001 RED 固化测试（tf_t4_stub_silent_success）、AI 同效候选测试（tf_t4_ai_candidate_v1）、SRG-D004-01 四判据验收器（verify_srg_d004_acceptance.py）。
- 范围：仅测试工具与测试文件 + 证据；**零产品源码修改**。
- 接口变化：test_factory CLI 增加 `run --test <name>`；run_suite 返回新增 `raw` 字段。

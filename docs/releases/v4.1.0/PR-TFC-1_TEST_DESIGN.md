# PR-TFC-1 TEST DESIGN
<!-- section:TEST_DESIGN -->
## 具体测试方法
- 单元回归：tools/test_test_factory_counterexample.py（8 例，monkeypatch 隔离 cargo 输出）。
- 真实冒烟：cargo 零测试运行（40.2s, exit 0）→ 断言 class=NOT_RUN（evidence/RERUN-ZEROTEST/工具输出）。
- RED 固化：tf_t4_stub_silent_success 4/4 FAILED, CARGO_EXIT=101（两轮独立复测一致）。
- AI 同效验证：Task B（252 写）→ Task D（8021 审出 HIGH 假阴）→ Task C（整改+编译反馈 2 轮）→ 终验 4/4 RED 与人工同效。
- 崩溃族：crash_commit_fsync_b1（真实 SIGKILL 注入，需 --all-features 服务器二进制）2/2 PASS；torn_write_recovery 1/1 PASS。
- 验收器 pre 阶段：verify_srg_d004_acceptance.py --phase pre → RED 判据成立（exit 0）。

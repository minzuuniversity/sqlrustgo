# R1 工件漂移与传播整改方案

provenance: source_agent=Codex; source_run=TF-DESIGN-FREEZE-20261121; timestamp=2026-11-21T07:07:39+08:00; evidence_hash=sha256:24c79af689bdd4fa8f34d90b4a02591e65f2ef979435463bec5d056f63532e20; conflict_resolution=采用 DSH-B 只读重放矩阵作为待独立复验输入，不把自动 APPROVED 当技术验收。

## 已观察范围

DSH-B 报告显示 #5919/#5920/#5921/#5922 的 PR HEAD 仍传播旧恒真 `ca01_commit_frontier.rs`；#5920/#5921 还传播空函数版 `ca01_backend_caps.rs`。#5918 的对应工件为修订版，但仍受 ADR/HUMAN_GATE 阻断。

已报告的完整 blob SHA256：frontier 恒真版 `c688a673aaefda51b5023f0f3ad3d3a6846e3cee84a784820ed875252c2f1525`；frontier 修订版 `72f7afda397e08c565c5d89928dc5bd629d46adbe4a27880b6e7c958dac1a16d`；caps 空函数版 `ca856f255bd36da825a1c1dd67a0c2a67c115263db7f6fecbde37e1405f234d9`；caps 修订版 `8d1b99397a589e5d588dd3fcf9a1e197b82aa14ce7f140bb855d0b4d88c0d6da`。

## 传播整改算法

1. 冻结每个受影响 PR 的 `pr_number/head_sha/file_sha256`，禁止用分支名替代 SHA。
2. 根据 TF-S1-001..003 的最终行为规格重建单一规范工件；不得把静态 gap probe 宣称为 Contract Test。
3. 对每个受影响 PR 逐一 rebase/cherry-pick 或重建，禁止覆盖不相关增量。
4. 逐 PR 复算三项 S1 文件 SHA、case_id 与 assertion_semantics；任何不一致均保持 `R1_DRIFT`。
5. 更新 RED 映射的 `source_ref` 为具体 PR HEAD 与 blob SHA；HEAD 推进后旧收据自动失效。
6. 独立 Reviewer 从每个固定 HEAD 重跑 mutation：名称注入不能骗过行为契约、固定 None 判 INEFFECTIVE_TEST、缺接口为 NOT_IMPLEMENTED/NOT_RUN 而非 PASS。
7. 所有收据齐全后，R1 才能从 BLOCKED 转为 REVIEWED；不得自动合并。

## 审批传播隔离

自动巡检只核对 mergeable、draft、REQUEST_CHANGES、分支保护或 CI 存在性时，其输出只能是 `FORMAL_CHECK_PASSED`。旧自动 APPROVED 不因后续工件修订自动继承；HEAD 改变即失效。

## TF-01 出口

- 受影响 PR HEAD 与三项 S1 工件完整哈希齐备。
- Gap Probe/Contract Test/INEFFECTIVE_TEST 分类一致。
- 正反 mutation 实际执行，零 SKIP 冒充 PASS。
- Reviewer 与实现者不同，证据绑定最终 HEAD。
- HUMAN_GATE、合并与发布权限保持未授权。

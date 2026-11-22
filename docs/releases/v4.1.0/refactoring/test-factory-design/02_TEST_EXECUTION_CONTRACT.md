# 契约测试执行协议

## 双轴模型

执行轴：`EXECUTED | NOT_RUN | COMPILE_ERROR | TIMEOUT | INFRA_ERROR`。

契约轴：`CONTRACT_PASS | EXPECTED_RED | UNEXPECTED_RED | UNEXPECTED_GREEN | INEFFECTIVE_TEST | NOT_APPLICABLE`。

只有 `EXECUTED + CONTRACT_PASS` 可证明该 case 在该固定 SHA 满足契约。`EXECUTED + EXPECTED_RED` 只证明指定缺陷被指定签名复现，不算产品 PASS，不允许关闭工作包或发布。

## 分类算法

1. 验证计划清单签名、base SHA、case_id 唯一性和测试源 SHA。
2. 编译失败 → `COMPILE_ERROR + NOT_APPLICABLE`。
3. 超时 → `TIMEOUT + NOT_APPLICABLE`。
4. runner/fixture/磁盘/网络错误 → `INFRA_ERROR + NOT_APPLICABLE`。
5. 进程成功但实际 case 集为空或缺项 → `NOT_RUN + NOT_APPLICABLE`。
6. case 执行失败，身份和失败签名精确匹配开放基线 → `EXECUTED + EXPECTED_RED`。
7. case 失败但签名不符或未登记 → `EXECUTED + UNEXPECTED_RED`。
8. 登记为 EXPECTED_RED 的 case 转绿 → `EXECUTED + UNEXPECTED_GREEN`，触发独立复验，不自动关 Issue。
9. 有效正向契约执行且断言通过 → `EXECUTED + CONTRACT_PASS`。
10. 恒真、空函数、仅名称存在、仅注释/字符串匹配、SKIP 后成功返回 → `INEFFECTIVE_TEST` 或 `NOT_RUN`，不得计覆盖。

## 退出码

- `0`：所有适用 case 均为 `EXECUTED + CONTRACT_PASS`，或专用缺陷复现命令的全部 case 均为已验证 EXPECTED_RED 且调用者显式选择 `--mode reproduce`。后者仍不得标产品 PASS。
- `1`：UNEXPECTED_RED、UNEXPECTED_GREEN、INEFFECTIVE_TEST 或身份/签名不匹配。
- `2`：NOT_RUN、COMPILE_ERROR、TIMEOUT、INFRA_ERROR、计划/证据不可判定。
- `3`：输入 Schema、授权或基线签名错误。

原始 Cargo `101` 仅是进程证据，不直接决定 EXPECTED_RED。

## 失败签名

签名至少包含：case_id、test binary、test function、assertion key、标准化消息摘要、panic 类型/错误类型、base SHA、source SHA。禁止只用退出码、行号或宽泛子串。

# CHECK / UPDATE_BASELINE 设计

## 已确认风险

基线脚本当前在计数不下降且 ignore 条件满足时直接覆写 `tests/baseline/test_count.json`。这使 CHECK 同时承担验证和更新职责。

## 目标接口

- `check_test_count_monotonic.sh`：默认且唯一行为是只读比较；前后 baseline SHA 和工作树必须一致。
- `update_test_count_baseline.sh --expected-head <40hex> --approval-ref <ref>`：唯一更新入口；拒绝缺参数、HEAD 漂移、计数下降、ignore 增加、零测试或 manifest 不诚实。
- 更新先写临时文件，验证 JSON、计数方法、完整 diff 后原子替换；生成审计收据，不自称门禁 PASS。

## 授权规则

UPDATE_BASELINE 必须绑定 Owner/正式 Reviewer 授权引用、固定 HEAD、旧/新完整 SHA256、计数差异、source_agent/source_run/timestamp/evidence_hash/conflict_resolution。普通 CI 不调用更新入口。

## 验收

1. CHECK 前后 `sha256sum tests/baseline/test_count.json` 相同。
2. CHECK 前后 `git diff --exit-code -- tests/baseline/test_count.json` 成功。
3. 未授权 update 返回 3，文件不变。
4. 计数下降、ignore 增加、零测试或 manifest gate 失败时 update 返回非零，文件不变。
5. 授权 update 只修改 baseline 和审计收据；旧/新 SHA 可复算。

本轮只冻结设计，不修改现有脚本或基线。

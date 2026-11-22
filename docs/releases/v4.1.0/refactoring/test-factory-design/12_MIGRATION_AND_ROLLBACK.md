# 迁移与回滚

## 顺序

1. TF-01 先纠正 S1 身份和语义。
2. TF-02 建独立包但不接 CI。
3. TF-03 分类器通过全部 fixture。
4. TF-04 在固定 SHA 上独立复核候选 RED，补全完整 hash。
5. TF-05 分离 baseline 职责。
6. TF-06 经单独 HUMAN_GATE 后接 CI/发布门禁。
7. TF-07 独立验收后才可申请正式 DAG/Manifest 重签。

每步独立 commit，后一步不得掩盖前一步失败。任何 managed object 在维护后再次变化，先前收据失效。

## 回滚

- TF-01..05 可按单 commit 逆序回退；不得删除历史 RED 或证据。
- TF-06 workflow 与 gate 分开提交，可单独撤回接入而保留独立包和分类器。
- baseline 更新只能由显式 UPDATE 回退到记录的旧 SHA，不能运行 CHECK“恢复”。
- 若迁移映射不完整，保留旧运行入口为只读复现路径，状态为 BLOCKED，不宣布新域接管。

## 停止条件

HEAD 漂移、源码 SHA 不完整、case 身份冲突、失败签名歧义、第三方修改、HUMAN_GATE 缺失、GitNexus/AOCI 要求无法满足且影响正式写入时停止并升级 Owner。

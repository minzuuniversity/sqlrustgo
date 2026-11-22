> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0110-run-20261121T030705.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-10 SPEC — C01-10 契约测试包

## 设计/范围/接口
- 覆盖不变量：**INV-05 / INV-15**（源：docs/releases/v4.1.0/refactoring/TRANSACTION_INVARIANTS.md）
- 范围：仅新增契约测试文件与 Cargo.toml `[[test]]` 声明；**零产品源码修改**。
- 允许路径：`tests/integration/test_factory/ca01_complex_dml.rs`、`docs/releases/v4.1.0/refactoring/evidence/ca01/C01-10`、`Cargo.toml`（契约 allowed_paths 内）
- 禁止路径：`src/`、`crates/*/src/`、`scripts/gate/`、`docs/governance/`、`docs/releases/v4.1.0/STAGE.yaml`、`.gitea/`、`.aoci/`、`.env`、`secrets/`
- 测试名与断言语义以已批准契约为准（**不把当前错误行为固化为正确**）。

## 判据来源（Oracle 独立性）
- 断言依据 = 已批准的不变量文本 + 已裁决 ADR 方向；**模型输出不充当唯一正确性 Oracle**。
- 每个失败均保留原始断言与最小复现，不以删改断言换取绿色。

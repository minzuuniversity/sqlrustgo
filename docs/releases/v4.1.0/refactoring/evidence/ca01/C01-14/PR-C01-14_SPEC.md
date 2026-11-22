> source_agent=DSH-B · source_run=dshb-owner-ruling-r2 · timestamp=2026-11-21T04:35+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0114-run-*.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；Owner 2026-11-21 裁决）

# PR-C01-14 SPEC — C01-14 契约测试包

## 设计/范围/接口
- 覆盖不变量：**INV-02 / INV-04 / INV-13 / INV-15**（TRANSACTION_INVARIANTS.md）
- 范围：仅新增契约测试文件 + Cargo.toml `[[test]]` 声明；**零产品源码修改**。
- 允许路径：`tests/integration/test_factory/ca01_visibility_oracle.rs`、`docs/releases/v4.1.0/refactoring/evidence/ca01/C01-14`、`Cargo.toml`
- 禁止路径：`src/`、`crates/*/src/`、`scripts/gate/`、`docs/governance/`、`docs/releases/v4.1.0/STAGE.yaml`、`.gitea/`、`.aoci/`、`.env`、`secrets/`
- 语义分类（Owner §一）：**Contract Test + Regression Test**（无 Gap Probe）。

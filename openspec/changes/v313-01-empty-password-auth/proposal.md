## Why

V312-21 / ISSUE #3908 的Disposition表中记录了 `empty_password_auth` 为 deferred surface，owner: openclaw，expiry: 2027-06-30。该 fixture 在 V312-21 周期中未实现，是 MySQL 兼容性测试套件的已知缺口。

空密码认证是 MySQL 兼容性的基本场景：开发环境中常见 `mysql -u root`（无密码）连接方式，若该场景未覆盖，则兼容性声明不完整。V313-01 作为 V312-21 的 follow-up，目标是补齐此 fixture，验证服务端对空密码认证的处理行为。

## What Changes

- 在 `tests/compat/mysql_v3_12/` 下创建 `empty_password_auth.sql` fixture，验证空密码连接场景
- 通过 `tools/compat-runner/src/main.rs` 运行 fixture，生成 evidence
- 更新 `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` 中 `empty_password_auth` 行：将 `decision=deferred` 更新为实际运行结果
- 在 `openspec/changes/v313-01-empty-password-auth/specs/` 下创建规格说明文档

## Capabilities

### New Capabilities

- `empty-password-auth-fixture`: SQL fixture 验证 MySQL 客户端使用空密码连接时的认证行为

### Modified Capabilities

- `mysql-compat-surface-disposition`: `empty_password_auth` 行从 deferred 更新为有 evidence 支撑的 PASS 或 unsupported 决策

## Impact

- **新增**: `tests/compat/mysql_v3_12/empty_password_auth.sql`（fixture 文件）
- **新增**: `tests/compat/mysql_v3_12/empty_password_auth.out`（预期输出）
- **修改**: `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` 中 `empty_password_auth` 行
- **无**: 无新增 crate 依赖，无 parser/optimizer 改动，纯测试 + governance
- **风险**: 低——fixture 结果仅为记录，不改变服务端行为；若 fixture 暴露认证问题则开 follow-up issue

## Constraints

- Fixture 必须使用现有 `tools/compat-runner/src/main.rs` 基础设施，不引入新 runner
- Fixture 输出必须与 `.out` 文件精确匹配（无可协商的模糊容差）
- Disposition 行 evidence_hash 必须反映 fixture 运行时的实际 SHA256
- Owner: openclaw，Expiry: 2027-06-30（与 V312-21 deferral 保持一致）

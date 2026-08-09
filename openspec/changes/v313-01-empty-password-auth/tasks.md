## 1. Fixture 创建与验证

- [ ] 1.1 创建 `tests/compat/mysql_v3_12/empty_password_auth.sql` fixture，验证空密码认证场景
- [ ] 1.2 创建 `tests/compat/mysql_v3_12/empty_password_auth.out` 预期输出文件
- [ ] 1.3 验证 fixture 格式符合 runner 规范（`# name:`、`# expect:`、SQL 语句）

## 2. Runner 执行

- [ ] 2.1 在 `tools/compat-runner/` 目录下运行 `cargo run --release` 执行 compat fixture 套件
- [ ] 2.2 确认 `empty_password_auth` fixture 运行结果与 `.out` 文件匹配
- [ ] 2.3 验证 `SURFACE_DISPOSITION.md` 中 `empty_password_auth` 行已更新（evidence_hash、decision）

## 3. Disposition 更新

- [ ] 3.1 确认 `docs/releases/v3.12.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` 中 `empty_password_auth` 行 decision 从 `deferred` 更新为实际运行结果
- [ ] 3.2 确认 evidence_hash 与 fixture 运行日志一致
- [ ] 3.3 若运行结果为 unsupported，记录具体原因和 follow-up issue 链接

## 4. PR 与归档

- [ ] 4.1 在 Gitea 上创建 PR，target `develop/v3.13.0`
- [ ] 4.2 获得 1 位 reviewer 批准
- [ ] 4.3 Force-merge（admin 权限）
- [ ] 4.4 同步到 gitcode + gitee 镜像
- [ ] 4.5 更新 ISSUE #3908 和相关 follow-up issue，附上 PR 链接和 disposition 截图

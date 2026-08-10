# V312-26: E2E 脚本 WARN-only 修复 — tasks

> **Status**: 🔵 OPEN — created 2026-08-09
> **Owner**: minimax
> **Expiry**: 2026-08-30

- [ ] 1.1 重写 `scripts/gate/e2e/e2e_07_json_vector.sh`：删除 `grep -q name` 占位，改 byte-exact JSON+vector fixture 断言
- [ ] 1.2 重写 `tests/e2e/backup_restore.sh`：用 `mysqldump` + `mysql` round-trip，删 `|| true`
- [ ] 1.3 重写 `tests/e2e/sysbench_wired.sh`：`sysbench --version` 预检 + 缺失时 exit 1
- [ ] 1.4 加 `scripts/gate/check_no_warn_only.sh`，扫描 e2e/ 与 gate/e2e/ 下 `|| true` + 静默 if-then-PASS
- [ ] 1.5 失败注入测试：故意把 fixture 移走，确认 3 个脚本退出码 ≠ 0
- [ ] 1.6 写 `docs/releases/v3.12.0/V312-26_warn_only_fix_report.md`，含关闭边界 5 项的实测输出

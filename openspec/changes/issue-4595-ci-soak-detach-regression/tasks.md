# Tasks — issue-4595 CI SOAK detach regression

## T1: CI 集成 (核心)

- [x] T1.1 在 `.github/workflows/ci-pr.yml` 新增 `soak-detach-regression` job
- [x] T1.2 job 调用 `scripts/soak/test_run_soak_loop_detach.sh` (S1 静态检查)
- [x] T1.3 job 调用 `scripts/soak/test_run_soak_loop_tier1.sh` (P-2/P-3/P-4/P-9)
- [x] T1.4 YAML 语法验证通过 (`python3 -c yaml.safe_load`)

## T2: 回归检测验证

- [x] T2.1 本地跑 `test_run_soak_loop_detach.sh` → PASS=8 FAIL=0
- [x] T2.2 本地跑 `test_run_soak_loop_tier1.sh` → PASS=15 FAIL=0
- [x] T2.3 故意删除 setsid 行 → 测试 FAIL (PASS=7 FAIL=1, exit 1) ✅
- [x] T2.4 恢复 setsid 行 → 测试 PASS ✅

## T3: OpenSpec change 记录

- [x] T3.1 创建 `openspec/changes/issue-4595-ci-soak-detach-regression/` 目录
- [x] T3.2 写 proposal.md (Why / What Changes / Impact / Evidence)
- [x] T3.3 写 tasks.md (本文件)

## T4: PR 提交 (待用户确认后)

- [ ] T4.1 创建 feature 分支 `feature/v312-4595-ci-soak-detach-regression`
- [ ] T4.2 commit: `ci(soak): #4595 Tier-1 — add soak-detach-regression job to ci-pr.yml`
- [ ] T4.3 推送 + 创建 PR (关联 Issue #4595)
- [ ] T4.4 PR 合并后关闭 Issue #4595

## 验收标准 (来自 Issue #4595)

- [x] CI workflow 在每个 PR 上跑
- [x] 故意删除 detach 块后, CI 失败 (validation of detection)
- [x] 恢复 detach 块后, CI 通过

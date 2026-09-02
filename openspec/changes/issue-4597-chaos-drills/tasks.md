# Tasks — issue-4597 SOAK chaos drills

## T1: chaos_drills.sh 主体框架

- [x] T1.1 文件头注释 (4 drill 说明 + 用法 + env vars + 退出码 + 依赖)
- [x] T1.2 `set -uo pipefail` + 路径变量 (SCRIPT_DIR / REPO_ROOT / BINARY / CHAOS_PY)
- [x] T1.3 env vars: SOAK_PORT=3397 / SOAK_RESULTS_DIR / SOAK_DATA_DIR / CHAOS_DRILL_TIMEOUT
- [x] T1.4 工具函数: log / pass / fail / skip + PASS/FAIL/SKIP 计数器
- [x] T1.5 `preflight()` 检查 binary + chaos_inject.py
- [x] T1.6 `start_server()` / `stop_server()`
- [x] T1.7 参数解析: 单 drill / --all / --list
- [x] T1.8 退出码语义 (0/1/2)
- [x] T1.9 总结输出 PASS/FAIL/SKIP 计数

## T2: 4 个 chaos drill 实现

- [x] T2.1 `drill_1_sigkill_restart()` — SIGKILL + WAL replay
- [x] T2.2 `drill_2_disk_full()` — fill + ENOSPC + 清理恢复
- [x] T2.3 `drill_3_netem_loss()` — netem 5% loss + err/s<1% 检查
- [x] T2.4 `drill_4_clock_skew()` — date -s + checkpoint 验证
- [x] T2.5 drill-3/4 无 sudo 时 SKIP (不 FAIL)
- [x] T2.6 单个 drill 超时保护 (CHAOS_DRILL_TIMEOUT=120s)

## T3: 静态测试 test_chaos_drills.sh

- [x] T3.1 18 项结构检查 (S1~S6)
- [x] T3.2 本地 → PASS=18 FAIL=0

## T4: CI 集成

- [x] T4.1 ci-pr.yml soak-detach-regression job 新增 chaos drills step
- [x] T4.2 YAML 验证 → valid, 10 jobs

## T5: OpenSpec change 记录

- [x] T5.1 proposal.md
- [x] T5.2 tasks.md (本文件)

## T6: PR 提交

- [x] T6.1 feature 分支 feature/issue-4597-chaos-drills
- [ ] T6.2 commit + 推送 + 创建 PR (关联 Issue #4597)
- [ ] T6.3 PR 合并后关闭 Issue #4597

## 验收标准 (来自 Issue #4597)

- [x] 4 个 chaos drill 实现完整 (SIGKILL+restart / disk-full / netem / clock-skew)
- [x] 每个 drill 可独立运行: `bash scripts/soak/chaos_drills.sh drill-1`
- [x] chaos_drills.log 输出 PASS/FAIL/SKIP 总结
- [x] CI 集成 (静态结构检查 PR CI; 完整 drill Z6G4 nightly)
- [x] drill-3/4 无 sudo 时 SKIP 不 FAIL

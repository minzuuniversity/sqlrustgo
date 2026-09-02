## Why

P-1 (parent-shell reap kills the whole process group) 已在 PR #4591 / commit `66dac1aab2` 修复,
在 `run_soak_loop.sh:86-118` 加入 setsid + nohup + disown self-detach 块。

**残留风险**: 未来某个 PR 如果误删 self-detach 块 (例如移到 preflight() 之后, 或被 refactor 拆掉),
bug 会静默回归 — 因为本地 SOAK 通常从 AI agent Bash tool / nohup / systemd 启动,
父 shell reap 路径很少被触发, 普通开发不会立刻注意到。

需要一个 CI 测试作为回归门禁, 在每个 PR 上自动验证 detach 块仍然存在且行为正确。

来源: [SOAK-INFRA-REVIEW-2026-08-30.md §1 P-1 §6.1](../../docs/releases/v3.12.0/evidence/issue-4560/SOAK-INFRA-REVIEW-2026-08-30.md)
关联: Issue #4595, PR #4591 (commit 66dac1aab2)

## What Changes

- **CI 集成 (核心)**: 在 `.github/workflows/ci-pr.yml` 新增 `soak-detach-regression` job,
  在每个 PR 上并行运行两个已有的静态测试脚本:
  - `scripts/soak/test_run_soak_loop_detach.sh` — S1 静态检查 (8 项): setsid+nohup+disown 三件套,
    SOAK_DETACHED/SOAK_NO_DETACH env guard, `! -t 0` 触发条件, trap cleanup, `</dev/null`, PID 文件
  - `scripts/soak/test_run_soak_loop_tier1.sh` — Tier-1 env override + QPS 解析 (15 项):
    P-2 SOAK_DATA_DIR, P-3 SOAK_NICE_*, P-4 sysbench --time 派生, P-9 server_qps total_q 解析
- **行为测试 (不阻塞 PR)**: `--behavioral` 选项保留给 nightly / Z6G4 runner,
  实际 SIGKILL 父 bash 验证子进程存活 (~30s, 太慢不适合 PR CI)。
- **fail-on-missing**: 静态检查若发现 setsid 行被删除, CI job 失败, 阻止合并。

## Impact

- **回归检测已验证**: 删除 `setsid nohup bash "$0"` 行后, `test_run_soak_loop_detach.sh` 报
  `❌ FAIL: 缺 setsid nohup bash "$0" 命令` (PASS=7 FAIL=1), exit 1 — 符合验收标准。
- **零运行时影响**: 纯 CI 配置 + 已有测试脚本, 不改 Rust 代码, 不改 SOAK 行为。
- **PR CI 时间**: < 5s (静态检查 only), 不阻塞 PR 流水线。
- **Out of scope**:
  - #4596 (workload sweep + JSON reports + disk-restart boundary) — Tier-2, 独立 PR
  - #4597 (chaos drills) — Tier-2, 独立 PR
  - #4598 (alerting webhook + thread-ramp + 168h SOP) — Tier-3, 独立 PR

## Evidence

- 修复 commit: `66dac1aab2` (PR #4591)
- CI 集成 commit: 待 PR 合并后填入
- 静态测试输出 (PASS=8 FAIL=0): `test_run_soak_loop_detach.sh`
- Tier-1 测试输出 (PASS=15 FAIL=0): `test_run_soak_loop_tier1.sh`
- 回归验证 (删 setsid 后 PASS=7 FAIL=1): 验证 fail-on-missing 有效

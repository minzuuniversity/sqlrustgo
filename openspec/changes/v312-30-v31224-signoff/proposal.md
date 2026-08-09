# V312-30: V312-24 PR 关闭 + 签收 — proposal

> **Issue**: V312-30（V312-24 Phase 7.3 + Phase 8 follow-up）
> **Owner**: minimax
> **Expiry**: 2026-09-05
> **前置依赖**: V312-25 ~ V312-29 中任意未完成项不可关闭 V312-24。
> **Baseline evidence**: `docs/releases/v3.12.0/evidence/V312-30_baseline_evidence.txt`

## Why

V312-24 PR 合入后需要：写 ISSUE #3911 评论（含 Phase 1 实际数字 +
5 个 follow-up issue 编号 + 重新计算的 evidence_hash）+ 跑 6 项 gate
+ 2 reviewer 签收。V312-24 本身不写这些动作的边界，由 V312-30 接管。

## 关闭边界（必须全部满足，命令 + 数字 + 文件存在）

1. `gh pr view <PR_NUMBER> --json state,mergedAt` 输出 `state: MERGED` + `mergedAt` 非空。Baseline: PR 未开。
2. ISSUE #3911 评论含：
   - Phase 1 12 tasks + 10 integration tests + 20 lib tests 全 PASS 的实际数字（baseline 已记录，关闭时再跑一次确认）
   - 5 个 V312-25~29 follow-up issue 编号 + 链接
   - **重新计算**的 evidence_hash = `sha256sum <17 files>` 第一个值（**不允许用 baseline hash `956eaf4d...` 顶替**；V312-25~29 会修改 proposal 文件，hash 必须随之更新）
3. Phase 8.1-8.6 全跑通：
   - `cargo test --workspace --no-fail-fast` 退出 0
     （baseline 有 2 pre-existing FAIL 在 `sqlrustgo-mysql-server`，见 `evidence/G2_test_count.txt`；V312-30 owner 必须确认这 2 个 FAIL 仍 non-blocking 或已被 V312-17 修复）
   - `cargo clippy --all-features -- -D warnings` 退出 0
     （baseline 有 2 pre-existing errors 在 `sqlrustgo-storage/binary_storage.rs:147/149`；V312-30 owner 必须确认已合并修复或显式列入豁免清单）
   - `cargo fmt --check` 退出 0
   - `bash scripts/gate/check_anti_fabrication.sh` 退出 0
   - `bash scripts/gate/check_beta_gate.sh` B10_SQLANCER 通过（依赖 V312-29 升级为 `check_fail`）
   - 至少 2 名 reviewer 在 PR 上写了 APPROVED
4. 关闭报告 `docs/releases/v3.12.0/V312-30_signoff_report.md` 含：
   - 6 项 gate 命令的实际输出（不是"通过"二字）
   - reviewer 名单（用户名 + APPROVED 时间戳）
   - evidence_hash（关闭时重算，**非 baseline**）
   - pre-existing 豁免清单（若有 mysql-server / storage FAIL/error 未修）
   - sha256

## 禁止关闭条件

1. 不允许"openspec 标 done"、"报告标题写已完成"、"PR 已合并"作为关闭证据。
2. 不允许用 baseline evidence_hash 顶替关闭时重算的 hash。
3. pre-existing FAIL/error 必须显式列入豁免清单（带 owner + expiry + replacement-gate），不能默默忽略。
4. 不允许 0 reviewer 时关闭 — 至少 2 个 APPROVED。

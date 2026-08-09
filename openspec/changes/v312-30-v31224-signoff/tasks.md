# V312-30: V312-24 PR 关闭 + 签收 — tasks

> **Status**: 🔵 OPEN — created 2026-08-09
> **Owner**: minimax
> **Expiry**: 2026-09-05
> **前置依赖**: V312-25 ~ V312-29 全 CLOSED

- [ ] 1.1 跑 `cargo test --workspace --no-fail-fast` 并保存 log
- [ ] 1.2 跑 `cargo clippy --all-features -- -D warnings` 并保存 log
- [ ] 1.3 跑 `cargo fmt --check` 并保存 log
- [ ] 1.4 跑 `bash scripts/gate/check_anti_fabrication.sh` 并保存 log
- [ ] 1.5 跑 `bash scripts/gate/check_beta_gate.sh` 并确认 B10_SQLANCER 通过
- [ ] 1.6 确认 2 名 reviewer 写 APPROVED
- [ ] 1.7 PR 合入后 `gh pr view` 拿 mergedAt
- [ ] 1.8 写 ISSUE #3911 评论（Phase 1 数字 + 5 follow-up 编号 + evidence_hash）
- [ ] 1.9 写 `docs/releases/v3.12.0/V312-30_signoff_report.md`

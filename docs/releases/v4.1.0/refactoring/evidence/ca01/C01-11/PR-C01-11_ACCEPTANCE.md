> source_agent=DSH-B · source_run=dshb-autonomy-r11-audit · timestamp=2026-11-21T03:50:08+08:00
> evidence_hash: takeover/dsh-b/artifacts/c0111-final-20261121T031756.log
> conflict_resolution: delegation DSH-B-SRG-CALPHA01（条件授权；ADR 集中裁决 2026-11-21）

# PR-C01-11 ACCEPTANCE

## 门禁执行证据
- 5 字段：source_agent=DSH-B；source_run=dshb-autonomy-r11-audit；timestamp=2026-11-21T03:50:08+08:00；evidence_hash=takeover/dsh-b/artifacts/c0111-final-20261121T031756.log；conflict_resolution=delegation DSH-B-SRG-CALPHA01。
- 执行：`cargo test -p sqlrustgo --test ca01_bufferpool` → 退出码 101（3 项 RED 存在）；原始日志见 evidence_hash 指向文件；base_sha=e9ff21fca3。
- 硬门禁影响评估：本包 `[[test]]` 目标仅存在于功能分支；CI 触发分支为 develop/main 等，不含 DSH-B 分支名，故**不影响受保护分支门禁**。若在架构修复前被合并，根包 `cargo test --all-features` 将因这些 RED 变红——合并时机须与实现同批（属主决策）。
- 回滚：git revert 本包 commit 即恢复；无持久化格式/阶段/配置变更。
- Token：执行域本地模型调用全量入账（takeover/control-plane/call_ledger.jsonl），云端 0。

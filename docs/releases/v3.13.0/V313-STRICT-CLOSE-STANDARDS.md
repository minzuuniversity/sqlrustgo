# v3.13 严格关闭标准 (Strict Close V313 Governance)

## §1 PR 合并规则

PR 必须合并到 develop/v3.13.0(主) 或 develop/v3.12.0(维护),merge commit 在 develop 分支 reachable。

## §2 关闭证据四要素

1. **命令**:`bash scripts/gate/<name>.sh` 或 `cargo test ...`
2. **退出码**:0 = PASS,1 = FAIL/BLOCKED,>1 = ERROR
3. **输出摘要**:精炼到 PASS:X/Y · BLOCKERS:Z 格式
4. **证据哈希**:SHA-256 64-char hex(对 stdout 或 log 文件)

## §3 禁用关闭标记

- ❌ `ACCEPTED-WITH-BINDING-MANIFEST (not DONE)`
- ❌ `SUBSTANTIALLY_COMPLETE`
- ❌ `DEFERRED-without-tracking`
- ❌ `NEXT_STEPS` 与 close 同一 PR

## §4 Deferral 强制规则

若需延期:issue 保持 open + 必须绑定到:
- tracking issue(本例 #4313)
- owner(openclaw)
- expiry date(2027-06-30)
- closing boundary(具体可验证标准,例如 "SF=1 fixture 实测 PASS")

## §5 Anti-Fabrication-Policy-v1.0

任何 gate 失败必须诚实披露。伪造 PASS 或省略 FAIL = policy 违反,自动 invalidate closure。

## Evidence Hash

`sha256=114f12b123c3d64af74e6516e1ddfa6d62d565382b2c8fa276853148834a5eb6` (computed on file content at HEAD)

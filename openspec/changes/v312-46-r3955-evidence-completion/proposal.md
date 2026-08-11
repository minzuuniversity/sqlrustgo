## Why

#3955 当前的 issue body (Issue Description) 缺少 #3887 严格关闭硬性条件要求的字段:

- ❌ 缺 `evidence_hash` (实际命令输出的 sha256)
- ❌ 缺 `log` 路径 (实跑日志的 filesystem 路径)
- ❌ 缺 #3887 总控清单勾选状态更新

虽然 #3955 已 closed, 但 #3887 严格复核要求 **每次关闭必须附完整证据链**, 后续 #3906 / #3900 / #3972 都已遵循此模式, #3955 不可遗漏。

## What Changes

* **#3955 issue body** (post-close edit): 补充 5 个缺失字段:
  1. `source_agent` / `source_run` / `timestamp` (已经有 agent/source_run/timestamp)
  2. `commit SHA` (已经有 `79289593c142d4f3c474e9bccaf137a05d0529ff`)
  3. `命令` 完整列表 (已经有部分)
  4. **`PASS/FAIL 摘要`** (已经有部分)
  5. **`evidence_hash`** ❌ (实跑 R2.7 / corpus runner 的 sha256 缺失)
  6. **`log 路径`** ❌ (实跑日志的 path 缺失)
  7. **`#3887 总控勾选状态`** ❌ (需新增 comment 说明已更新 #3887)

* **#3887 issue 新增 comment** (per #3887 condition #7): 报告 #3955 关闭时已同步勾选, evidence 已完整化。

## Capabilities

### Modified Capabilities

- `sql-corpus-all-targets-report`: issue body 添加完整证据链字段
- `arch-invariant-r2-unified-report`: #3955 evidence 升级到 #3887 标准

## Impact

- **Modified**: Issue #3955 body (post-close edit, 不影响 code)
- **New**: #3887 issue comment
- **Affected artifacts**: 无 (只是文档完整性提升)

## Acceptance criteria

- #3955 body 含所有 7 个 #3887 关闭硬性条件字段
- #3887 收到 #3955 evidence 完成通知 comment
- evidence_hash 字段是 4 个实测命令的 sha256 (R2.7 / corpus / wire / mysql_compat)
- log 路径指向实际可访问的 log 文件

## Risk

极低。Issue body 编辑是 documentation-only, 不影响代码或 behavior。

## Out of scope

- #3955 实际代码已 merged, 本次只补 evidence 文档
- 重新打开 #3955 不必要 (issue 本身已 closed, 只是 evidence 不完整)
- 重新运行 R2.7 / corpus runner 不必要 (已有当时的日志)

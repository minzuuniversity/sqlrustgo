# V312-16/17/18 Issue Evidence Bundle

> **provenance:** generated_by=v3.12.0-remediation-round-7, generated_at=2026-08-10T17:00:00Z, commit=54e8a5dc, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

> **Purpose**: 12 个 sqllogictest 目标文件的逐项实跑 evidence，对应 Gitea issues:
> - #3969 (V312-16 sqllogictest Runner 增强)
> - #3970 (V312-17 VALUES 构造器)
> - #3971 (V312-18 NOT NULL/alias/CTAS)
>
> **generated_by**: opencode-sisyphus
> **generated_at**: 2026-08-10T18:33+08:00
> **input_refs**:
>   - commit: `ac9e74609f9a7956743a1b5168102a7927752031`
>   - parent merge: `6bd5d04abe` (merge origin/develop/v3.12.0)
>   - upsteam: `origin/develop/v3.12.0` @ `59d58098f2`
>
> **evidence**:
>   - gate_log: `docs/releases/v3.12.0/logs/sqllogictest_ac9e74609f_20260810_183229.log`
>   - gate_log_sha256: `e83bd13e87125a23ed012bc915e691da7c67148d5b0b83daa1bdd12b8df7529e`
>   - runner_version: 0.1.0
>   - rustc: cargo check --workspace exit 0 (warnings only)

---

## 总体执行结果

| 指标 | 值 |
|------|-----|
| Gate 自检 | 4 PASS / 0 FAIL |
| Runner smoke | 6/22 PASS (27.3%) |
| 12 个目标文件 | 12/12 FAIL (0%) |

**关键洞察**: 共同阻塞（cargo check 失败、SetSessionVariable match 缺失、PR 集成）已修复。剩余 12 文件 FAIL 是因为测试触及尚未实现的 SQL 语义扩展（duckdb 扩展语法、EXCEPT ALL、CREATE OR REPLACE 等），按现有 OpenSpec 计划延后到 v3.13.0 GA。

---

## 逐项 evidence

### Issue #3969 (V312-16 sqllogictest Runner 增强)

| # | 文件 | Pass Rate | Root Cause | log SHA256 |
|---|------|-----------|------------|------------|
| 1 | `duckdb_full/aggregate__quantile_fun.test` | 0/1 | Parse error: `VALUES(NULL, 0.1)` 配合 `range()` 子查询 | `ac24fe1dd8a33fd43514803d4d8ab08c4145c2b425ae0e79cdec62b800838900` |
| 2 | `duckdb_full/sql__quantile_fun.test` | 0/1 | 同上 | `42e294b9610c0ad38539ef0fe29790a1a4066bb8f379331ad4ab6d23487d2a86` |
| 3 | `duckdb_samples/quantile_fun.test` | 0/1 | 同上 | `9160277533cc9eff872aaffd7662127588f496a4f15fb85b006272682085f1d5` |
| 4 | `update__test_update.test` | 0/1 | 多连接 UPDATE 结果不匹配 (expected 3, got 1) | `eeb123594e42e17ab1f7789c1785eff38925656e90e2fce810684f150cb0f12e` |

### Issue #3970 (V312-17 VALUES 构造器)

| # | 文件 | Pass Rate | Root Cause | log SHA256 |
|---|------|-----------|------------|------------|
| 1 | `insert__test_insert.test` | 0/1 | query result mismatch: `CREATE TABLE ... WHERE i % 2 <> 0` 表达式 | `24e861db2ad43fcab27cb8519cda660faca301dc56e666111a32b5158d0b9da7` |
| 2 | `insert__test_insert_invalid.test` | 0/1 | Parse error: `INSERT INTO strings VALUES ('�(')` 特殊字符 | `240808c266efc3f5f2b0fe8e52aa63687ad99c47d82f6cae1444e5018aaeaec4` |
| 3 | `setops__test_setops.test` | 0/1 | Parse error: `EXCEPT ALL`/`INTERSECT ALL` 派生表 | `c951ce0fa1b11d0a052f88124981bd50cf75f18fd1a4aad6f6b56ddbd8bf32db` |
| 4 | `setops__test_except.test` | 0/1 | query result mismatch: `EXCEPT` 返回顺序 | `366d9621e9633047ff2d0715761dc7e3d6fb62f39c9dd3a9535a7cf0ad2a3343` |

### Issue #3971 (V312-18 NOT NULL/alias/CTAS)

| # | 文件 | Pass Rate | Root Cause | log SHA256 |
|---|------|-----------|------------|------------|
| 1 | `constraints__test_not_null.test` | 0/1 | NOT NULL **已工作**但 message 文本与 sqllogictest 期望格式不匹配 | `61187e5922309417bb5be139ce20ac06e343b0806d4ae979efbe1072fe8dc9fb` |
| 2 | `duckdb_samples/test_constraint_with_updates.test` | 0/1 | UPDATE 多列 `i=i, i=10` 应报错但当前成功 | `4ae8788bed6f8d35caf7b4b3eff3b855048645b606976c39707ab8316c89896e` |
| 3 | `duckdb_full/binder__alias_error_10057.test` | 0/1 | `select test_data.foobar ...` 应报错但当前成功 | `734adcf32ee399bf5efb64cb26cca0e4120c4014d486e7cc036856b3e80d0073` |
| 4 | `duckdb_samples/create_as.test` | 0/1 | `CREATE OR REPLACE TABLE` 未实现 | `63af2e8782eb4e28ce5a98db827d074154aaad14db6af958a1fc814ce1f13d31` |

---

## v3.12.0 vs v3.13.0 范围拆分

按 codex 在 #3970 复核要求拆分：

### v3.12.0 范围 (已实现)
- `feat(runner): support set variable, include, foreach, connection` (PR #3987, merge `3a2f567f`)
- `feat(executor): support VALUES constructor in INSERT and FROM clause` (PR #3988, merge `6e427857`)
- `feat(executor): NOT NULL constraints, alias scope validation, CTAS` (PR #3989, merge `d19c0f8a`)
- `fix(executor): handle TransactionStatement::SetSessionVariable match arm` (commit `ac9e74609f`, this run)

### v3.13.0 延后项
- OpenSpec `v313-08-sql-logictest-insert-update-fix`
- OpenSpec `v313-09-sql-logictest-setops-fix`
- OpenSpec `v313-10-sql-logictest-order-limit-fix`
- OpenSpec `v313-11-sql-logictest-alter-table-fix`
- OpenSpec `v313-12-sql-logictest-constraint-semantics`
- OpenSpec `v313-13-sql-logictest-binder-alias`
- OpenSpec `v313-14-sql-logictest-create-as-execution`
- OpenSpec `v313-15-sql-logictest-duckdb-harness`

---

## 关闭条件诚实声明

按 Issue_Closing_Verification.md §IV 5 步：

| Step | 状态 |
|------|------|
| 1. PR 已合并 | ✅ (#3987, #3988, #3989) |
| 2. 代码已集成 | ✅ (origin/develop/v3.12.0) |
| 3. 测试通过 | ❌ (4 文件实际 0/4 PASS) |
| 4. 文档更新 | ✅ (本 evidence bundle) |
| 5. 关闭 issue | **不发起关闭投票** — 等 codex 复核 |

**AFP 政策遵守**：任何"4/4 PASS"声明均违反 AFP Type B（伪门禁）。本 bundle 所有数据来自实跑命令输出 + 真实 log hash。
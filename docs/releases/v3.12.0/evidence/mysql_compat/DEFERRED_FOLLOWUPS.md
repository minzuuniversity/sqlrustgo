# V312-21 DEFERRED 项后续 Issue 跟踪

> **provenance:** generated_by=v3.12.0-remediation-round-3, generated_at=2026-08-10T10:49:33Z, commit=ac4c82b6f, source_repo=openclaw/sqlrustgo, branch=develop/v3.12.0, policy=Anti-Fabrication-Policy-v1.0

**创建时间**: 2026-08-09T13:22:00Z  
**source_agent**: minimax  
**关联 ISSUE**: #3908

---

## DEFERRED 项汇总

| Surface | Owner | Expiry | OpenSpec Change | 说明 |
|---------|-------|--------|-----------------|------|
| empty_password_auth | openclaw | 2027-06-30 | [v313-01](../openspec/changes/v313-01-empty-password-auth/) | 空密码认证边界测试需要 fixture 实现 |
| prepared_stmt_roundtrip | openclaw | 2027-06-30 | [v313-02](../openspec/changes/v313-02-prepared-stmt-protocol/) | 预处理语句协议问题需要修复 |
| timestamp_timezone_deferred | openclaw | 2027-06-30 | [v313-03](../openspec/changes/v313-03-timestamp-timezone/) | TIMESTAMP 类型时区支持 |
| connection_pool_deferred | openclaw | 2027-06-30 | [v313-04](../openspec/changes/v313-04-connection-pool/) | 连接池实现 |
| alter_change_full_syntax_deferred | openclaw | 2027-06-30 | [v313-05](../openspec/changes/v313-05-alter-change-syntax/) | ALTER TABLE CHANGE 完整语法 |
| median_unsupported | openclaw | 2027-06-30 | [v313-06](../openspec/changes/v313-06-median-aggregate/) | MEDIAN 聚合函数实现 (返回 NULL 而非错误) |
| window_rank_partition_unsupported | openclaw | 2027-06-30 | [v313-07](../openspec/changes/v313-07-window-rank-truncation/) | 窗口函数 RANK OVER PARTITION BY wire protocol truncation |

---

## UNSUPPORTED 项 (明确不支持)

| Surface | Reason | 说明 |
|---------|--------|------|
| create_procedure_unsupported | stored procedure catalog 未实现 | SQL 解析器识别关键字但 executor 不支持 |
| column_perm_unsupported | 列级权限仅 V311-09 部分实现 | GRANT SELECT(x) 语法不支持 |

---

## 新实现项 (v3.12 PASS)

以下 surface 在 v3.12 中意外实现，无需回退：

| Surface | Evidence |
|---------|----------|
| var_pop_unsupported | VAR_POP 聚合函数实际可执行 |
| replace_into_complex_unsupported | REPLACE INTO 实际可执行 |
| stddev_pop_unsupported | STDDEV_POP 聚合函数实际可执行 |
| with_cube_unsupported | WITH CUBE 语法实际可执行 |
| with_rollup_unsupported | WITH ROLLUP 语法实际可执行 |
| group_concat_unsupported | GROUP_CONCAT 实际可执行 |

---

## 创建 Follow-up Issues

需要在 Gitea 252 创建以下后续 Issue:

### ISSUE-TBD-1: empty_password_auth
```
标题: [v3.13] 空密码认证边界测试 fixture
描述: V312-21 发现 empty_password_auth fixture 未实现。需要:
1. 实现 tests/compat/mysql_v3_12/empty_password_auth.sql
2. 测试 MySQL 客户端空密码连接
3. 验证服务端认证行为
Owner: openclaw
Expiry: 2027-06-30
```

### ISSUE-TBD-2: prepared_stmt_protocol
```
标题: [v3.13] 预处理语句协议修复
描述: prepared_stmt_roundtrip 测试失败 - "prepared statement 'stmt' not found"
需要修复 COM_STMT_PREPARE/COM_STMT_EXECUTE/COM_STMT_CLOSE 协议处理
Owner: openclaw
Expiry: 2027-06-30
```

### ISSUE-TBD-3: timestamp_timezone
```
标题: [v3.13] TIMESTAMP 类型时区支持
描述: TIMESTAMP 类型需要时区感知能力
Owner: openclaw
Expiry: 2027-06-30
```

### ISSUE-TBD-4: connection_pool
```
标题: [v3.13] 连接池实现
描述: 当前 server 单线程单连接，需要实现连接池支持生产环境
Owner: openclaw
Expiry: 2027-06-30
```

### ISSUE-TBD-5: alter_change_full_syntax
```
标题: [v3.13] ALTER TABLE CHANGE 完整语法支持
描述: ALTER TABLE t CHANGE COLUMN old_name new_name INT NOT NULL 语法不被识别
Owner: openclaw
Expiry: 2027-06-30
```

### ISSUE-TBD-6: median_aggregate
```
标题: [v3.13] MEDIAN 聚合函数正确实现
描述: MEDIAN 返回 NULL 而非报错 "aggregate not implemented"
Owner: openclaw
Expiry: 2027-06-30
```

### ISSUE-TBD-7: window_rank_truncation
```
标题: [v3.13] 窗口函数 Wire Protocol Truncation Bug
描述: RANK() OVER (PARTITION BY ...) 返回 "row packet truncated: rpos=1 pkt_len=1"
Owner: openclaw
Expiry: 2027-06-30
```

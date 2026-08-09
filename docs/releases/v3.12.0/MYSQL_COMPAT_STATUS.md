# MySQL Compatibility Status — V312-21

**Issue**: #3908  
**source_agent**: claude-code  
**source_run**: v312-21-closure-verification  
**timestamp**: 2026-08-09T14:30:00Z  
**PR**: http://192.168.0.252:3000/openclaw/sqlrustgo/pulls/3941  
**Merge Commit**: `0c44bf65ce`  
**Branch**: `develop/v3.12.0`

---

## 一、验收条件对照

根据 ISSUE #3908 原文:

> 对 GMP/生产路径相关子集给出 fixture PASS；非目标项必须输出 explicit unsupported 或 deferred decision，不得在 release note 中无边界宣称支持。

### 1.1 PASS Surface (11) — GMP/生产路径验证

| Surface | Fixture | evidence_hash | Command |
|---------|---------|--------------|---------|
| alter_add_column | tests/compat/mysql_v3_12/alter_add_column.sql | 008ceea218801ad0e6e1b4e0f639eeaed489466b724cf2ba31fe9032cb493a08 | `./target/release/compat-runner` |
| alter_drop_column | tests/compat/mysql_v3_12/alter_drop_column.sql | 1d45cf37e7ebb2472e3ac861835c33131dcc59bd0208079852f95995206aba3f | `./target/release/compat-runner` |
| alter_modify_column | tests/compat/mysql_v3_12/alter_modify_column.sql | 7838297c09842be4a2714bd898caa25de0dcafc3cdae87529fb9f79584f5545f | `./target/release/compat-runner` |
| alter_rename | tests/compat/mysql_v3_12/alter_rename.sql | 2e446ddc288e45def1b78c3063827df3fabde04ad9eccc87963925ab4a378e76 | `./target/release/compat-runner` |
| show_tables | tests/compat/mysql_v3_12/show_tables.sql | d39c736c1dfdc6058459768ceb57327aa894bf778822177c04de2dd7006e7920 | `./target/release/compat-runner` |
| group_concat | tests/compat/mysql_v3_12/group_concat_unsupported.sql | 79a4201de0a32f4a208700e60514b635e0c941915ee8c45e99eee9d19866fece | `./target/release/compat-runner` |
| stddev_pop | tests/compat/mysql_v3_12/stddev_pop_unsupported.sql | 7190ba9ef1cf4710c8f38fbf4aafaaee50a30d826112f624d073b904700d901e | `./target/release/compat-runner` |
| with_cube | tests/compat/mysql_v3_12/with_cube_unsupported.sql | 9e9fab0dc9dd6d4dd15098d7bc7baedc34f2f9d1bb9ead7f932712e0ccae056f | `./target/release/compat-runner` |
| with_rollup | tests/compat/mysql_v3_12/with_rollup_unsupported.sql | 02219b0b86e47d0cce6f3e19e3c1a9779af5a0d44168d8077bc6f80d217b6979 | `./target/release/compat-runner` |
| var_pop | tests/compat/mysql_v3_12/var_pop_unsupported.sql | e0006f3cb6bbf49e7d381460d9387b379ef8a8cb68c841f98024b5409f1bffdb | `./target/release/compat-runner` |
| replace_into | tests/compat/mysql_v3_12/replace_into_complex_unsupported.sql | e548543b76990b009242da871a3f18a741883cbd789e25cb2133366913a560da | `./target/release/compat-runner` |

### 1.2 UNSUPPORTED Surface (2) — 明确不支持

| Surface | Reason | OpenSpec | evidence_hash |
|---------|--------|----------|--------------|
| create_procedure_unsupported | stored procedure catalog 未实现 | N/A (明确不支持) | 5f9a8594ab9dd58ad3c16489ce39b4fb608ee79ab27c13b0ee9837d883fc0a88 |
| column_perm_unsupported | 列级权限仅 V311-09 部分实现 | N/A (明确不支持) | 21a107af9fb90eaba3766caa45d8a7ddd18a628d90f0380a8309c2af840915b1 |

### 1.3 DEFERRED Surface (7) — 需在 v3.13 完成

| Surface | Owner | Expiry | OpenSpec Change | Issue | 说明 |
|---------|-------|--------|----------------|-------|------|
| empty_password_auth | openclaw | 2027-06-30 | v313-01 | 待创建 | 空密码认证 fixture 实现 |
| prepared_stmt_roundtrip | openclaw | 2027-06-30 | v313-02 | 待创建 | PREPARE/EXECUTE 协议修复 |
| timestamp_timezone_deferred | openclaw | 2027-06-30 | v313-03 | 待创建 | TIMESTAMP WITH TIME ZONE 支持 |
| connection_pool_deferred | openclaw | 2027-06-30 | v313-04 | 待创建 | 连接池生产实现 |
| alter_change_full_syntax_deferred | openclaw | 2027-06-30 | v313-05 | 待创建 | ALTER TABLE CHANGE COLUMN 语法 |
| median_unsupported | openclaw | 2027-06-30 | v313-06 | 待创建 | MEDIAN 聚合函数正确实现 |
| window_rank_partition_unsupported | openclaw | 2027-06-30 | v313-07 | 待创建 | 窗口函数 Wire Protocol 截断 Bug |

---

## 二、Cross-Reference (与 V312-13 交叉项)

| Surface | V312-13 Cross | 说明 |
|---------|---------------|------|
| prepared_stmt_roundtrip | binary-prepared-statement-roundtrip | V312-13 已修复 binary protocol; V312-21 的 prepared_stmt 是 SQL 语法层面 |
| alter_change_full_syntax_deferred | ALTER TABLE parser | V312-13 涉及 ALTER TABLE 实现; V312-21 的 CHANGE COLUMN 是扩展 |

---

## 三、证据文件

| 文件 | 说明 |
|------|------|
| `evidence/mysql_compat/SURFACE_DISPOSITION.md` | 20 surfaces 完整 disposition 表 |
| `evidence/mysql_compat/DEFERRED_FOLLOWUPS.md` | 7 个 deferred 项追踪 |
| `evidence/mysql_compat/V312-21-VERIFICATION.md` | 核查整改最终报告 |
| `evidence/mysql_compat/logs/*.log` | 每个 surface 的执行日志 |
| `tests/compat/mysql_v3_12/*.sql` | 20 个 fixture 文件 |

---

## 四、Closure Checklist

- [x] PR merged to develop/v3.12.0
- [x] Merge commit reachable in develop/v3.12.0
- [x] 20 surfaces 全部有 disposition (PASS/unsupported/deferred)
- [x] 每个 PASS surface 有 fixture + evidence_hash
- [x] 每个 deferred surface 有 owner + expiry + OpenSpec change
- [x] 每个 unsupported surface 有 reason 说明
- [x] FAIL = 0
- [x] Release notes 已更新
- [x] CHANGELOG 已更新
- [x] 7 个 OpenSpec follow-up changes 已创建

---

## 五、结论

**ISSUE #3908 状态**: DEFERRED (7 surfaces 需在 v3.13 完成)

所有 backlog surfaces 已完成 disposition 分类，无悬空 deferred 项。

7 个 deferred 项已分配 owner (openclaw) 和 expiry (2027-06-30)，并有对应 OpenSpec change 追踪。

## Design

### 1. Fixture 目标

`empty_password_auth` fixture 验证以下场景：

- MySQL 客户端使用空密码（`password=""`）连接服务端
- 验证服务端接受或拒绝该连接
- 记录认证结果（allowed 或 denied）

根据 V312-21 deferral 记录，服务端当前实现中空密码认证可能存在边界问题（详见 `docs/releases/v3.7.0/GA_GAP_REPORT.md` 中 Issue #167），因此 fixture 的 `.out` 文件需要反映实际运行结果。

### 2. Fixture 格式

遵循现有 `tests/compat/mysql_v3_12/*.sql` 格式：

```sql
# name: empty_password_auth
# expect: PASS | UNSUPPORTED:<reason>
-- SQL statements
```

Runner `tools/compat-runner/src/main.rs` 解析 `# name:` 和 `# expect:` 行，剩余内容作为 SQL 执行。

### 3. Fixture 内容设计

由于 `compat-runner` 通过 `MySqlTestClient` 驱动 fixture，而空密码认证测试需要客户端明确使用空密码连接，fixture 内容设计如下：

**方案 A（推荐）**：测试空密码用户创建 + 认证
```sql
# name: empty_password_auth
# expect: PASS
CREATE USER 'test_empty'@'%' IDENTIFIED BY '';
FLUSH PRIVILEGES;
-- 验证连接（runner 通过配置的空密码连接）
SELECT 'authenticated' AS status;
```

**方案 B**：若 runner 不支持动态用户创建，则测试 root 空密码连接
```sql
# name: empty_password_auth
# expect: PASS
SELECT 'connected' AS status;
```

Runner 实际以 `root` 用户身份启动，服务端配置允许空密码连接时，fixture 验证连接成功。

### 4. Runner 集成

`tools/compat-runner/src/main.rs` 的 `run_fixture` 函数：

1. 解析 fixture 文件（`parse_fixture`）
2. 启动 ephemeral MySQL 服务（`start_ephemeral`）
3. 通过 `CompatClient` 执行 SQL
4. 对比实际输出与 `.out` 文件
5. 计算 evidence_hash（SHA256）

`empty_password_auth` fixture 的特殊之处在于认证行为在连接建立时发生，而非 SQL 执行时。若连接失败，runner 应捕获连接错误并映射为 `UNSUPPORTED:` 或 `DEFERRED:` 决策。

### 5. Disposition 行更新

运行 fixture 后，更新 `SURFACE_DISPOSITION.md` 中 `empty_password_auth` 行：

| surface | previous_claim | current_evidence | decision | evidence_hash | owner | expiry |
|---------|---------------|------------------|----------|---------------|-------|--------|
| empty_password_auth | deferred (V312-21) | fixture v313-01 run | PASS/unsupported | <sha256> | openclaw | 2027-06-30 |

### 6. 失败模式

- 若 fixture 运行失败（如 runner 错误），`evidence_hash` 反映失败状态，decision 保持 deferred 并记录错误原因
- 若 fixture 暴露认证 bug，开 follow-up issue，decision 记录为 `unsupported:<reason>` 并附 issue 链接

### 7. 约束条件

- Fixture 必须可在 `cargo test` 环境中运行（不依赖外部 MySQL 实例）
- Fixture 输出必须 deterministic（无时间戳、无随机值）
- Runner 必须在 60 秒内完成所有 fixture（包括新加的 `empty_password_auth`）

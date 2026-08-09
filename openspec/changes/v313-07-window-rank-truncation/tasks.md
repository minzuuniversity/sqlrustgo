# V313-07 Tasks: 修复窗口函数 Wire Protocol 截断 Bug

## 1. 定位 Wire Protocol 截断 Bug

- [ ] 1.1 在 `crates/mysql-server/src/` 中搜索 `encode_rows`、`write_row`、`ResultSet` 相关实现，确认结果集序列化代码位置
- [ ] 1.2 在 `crates/mysql-server/src/` 中搜索 `length_encoded`、`write_length`、`lenenc` 相关实现，确认长度编码逻辑
- [ ] 1.3 读取 `client.rs` 中错误信息 `"row packet truncated"` 的来源，定位截断检测点
- [ ] 1.4 确认错误 `rpos=1 pkt_len=1` 的含义：数据包仅 1 字节（0xFB NULL 标记），无实际数据

## 2. 分析 Bug 根因

- [ ] 2.1 在 `encode_rows` 或 `write_row` 入口处添加临时日志，输出每个列值的类型和十六进制编码
- [ ] 2.2 运行 `SELECT 1 AS x` 验证基础整数编码正常
- [ ] 2.3 运行 `SELECT RANK() OVER () FROM t`（无 PARTITION BY）验证单列窗口函数编码正常
- [ ] 2.4 运行 `SELECT a, b, RANK() OVER (PARTITION BY a, b ORDER BY c) FROM t`（当前 Bug 触发点），观察编码日志
- [ ] 2.5 确认截断发生在哪一列（a 列、b 列、c 列、还是 rank 列）

## 3. 修复 Packet 序列化截断逻辑

- [ ] 3.1 根据分析结果，修复 `encode_rows` 或等价方法中的窗口函数结果编码逻辑
- [ ] 3.2 修复后再次运行步骤 2.4，确认数据包长度正常（不再 `pkt_len=1`）
- [ ] 3.3 运行 `cargo test -p sqlrustgo-executor window` 确认窗口函数执行器无回归
- [ ] 3.4 运行 `cargo test --all-features` 确认无通用回归

## 4. 创建新增 Fixture

- [ ] 4.1 新建 `tests/compat/mysql_v3_13/window_rank_partition_fix.sql`：
  - 包含 `PARTITION BY` 单列和多列场景
  - 包含 `RANK()`、`DENSE_RANK()`、`ROW_NUMBER()` 三种窗口函数
  - 每条 SELECT 后跟 `ORDER BY` 确保结果确定性
- [ ] 4.2 新建 `tests/compat/mysql_v3_13/window_rank_partition_fix.out`：
  - 记录每条 SELECT 的期望输出（列头 + 数据行）
- [ ] 4.3 新建 `tests/compat/mysql_v3_13/window_rank_basic.sql`：
  - 包含无 PARTITION BY 的基础窗口函数测试
  - 包含 `VARCHAR` 列和 `INT` 列混合场景
- [ ] 4.4 新建 `tests/compat/mysql_v3_13/window_rank_basic.out`：
  - 记录期望输出
- [ ] 4.5 将 `tests/compat/mysql_v3_13/` 路径加入 compat-runner 扫描范围（若尚未支持 v3_13 目录）

## 5. 更新 V312-21 Deferred Fixture

- [ ] 5.1 修改 `tests/compat/mysql_v3_12/window_rank_partition_unsupported.sql`：
  - 将 `# expect: DEFERRED: wire protocol truncation bug; see ISSUE #39XX` 改为 `# expect: PASS`
  - 保留原 SQL 语句（CREATE + INSERT + SELECT）
- [ ] 5.2 新建或更新 `tests/compat/mysql_v3_12/window_rank_partition_unsupported.out`：
  - 记录期望输出（`RANK() OVER (...)` 应返回 `1`）
- [ ] 5.3 删除旧的 deferred 日志文件（若存在 `logs/window_rank_partition_unsupported.log`）

## 6. 更新 SURFACE_DISPOSITION.md

- [ ] 6.1 若 `docs/releases/v3.13.0/evidence/mysql_compat/SURFACE_DISPOSITION.md` 存在，更新 `window_rank_partition` 行：
  - `decision`：从 `deferred` 改为 `PASS`
  - `evidence_hash`：更新为新的 SHA256
  - `owner`：保持 `openclaw`
  - `expiry`：移除（已解决）
- [ ] 6.2 若文件不存在：在 `docs/releases/v3.13.0/evidence/mysql_compat/` 下创建（参考 V312-21 格式）

## 7. 运行 Compat Runner 验证

- [ ] 7.1 构建 compat-runner：`cargo build -p compat-runner`
- [ ] 7.2 运行完整兼容测试：`./scripts/gate/run_compat_tests.sh`（或等效命令）
- [ ] 7.3 确认 `window_rank_partition_unsupported` fixture PASS
- [ ] 7.4 确认新增 `window_rank_partition_fix` 和 `window_rank_basic` fixture PASS
- [ ] 7.5 运行 `cargo clippy --all-features -- -D warnings` 确认 lint 通过
- [ ] 7.6 运行 `cargo fmt --check` 确认代码格式正确

## 8. PR 与合并

- [ ] 8.1 提交所有变更到特性分支
- [ ] 8.2 打开 PR 指向 `develop/v3.13.0`
- [ ] 8.3 获得至少 1 个 reviewer 批准
- [ ] 8.4 合并到 develop/v3.13.0
- [ ] 8.5 更新 ISSUE #39XX（V312-21 deferred issue），记录 PR 链接

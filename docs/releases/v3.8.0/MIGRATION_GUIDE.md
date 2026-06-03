SQLRustGo v3.8.0 - 迁移指南
============================================================

> 目标读者：正在使用 v3.7.0 / v3.6.0 / v3.5.0 的工程师、DBA、运维。
> 阅读时间：15 - 30 分钟。
> 范围：v3.7.0 → v3.8.0 升级路径，breaking change、数据迁移、配置迁移、
> 兼容性矩阵、已知问题 + workaround。
> 前置阅读：INSTALL.md、DEPLOYMENT_GUIDE.md。
> 状态：v3.8.0 当前为 Alpha；GA 阶段会重审本文档。


目录
------------------------------------------------------------

  1. TL;DR（30 秒看完）
  2. 升级路径概览
  3. Breaking Changes（重点）
  4. 数据迁移
  5. 配置迁移
  6. 客户端/驱动迁移
  7. 兼容性矩阵
  8. 已知问题 + Workaround
  9. 回滚方案
 10. 升级 Checklist
 11. 引用
 12. 已知空白


1. TL;DR（30 秒看完）
------------------------------------------------------------

v3.8.0 = "Architecture Unification Release"
（详见 `CHANGELOG.md` §[3.8.0]）

最重要的 3 件事：

  1) 5 个旧 binary → 1 个 canonical binary `sqlrustgo-mysql-server`。
  2) 双执行路径被合并为单路径（SQL → AST → Plan → Execution）。
  3) WAL 强制开启，AUTOCOMMIT 默认，TX Lifecycle 强制。

如果你当前线上跑的是 v3.7.0，启动命令大概率要从

    sqlrustgo                  →  sqlrustgo-mysql-server serve
    sqlrustgo-sql-cli          →  sqlrustgo-mysql-server exec "..."
    sqlrustgo-bench            →  sqlrustgo-mysql-server bench
    sqlrustgo-bench-cli        →  sqlrustgo-mysql-server bench
    sqlrustgo-tools            →  sqlrustgo-mysql-server diag

详见 §3。


2. 升级路径概览
------------------------------------------------------------

升级路径（官方支持）：

    v3.5.0  ──►  v3.6.0  ──►  v3.7.0  ──►  v3.8.0
      (GA)       (GA)       (GA)       (Alpha)

不建议跨大版本直跳（如 v3.5.0 直接到 v3.8.0），原因：

  * 中间版本有中间修复的 schema/配置项
  * 回滚复杂
  * 验证矩阵不完整（**待 v3.8.0 GA 补齐跨版本验证数据**）

标准升级流程：

  +-----------------------+----------------------------------------+
  | 步骤                  | 动作                                   |
  +=======================+========================================+
  | 0. 备份               | 数据目录 + WAL + checkpoint 全部打包   |
  | 1. 读 CHANGELOG       | v3.7.x → v3.8.0 的全部 Alpha 阶段 PR   |
  | 2. 演练               | staging 跑完整 smoke + 业务回归        |
  | 3. 切流               | 灰度 5% → 25% → 100%                  |
  | 4. 监控               | §4/§8 关注关键指标                     |
  | 5. 观察窗口           | 至少 24h（生产请按各自 SLA 决定）       |
  +-----------------------+----------------------------------------+


3. Breaking Changes（重点）
------------------------------------------------------------

3.1 单一 Canonical Binary
............................................................

v3.8.0 起 `sqlrustgo-mysql-server` 是唯一执行入口
（CHANGELOG.md §[3.8.0] "Canonical Binary Consolidation"）。

旧 binary 行为：打印 deprecation 提示并指向 canonical entry。

迁移表：

  +---------------------+--------------------------------------+--------------------+
  | 旧 binary (≤v3.7)   | 等价 v3.8.0 命令                     | 备注               |
  +=====================+======================================+====================+
  | sqlrustgo           | sqlrustgo-mysql-server serve         | 默认端口 3306      |
  | sqlrustgo-sql-cli   | sqlrustgo-mysql-server exec "<sql>"  | 单次执行           |
  | sqlrustgo-bench     | sqlrustgo-mysql-server bench         | v3.8.0 placeholder |
  | sqlrustgo-bench-cli | sqlrustgo-mysql-server bench         | 同上               |
  | sqlrustgo-tools     | sqlrustgo-mysql-server diag          | v3.8.0 placeholder |
  +---------------------+--------------------------------------+--------------------+

3.2 Execution Semantics 冻结
............................................................

v3.8.0 起（commit 087bb12d）：

  * AUTOCOMMIT 强制为 ON（不接受 `SET AUTOCOMMIT=0` 的旧 wire 路径）
  * WAL 强制开启（`wal_enabled: true`）
  * MVCC 强制启用
  * TX Lifecycle 强制：所有 DML 必须经过 Begin → ... → Commit/Rollback

如果你在 v3.7.0 依赖了某个"绕过 WAL 的写入路径"（如直接调 `FileStorage::insert`），
v3.8.0 会在 WAL 不可用时拒绝写入。

3.3 旧 binary 启动脚本
............................................................

如果你现有 systemd unit / k8s manifest 写死了旧 binary 名，
需要替换为新命令。常见模式：

    # OLD
    ExecStart=/usr/local/bin/sqlrustgo
    ExecStart=/usr/local/bin/sqlrustgo-sql-cli "SELECT 1"

    # NEW
    ExecStart=/usr/local/bin/sqlrustgo-mysql-server serve
    ExecStart=/usr/local/bin/sqlrustgo-mysql-server exec "SELECT 1"

3.4 CLI 参数变更
............................................................

`sqlrustgo-mysql-server` 的 CLI（来自 `crates/mysql-server/src/main.rs`）：

  * 全局：`--log-level <LEVEL>`
  * `serve` 子命令：`--host <HOST>`, `--port <PORT>`
  * 子命令：`serve` / `exec` / `repl` / `bench` / `gmp` / `diag`

没有 `--data-dir`、没有 `--buffer-pool-capacity`、
没有 `--checkpoint-interval`（**待 v3.8.0 GA 补**）。

3.5 Server Version 字符串
............................................................

握手包中 `SERVER_VERSION = "8.0.33-SQLRustGo"`
（`crates/mysql-server/src/lib.rs:20`）。

如果有客户端基于 version 字符串做版本判断，需要更新白名单。


4. 数据迁移
------------------------------------------------------------

4.1 Storage Format 兼容性
............................................................

v3.8.0 的存储层（`crates/storage/src/file_storage.rs`）声明：

    //! Persists table data to JSON files

即 v3.7.0 及之前 v3.x 系列沿用 JSON 文件格式。
v3.8.0 默认沿用同一 FileStorage 路径，**理论上文件可直接复用**。

但因为 v3.8.0 强制 WAL，迁移后必须满足：

  1) 目标 data_dir 可写
  2) 目标 data_dir 与旧 v3.7.0 的 data_dir 一致
     （或完成 dump/load 迁移，详见 §4.2）
  3) 没有遗留 v3.7.0 旧 binary 的临时文件/锁

如果你没有 100% 把握文件兼容，请走 §4.2 的 dump/load。

4.2 跨主版本 dump / load
............................................................

当 storage format 不可信时（升级跨 ≥ 2 个主版本，或你做过 schema 改造），
推荐 dump / load：

    # 1) 旧版本导出
    sqlrustgo-mysql-server exec "EXPORT DATABASE '/backup/v370.json'"

    # 注：v3.8.0 阶段 `EXPORT` 关键字的 wire-protocol 入口
    #     **待 v3.8.0 GA 实测**；
    #     现可用 exec 跑一系列 SELECT INTO OUTFILE 风格语句。

    # 2) 复制到新环境
    rsync -av /backup/v370.json newhost:/backup/

    # 3) 新版本导入
    sqlrustgo-mysql-server exec "IMPORT DATABASE '/backup/v370.json'"

⚠️ 上述 EXPORT/IMPORT 的完整语法在 v3.8.0 Alpha 阶段**未文档化**；
请以 `crates/executor` 实际支持的语法为准，**待 v3.8.0 GA 实测**。

4.3 WAL 文件
............................................................

v3.8.0 强制 WAL，但 WAL 文件格式与 v3.7.0 沿用相同设计
（`crates/storage/src/wal_storage.rs`）。

迁移步骤：

  1) 停 v3.7.0 服务（先 `CHECKPOINT` 让 v3.7.0 把 WAL 落到 data_dir）
  2) 保留 data_dir（含 WAL 历史）
  3) 启动 v3.8.0：会从最近一次 checkpoint + WAL replay 恢复

4.4 Checkpoint 文件
............................................................

`CheckpointMetadata` JSON schema：

    {
      "lsn":           <u64>,
      "timestamp":     <u64 epoch ms>,
      "dirty_pages":   <u64>,
      "tx_count":      <u64>,
      "file_path":     "<path>"
    }

v3.7.0 → v3.8.0 字段名一致；
如 v3.6.0 之前的 metadata 字段不同，**待 v3.8.0 GA 确认兼容性**。


5. 配置迁移
------------------------------------------------------------

v3.8.0 Alpha **没有独立配置文件**，全部走 CLI + 环境变量。
因此从 v3.7.0 升级基本是"启动命令替换"，不需要搬运配置文件。

5.1 环境变量映射
............................................................

+---------------------+------------------+--------------------------------+
| 用途                | v3.7.0           | v3.8.0                         |
+=====================+==================+================================+
| 日志级别            | RUST_LOG         | RUST_LOG（不变）               |
| 监听 host           | --host / 默认    | --host（不变）                 |
| 监听 port           | --port / 默认    | --port（不变）                 |
| 数据目录            | --data-dir       | **CLI 暂未暴露，待 GA**        |
| BufferPool 大小     | --bp-size        | **CLI 暂未暴露，待 GA**        |
| Checkpoint 间隔     | --ckpt-interval  | **CLI 暂未暴露，待 GA**        |
+---------------------+------------------+--------------------------------+

5.2 工作目录 / 数据目录
............................................................

v3.8.0 推荐 `WorkingDirectory=/var/lib/sqlrustgo`
（DEPLOYMENT_GUIDE.md §3.4）。
从 v3.7.0 升级时，把 data_dir 软链或挂到同一位置：

    sudo mkdir -p /var/lib/sqlrustgo
    sudo rsync -av /old/data/dir/ /var/lib/sqlrustgo/
    sudo chown -R sqlrustgo:sqlrustgo /var/lib/sqlrustgo

5.3 systemd unit
............................................................

替换 `ExecStart` 即可，详见 §3.3。其它字段（Restart, LimitNOFILE,
WorkingDirectory, User）保持不变。


6. 客户端/驱动迁移
------------------------------------------------------------

6.1 协议层
............................................................

v3.8.0 协议：

  * Protocol v10
  * mysql_native_password（SHA1）
  * Capability flags：见 `crates/mysql-server/src/lib.rs:36-64`
  * TLS：rustls 0.23 + rcgen 自签证书
  * SERVER_VERSION = "8.0.33-SQLRustGo"

理论上任何 MySQL 兼容驱动都可直连，**无需修改**。
但若驱动对 version 字符串做强校验，请更新白名单。

6.2 旧客户端坑位
............................................................

  * mysql-connector-java 5.x：偶尔会因为 SERVER_VERSION 含 `-` 报 warning；
    不影响功能。
  * 旧 mysql cli（< 5.5）：不支持 `caching_sha2_password`，
    但 v3.8.0 用 `mysql_native_password` 所以**没有**这个坑。

6.3 嵌入式测试 harness
............................................................

仓库内新增 `sqlrustgo-mysql-server::testing::start_ephemeral`
（CHANGELOG.md §[3.8.0]）。
如果你 e2e 测试曾经直接 fork 旧 binary，迁移到 `start_ephemeral` 可获得
"进程内 MySQL server"，**不需要外部端口**。


7. 兼容性矩阵
------------------------------------------------------------

7.1 与 v3.x 的兼容性
............................................................

+----------+--------------------+--------------------+--------------------+
| 项目     | v3.5.0             | v3.6.0             | v3.7.0             |
+==========+====================+====================+====================+
| 协议     | ✅ 同 wire 协议    | ✅ 同 wire 协议    | ✅ 同 wire 协议    |
| binary   | ⚠️ 5 个旧 binary  | ⚠️ 5 个旧 binary  | ⚠️ 5 个旧 binary  |
|          | 退役 → 1 个        | 退役 → 1 个        | 退役 → 1 个        |
| WAL      | 可选               | 可选               | 可选               |
| 强制     | v3.8.0 起强制      | 同左               | 同左               |
| storage  | JSON 文件          | JSON 文件          | JSON 文件          |
| format   | 兼容（**待 GA**）  | 兼容（**待 GA**）  | 兼容（**待 GA**）  |
| 认证     | mysql_native_pwd   | 同左               | 同左               |
| 默认用户 | root/(空), mysql   | 同左               | 同左               |
+----------+--------------------+--------------------+--------------------+

7.2 客户端/驱动兼容
............................................................

+--------------------+----------+--------------------------------+
| 驱动/客户端        | 兼容     | 备注                           |
+====================+==========+================================+
| mysql 5.7+ CLI     | ✅       | 需支持 mysql_native_pwd        |
| mysql 8.0+ CLI     | ✅       | 同上                           |
| mariadb CLI        | ✅       | 同上                           |
| mysql-connector-j  | ✅       | 5.x 警告 - 字符在 version      |
| mysql-connector-py | ✅       | 默认 pwd plugin = native       |
| MyBatis            | ✅       | 走标准 JDBC                    |
| SQLAlchemy         | ✅       | mysql+pymysql                  |
| Prisma             | ✅       | mysql connector                |
| navicat / dbeaver  | ✅       | 标准 MySQL 客户端              |
+--------------------+----------+--------------------------------+

7.3 操作系统
............................................................

+----------+----------+--------------------------------+
| OS       | 兼容     | 备注                           |
+==========+==========+================================+
| Linux    | ✅       | Tier-1                         |
| macOS    | ✅       | Tier-1                         |
| Windows  | ⚠️       | 建议 WSL2；原生未 CI 验证      |
+----------+----------+--------------------------------+


8. 已知问题 + Workaround
------------------------------------------------------------

8.1 bench / gmp / diag 占位
............................................................

现象：`bench`、`gmp`、`diag` 子命令存在但只打印占位提示并以退出码 2 退出。

影响：依赖这些子命令做 CI/批处理的脚本会失败。

Workaround：

  * 升级到 v3.8.0 之前，从 CI 中移除对这些子命令的依赖；
  * 或继续用旧 v3.7.0 binary（仓库内仍可编译）作为过渡；
  * 跟踪 openspec 相关变更（**待 v3.8.0 GA 补全**）。

8.2 默认账号 root/空密码
............................................................

现象：`UserStore::new()` 注入 `root`（空密码）和 `mysql/mysql`。

影响：未授权访问。

Workaround：

  * 任何面向网络暴露的部署，必须改 host 为非 0.0.0.0
    （或加防火墙 / 跳板机）；
  * 长期修复在 openspec 引入 CREATE USER/GRANT 路径后
    （**待 v3.8.0 GA**）。

8.3 CLI 未暴露 data_dir / BufferPool / Checkpoint 参数
............................................................

现象：调优只能改源码 / 等后续 openspec 变更。

Workaround：

  * 短期：维持 v3.7.0 默认即可；
  * 中期：通过 openspec 变更添加 CLI flag；
  * 长期：引入配置文件（**待 v3.8.0 GA**）。

8.4 Performance Schema 未通过 wire 暴露
............................................................

现象：`tests/performance_schema_test.rs` 实现了 F-31 的内存 instrumentation，
但注释明示 "Real integration in v3.9.0"。

影响：业务侧无法 SQL 访问 `performance_schema.*` 视图。

Workaround：

  * 短期：解析 tracing 日志；
  * 长期：等 v3.9.0 落地。

8.5 启动期内存不可控
............................................................

现象：BufferPool capacity 是内部硬编码。

Workaround：

  * 暂以 Cargo feature 或源码常量调整（**待 v3.8.0 GA**）；
  * 或在 openspec 提 PR 增加 CLI 参数。

8.6 多 connection 长跑可能 fd 耗尽
............................................................

现象：每条连接一个 OS 线程（参考 lib.rs 派发模型）。

Workaround：

  * systemd `LimitNOFILE=65535`
  * 应用层使用连接池
  * 监控 `lsof -p $(pgrep sqlrustgo-mysql-server) | wc -l`


9. 回滚方案
------------------------------------------------------------

如果 v3.8.0 上线后需要回退到 v3.7.0：

  1) 停 v3.8.0 服务
  2) 恢复旧 v3.7.0 binary（保留在 /usr/local/bin/sqlrustgo 等）
  3) data_dir 保留（FileStorage JSON 格式未变）
  4) 旧 systemd unit（`ExecStart=/usr/local/bin/sqlrustgo`）重新启用
  5) 启动 v3.7.0，确认 SELECT 校验数据完好
  6) 收集 root cause，提 issue

> 跨大版本回滚时，建议先在 staging 复现一次再在生产回滚。


10. 升级 Checklist
------------------------------------------------------------

  [ ] 备份数据目录（含 WAL + checkpoint）
  [ ] 备份 v3.7.0 binary（用于回滚）
  [ ] 阅读 CHANGELOG.md §[3.8.0] 全部 PR 列表
  [ ] staging 跑 smoke + 业务回归
  [ ] 检查所有启动脚本 / systemd unit / k8s manifest
       把旧 binary 名替换为 sqlrustgo-mysql-server
  [ ] 替换客户端白名单（如果依赖 version 字符串）
  [ ] 灰度切流（5% → 25% → 100%）
  [ ] 监控日志关键字（ERROR / panic）
  [ ] 至少 24h 观察窗口
  [ ] 文档归档：保留 v3.7.0 DEPLOY/MIGRATION 链接


11. 引用
------------------------------------------------------------

* `CHANGELOG.md` §[3.8.0] —— 入口说明
* `crates/mysql-server/src/main.rs` —— canonical CLI
* `crates/mysql-server/src/lib.rs` —— wire protocol + UserStore
* `crates/storage/src/file_storage.rs` —— FileStorage
* `crates/storage/src/wal_storage.rs` —— WAL
* `crates/storage/src/checkpoint.rs` —— Checkpoint
* `tests/performance_schema_test.rs` —— F-31 占位
* `docs/releases/v3.8.0/INSTALL.md`
* `docs/releases/v3.8.0/DEPLOYMENT_GUIDE.md`


12. 已知空白（"未知"清单）
------------------------------------------------------------

下列项 v3.8.0 Alpha 阶段未给出实测数据，保持"未知 / 待实测"：

  * v3.5.0 / v3.6.0 → v3.8.0 storage format 精确兼容性
  * EXPORT / IMPORT wire-protocol 入口的完整语法
  * 跨大版本 dump/load 的官方工具
  * CLI 参数（data_dir / BufferPool / Checkpoint）补全时间
  * Performance Schema 通过 wire protocol 暴露（仓库注释明示 v3.9.0）
  * 灰度切流过程中的实际回滚次数 / 失败率
  * 各 OS 下 v3.7.0 → v3.8.0 升级平均时长
  * 长期证书 / 吊销 / 备份恢复流程


— 完 —

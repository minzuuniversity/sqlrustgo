SQLRustGo v3.8.0 - 部署指南
============================================================

> 目标读者：负责把 SQLRustGo 部署到测试/生产环境的工程师和运维。
> 前置阅读：INSTALL.md（先跑通 smoke test）。
> 范围：系统要求、安装、启动、客户端连接、配置、性能调优、监控、故障排查。
> 状态说明：v3.8.0 当前为 Alpha；本文档描述的部署形态以
> 仓库内 `crates/mysql-server/src/main.rs` 和 `docs/releases/v3.8.0/CHANGELOG.md`
> 为准；GA 实测数字用"未知 / 待实测"标记。


目录
------------------------------------------------------------

  1. v3.8.0 部署模型
  2. 系统要求
  3. 安装步骤
  4. 启动服务
  5. 客户端连接
  6. 配置
  7. 性能调优
  8. 监控
  9. 故障排查
 10. 升级与回滚
 11. 引用
 12. 已知空白


1. v3.8.0 部署模型
------------------------------------------------------------

v3.8.0 引入了 "Execution Semantics Freeze" 和 "Canonical Binary Consolidation"
（见 CHANGELOG.md §[3.8.0]）。这意味着部署形态相对 v3.7.0 有显著简化：

  1) 单一执行入口：`sqlrustgo-mysql-server`，子命令 `serve` 启动 MySQL 协议服务。
  2) 5 个旧 binary 已退役：见 CHANGELOG.md（"旧 binary 退役"）。
  3) WAL 强制开启：v3.8.0 起 WAL mandatory（commit 087bb12d）。
  4) AUTOCOMMIT 默认开启；TX Lifecycle 强制。
  5) 默认监听：127.0.0.1:3306（与 MySQL 兼容）。

Canonical binary 子命令表（来自 `crates/mysql-server/src/main.rs`）：

  +-----------+--------------------------+--------------------------------+
  | 子命令    | 含义                     | 部署阶段使用                   |
  +===========+==========================+================================+
  | serve     | MySQL wire 协议服务       | ✅ 生产主路径                  |
  | exec      | 单条 SQL 一次执行后退出   | 脚本/批处理                    |
  | repl      | 交互式 REPL              | 调试/排障                      |
  | bench     | 基准测试（v3.8.0 占位）   | 性能回归（v3.8.0 placeholder）  |
  | gmp       | AI Native 工作流（占位）  | 待 v3.8.0 GA 实测              |
  | diag      | 诊断/目录 dump（占位）   | 待 v3.8.0 GA 实测              |
  +-----------+--------------------------+--------------------------------+

> "占位" = 入口存在、打印 migration 提示并退出码 2；
> 完整功能在后续 openspec 变更中迁移（见 CHANGELOG.md）


2. 系统要求
------------------------------------------------------------

2.1 硬件基线
............................................................

+--------------------+---------------------+--------------------------------+
| 维度               | 最低 / 推荐         | 说明                           |
+====================+=====================+================================+
| CPU                | 2 vCPU / 4 vCPU+    | x86_64 或 aarch64              |
|                    | 启用 AVX2 加速 SHA1  | TLS / 认证计算                  |
+--------------------+---------------------+--------------------------------+
| RAM                | 2 GB / 8 GB+        | 与 BufferPool 容量线性相关     |
+--------------------+---------------------+--------------------------------+
| Disk               | 10 GB / 50 GB+      | 数据文件 + WAL + checkpoint    |
+--------------------+---------------------+--------------------------------+
| 文件描述符         | 65535               | 单进程连接数 / WAL 文件         |
+--------------------+---------------------+--------------------------------+
| 网络端口           | 1 个 (默认 3306)    | 可改 --port                    |
+--------------------+---------------------+--------------------------------+

2.2 操作系统
............................................................

* Linux：kernel 5.10+，glibc 2.31+（CentOS 8 / Ubuntu 20.04+）
* macOS：13+，Apple Silicon 优先
* Windows：原生未在 CI 跑；建议 WSL2（按 Linux 部署）

2.3 运行时依赖
............................................................

* libssl 1.1+ 或 3.x（rustls 自带加密，但系统链接可能需要）
* zlib / lz4（如启用相应 feature）
* 一个标准 POSIX shell（systemd / launchd 集成）

2.4 容量规划
............................................................

粗略公式（"未知"处等待 v3.8.0 GA 实测）：

  BufferPool RAM  ≈  capacity_pages × page_size
  WAL 磁盘        ≈  QPS × avg_wal_bytes × 保留窗口
  Checkpoint 磁盘 ≈  当前数据快照 + 增量日志

v3.8.0 内置默认值（来自 `crates/storage/src/checkpoint.rs`）：

  CheckpointConfig::default:
    interval           = 5 min
    max_wal_size_mb    = 100 MB
    incremental        = true

BufferPool 默认 page size 与 capacity 由 `BufferPool::new(capacity)` 控制；
v3.8.0 启动路径目前未暴露 CLI 参数（**待 v3.8.0 GA 实测**），需通过后续
openspec 变更提供的配置文件注入。


3. 安装步骤
------------------------------------------------------------

3.1 二进制安装（推荐）
............................................................

从 Gitea/GitHub Release 页面下载匹配你平台的 tarball：

    curl -L -o sqlrustgo.tar.gz \
        <RELEASE_URL>/sqlrustgo-mysql-server-v3.8.0-<TARGET>.tar.gz
    tar -xzf sqlrustgo.tar.gz
    sudo install -m 0755 sqlrustgo-mysql-server /usr/local/bin/

验证：

    sqlrustgo-mysql-server --version
    sqlrustgo-mysql-server --help

3.2 从源码编译
............................................................

    git clone <REPO_URL> && cd sqlrustgo
    git checkout develop/v3.8.0
    cargo build --release --bin sqlrustgo-mysql-server --all-features

    install -m 0755 target/release/sqlrustgo-mysql-server /usr/local/bin/

3.3 容器（v3.8.0 状态）
............................................................

⚠️ v3.8.0 Alpha 阶段**没有官方镜像**（待 v3.8.0 GA 实测）。
社区最小化做法：

    cat > Dockerfile <<'EOF'
    FROM rust:1.78-bookworm AS build
    WORKDIR /src
    COPY . .
    RUN cargo build --release --bin sqlrustgo-mysql-server --all-features

    FROM debian:bookworm-slim
    RUN apt-get update && \
        apt-get install -y --no-install-recommends libssl3 ca-certificates && \
        rm -rf /var/lib/apt/lists/*
    COPY --from=build /src/target/release/sqlrustgo-mysql-server /usr/local/bin/
    EXPOSE 3306
    ENTRYPOINT ["/usr/local/bin/sqlrustgo-mysql-server", "serve"]
    EOF

    docker build -t sqlrustgo:v3.8.0 .
    docker run --rm -p 3306:3306 sqlrustgo:v3.8.0

镜像体积 / 启动时间待 v3.8.0 GA 实测。

3.4 systemd unit（Linux 部署）
............................................................

    cat > /etc/systemd/system/sqlrustgo.service <<'EOF'
    [Unit]
    Description=SQLRustGo MySQL Server
    After=network-online.target
    Wants=network-online.target

    [Service]
    Type=simple
    User=sqlrustgo
    Group=sqlrustgo
    ExecStart=/usr/local/bin/sqlrustgo-mysql-server serve --host 0.0.0.0 --port 3306
    Restart=on-failure
    RestartSec=2
    LimitNOFILE=65535
    Environment=RUST_LOG=info,sqlrustgo=info
    WorkingDirectory=/var/lib/sqlrustgo

    [Install]
    WantedBy=multi-user.target
    EOF

    sudo useradd -r -d /var/lib/sqlrustgo -s /usr/sbin/nologin sqlrustgo
    sudo systemctl daemon-reload
    sudo systemctl enable --now sqlrustgo
    sudo systemctl status sqlrustgo


4. 启动服务
------------------------------------------------------------

4.1 基本启动
............................................................

默认（监听 127.0.0.1:3306）：

    sqlrustgo-mysql-server serve

显式 host/port：

    sqlrustgo-mysql-server serve --host 0.0.0.0 --port 3306

4.2 CLI 全参
............................................................

    sqlrustgo-mysql-server [GLOBAL] [SUBCOMMAND]

全局参数：

    --log-level <LEVEL>     默认 "info"
                            可选：trace, debug, info, warn, error
                            也可由环境变量 RUST_LOG 覆盖
                            （取自 crates/mysql-server/src/main.rs:65-67）

`serve` 子命令参数：

    --host <HOST>           默认 127.0.0.1
    --port <PORT>           默认 3306

4.3 启动后行为
............................................................

成功启动后：

    SQLRustGo MySQL Server starting on 0.0.0.0:3306

服务进入 accept loop，每条客户端连接一个工作线程
（参考 `crates/mysql-server/src/lib.rs` 的 std::thread 派发模型）。

4.4 优雅停止
............................................................

v3.8.0 Alpha 阶段：

    Ctrl-C / SIGTERM

    收到信号后，正在进行的连接会被丢弃；
    不会自动 fsync checkpoint（**待 v3.8.0 GA 实测**）。

> 建议：在 systemd unit 里加 `TimeoutStopSec=30` 给应用留出收尾窗口。


5. 客户端连接
------------------------------------------------------------

5.1 默认账号
............................................................

v3.8.0 Alpha 在 `UserStore::new()` 注入两个测试账号
（`crates/mysql-server/src/lib.rs` 附近）：

    +-------+----------+--------------------+
    | user  | password | 用途               |
    +-------+----------+--------------------+
    | root  | (空)     | 默认管理员         |
    | mysql | mysql    | mysql.user 兼容    |
    +-------+----------+--------------------+

⚠️ **生产前必须改**——见 §6.3。

5.2 协议
............................................................

* MySQL 协议 v10 (Protocol::VERSION = 0x0a)
* mysql_native_password 认证（SHA1-based）
* capability flags：见 `crates/mysql-server/src/lib.rs` cap::SERVER_DEFAULT
* TLS：rustls 0.23 + rcgen 0.13 自签证书（mariadb-connector-c 3.4+ 兼容）
* SERVER_VERSION = "8.0.33-SQLRustGo"

5.3 mysql client
............................................................

    # 8.0+ 推荐
    mysql -h 127.0.0.1 -P 3306 -u root --protocol=TCP

    # 旧版兼容
    mysql -h 127.0.0.1 -P 3306 -u root -p --ssl-mode=DISABLED

5.4 任意 MySQL 驱动
............................................................

任何遵循 MySQL wire protocol 的客户端/驱动都可以直接连接，包括：

* mysql / mariadb 命令行
* mysql-connector-j / mysql-connector-python
* DBeaver / DataGrip / TablePlus
* 应用层：MyBatis / SQLAlchemy / Prisma

5.5 嵌入式测试客户端
............................................................

仓库内 `tests/common/mod.rs::MySqlTestClient` 实现了 raw TCP + HandshakeResponse41，
不依赖 `mysql` crate。E2E 测试推荐复用此客户端（CHANGELOG.md §[3.8.0] 末段）。


6. 配置
------------------------------------------------------------

v3.8.0 Alpha 的配置以 CLI 参数 + 环境变量为主，
**没有独立配置文件格式**（**待 v3.8.0 GA 引入**）。

6.1 日志级别
............................................................

    # CLI
    sqlrustgo-mysql-server --log-level debug serve

    # 环境变量（优先级更高）
    RUST_LOG=info,sqlrustgo_mysql_server=trace \
        sqlrustgo-mysql-server serve

    # 模块级细粒度（EnvFilter 语法）
    RUST_LOG=warn,sqlrustgo=debug,sqlrustgo_storage=info,sqlrustgo_executor=info

6.2 环境变量
............................................................

+---------------------+----------------------------+--------------------------+
| 变量                | 含义                       | 默认                     |
+=====================+============================+==========================+
| RUST_LOG            | tracing EnvFilter          | --log-level 或 info      |
+---------------------+----------------------------+--------------------------+

6.3 用户/认证
............................................................

v3.8.0 Alpha 阶段账号硬编码在 `UserStore::new()`。
生产改造需要：

1. 在 openspec 中提出配置注入路径（**待 v3.8.0 GA**）。
2. 或在 SQL 层实现 `CREATE USER` / `GRANT` 的 wire-protocol 入口
   （**待 v3.8.0 GA 实测**）。

6.4 数据目录
............................................................

v3.8.0 启动时使用的 `data_dir` 由内部 FileStorage 决定（`FileStorage::new(data_dir)`）。
在 canonical binary `serve` 路径下，目前没有 CLI 参数暴露 data_dir
（**待 v3.8.0 GA 补**）；

当下推荐：先在 `crates/storage/src/file_storage.rs` 内 `data_dir: PathBuf`
的初始值上 PR，或在 openspec 变更中加 `--data-dir` 参数。


7. 性能调优
------------------------------------------------------------

7.1 BufferPool
............................................................

相关代码：`crates/storage/src/buffer_pool.rs`

* `BufferPool::new(capacity)` —— 容量（页数）
* `BufferPool::with_prefetch(capacity, prefetch_window)` —— 自定义预取窗口
* 统计：`BufferPoolStats { hits, misses, evictions, prefetch_hits }`
  * `hit_rate()` = hits / (hits + misses)

调优目标：命中率 ≥ 95%。若 < 80%：

* 增大 capacity（注意 RAM）
* 调整 prefetch_window（顺序扫描/范围扫描场景）
* 检查热点表是否被频繁全表扫描

7.2 WAL
............................................................

相关代码：`crates/storage/src/wal_storage.rs` + `wal.rs`

v3.8.0 强制开启 WAL（`wal_enabled: true`）。
关键参数：

* `next_lsn` —— 单调递增 LSN，每次 `append_wal_entry` 自增
* `current_tx_id` —— 由 `ExecutionEngine` 通过 `set_current_tx_id` 注入
  （0 = autocommit）
* checkpoint 触发（见 `CheckpointConfig::default`）：
  * 5 分钟一次
  * WAL ≥ 100 MB
  * incremental = true

调优建议：

* 写密集：增大 `max_wal_size_mb`，减少 checkpoint 频率
* 恢复时间敏感：减小 `interval`，增加 checkpoint 频率
* 监控 `next_lsn` 与最近一次 checkpoint LSN 的差值
  （差值越大，恢复时 replay 越长）

7.3 Checkpoint
............................................................

相关代码：`crates/storage/src/checkpoint.rs`

CheckpointMetadata 字段：

    +---------------+-----------------------------------+
    | 字段          | 含义                              |
    +===============+===================================+
    | lsn           | checkpoint 时的 LSN               |
    | timestamp     | Unix epoch ms                     |
    | tx_count      | checkpoint 时累积 tx 数           |
    | dirty_pages   | 脏页数                            |
    | file_path     | checkpoint 文件路径                |
    +---------------+-----------------------------------+

7.4 网络/连接
............................................................

* 每条连接一个 OS 线程（参考 `std::thread::spawn` 在 lib.rs）；
  调整 `LimitNOFILE` 防止 fd 耗尽。
* 长连接优于短连接（避免反复 TCP + auth）。
* 监控 `tcp_*` 计数器（`ss -s` / `netstat -s`）确认无 backlog 累积。

7.5 操作系统
............................................................

    # Linux
    echo 'vm.swappiness = 10' >> /etc/sysctl.d/99-sqlrustgo.conf
    echo 'net.core.somaxconn = 4096' >> /etc/sysctl.d/99-sqlrustgo.conf
    sysctl --system

    ulimit -n 65535     # 在 systemd unit 中以 LimitNOFILE 体现

7.6 不建议在 v3.8.0 Alpha 做的事
............................................................

* 把监听暴露到公网 0.0.0.0 + 默认账号（root/空密码）
* 启用 TLS 长期证书（自签证书仅用于开发）
* 用作主库承担真实生产写入负载


8. 监控
------------------------------------------------------------

8.1 内置暴露
............................................................

* 日志：通过 `tracing_subscriber` 写出（结构化，取决于 EnvFilter）
* BufferPoolStats：内存对象，**未通过 SQL 暴露**
* CheckpointMetadata：写入 JSON 文件，可外部采集
* Performance Schema：仓库内 `tests/performance_schema_test.rs`
  实现了 F-31 的内存 instrumentation，但**未通过 wire protocol 暴露**
  （注释明示 "Real integration in v3.9.0"）

8.2 外部采集建议
............................................................

* 进程存活：`systemctl is-active sqlrustgo`
* 端口：`(echo > /dev/tcp/127.0.0.1/3306) 2>/dev/null && echo OK`
* 日志关键字：错误计数（`grep -c ERROR`）
* 文件：
  * WAL 文件增长率
  * checkpoint 文件数 / 大小
  * 数据目录大小
* 资源：CPU / 内存 / fd / 磁盘 IO

8.3 接入 Prom 生态
............................................................

建议（**待 v3.8.0 GA 实测**）：

    # 伪 exporter：用 shell 拉指标
    #!/bin/bash
    echo "sqlrustgo_bufferpool_hits $(jq ...)"
    echo "sqlrustgo_wal_lsn $(...)"

或：等仓库提供 `/metrics` HTTP 端点（**待 v3.8.0 GA**）。


9. 故障排查
------------------------------------------------------------

+------------------------------------+------------------------------------+
| 现象                               | 排查                               |
+====================================+====================================+
| 启动报 "address already in use"    | `ss -tlnp | grep 3306`             |
|                                    | 换 --port 或停占用方               |
+------------------------------------+------------------------------------+
| mysql 客户端连不上                 | 检查 --host (默认 127.0.0.1)      |
|                                    | 防火墙 / 安全组                    |
+------------------------------------+------------------------------------+
| `access denied for user 'root'`    | v3.8.0 默认 root 无密码；         |
|                                    | 旧客户端可能自动发送旧密码         |
+------------------------------------+------------------------------------+
| 服务能起但 DML 不写                | 检查 data_dir 写权限、WAL 目录     |
|                                    | (audit/var/lib/sqlrustgo)         |
+------------------------------------+------------------------------------+
| 重启后数据丢失                     | v3.8.0 强制 WAL，理论上不会；     |
|                                    | 检查 data_dir 是否复用             |
+------------------------------------+------------------------------------+
| OOM / 内存飙升                     | 减小 BufferPool capacity；         |
|                                    | 检查长事务未提交                   |
+------------------------------------+------------------------------------+
| BufferPool hit rate < 80%          | 见 §7.1                            |
+------------------------------------+------------------------------------+
| 日志大量 "server error"            | 看 trace：                         |
|                                    | RUST_LOG=trace                    |
+------------------------------------+------------------------------------+

通用排障套路：

    # 1) 看进程是否在
    pgrep -af sqlrustgo-mysql-server

    # 2) 看监听
    ss -tlnp | grep sqlrustgo

    # 3) 看最后 100 行日志
    journalctl -u sqlrustgo -n 100 --no-pager

    # 4) 最小化连通性测试
    nc -zv 127.0.0.1 3306

    # 5) 直连 REPL 排除网络问题
    sqlrustgo-mysql-server repl


10. 升级与回滚
------------------------------------------------------------

* 升级：见 MIGRATION_GUIDE.md（breaking change + 数据迁移）。
* 回滚：保留 v3.7.0 旧 binary 路径；切回 systemd unit 的
  `ExecStart=/path/to/legacy/sqlrustgo serve` 即可
  （v3.7.0 旧 binary 仍在仓库内编译，详见 MIGRATION_GUIDE.md §3）。


11. 引用
------------------------------------------------------------

* `crates/mysql-server/src/main.rs` —— canonical binary CLI
* `crates/mysql-server/src/lib.rs` —— wire protocol 实现
* `crates/storage/src/buffer_pool.rs` —— BufferPool
* `crates/storage/src/wal_storage.rs` —— WAL Storage
* `crates/storage/src/checkpoint.rs` —— Checkpoint
* `tests/performance_schema_test.rs` —— F-31 占位
* `CHANGELOG.md` §[3.8.0] —— v3.8.0 入口
* `docs/releases/v3.8.0/INSTALL.md`
* `docs/releases/v3.8.0/MIGRATION_GUIDE.md`


12. 已知空白（"未知"清单）
------------------------------------------------------------

下列项 v3.8.0 Alpha 阶段未给出实测数据，文档中保持"未知 / 待实测"：

* BufferPool capacity 的 CLI 暴露（**待 v3.8.0 GA**）
* data_dir 的 CLI 暴露（**待 v3.8.0 GA**）
* CheckpointConfig 的 CLI / 文件配置入口（**待 v3.8.0 GA**）
* Performance Schema 通过 wire protocol 的访问（仓库注释明示 v3.9.0）
* 官方 docker 镜像（**待 v3.8.0 GA**）
* 官方 helm chart / operator（**待 v3.8.0 GA**）
* 各负载下的 QPS / p99 latency / 吞吐（**待 v3.8.0 GA 实测**）
* TLS 长期证书的签发/吊销流程（自签证书仅开发用）
* 备份/恢复（`crates/storage/src/backup.rs` 存在但未审计覆盖度，**待 v3.8.0 GA**）


— 完 —

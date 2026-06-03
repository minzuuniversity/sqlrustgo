SQLRustGo v3.8.0 - 安装指南
============================================================

> 目标读者：第一次接触 SQLRustGo 的工程师、运维、DBA。
> 阅读时间：5 - 10 分钟。
> 范围：从"零"到"能跑起来一个 MySQL 兼容服务"。
> 与本指南相关：DEPLOYMENT_GUIDE.md（生产部署）、MIGRATION_GUIDE.md（从 v3.7.0 升级）。


1. 一句话总结
------------------------------------------------------------

v3.8.0 的执行入口是一个 canonical binary：

    sqlrustgo-mysql-server

它通过子命令（`serve` / `exec` / `repl` / `bench` / `gmp` / `diag`）
统一了 v3.7.0 之前的 5 个 binary（`sqlrustgo`、`sqlrustgo-sql-cli`、
`sqlrustgo-bench`、`sqlrustgo-bench-cli`、`sqlrustgo-tools`）。
你只需要安装/编译这一个 binary 就能跑 v3.8.0。


2. 平台支持矩阵
------------------------------------------------------------

+----------+-----------------------+----------------------------+
| 平台     | 状态                  | 备注                       |
+==========+=======================+============================+
| Linux    | ✅ Tier-1 (CI 全跑)    | x86_64 / aarch64           |
+----------+-----------------------+----------------------------+
| macOS    | ✅ Tier-1              | Apple Silicon + x86_64     |
+----------+-----------------------+----------------------------+
| Windows  | ⚠️ 社区报告            | 未在 CI 矩阵中；建议 WSL2  |
+----------+-----------------------+----------------------------+

> Tier-1 = CI 在每次提交时实际跑过 release build + smoke test。


3. 快速安装（5 分钟）
------------------------------------------------------------

3.1 使用预编译 binary（如果你的平台有 release artifact）
............................................................

    # 下载对应平台的 release artifact
    # （实际文件名以 GitHub/Gitea Release 页面为准，路径占位）
    curl -L -o sqlrustgo.tar.gz \
        <RELEASE_URL>/sqlrustgo-mysql-server-v3.8.0-<TARGET>.tar.gz

    # 解压
    tar -xzf sqlrustgo.tar.gz

    # 验证
    ./sqlrustgo-mysql-server --version
    ./sqlrustgo-mysql-server --help

    # 启动（默认 127.0.0.1:3306）
    ./sqlrustgo-mysql-server serve

预期输出形如：

    SQLRustGo MySQL Server starting on 127.0.0.1:3306

3.2 从源码编译
............................................................

    # 1) 克隆仓库
    git clone https://gitea-macmini/openclaw/sqlrustgo.git
    cd sqlrustgo

    # 2) 切到 v3.8.0
    git checkout develop/v3.8.0

    # 3) 编译 canonical binary（启用全部 features）
    cargo build --release --bin sqlrustgo-mysql-server --all-features

    # 产物路径
    ls -lh target/release/sqlrustgo-mysql-server


4. 编译时间和资源需求
------------------------------------------------------------

+----------+----------------+----------------+----------------------+
| 目标     | 冷启动编译     | 增量编译       | 预估磁盘             |
+==========+================+================+======================+
| x86_64   | 8 - 15 min     | 30 - 90 s      | 6 - 10 GB target/    |
+----------+----------------+----------------+----------------------+
| aarch64  | 10 - 18 min    | 30 - 120 s     | 6 - 10 GB target/    |
+----------+----------------+----------------+----------------------+

> 数字来自典型 16-core / 32 GB 机器；
> 实际值取决于 Rust 工具链版本、feature 组合、磁盘类型。
> 数字仅供预算参考；如果你在 CI 跑，请自己跑一次校准。


5. 依赖
------------------------------------------------------------

5.1 Rust 工具链
............................................................

仓库使用 Rust edition 2021。最低要求：

    rustc 1.74.0+
    cargo 1.74.0+

推荐使用 rustup 锁定版本：

    rustup toolchain install 1.78.0
    rustup default 1.78.0

5.2 系统库
............................................................

最小集合（Linux）：

    # Debian / Ubuntu
    sudo apt-get update
    sudo apt-get install -y \
        build-essential pkg-config libssl-dev \
        libsqlite3-dev liblz4-dev zlib1g-dev

    # RHEL / Rocky
    sudo dnf install -y gcc gcc-c++ pkgconfig \
        openssl-devel sqlite-devel lz4-devel zlib-devel

macOS（Homebrew）：

    brew install openssl@3 cmake pkg-config

Windows（WSL2 推荐）：

    在 WSL2 内按 Linux 步骤安装；
    原生 Windows 构建未在 CI 验证，可能需要额外的 vcpkg 步骤。

5.3 网络
............................................................

编译期会从 crates.io 拉依赖。
请确保：

    * 能够访问 https://crates.io
    * 或在企业内部配置 cargo registry 镜像（[source] 替换）。

5.4 文件描述符 / 端口
............................................................

    # 生产部署建议
    ulimit -n 65535

默认监听 127.0.0.1:3306。如果端口被占用，启动会立即报错退出，
需要通过 `--port` 切换（见 DEPLOYMENT_GUIDE.md）。


6. 验证安装（Smoke Test）
------------------------------------------------------------

6.1 版本
............................................................

    ./sqlrustgo-mysql-server --version

预期看到 SQLRustGo 的版本字符串（语义化版本 + git sha + commit date）。

6.2 启动并连接
............................................................

启动服务（前台）：

    ./sqlrustgo-mysql-server serve

另开一个终端，用系统自带的 mysql 客户端连：

    mysql -h 127.0.0.1 -P 3306 -u root

> v3.8.0 默认用户：
>
> +-------+----------+--------------------+
> | user  | password | 用途               |
> +-------+----------+--------------------+
> | root  | (空)     | 默认管理员         |
> | mysql | mysql    | 兼容 mysql.user     |
> +-------+----------+--------------------+
>
> ⚠️ v3.8.0 处于 Alpha；默认账号仅用于本地开发。
> 生产请使用 DEPLOYMENT_GUIDE.md 描述的强认证方式。

进入后跑：

    SELECT 'hello, sqlrustgo' AS msg;

预期看到一行返回。

6.3 exec 子命令
............................................................

    ./sqlrustgo-mysql-server exec "SELECT 1+1 AS two"

预期输出：

    2
    (1 rows)

6.4 REPL 子命令
............................................................

    ./sqlrustgo-mysql-server repl

进入交互式 REPL：

    sqlrustgo> SELECT 1+1;
    2
    (1 rows)
    sqlrustgo> .exit

6.5 单元/集成测试（可选）
............................................................

    cargo test --all-features -p sqlrustgo-mysql-server
    cargo test --all-features --test performance_schema_test

> 全 workspace `cargo test --all-features` 较慢（5 - 30 分钟），
> 初次安装验证建议先跑子集。


7. 常见安装问题
------------------------------------------------------------

+--------------------------------+--------------------------------+
| 现象                           | 排查                           |
+================================+================================+
| `linker not found`             | 安装 build-essential /        |
|                                | gcc / clang                    |
+--------------------------------+--------------------------------+
| `failed to fetch crate`        | 配置 cargo 镜像                |
|                                | (vendor / crates.io mirror)    |
+--------------------------------+--------------------------------+
| `address already in use`       | 端口冲突，换 --port 或         |
|                                | 停掉占用进程                   |
+--------------------------------+--------------------------------+
| 编译 OOM                       | 减少并行 `cargo build -j 4`    |
|                                | 或加 swap                      |
+--------------------------------+--------------------------------+
| `OPENSSL_DIR` not set          | apt 装 libssl-dev /            |
| (Linux)                        | brew 装 openssl@3              |
+--------------------------------+--------------------------------+


8. 下一步
------------------------------------------------------------

* 上生产：阅读 DEPLOYMENT_GUIDE.md（系统要求、调优、监控、故障排查）。
* 从 v3.7.0 升上来：阅读 MIGRATION_GUIDE.md（breaking change + 数据迁移）。
* 协议层细节：见 docs/releases/v3.8.0/ARCHITECTURE.md。


9. 引用
------------------------------------------------------------

* `CHANGELOG.md` - v3.8.0 entry（Alpha 状态）
* `crates/mysql-server/src/main.rs` - canonical binary CLI
* `Cargo.toml` - workspace 配置
* `docs/releases/v3.8.0/DEPLOYMENT_GUIDE.md`
* `docs/releases/v3.8.0/MIGRATION_GUIDE.md`


10. 已知空白（"未知"清单）
------------------------------------------------------------

下列数据在 v3.8.0 Alpha 阶段尚未产生实测数字，文档中保持"未知 / 待实测"：

* 各目标平台预编译 binary 的精确 SHA256（由 Release 阶段产出）
* WSL2 / Windows 原生环境下的精确编译时间
* 容器镜像（docker）支持：v3.8.0 阶段未提供，**待 v3.8.0 GA 实测**


— 完 —

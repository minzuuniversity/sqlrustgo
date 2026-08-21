# 第一周上机任务：sqlrustgo-cli 构建与 SQL 命令验证（Lab 01）

- 对应课程：第 1 周《从 MySQL 使用者到数据库建造者》
- 教材映射：第 1-2 章
- 里程碑：M0（能用工具链构建 sqlrustgo 并运行第一条 SQL）
- 预计用时：60-90 分钟

---

## 1. 本周目标

第一周不写数据库核心代码，而是把"造数据库"的工程环境一次性打通。完成本次上机后，每个学生（或每组）应当拥有：

1. 可用的 **Rust 编译环境**（`rustc` / `cargo` / `clippy` / `rustfmt`）。
2. 可用的 **Git** 与本仓库 `openclaw/sqlrustgo` 的本地副本。
3. 在本仓库内完成一次 `cargo build` 与 CLI 冒烟测试。
4. 一份提交证据（截图 + 命令输出摘要）。

> 验收对齐：**每组提交环境截图、仓库初始化、第一条 SQL 运行记录。**

---

## 2. 前置说明：为什么用 WSL + Ubuntu

本课程工程载体是 Rust，推荐在 **WSL2 + Ubuntu 24.04** 下开发，理由：

- Rust 生态、`cargo`、B+ Tree/Buffer Pool 等实验在 Linux 下行为最稳定。
- 与 CMU 15-445 / BustubX / mini-db 的运行环境一致，差分测试脚本可直接复用。

Windows 用户第一次准备 WSL：

```powershell
# 在 Windows PowerShell（管理员）中执行
wsl --install -d Ubuntu-24.04
# 安装后重启，设置 Linux 用户名与密码
```

---

## 3. 步骤一：准备 Rust 编译环境

### 3.1 检查是否已安装

```bash
rustc --version
cargo --version
gcc --version | head -1     # Rust 链接需要 C 链接器 (rusqlite bundled feature 依赖)
git --version
```

### 3.2 若未安装 Rust（推荐 rustup）

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
rustup component add clippy rustfmt
```

### 3.3 配置国内 crates 镜像（强烈建议）

由于校园网/国内网络访问 crates.io 较慢，建议配置镜像，否则 `cargo build` 会卡在依赖下载。

创建 `~/.cargo/config.toml`：

```toml
[source.crates-io]
replace-with = 'rsproxy-sparse'

[source.rsproxy]
registry = "https://rsproxy.cn/crates.io-index"

[source.rsproxy-sparse]
registry = "sparse+https://rsproxy.cn/index/"

[registries.rsproxy]
index = "https://rsproxy.cn/crates.io-index"

[net]
git-fetch-with-cli = true
```

---

## 4. 步骤二：获取本仓库并构建 CLI

### 4.1 克隆

本仓库 252 Gitea 地址：`http://192.168.0.252:3000/openclaw/sqlrustgo.git`

```bash
cd ~
# 公开仓库, 可匿名 clone; 若需推送则用 PAT
git clone --depth 1 --branch develop/v3.12.0 http://192.168.0.252:3000/openclaw/sqlrustgo.git sqlrustgo
cd sqlrustgo
```

> 若校园网对 252 不可达，由教师下发离线源码包。

### 4.2 构建

```bash
# 注意: 根包 sqlrustgo 是库 (无 bin), 必须用 -p 显式选包。
# sqlrustgo-cli 是 sqlrustgo-mysql-server 的薄包装, 运行时通过子进程调用后者,
# 因此两个二进制必须同时构建。
cargo build -p sqlrustgo-cli -p sqlrustgo-mysql-server
```

构建产物（位于 `target/debug/`）：

| 二进制 | 角色 | 说明 |
|---|---|---|
| `sqlrustgo-cli` | CLI 薄包装 | 统一入口，子命令 `exec`/`serve`/`repl`/`cli` 等 |
| `sqlrustgo-mysql-server` | 实际执行入口 | REPL、exec、MySQL 线协议服务均在此实现 |

> **架构提示**：`sqlrustgo-cli` 把子命令透传给 `sqlrustgo-mysql-server`，运行时按 `./target/{debug,release}/sqlrustgo-mysql-server` 或 `PATH` 查找后者，因此**必须在仓库根目录下运行**（或把后者加入 `PATH`）。

### 4.3 可用入口（本课堂已验证）

```bash
cd ~/sqlrustgo

# 入口 A：交互式 REPL（会话内持久，推荐用于第一周 SQL 验证）
./target/debug/sqlrustgo-mysql-server repl
# 进入后输入:
#   CREATE TABLE users (id INTEGER, name TEXT);
#   INSERT INTO users VALUES (1, 'Ada');
#   SELECT name FROM users WHERE id = 1;
#   .exit

# 入口 B：sqlrustgo-cli repl（等价于入口 A，已修复 --port 误传 bug）
./target/debug/sqlrustgo-cli repl

# 入口 C：单条 SQL（每次独立进程, 会话内不持久）
./target/debug/sqlrustgo-cli exec "SELECT 1"

# 入口 D：启动 MySQL 线协议服务, 用 mysql 客户端连接
./target/debug/sqlrustgo-cli serve --port 3307
# 另开终端: mysql -h 127.0.0.1 -P 3307 -u root

# 入口 E：连接运行中的 server 执行单条 query (位置参数)
./target/debug/sqlrustgo-cli cli -p 3307 "SELECT 1+1"
```

REPL 还支持 `--init-sql <file>`（启动时重放 SQL 建表插数）和 `--save-on-exit <file>`（退出时把当前 catalog 导出为 CREATE/INSERT 语句），便于跨会话保持状态、做差分测试。

---

## 5. 步骤三：运行第一条 SQL

### 5.1 在 REPL 中运行

```bash
./target/debug/sqlrustgo-mysql-server repl
```

```sql
create table t (a int, b int);
insert into t values (1, 2), (3, 4);
select * from t;
.exit
```

> **验收要点**：能看到 `select` 返回结果，即完成"第一条 SQL 运行记录"。

### 5.2 常见问题排查

| 现象 | 原因 | 处理 |
|---|---|---|
| `cargo build` 卡在 `Updating crates.io index` | 未配镜像 | 按 3.3 配置 rsproxy |
| `linker 'cc' not found` | 缺少 C 链接器 | `sudo apt install build-essential` |
| LFS smudge filter failed | clone 时拉 LFS 失败 | `GIT_LFS_SKIP_SMUDGE=1 git clone ...` 跳过 LFS |

---

## 6. 提交证据清单（本周验收）

每组在提交中包含以下证据（截图或文本粘贴均可）：

- [ ] **环境截图**：`rustc --version`、`cargo --version`、`git --version` 的输出。
- [ ] **仓库初始化**：`git -C ~/sqlrustgo log --oneline -5` 或 `git status`。
- [ ] **构建成功**：`cargo build -p sqlrustgo-cli -p sqlrustgo-mysql-server` 的 `Finished` 末尾输出。
- [ ] **第一条 SQL**：`sqlrustgo-mysql-server repl` 中 `create/insert/select` 的运行截图。
- [ ] **一句话反思**：本周遇到的最大障碍及解决方式（尤其是网络/工具链问题）。

---

## 7. 下周衔接

第 2 周进入《SQL 与关系模型》，将用本周装好的 MySQL/SQLite 做行为真值，用 sqlrustgo 做对照。本周构建好的 sqlrustgo REPL（`--init-sql`/`--save-on-exit`）也可作为差分测试真值与教材代码对照。请确保本周环境完全打通，避免把环境问题带入后续实验。

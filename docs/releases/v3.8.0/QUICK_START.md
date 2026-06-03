# v3.8.0 QUICK START (5 分钟上手)

> **目标读者**: 想快速试用 SQLRustGo v3.8.0 的开发者和 DBA
> **预计时间**: 5 分钟
> **前置条件**: Linux / macOS, 8GB+ 内存, Rust 1.75+ (仅源码编译需要)
> **重要提示**: v3.8.0 仍为 **ALPHA 阶段**, 仅供开发/测试使用, 切勿用于生产数据

---

## 0. 30 秒总览 (TL;DR)

| 项目 | 状态 |
|------|------|
| 当前阶段 | **ALPHA** (未达 GA) |
| Binary 名称 | `sqlrustgo-mysql-server` (v3.8.0 canonical) |
| 默认端口 | 3306 (与 MySQL 5.7/8.0 兼容) |
| 协议 | MySQL wire protocol (兼容标准 mysql client) |
| 架构 | x86_64 Linux / ARM64 (Apple Silicon 需自编译) |
| 首个 PR 路径 | 本文档位于 `docs/releases/v3.8.0/QUICK_START.md` |

**v3.8.0 = Architecture Unification Release** — 16 个 feature 全部 CLOSED,
9 维门禁 7/8 PASS, 0 FAIL. 但距离 MySQL 5.7 生产替代仍需 12-18 个月
(详见 `V380_COMPREHENSIVE_ASSESSMENT.md`).

---

## 1. 获取 Binary (2 种方式)

### 方式 A: 下载预编译 (推荐)

```bash
# 暂未提供官方预编译包 (ALPHA 阶段, 见 P0-2 行动项)
# 计划: v3.8.0-RC1 起在 Gitea Release 提供
#   http://192.168.0.252:3000/openclaw/sqlrustgo/releases

# 占位命令 (待 Gitea Release 启用后填充)
# curl -L -o sqlrustgo-mysql-server \
#   http://192.168.0.252:3000/openclaw/sqlrustgo/releases/download/v3.8.0/sqlrustgo-mysql-server-x86_64-unknown-linux-gnu
# chmod +x sqlrustgo-mysql-server
```

### 方式 B: 从源码编译 (当前唯一可用方式)

```bash
# 1. 克隆仓库
git clone https://github.com/your-fork/sqlrustgo.git
cd sqlrustgo

# 2. 切换到 v3.8.0 分支
git checkout origin/develop/v3.8.0

# 3. 编译 canonical binary (release 模式, 约 5-10 分钟)
cargo build --release -p sqlrustgo-mysql-server

# 4. 验证编译产物
ls -lh target/release/sqlrustgo-mysql-server
./target/release/sqlrustgo-mysql-server --version
```

**预期输出**:
```
sqlrustgo-mysql-server 0.1.0
```

**注意**:
- 必须使用 `--release` 模式, debug 模式性能差 10-50x
- 首次编译需联网拉取 ~400 个 crate 依赖
- Apple Silicon (M1/M2) 用户需 `rustup target add aarch64-apple-darwin`

---

## 2. 启动服务 (30 秒)

```bash
# 启动 MySQL 协议服务 (默认监听 127.0.0.1:3306)
./target/release/sqlrustgo-mysql-server serve

# 等价形式 (无 subcommand 默认 serve)
./target/release/sqlrustgo-mysql-server

# 自定义端口
./target/release/sqlrustgo-mysql-server serve --port 3307

# 自定义绑定地址 (允许远程访问, **仅开发环境**)
./target/release/sqlrustgo-mysql-server serve --host 0.0.0.0 --port 3306

# 调整日志级别
./target/release/sqlrustgo-mysql-server serve --log-level debug
```

**预期启动日志**:
```
2026-06-04T05:00:00 INFO sqlrustgo_mysql_server: SQLRustGo MySQL Server starting on 127.0.0.1:3306
2026-06-04T05:00:00 INFO sqlrustgo_mysql_server: WAL recovery started
2026-06-04T05:00:00 INFO sqlrustgo_mysql_server: WAL recovery completed, 0 records replayed
2026-06-04T05:00:00 INFO sqlrustgo_mysql_server: Server ready, listening on 127.0.0.1:3306
```

**启动失败排查**:
- `Address already in use (os error 98)` → 端口被占用, 换 `--port` 或 `lsof -i:3306` 查谁占的
- `WAL recovery failed` → 旧数据损坏, 见 `DEPLOYMENT_GUIDE.md` §4.1
- 其他 → 见 `DEPLOYMENT_GUIDE.md` §6 故障排查

---

## 3. 客户端连接 (30 秒)

```bash
# 标准 MySQL client (推荐)
mysql -h 127.0.0.1 -P 3306 -u root

# 无密码 (ALPHA 阶段默认无认证, **生产前必须修改**)
mysql -h 127.0.0.1 -P 3306 -u root --skip-password

# 指定协议版本 (避免连接错误)
mysql -h 127.0.0.1 -P 3306 -u root --protocol=TCP

# 一次性查询
mysql -h 127.0.0.1 -P 3306 -u root -e "SELECT 'hello v3.8.0' AS greeting"
```

**预期输出**:
```
Welcome to the MySQL monitor.  Commands end with ; or \g.
Your MySQL connection id is 1
Server version: 5.7.0-sqlrustgo-v3.8.0-alpha

Copyright (c) 2000, 2026, Oracle and/or its affiliates.

Type 'help;' for '\h' for help. Type '\c' to clear the current input statement.

mysql>
```

**其他客户端**:
- `mycli` / `mariadb` / `DBeaver` / `TablePlus` 均可直连 (wire protocol 兼容)
- 任何 MySQL 5.7/8.0 客户端无需修改

---

## 4. 第一个 SQL (1 分钟)

```sql
-- 4.1 创建表
CREATE TABLE users (
    id      INTEGER PRIMARY KEY,
    name    VARCHAR(64) NOT NULL,
    email   VARCHAR(128),
    age     INTEGER
);

-- 4.2 插入数据
INSERT INTO users (id, name, email, age) VALUES
    (1, 'Alice',   'alice@example.com',   30),
    (2, 'Bob',     'bob@example.com',     25),
    (3, 'Charlie', 'charlie@example.com', 35);

-- 4.3 查询
SELECT id, name, age FROM users WHERE age >= 30 ORDER BY id;

-- 4.4 聚合
SELECT COUNT(*) AS total, AVG(age) AS avg_age FROM users;

-- 4.5 退出
\q
```

**预期输出** (`SELECT` 步骤):
```
+----+---------+------+
| id | name    | age  |
+----+---------+------+
|  1 | Alice   |   30 |
|  3 | Charlie |   35 |
+----+---------+------+
2 rows in set (0.001 sec)
```

**v3.8.0 已支持的特性** (完整列表见 `FEATURE_MATRIX.md`):
- DDL: `CREATE TABLE`, `DROP TABLE`, `ALTER TABLE ADD/DROP COLUMN`
- DML: `INSERT [INTO] ... VALUES`, `UPDATE ... SET`, `DELETE FROM ... WHERE`
- 查询: `SELECT ... FROM`, `WHERE`, `GROUP BY`, `ORDER BY`, `LIMIT`, `OFFSET`
- JOIN: `INNER JOIN`, `LEFT JOIN`, 多表 (3-way 已验证)
- 聚合: `COUNT`, `SUM`, `AVG`, `MIN`, `MAX`
- 事务: `BEGIN`, `COMMIT`, `ROLLBACK`, 4 隔离级别

**已知限制** (v3.8.0 ALPHA):
- ❌ 窗口函数 (未实现, 见 ARCH-SEM 债务)
- ❌ 物化视图 / 存储过程
- ⚠️ 子查询: 简单 IN/EXISTS 支持, 相关子查询部分场景不通过
- ⚠️ TPC-H Q1-Q22 实测 10/22 (45%) — 复杂分析查询不保证正确性

---

## 5. 关闭服务 (10 秒)

```bash
# 前台运行: Ctrl+C
# 后台运行: 找到 PID 后 kill
ps aux | grep sqlrustgo-mysql-server
kill <PID>

# 优雅关闭 (处理完活跃事务, 刷 WAL)
kill -SIGTERM <PID>
```

**关闭时自动执行**:
- 等待活跃事务完成 (最多 30 秒)
- 刷 WAL buffer 到磁盘
- 关闭所有 client 连接
- 释放 buffer pool

**强制关闭** (不推荐, 可能丢数据):
```bash
kill -SIGKILL <PID>  # 仅当服务无响应时使用
```

---

## 6. 数据持久化 (默认位置)

| 类型 | 默认路径 | 说明 |
|------|----------|------|
| 数据文件 | `./data/` (相对启动目录) | B+Tree page 文件, 16KB/page |
| WAL 文件 | `./wal/` | Write-Ahead Log, 用于 crash recovery |
| 元数据 | `./catalog/` | 表结构/索引定义 |
| 临时文件 | `/tmp/sqlrustgo-*` | spill 临时文件 |

**数据迁移**: 关闭服务后, 整体打包 `data/ wal/ catalog/` 三个目录即可
**清理数据**: 删 `data/ wal/ catalog/` 后重启, 即恢复空库状态

---

## 7. 性能预期 (未实测, 估算)

> ⚠️ **重要**: 以下为设计估算, **未在 v3.8.0 真实跑过基准测试**
> 真实数据需 v3.8.0 sysbench + TPC-H SF=0.1 基准 (P0-3 行动项)

| 场景 | v3.8.0 估算 | v2.4.0 基准 | 对比 |
|------|------------|-------------|------|
| TPC-H Q1 (聚合) | ~70 µs | 74 µs | 持平 |
| 点查 (聚簇索引 F-23) | +20-50% | 1.0x | **提升** |
| 热数据读 (AHI F-24) | +100-1000% | 1.0x | **大幅提升** |
| 二级索引写 (ChangeBuf F-25) | +30-50% | 1.0x | **提升** |
| 写安全 (DoubleWrite F-26) | -5-10% | 1.0x | 略降 |

详细性能分析见 `V380_COMPREHENSIVE_ASSESSMENT.md` §7.

---

## 8. 故障排查 (Troubleshooting)

| 症状 | 可能原因 | 解决方案 |
|------|----------|----------|
| 编译失败 `linker not found` | 缺 gcc / build-essential | `apt install build-essential` (Ubuntu) |
| 编译失败 `openssl-sys` | 缺 libssl-dev | `apt install libssl-dev pkg-config` |
| 启动失败 `Address in use` | 3306 被占 | `lsof -i:3306` 或换端口 `--port 3307` |
| 连接失败 `Access denied` | v3.8.0 ALPHA 无认证, 但客户端要求 | 加 `--skip-password` 或环境变量 `MYSQL_PWD=` |
| 查询返回 `Not implemented` | 特性未实现 | 见 `FEATURE_MATRIX.md` 查支持列表 |
| WAL recovery 失败 | 上次异常关闭, 数据不一致 | 见 `DEPLOYMENT_GUIDE.md` §4.1 修复流程 |
| 性能远低于预期 | debug 模式 | 重新 `cargo build --release` |

**完整排查指南**: `DEPLOYMENT_GUIDE.md` (本目录, 如未生成请到根目录 `docs/DEPLOYMENT_GUIDE.md`)

**快速求助**:
- Gitea Issues: http://192.168.0.252:3000/openclaw/sqlrustgo/issues
- 必带信息: `sqlrustgo-mysql-server --version` 输出 + 启动日志 + 复现 SQL

---

## 9. 下一步 (Next Steps)

**试用完成后建议**:
1. 跑通 Beta E2E: `cargo test --test e2e_query_test --release`
2. 查看功能矩阵: `docs/releases/v3.8.0/FEATURE_MATRIX.md`
3. 阅读发布说明: `docs/releases/v3.8.0/RELEASE_NOTES.md`
4. 评估升级路径: `docs/releases/v3.8.0/MIGRATION_GUIDE.md` (v3.7.0 → v3.8.0)
5. 深入架构: `docs/releases/v3.8.0/design/ARCHITECTURE.md`

**生产部署前必读**:
- ⚠️ v3.8.0 **不适合**生产环境
- ⚠️ 数据无异地备份机制
- ⚠️ 无高可用 / 主从复制
- ⚠️ SQL 兼容度约 50-60% (vs MySQL 5.7)

**贡献代码**:
- 阅读 `docs/governance/AI_COLLABORATION.md` 了解协作规范
- 阅读 `docs/governance/ISSUE_CLOSING_VERIFICATION.md` 了解 Issue 关闭流程
- 阅读 `docs/governance/DOC_CHECK_CORRECTION_RULES.md` 了解文档修改流程

---

## 10. 命令速查 (Cheat Sheet)

```bash
# 编译
cargo build --release -p sqlrustgo-mysql-server

# 启动 (前台)
./target/release/sqlrustgo-mysql-server serve

# 启动 (后台 + 日志)
nohup ./target/release/sqlrustgo-mysql-server serve > server.log 2>&1 &

# 客户端连接
mysql -h 127.0.0.1 -P 3306 -u root

# 单条 SQL (in-process, 无需 server)
./target/release/sqlrustgo-mysql-server exec "SELECT 1+1"

# 交互式 REPL
./target/release/sqlrustgo-mysql-server repl

# 测试
cargo test --all-features -p sqlrustgo-mysql-server
cargo test --release --test e2e_query_test

# 门禁脚本
bash scripts/gate/check_rc_ga_gate.sh
bash scripts/gate/check_test_inventory.sh
bash scripts/gate/check_int_debt.sh
bash scripts/gate/check_arch_sem_debt.sh
bash scripts/gate/check_full_gate_verification.sh
```

---

## 附录: 关于本文档

| 项目 | 值 |
|------|-----|
| 文档版本 | v3.8.0-QUICK_START-1.0 |
| 最后更新 | 2026-06-04 |
| 维护者 | SQLRustGo 文档团队 |
| 状态 | ACTIVE |
| 反馈 | http://192.168.0.252:3000/openclaw/sqlrustgo/issues |
| 依赖 | `V380_COMPREHENSIVE_ASSESSMENT.md`, `FEATURE_MATRIX.md`, `RELEASE_NOTES.md` |

---

**祝你试用愉快!** v3.8.0 仍在快速迭代, 欢迎在 Gitea 提 Issue 反馈.

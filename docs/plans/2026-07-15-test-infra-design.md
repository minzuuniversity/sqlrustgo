# 测试基础设施改进设计方案

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 提升 5 个 focus crate 的测试覆盖率至 80%，通过三项基础设施改进实现

**Architecture:** 三项改进顺序依赖：EphemeralServerPool → WireClient测试 → tools I/O trait

---

## 1. EphemeralServerPool（静态端口分配池）

### 目标
将 46 个 e2e 测试的运行时间从 120-180s 降到 ~30s

### 架构

```rust
// crates/mysql-server/src/pool.rs - 新文件
pub struct EphemeralServerPool {
    servers: [Option<ServerHandle>; 4],  // 端口 9001-9004
}

impl EphemeralServerPool {
    /// 获取或启动 server 实例
    pub fn acquire(&self, port: u16) -> Result<PooledServer> {
        let idx = (port - 9001) as usize;
        if self.servers[idx].is_none() {
            self.servers[idx] = Some(start_ephemeral(port)?);
        }
        Ok(PooledServer { port })
    }
}

pub struct PooledServer {
    port: u16,
}
```

- 预启动 4 个 server 实例到固定端口 9001-9004
- 每个测试结束后 server 继续运行，不关闭
- 新测试 connect 到已有端口直接复用
- 现有 `start_ephemeral()` 调用改为 `Pool::acquire(port)`

### 覆盖目标
- mysql-server: 42% → 55%+（更多测试场景）
- admin wire_client: 14% → 60%+（通过集成测试）

---

## 2. WireClient 集成测试

### 目标
将 `wire_client.rs` 覆盖率从 14% 提升到 60%+

### 架构

```rust
// crates/admin/tests/wire_client_integration_tests.rs
#[test]
fn test_wire_client_ping() {
    let pool = EphemeralServerPool::global();
    let server = pool.acquire(9001).unwrap();
    let mut client = WireClient::connect("127.0.0.1", server.port, "root", "", "test").unwrap();
    let result = client.ping();
    assert!(result.is_ok());
}
```

- 新建 `tests/wire_client_integration_tests.rs`
- 利用 EphemeralServerPool 中的真实 server 做集成测试
- 覆盖 `ping/version/status/logical_backup` 四个方法

### 测试场景
- `test_wire_client_ping` - COM_PING
- `test_wire_client_version` - 版本查询
- `test_wire_client_status` - 状态报告
- `test_wire_client_logical_backup` - 逻辑备份
- 连接错误路径（已在 admin_wire_client_tests.rs 覆盖）

---

## 3. tools 全局 I/O trait

### 目标
将 `backup_restore.rs` 覆盖率从 57% 提升到 80%+

### 架构

```rust
// crates/tools/src/traits.rs - 新文件
pub trait SqlRustGoIo: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, BackupError>;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), BackupError>;
    fn create_dir_all(&self, path: &Path) -> Result<(), BackupError>;
    fn remove_dir_all(&self, path: &Path) -> Result<(), BackupError>;
    fn open_tar_read(&self, path: &Path) -> Result<tar::Archive<fs::File>, BackupError>;
    fn exists(&self, path: &Path) -> bool;
}

pub struct RealIo;
impl SqlRustGoIo for RealIo { /* 使用 std::fs */ }

pub struct TestIo<F> {
    read_file: F,
}
```

### 文件修改
- 创建: `crates/tools/src/traits.rs`
- 修改: `crates/tools/src/backup_restore.rs` - 注入 trait
- 修改: `crates/tools/src/lib.rs` - 导出 trait
- 创建: `crates/tools/tests/backup_restore_io_tests.rs` - mock 测试

### 错误注入场景
- 文件不存在
- 目录创建失败（权限拒绝）
- tar 解析失败（损坏文件）
- 磁盘满（写入失败）

---

## 实现顺序

1. **EphemeralServerPool** - 基础组件，是 2 的依赖
2. **WireClient 集成测试** - 依赖 1
3. **tools I/O trait** - 独立，可并行

## Tech Stack

- Rust 2024 + Tokio
- `tar` crate for archive operations
- `tempfile` for test fixtures
- 现有 `EphemeralConfig` / `start_ephemeral()` 基础设施

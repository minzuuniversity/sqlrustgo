# 测试基础设施改进实现计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 实现 EphemeralServerPool、WireClient集成测试、tools I/O trait 三项改进

**Architecture:** 顺序实现：Pool → WireClient测试 → tools I/O trait

---

## 阶段 1: EphemeralServerPool

### Task 1: 创建 pool.rs 模块

**Files:**
- Create: `crates/mysql-server/src/pool.rs`

**Step 1: 创建基本结构**

```rust
use std::sync::Mutex;
use crate::executor::EphemeralConfig;

pub struct ServerHandle {
    pub port: u16,
    // 内部 shutdown channel 等
}

pub struct EphemeralServerPool {
    servers: Mutex<Vec<Option<ServerHandle>>>,
    base_port: u16,
    size: usize,
}

impl EphemeralServerPool {
    pub fn new(base_port: u16, size: usize) -> Self {
        Self {
            servers: Mutex::new(vec![None; size]),
            base_port,
            size,
        }
    }

    pub fn acquire(&self, port: u16) -> Result<ServerHandle, String> {
        let idx = (port - self.base_port) as usize;
        if idx >= self.size {
            return Err(format!("Port {} out of pool range", port));
        }
        let mut servers = self.servers.lock().unwrap();
        if servers[idx].is_none() {
            let config = EphemeralConfig {
                data_dir: None,
                host: "127.0.0.1".to_string(),
                bootstrap_users: true,
                bootstrap_tables: false,
                bootstrap_sql: Vec::new(),
                bulk_insert_buffer_size: 1_048_576,
                server_threads: 2,
                storage: None,
            };
            let handle = start_ephemeral_on_port(config, port)?;
            servers[idx] = Some(handle);
        }
        Ok(servers[idx].as_ref().unwrap().clone())
    }
}
```

**Step 2: 添加 global 实例**

```rust
use once_cell::sync::Lazy;

static SERVER_POOL: Lazy<EphemeralServerPool> = Lazy::new(|| {
    EphemeralServerPool::new(9001, 4)
});

impl EphemeralServerPool {
    pub fn global() -> &'static EphemeralServerPool {
        &SERVER_POOL
    }
}
```

**Step 3: 导出到 lib.rs**

在 `crates/mysql-server/src/lib.rs` 添加:
```rust
pub mod pool;
pub use pool::{EphemeralServerPool, ServerHandle};
```

**Step 4: 测试编译**

Run: `cargo build -p sqlrustgo-mysql-server`
Expected: OK（无错误）

### Task 2: 修改现有 e2e 测试使用 Pool

**Files:**
- Modify: `crates/mysql-server/tests/e2e_wire_protocol.rs`

**Step 1: 修改 connect helper**

```rust
// 原来:
fn connect(port: u16) -> Result<MySqlConnection, sqlrustgo_mysql_client::MySqlClientError> {
    MySqlConnection::connect(&SocketAddr::from(([127, 0, 0, 1], port)), "root", "", "test")
}

// 改为使用 pool:
fn connect(port: u16) -> Result<MySqlConnection, sqlrustgo_mysql_client::MySqlClientError> {
    let pool = EphemeralServerPool::global();
    let _handle = pool.acquire(port)?;
    MySqlConnection::connect(&SocketAddr::from(([127, 0, 0, 1], port)), "root", "", "test")
}
```

**Step 2: 运行测试验证**

Run: `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol test_e2e_create_database -- --nocapture`
Expected: PASS in ~1s（而非 40s）

---

## 阶段 2: WireClient 集成测试

### Task 3: 创建 wire_client_integration_tests.rs

**Files:**
- Create: `crates/admin/tests/wire_client_integration_tests.rs`

**Step 1: 基础 ping 测试**

```rust
use sqlrustgo_admin::wire_client::WireClient;
use sqlrustgo_mysql_server::pool::EphemeralServerPool;

#[test]
fn test_wire_client_ping() {
    let pool = EphemeralServerPool::global();
    let handle = pool.acquire(9001).unwrap();
    
    let mut client = WireClient::connect("127.0.0.1", handle.port, "root", "", "test")
        .expect("connect failed");
    let result = client.ping();
    assert!(result.is_ok(), "ping failed: {:?}", result);
}
```

**Step 2: version/status 测试**

```rust
#[test]
fn test_wire_client_version() {
    let pool = EphemeralServerPool::global();
    let handle = pool.acquire(9001).unwrap();
    
    let mut client = WireClient::connect("127.0.0.1", handle.port, "root", "", "test")
        .expect("connect failed");
    let version = client.version().expect("version failed");
    assert!(!version.is_empty());
}

#[test]
fn test_wire_client_status() {
    let pool = EphemeralServerPool::global();
    let handle = pool.acquire(9001).unwrap();
    
    let mut client = WireClient::connect("127.0.0.1", handle.port, "root", "", "test")
        .expect("connect failed");
    let status = client.status().expect("status failed");
    assert!(status.uptime_secs >= 0);
}
```

**Step 3: logical_backup 测试**

```rust
#[test]
fn test_wire_client_logical_backup() {
    use tempfile::TempDir;
    use std::path::PathBuf;
    
    let pool = EphemeralServerPool::global();
    let handle = pool.acquire(9001).unwrap();
    
    let mut client = WireClient::connect("127.0.0.1", handle.port, "root", "", "test")
        .expect("connect failed");
    
    // 创建测试表
    client.execute("CREATE TABLE IF NOT EXISTS test_backup (id INT, val TEXT)")
        .expect("create table failed");
    client.execute("INSERT INTO test_backup VALUES (1, 'hello')")
        .expect("insert failed");
    
    let tmp = TempDir::new().unwrap();
    let output_path: PathBuf = tmp.path().join("backup.csv");
    
    let result = client.logical_backup(&output_path);
    assert!(result.is_ok(), "backup failed: {:?}", result);
    assert!(output_path.exists());
}
```

**Step 4: 编译和运行**

Run: `cargo build -p sqlrustgo-admin --test wire_client_integration_tests`
Expected: OK

Run: `cargo test -p sqlrustgo-admin --test wire_client_integration_tests`
Expected: 4 tests PASS

---

## 阶段 3: tools I/O trait

### Task 4: 创建 traits.rs

**Files:**
- Create: `crates/tools/src/traits.rs`

**Step 1: 定义 SqlRustGoIo trait**

```rust
use std::path::Path;
use crate::backup::BackupError;
use tar::Archive;
use std::fs::File;

pub trait SqlRustGoIo: Send + Sync {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, BackupError>;
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), BackupError>;
    fn create_dir_all(&self, path: &Path) -> Result<(), BackupError>;
    fn remove_dir_all(&self, path: &Path) -> Result<(), BackupError>;
    fn open_tar_read(&self, path: &Path) -> Result<Archive<File>, BackupError>;
    fn exists(&self, path: &Path) -> bool;
}

pub struct RealIo;
impl SqlRustGoIo for RealIo {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, BackupError> {
        std::fs::read(path).map_err(|e| BackupError::Io(e.to_string()))
    }
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), BackupError> {
        std::fs::write(path, data).map_err(|e| BackupError::Io(e.to_string()))
    }
    fn create_dir_all(&self, path: &Path) -> Result<(), BackupError> {
        std::fs::create_dir_all(path).map_err(|e| BackupError::Io(e.to_string()))
    }
    fn remove_dir_all(&self, path: &Path) -> Result<(), BackupError> {
        std::fs::remove_dir_all(path).map_err(|e| BackupError::Io(e.to_string()))
    }
    fn open_tar_read(&self, path: &Path) -> Result<Archive<File>, BackupError> {
        let file = File::open(path).map_err(|e| BackupError::Io(e.to_string()))?;
        Ok(Archive::new(file))
    }
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }
}
```

**Step 2: 导出到 lib.rs**

在 `crates/tools/src/lib.rs` 添加:
```rust
pub mod traits;
pub use traits::{SqlRustGoIo, RealIo};
```

**Step 3: 编译验证**

Run: `cargo build -p sqlrustgo-tools`
Expected: OK

### Task 5: 修改 backup_restore.rs 注入 trait

**Files:**
- Modify: `crates/tools/src/backup_restore.rs`

**Step 1: 添加 Io 参数到函数**

```rust
pub fn physical_restore(
    input: &Path,
    target_dir: &Path,
    io: &dyn SqlRustGoIo,
) -> Result<RestoreResult, BackupError> {
    if !io.exists(input) {
        return Err(BackupError::EntryNotFound(input.to_string_lossy().to_string()));
    }
    let archive = io.open_tar_read(input)?;
    io.create_dir_all(target_dir)?;
    // ... 解压逻辑
}
```

**Step 2: 添加默认参数**

```rust
impl physical_restore {
    pub fn new(input: &Path, target_dir: &Path) -> Result<RestoreResult, BackupError> {
        physical_restore(input, target_dir, &RealIo)
    }
}
```

**Step 3: 编译验证**

Run: `cargo build -p sqlrustgo-tools`
Expected: OK

### Task 6: 创建 backup_restore_io_tests.rs

**Files:**
- Create: `crates/tools/tests/backup_restore_io_tests.rs`

**Step 1: Mock Io 实现**

```rust
use sqlrustgo_tools::{SqlRustGoIo, BackupError};
use std::path::Path;
use std::sync::Arc;
use std::cell::RefCell;

pub struct MockIo {
    pub read_file_result: Result<Vec<u8>, BackupError>,
    pub create_dir_error: Option<BackupError>,
    pub tar_open_error: Option<BackupError>,
}

impl SqlRustGoIo for MockIo {
    fn read_file(&self, path: &Path) -> Result<Vec<u8>, BackupError> {
        self.read_file_result.clone()
    }
    fn write_file(&self, path: &Path, data: &[u8]) -> Result<(), BackupError> {
        Ok(())
    }
    fn create_dir_all(&self, path: &Path) -> Result<(), BackupError> {
        self.create_dir_error.clone().unwrap_or(Ok(()))
    }
    fn remove_dir_all(&self, path: &Path) -> Result<(), BackupError> {
        Ok(())
    }
    fn open_tar_read(&self, path: &Path) -> Result<tar::Archive<std::fs::File>, BackupError> {
        self.tar_open_error.clone()?;
        Err(BackupError::Io("mock".to_string()))
    }
    fn exists(&self, path: &Path) -> bool {
        true
    }
}

#[test]
fn test_physical_restore_file_not_found() {
    use sqlrustgo_tools::backup_restore::physical_restore;
    
    let mock = MockIo {
        read_file_result: Err(BackupError::EntryNotFound("test.tar.gz".to_string())),
        create_dir_error: None,
        tar_open_error: None,
    };
    
    let result = physical_restore(
        Path::new("/nonexistent/backup.tar.gz"),
        Path::new("/tmp/restore"),
        &mock,
    );
    assert!(result.is_err());
}
```

**Step 2: 更多错误路径测试**

```rust
#[test]
fn test_physical_restore_create_dir_failed() {
    let mock = MockIo {
        read_file_result: Ok(vec![]),
        create_dir_error: Some(BackupError::Io("Permission denied".to_string())),
        tar_open_error: None,
    };
    
    let result = physical_restore(
        Path::new("/tmp/backup.tar.gz"),
        Path::new("/root/forbidden"),
        &mock,
    );
    assert!(result.is_err());
}
```

**Step 3: 编译和运行**

Run: `cargo test -p sqlrustgo-tools --test backup_restore_io_tests`
Expected: 3+ tests PASS

---

## 验证阶段

### Task 7: 测量覆盖率提升

Run: `cargo llvm-cov -p sqlrustgo-mysql-server --lib` → 期望 44% → 55%+
Run: `cargo llvm-cov -p sqlrustgo-admin --tests` → 期望 68% → 75%+
Run: `cargo llvm-cov -p sqlrustgo-tools --lib` → 期望 59% → 70%+

### Task 8: 性能验证

Run: `cargo test -p sqlrustgo-mysql-server --test e2e_wire_protocol`
Expected: 46 tests in ~30s（而非 120-180s）

---

## 提交信息

```
feat(test-infra): add EphemeralServerPool for faster e2e tests

- Add EphemeralServerPool with 4 pre-started instances on ports 9001-9004
- Modify e2e_wire_protocol.rs to use pool for server instances
- Add wire_client_integration_tests with real server tests
- Add SqlRustGoIo trait to tools crate for testable I/O
- Add backup_restore_io_tests with mock error injection
```

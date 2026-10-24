//! #5571 —— 测试临时路径的**唯一化**（本 crate 内唯一的一份实现）
//!
//! # 问题
//!
//! storage 的测试把工作目录放在**固定全局路径**上，例如
//!
//! ```ignore
//! let tmp = std::env::temp_dir().join("bin_test_float"); // → /tmp/bin_test_float
//! ```
//!
//! 固定路径是**全机共享**的。同一台宿主机上并发跑两个测试进程会互相覆盖
//! 对方的目录 ⇒ **结构性的随机红**（单独跑必绿、并发跑必红）。
//!
//! CI 上这个碰撞是必然的，不是概率问题：
//!
//! * `test-crates` job 跑 `cargo test --workspace --exclude sqlrustgo`
//!   ⇒ 会跑 `sqlrustgo_storage` 的 lib 测试二进制；
//! * `postcheck` job 跑 AFP 门禁，门禁用 `test-runner` 跑**同一个**
//!   `target/release/sqlrustgo_storage` 二进制。
//!
//! 两个 job 在同一台 runner 上并行 ⇒ 两份进程同时写 `/tmp/bin_test_float`。
//! `postcheck` 是 GA 的必经门禁，所以这条路径上 GA 会被随机卡住。
//!
//! # 规则（三条，缺一不可）
//!
//! 1. **唯一** —— 路径里带「进程 id + 进程内随机 token + 单调计数器」。
//!    并发的两个进程不可能取到同一个路径；同一进程内的不同测试也不会互相踩。
//! 2. **开头清理** —— 返回前先删掉同名残骸 ⇒ 覆盖「上次被 kill / OOM /
//!    机器重启」留下的目录，残骸不会污染本次测试。
//! 3. **返回时不存在** —— 路径交到测试手上时**还没有被创建**，由测试自己
//!    创建（`BinaryTableStorage::new` 内部会 `create_dir_all`）。
//!    这与修复前的语义完全一致，所以不需要改动任何断言。
//!
//! # 一份实现
//!
//! [`unique_test_path`] 是本 crate 里**唯一**的临时路径生成器：
//!
//! * 单元测试（`src/**.rs` 的 `mod tests`）用
//!   `crate::test_support::unique_test_path`；
//! * 集成测试（`tests/**.rs`）用
//!   `sqlrustgo_storage::test_support::unique_test_path`。
//!
//! 集成测试链接库时没有 `cfg(test)`，所以本模块和
//! [`fault_injection`](crate::fault_injection) 一样走 feature 开关：
//! `Cargo.toml` 里的 **self dev-dependency** 打开 `test-support`，
//! 于是「单元测试」与「集成测试」拿到的是**同一份**实现。
//!
//! # 与 #5478 的关系
//!
//! #5478 讲的是故障注入的**进程级 env var**；本模块讲的是**临时路径**的
//! 全局共享。是同一族问题（测试之间不隔离），但不是同一处。

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::LazyLock;

/// 同一进程内的单调计数器 —— 区分同一进程内的不同调用点。
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// 每个进程一个随机 token。
///
/// [`RandomState`] 的种子来自操作系统熵源，因此**不同进程**拿到的 token
/// 不同。这条很重要：CI 里不同 job 可能跑在**不同的 PID namespace**（各自的
/// `getpid()` 都是同一个小数字），单靠 pid 并不能区分它们。
static PROCESS_TOKEN: LazyLock<u64> = LazyLock::new(|| {
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(std::process::id() as u64);
    hasher.finish()
});

/// 生成一个**唯一**的临时路径，返回时该路径**尚不存在**。
///
/// * `prefix` —— 路径前缀，保留它是为了让崩溃现场的目录名还能看出是哪个测试
///   留下的（例如 `bin_test_float`）。
/// * 目录或文件都可以：返回前会把同名的**目录和文件**都清掉，所以调用方可以
///   放心地把它交给 `create_dir_all` / `File::create`。
///
/// # Examples
///
/// ```ignore
/// let tmp = unique_test_path("bin_test_float");
/// let storage = BinaryTableStorage::new(tmp.clone()).unwrap();
/// // … 测试逻辑 …
/// std::fs::remove_dir_all(&tmp).ok();
/// ```
pub fn unique_test_path(prefix: &str) -> PathBuf {
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "sqlrustgo_{prefix}_{}_{}_{seq}",
        std::process::id(),
        *PROCESS_TOKEN
    ));
    // 规则 2：开头清理，覆盖上次被中断留下的残骸。
    remove_any(&path);
    path
}

/// 清掉 `path` 上的残骸 —— 目录和文件两种都试。
///
/// `remove_dir_all` 对文件报错、`remove_file` 对目录报错，两次都忽略错误即可。
fn remove_any(path: &Path) {
    let _ = std::fs::remove_dir_all(path);
    let _ = std::fs::remove_file(path);
}

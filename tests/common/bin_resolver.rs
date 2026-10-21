//! 统一的「外部可执行文件定位」——#5406 / #5477。
//!
//! c6-allow-home-paths — 本文件**故意**包含机器相关的样本路径：
//! 它是 C6 门禁（`scripts/gate/check_test_manifest_honesty.sh`）的检测器
//! 本身，必须带着样本才能自证有判别力。
//!
//! # 为什么这个模块存在
//!
//! 本仓库里有两代写法，都坏了：
//!
//! 1. **硬编码某台开发机的绝对路径**
//!    ```text
//!    /Users/liying/dev/sqlrustgo/target/debug/sqlrustgo
//!    /home/openclaw/sqlrustgo_work/tests/data/tpch-sf01
//!    ```
//!    这条路径**只在一台机器上存在**。别人 clone 下来、或 CI 换一台 runner，
//!    它就不存在了。于是测试不是「失败」，而是**静默降级**：找不到二进制 ⇒
//!    「跳过 ⇒ 通过」；找不到夹具 ⇒ 「跑空表 ⇒ 断言为空 ⇒ 通过」。
//!    #5406 记录的正是这个：36 个差分用例 0.00s「通过」，一次比对都没做。
//!
//! 2. **硬编码 `<workspace>/target/debug/<bin>`**
//!    ```rust
//!    let mut p = workspace_root();
//!    p.push("target"); p.push("debug"); p.push("sqlrustgo-mysql-server");
//!    ```
//!    这条路径**忽略 `CARGO_TARGET_DIR`**。只要构建产物落在别处（共享
//!    target dir、隔离构建、CI 缓存分层），测试就必然报「binary not found」，
//!    而失败信息还会误导成「你没 build 过」。#5477 记录了这个。
//!
//! # 本模块的规则
//!
//! 定位顺序（**从最具体到最宽泛**）：
//!
//! 1. **显式环境变量**（如 `SQLRUSTGO_BIN`）——最高优先级，调用方说了算。
//! 2. **`CARGO_TARGET_DIR`** —— cargo 真正把产物放的地方。
//!    profile 依次试 `debug`、`release`。
//! 3. **`<package-root>/target/{debug,release}`** —— `CARGO_TARGET_DIR` 未设时的默认。
//! 4. **`PATH`** —— 已安装到系统里的二进制。
//!
//! **任何一步都不得回退到某个开发机的家目录路径。** 家目录路径是「这台机器
//! 专属」的证据 —— 它出现在哪，哪儿就会在别的机器上假绿。
//!
//! # 为什么「找不到」必须是错误而不是「跳过」
//!
//! 返回 `None` 的含义是「**这个测试现在跑不了**」，调用方必须把它变成
//! **明确的错误**或 **NOT-RUN 披露**，绝不能变成「通过」。
//! [`missing_binary_message`] 给出的错误文本会点名该设哪个环境变量 ——
//! 读者拿到的是「该做什么」，而不是一个空结果。

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// 引擎 CLI 二进制的环境变量名。
pub const ENGINE_BIN_ENV: &str = "SQLRUSTGO_BIN";

/// SQLite CLI 二进制的环境变量名。
pub const SQLITE_BIN_ENV: &str = "SQLITE_BIN";

/// MySQL server 二进制的环境变量名。
pub const MYSQL_SERVER_BIN_ENV: &str = "SQLRUSTGO_MYSQL_SERVER_BIN";

/// cargo 的构建 profile 目录，按此顺序尝试。
const PROFILES: [&str; 2] = ["debug", "release"];

/// 定位引擎 CLI（`sqlrustgo` / `sqlrustgo-cli`）。
///
/// 依次尝试环境变量 [`ENGINE_BIN_ENV`]、`CARGO_TARGET_DIR`、包根 `target/`、`PATH`。
pub fn resolve_engine_binary() -> Option<PathBuf> {
    resolve_binary(ENGINE_BIN_ENV, &["sqlrustgo-cli", "sqlrustgo"])
}

/// 定位 SQLite CLI（`sqlite3`）。
///
/// 依次尝试环境变量 [`SQLITE_BIN_ENV`]、`CARGO_TARGET_DIR`、包根 `target/`、`PATH`。
pub fn resolve_sqlite_binary() -> Option<PathBuf> {
    resolve_binary(SQLITE_BIN_ENV, &["sqlite3"])
}

/// 定位 MySQL server 二进制（`sqlrustgo-mysql-server`）。
///
/// 依次尝试环境变量 [`MYSQL_SERVER_BIN_ENV`]、`CARGO_TARGET_DIR`、包根 `target/`、`PATH`。
pub fn resolve_mysql_server_binary() -> Option<PathBuf> {
    resolve_binary(MYSQL_SERVER_BIN_ENV, &["sqlrustgo-mysql-server"])
}

/// 二进制缺失时的统一错误文本。
///
/// 错误文本必须点名**该设哪个环境变量** —— 读者拿到的是「该做什么」，
/// 而不是一句无法行动的「not found」。
pub fn missing_binary_message(which: &str, env_var: &str) -> String {
    format!(
        "{which} binary not found — set {env_var} to its absolute path, \
         or build it first (`cargo build -p sqlrustgo-cli` for the engine, \
         `cargo build -p sqlrustgo-mysql-server` for the MySQL server)"
    )
}

/// 通用定位逻辑。
///
/// `names` 是**候选文件名**，按顺序尝试（第一个存在的胜出）。
/// `env_var` 里的显式路径**优先于一切**。
fn resolve_binary(env_var: &str, names: &[&str]) -> Option<PathBuf> {
    // 1. 显式环境变量：调用方说了算，最优先。
    if let Ok(raw) = std::env::var(env_var) {
        let raw = raw.trim();
        if !raw.is_empty() {
            let path = PathBuf::from(raw);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    let mut candidates: Vec<PathBuf> = Vec::new();

    // 2. CARGO_TARGET_DIR —— cargo 真正把产物放的地方（#5477 的要害）。
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        let dir = dir.trim();
        if !dir.is_empty() {
            let base = PathBuf::from(dir);
            for name in names {
                for profile in PROFILES {
                    candidates.push(base.join(profile).join(name));
                }
            }
        }
    }

    // 3. 包根 target/{debug,release} —— 未设 CARGO_TARGET_DIR 时的默认。
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for name in names {
        for profile in PROFILES {
            candidates.push(manifest_dir.join("target").join(profile).join(name));
        }
    }

    // 4. PATH —— 系统里已安装的二进制。
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':').filter(|d| !d.is_empty()) {
            for name in names {
                candidates.push(PathBuf::from(dir).join(name));
            }
        }
    }

    candidates.into_iter().find(|c| c.is_file())
}

/// 定位失败时的完整诊断（供测试断言用）。
///
/// 返回 `None` 当且仅当 [`resolve_binary`] 也返回 `None`。
pub fn describe_resolution(env_var: &str, names: &[&str]) -> Option<String> {
    resolve_binary(env_var, names).map(|p| p.display().to_string())
}

/// 判断一个路径是否位于某个用户的家目录之下。
///
/// 这是 C6 门禁（`scripts/gate/check_test_manifest_honesty.sh`）在
/// **Rust 侧**的等价判据：家目录路径只在一台机器上存在，测试里出现它
/// 就是「这台机器专属」的证据。
pub fn is_home_directory_path(path: &str) -> bool {
    let p = Path::new(path);
    let mut components = p.components();
    let Some(std::path::Component::RootDir) = components.next() else {
        return false;
    };
    match components.next() {
        Some(std::path::Component::Normal(first)) => {
            let first = first.to_string_lossy();
            first == "Users" || first == "home"
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_directory_detection_matches_the_gate_criterion() {
        // 家目录（两种常见布局）
        assert!(is_home_directory_path(
            "/home/ai/sqlrustgo/target/debug/sqlrustgo"
        ));
        assert!(is_home_directory_path(
            "/Users/liying/dev/sqlrustgo/target/debug/x"
        ));
        // 家目录下的深层路径
        assert!(is_home_directory_path(
            "/home/openclaw/tpch_baseline/sf1/lineitem.tbl"
        ));
        // 非家目录
        assert!(!is_home_directory_path("/tmp/tpch-sf1/lineitem.tbl"));
        assert!(!is_home_directory_path("tests/data/tpch-sf01/lineitem.tbl"));
        assert!(!is_home_directory_path("/var/tmp/tpch-sf1/lineitem.tbl"));
        assert!(!is_home_directory_path("/opt/sqlrustgo/target/debug/x"));
        // 相对路径不是绝对家目录路径
        assert!(!is_home_directory_path("home/ai/sqlrustgo"));
        assert!(!is_home_directory_path("./home/ai/sqlrustgo"));
    }

    #[test]
    fn explicit_env_var_wins_over_every_other_candidate() {
        // 造一个临时的「假二进制」，用环境变量指过去。
        let dir = std::env::temp_dir().join(format!("bin-resolver-env-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let fake = dir.join("sqlrustgo-cli");
        std::fs::write(&fake, b"#!/bin/sh\nexit 0\n").expect("write fake binary");
        let fake = fake.display().to_string();

        // 注意：环境变量是**进程级**的，测试并行时可能互相干扰。
        // 这里只断言「显式路径被选中」，不依赖其他候选是否存在。
        std::env::set_var(ENGINE_BIN_ENV, &fake);
        let resolved = resolve_engine_binary();
        std::env::remove_var(ENGINE_BIN_ENV);

        let resolved = resolved.expect("explicit env var must resolve");
        assert_eq!(
            resolved.display().to_string(),
            fake,
            "an explicit {} must win over every other candidate",
            ENGINE_BIN_ENV
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_binary_message_names_the_env_var_to_set() {
        let msg = missing_binary_message("engine", ENGINE_BIN_ENV);
        assert!(
            msg.contains("not found"),
            "message must say it is missing: {msg}"
        );
        assert!(
            msg.contains(ENGINE_BIN_ENV),
            "message must name the env var to set: {msg}"
        );
    }
}

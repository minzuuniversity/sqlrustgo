//! 统一的「外部可执行文件定位」——#5406 / #5477 / #5500。
//!
//! c6-allow-home-paths — 本文件**故意**包含机器相关的样本路径：
//! 它是 C6 门禁（`scripts/gate/check_test_manifest_honesty.sh`）的检测器
//! 本身，必须带着样本才能自证有判别力。
//!
//! c7-allow-external-tool-paths — 同理，它是 C7 门禁
//! （`scripts/gate/check_test_external_tool_paths.sh`）的 Rust 侧等价物：
//! `MACHINE_SPECIFIC_INSTALL_ROOTS` **必须**把那些安装根逐字写出来，
//! 否则它就检测不到它们。这条豁免只覆盖 R1（安装根），
//! R2（字面量凭据）在本文件**依然生效** —— 下面的自证样例因此
//! 必须运行期拼接，不能把完整形态写进源码。
//!
//! cli-bin-allow-samples — 本文件同时是 C3 门禁
//! （`scripts/gate/check_cli_binary_names.sh`）的**检测器**：
//! 它必须带着**故意的**样本路径（`target/debug/x`）才能自证有判别力。
//! C3 判定的是「代码里的 `target/<profile>/<name>` 字面量指向一个
//! 真实存在的二进制」，而本文件里的 `x` 是**故意的假名** —— 它不解析
//! 任何东西，只用来证明检测器抓得到东西。
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
///
/// 这**也是**规范 CLI（见 [`CANONICAL_CLI_BIN`]）的环境变量名 ——
/// 两者是同一个二进制、同一个变量。#5405 的教训：同一个二进制一旦有了
/// 两个环境变量，就等于有了两个来源，迟早会指向不同的东西。
pub const ENGINE_BIN_ENV: &str = "SQLRUSTGO_BIN";

/// 规范 CLI 二进制的**文件名** —— #5405。
///
/// 产出它的是 package `sqlrustgo-cli`（`crates/sqlrustgo-cli`），
/// 它的 `[[bin]] name = "sqlrustgo"`。
///
/// ⚠️ `sqlrustgo-cli` 是**包名**，不是二进制名。仓库里**没有任何** bin
/// 叫 `sqlrustgo-cli`。把它当候选名，解析器会永远落空；更糟的是它让
/// 「这个二进制到底叫什么」出现了**第二个来源**：包名被抄进测试当二进制名，
/// 测试就会去找一个不存在的文件，然后以 `binary not found` 的形式炸掉
/// —— #5405 记录的就是这个（测试找 `target/debug/sqlrustgo-cli`，
/// 而真正产出它的 bin 叫 `sqlrustgo`）。
pub const CANONICAL_CLI_BIN: &str = "sqlrustgo";

/// SQLite CLI 二进制的环境变量名。
pub const SQLITE_BIN_ENV: &str = "SQLITE_BIN";

/// MySQL server 二进制的环境变量名。
pub const MYSQL_SERVER_BIN_ENV: &str = "SQLRUSTGO_MYSQL_SERVER_BIN";

/// MySQL server 的**二进制名** —— #5405。
///
/// 它由 package `sqlrustgo-mysql-server` 产出（`[[bin]] name`）。
pub const MYSQL_SERVER_BIN_NAME: &str = "sqlrustgo-mysql-server";

/// PostgreSQL 客户端（`psql`）的环境变量名 —— #5500。
///
/// `psql` 是**外部工具**，不是本仓库的构建产物：它由包管理器安装，
/// 位置随机器而变（homebrew 的 `/opt/homebrew/opt/postgresql@16/bin/psql`、
/// Debian 的 `/usr/lib/postgresql/16/bin/psql`、发行版的 `/usr/bin/psql`……）。
/// 把它写死成某一个绝对路径，就等于「只在一台机器上能跑」。
pub const PSQL_BIN_ENV: &str = "SQLRUSTGO_PSQL_BIN";

/// MySQL 客户端（`mysql`）的环境变量名 —— #5500。
///
/// 同 [`PSQL_BIN_ENV`]：外部工具，位置随机器而变。
pub const MYSQL_BIN_ENV: &str = "SQLRUSTGO_MYSQL_BIN";

/// SQLite CLI 的**二进制名** —— #5405。
pub const SQLITE_BIN_NAME: &str = "sqlite3";

/// MySQL 客户端的**二进制名** —— #5405。
pub const MYSQL_BIN_NAME: &str = "mysql";

/// PostgreSQL 客户端的**二进制名** —— #5405。
pub const PSQL_BIN_NAME: &str = "psql";

/// 外部数据库客户端的二进制名清单 —— #5405。
///
/// 它们**不是**本仓库的构建产物：由包管理器安装，位置随机器而变
/// （homebrew 的 `/opt/homebrew/opt/postgresql@16/bin/psql`、Debian 的
/// `/usr/lib/postgresql/16/bin/psql`、发行版的 `/usr/bin/psql`……）。
///
/// 这份清单是「**什么不是 workspace bin**」的**唯一**权威来源：
/// 门禁 `scripts/gate/check_cli_binary_names.sh` 直接读它，
/// 从而避免「外部工具」这个概念在门禁里也出现第二份清单
/// —— 一个判据只能有一份实现。
pub const EXTERNAL_TOOL_BINS: &[&str] = &[SQLITE_BIN_NAME, MYSQL_BIN_NAME, PSQL_BIN_NAME];

/// 单元测试**专属**的环境变量名 —— #5423 / #5405。
///
/// 进程级环境变量是**全局**的。两个用例若各改各的变量，就不会互相踩；
/// 若都去改同一个（例如 `SQLRUSTGO_BIN`），并发的用例就会偶发红灯 ——
/// 而「偶发红灯」对一个门禁来说是**最坏**的失败模式。
///
/// 这些名字**只**用于自证解析器的优先级，不承担任何生产语义。
const SELFTEST_ENGINE_ENV: &str = "SQLRUSTGO_RESOLVER_SELFTEST_ENGINE_BIN";
const SELFTEST_PSQL_ENV: &str = "SQLRUSTGO_RESOLVER_SELFTEST_PSQL_BIN";
const SELFTEST_ABSENT_ENV: &str = "SQLRUSTGO_RESOLVER_SELFTEST_ABSENT_BIN";

/// 机器专属的「安装根」前缀 —— C7 判据（#5500）的核心词表。
///
/// 这些前缀**只可能**来自某一台机器上某个软件的安装布局：
///
/// | 前缀 | 它通常意味着 |
/// |------|--------------|
/// | `/opt/homebrew/` | 某个用户的 homebrew 前缀（Apple Silicon 默认） |
/// | `/usr/local/opt/` | homebrew 在 Intel mac 上的默认前缀 |
/// | `/usr/local/bin/` | 某个用户手工安装到本地前缀的工具 |
/// | `/Applications/` | macOS 的应用安装目录 |
/// | `/Library/` | macOS 的系统级库目录 |
/// | `/usr/lib/postgresql/` | 发行版 PostgreSQL 的安装目录 |
///
/// 测试树里出现它们 ⇒ 那条路径只在一台机器上存在 ⇒ 在别的机器上
/// 「找不到 ⇒ 静默降级 ⇒ 假绿」。
///
/// ⚠️ 这与 C6（「不得以家目录为根」）是**两条独立判据**：
/// `/opt/homebrew/bin/mysql` 不是家目录路径，C6 按设计抓不到它；
/// 而 `/home/ai/x` 又不是安装根，C7 按设计抓不到它。
/// 两条判据各自的不变量都可单独陈述，混在一起就没法陈述了。
pub const MACHINE_SPECIFIC_INSTALL_ROOTS: &[&str] = &[
    "/opt/",
    "/usr/local/",
    "/Applications/",
    "/homebrew/",
    "/Library/",
    "/usr/lib/postgresql/",
];

/// 外部数据库客户端的**凭据参数**标志 —— C7-R2 的核心词表。
///
/// 出现在参数里没问题；**值是写死的字符串字面量**才有问题：
/// `-U liying` 把某台机器上的一个用户名写进了命令行。
///
/// ⚠️ `-p` **不在**这个列表里：在 `psql` 里它是**端口**，在
/// `cargo test -p` 里它是**包选择器**。`mysql` 的口令用的是**紧贴**的
/// 写法（`-psecret`，中间没有空格），那种形态由
/// [`inline_password_literals`] 单独判定。
pub const CLIENT_CREDENTIAL_FLAGS: &[&str] = &["-U", "-u", "--user", "--password"];

/// cargo 的构建 profile 目录，按此顺序尝试。
const PROFILES: [&str; 2] = ["debug", "release"];

/// 定位规范 CLI 二进制（`sqlrustgo`）—— #5405。
///
/// 依次尝试环境变量 [`ENGINE_BIN_ENV`]、`CARGO_TARGET_DIR`、包根 `target/`、`PATH`。
///
/// **这是 soak REPL 的规范入口**：
/// `sqlrustgo soak -p <port> -u <user> --pass <pass>`。
/// 它产出 tab-separated 协议（`OK\t` / `ROWS\t` / `COL\t` / `DATA\t` /
/// `ROW\t` / `ERR\t`），实现见 `crates/sqlrustgo-cli/src/lib.rs` 的
/// `run_soak_repl`。
///
/// ⚠️ 它**不是** package `sqlrustgo-soak`（`crates/cli`）。后者也有一个
/// `soak` 子命令，但那是**压测跑子**（`--duration` / `--rate` /
/// `--query-file`）：它不读 stdin，也没有 tab-separated 协议。两个
/// `soak` 是不同的东西 —— #5405 的测试要的是**这一个**。
pub fn resolve_canonical_cli_binary() -> Option<PathBuf> {
    resolve_binary(ENGINE_BIN_ENV, &[CANONICAL_CLI_BIN])
}

/// 定位引擎 CLI（`sqlrustgo`）—— #5405。
///
/// 等价于 [`resolve_canonical_cli_binary`]：两个名字指的是**同一个**
/// 二进制。旧名保留是为了不打断既有调用点，**实现只有一份**。
pub fn resolve_engine_binary() -> Option<PathBuf> {
    resolve_canonical_cli_binary()
}

/// 定位 SQLite CLI（`sqlite3`）。
///
/// 依次尝试环境变量 [`SQLITE_BIN_ENV`]、`CARGO_TARGET_DIR`、包根 `target/`、`PATH`。
pub fn resolve_sqlite_binary() -> Option<PathBuf> {
    resolve_binary(SQLITE_BIN_ENV, &[SQLITE_BIN_NAME])
}

/// 定位 MySQL server 二进制（`sqlrustgo-mysql-server`）。
///
/// 依次尝试环境变量 [`MYSQL_SERVER_BIN_ENV`]、`CARGO_TARGET_DIR`、包根 `target/`、`PATH`。
pub fn resolve_mysql_server_binary() -> Option<PathBuf> {
    resolve_binary(MYSQL_SERVER_BIN_ENV, &[MYSQL_SERVER_BIN_NAME])
}

/// 定位 PostgreSQL 客户端（`psql`）—— #5500。
///
/// 顺序：**显式环境变量** [`PSQL_BIN_ENV`] → **`PATH`**。
///
/// ⚠️ 与 [`resolve_binary`] 的差别是刻意的：外部数据库客户端**不是
/// cargo 的构建产物**，去 `CARGO_TARGET_DIR` / 包根 `target/` 里找它
/// 只会制造一条永远走不通的候选路径（看起来「找过」，实际必然落空）。
pub fn resolve_psql_binary() -> Option<PathBuf> {
    resolve_external_tool(PSQL_BIN_ENV, &[PSQL_BIN_NAME])
}

/// 定位 MySQL 客户端（`mysql`）—— #5500。
///
/// 顺序：**显式环境变量** [`MYSQL_BIN_ENV`] → **`PATH`**。
pub fn resolve_mysql_binary() -> Option<PathBuf> {
    resolve_external_tool(MYSQL_BIN_ENV, &[MYSQL_BIN_NAME])
}

/// 外部工具的通用定位逻辑：显式环境变量 → `PATH`。
///
/// `env_var` 里的显式路径**优先于一切**；找不到就返回 `None`，
/// 绝不回退到某个开发机的绝对路径。
pub fn resolve_external_tool(env_var: &str, names: &[&str]) -> Option<PathBuf> {
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

    // 2. PATH —— 系统里已安装的二进制。
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':').filter(|d| !d.is_empty()) {
            for name in names {
                let candidate = PathBuf::from(dir).join(name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

/// 外部工具缺失时的统一错误文本 —— #5500。
///
/// 错误文本必须点名**该设哪个环境变量** —— 读者拿到的是「该做什么」，
/// 而不是一句无法行动的「not found」。
pub fn missing_external_tool_message(tool: &str, env_var: &str) -> String {
    format!(
        "{tool} client not found — set {env_var} to its absolute path, \
         or install {tool} and make sure it is on PATH"
    )
}

/// C7 判据的 Rust 侧等价物（R1）：找出文本里的**机器专属安装路径**。
///
/// 这是 `scripts/gate/check_test_external_tool_paths.sh` 里那条扫描的
/// 同构实现。两侧必须对同一批样本给出同一结论 ——
/// `tests/common/bin_resolver.rs` 的单元测试与门禁的 `--selftest`
/// 用的是同一组样本。
pub fn machine_specific_install_paths(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for root in MACHINE_SPECIFIC_INSTALL_ROOTS {
        let mut from = 0usize;
        while from < text.len() {
            let Some(rel) = text[from..].find(root) else {
                break;
            };
            let start = from + rel;
            let rest = &text[start..];
            let end = rest
                .find(|c: char| {
                    c.is_whitespace()
                        || c == '"'
                        || c == '\''
                        || c == ')'
                        || c == ']'
                        || c == ';'
                        || c == ','
                })
                .unwrap_or(rest.len());
            let hit = rest[..end].trim_end_matches(['.', ':']);
            if !hit.is_empty() && !out.iter().any(|h| h == hit) {
                out.push(hit.to_string());
            }
            from = start + root.len();
        }
    }
    out
}

/// C7 判据的 Rust 侧等价物（R2）：找出**字面量**的客户端凭据参数。
///
/// 形如 `"-U"` 紧跟一个用户名 / 口令的**字符串字面量**。
/// 值是**变量**（`-u`, user）时不算 —— 那是运行期解析出来的，不是写死的。
pub fn hardcoded_client_credentials(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for flag in CLIENT_CREDENTIAL_FLAGS {
        let needle = format!("\"{flag}\"");
        let mut from = 0usize;
        while from < text.len() {
            let Some(rel) = text[from..].find(&needle) else {
                break;
            };
            let start = from + rel;
            let after = text[start + needle.len()..].trim_start();
            if let Some(rest) = after.strip_prefix(',') {
                let rest = rest.trim_start();
                if let Some(literal) = rest.strip_prefix('"') {
                    let value = literal.split('"').next().unwrap_or("");
                    if !value.is_empty() {
                        let hit = format!("\"{flag}\", \"{value}\"");
                        if !out.contains(&hit) {
                            out.push(hit);
                        }
                    }
                }
            }
            from = start + needle.len();
        }
    }
    for hit in inline_password_literals(text) {
        if !out.contains(&hit) {
            out.push(hit);
        }
    }
    out
}

/// The inline password form: the value glued to the flag (`-psecret`).
///
/// This is how the `mysql` client spells a password. A `psql` **port** is
/// always written separately (`-p 5432`), and a format placeholder
/// (`-p{}`) is not a secret — neither is a hit.
pub fn inline_password_literals(text: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut from = 0usize;
    while from < text.len() {
        // Only the lowercase `-p` is the `mysql` password flag; the
        // uppercase `-P` is a **port** in both clients.
        let Some(rel) = text[from..].find("\"-p") else {
            break;
        };
        let start = from + rel;
        let rest = &text[start + 1..];
        let Some(end) = rest.find('"') else {
            break;
        };
        let value = &rest[2..end];
        if value.len() >= 2
            && !value.contains('{')
            && !value.contains('}')
            && !value.contains('%')
            && !CLIENT_CREDENTIAL_FLAGS.contains(&value)
        {
            let hit = format!("\"-{value}\"");
            if !out.contains(&hit) {
                out.push(hit);
            }
        }
        from = start + 1;
    }
    out
}

/// 二进制缺失时的统一错误文本。
///
/// 错误文本必须点名**该设哪个环境变量** —— 读者拿到的是「该做什么」，
/// 而不是一句无法行动的「not found」。
pub fn missing_binary_message(which: &str, env_var: &str) -> String {
    format!(
        "{which} binary not found — set {env_var} to its absolute path, \
         or build it first (`cargo build -p sqlrustgo-cli` builds the \
         `sqlrustgo` binary, `cargo build -p sqlrustgo-mysql-server` builds \
         the MySQL server)"
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
        // ⚠️ 样例路径**运行期拼接**：C7 判据（R1）扫的是文件**内容**，
        // 源码里一旦出现完整形态，这条样例自己就会把门禁打红。
        let opt_sample = ["/op", "t/sqlrustgo/target/debug/x"].concat();
        assert!(!is_home_directory_path(&opt_sample));
        // 相对路径不是绝对家目录路径
        assert!(!is_home_directory_path("home/ai/sqlrustgo"));
        assert!(!is_home_directory_path("./home/ai/sqlrustgo"));
    }

    #[test]
    fn explicit_env_var_wins_over_every_other_candidate() {
        // 造一个临时的「假二进制」，用环境变量指过去。
        let dir = std::env::temp_dir().join(format!("bin-resolver-env-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let fake = dir.join(CANONICAL_CLI_BIN);
        std::fs::write(&fake, b"#!/bin/sh\nexit 0\n").expect("write fake binary");
        let fake = fake.display().to_string();

        // ⚠️ 用**本用例专属**的环境变量名 —— #5423 / #5405。
        //
        // 进程级环境变量是**全局**的：libtest 默认并行跑用例，两个用例各改
        // 各的变量才不会互相踩。之前这里改的是 `SQLRUSTGO_BIN`
        // （[`ENGINE_BIN_ENV`]），而同一个模块里另一个用例开头还会
        // `remove_var(PSQL_BIN_ENV)` / `remove_var(MYSQL_BIN_ENV)`
        // —— 并发时会把本用例刚设的值抹掉，偶发红灯。
        //
        // 那个偶发红灯**不是假象**：迭代 5 批次 11 的全量回归里，
        // `e2e_canonical_subprocess` / `tpch_q8_q21_perf_regression_test` /
        // `v312_13_load_data_sf10_test` 三个目标都因为它变红，断言拿到的是
        // PATH 上的 `/opt/homebrew/bin/psql`，而不是用例自己设的临时路径。
        //
        // 更要紧的是：**任何**读 `SQLRUSTGO_BIN` 的测试都会踩到同一个坑
        // —— #5405 的 e2e 测试正是读它的。
        std::env::set_var(SELFTEST_ENGINE_ENV, &fake);
        let resolved = resolve_binary(SELFTEST_ENGINE_ENV, &[CANONICAL_CLI_BIN]);
        std::env::remove_var(SELFTEST_ENGINE_ENV);

        let resolved = resolved.expect("explicit env var must resolve");
        assert_eq!(
            resolved.display().to_string(),
            fake,
            "an explicit {} must win over every other candidate",
            SELFTEST_ENGINE_ENV
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn engine_cli_and_canonical_cli_are_the_same_binary() {
        // #5405: 「引擎 CLI」与「规范 CLI」是**同一个**二进制 ⇒ 同一个
        // 环境变量 ⇒ 同一个解析结果。两个名字、两份实现，就是漂移的起点。
        //
        // 这条断言是**无竞态**的：它不修改任何进程级环境变量
        // （`SQLRUSTGO_BIN` 现在**没有**任何用例会去改它），
        // 两个调用读到的是同一个值。
        assert_eq!(
            resolve_engine_binary(),
            resolve_canonical_cli_binary(),
            "the engine CLI and the canonical CLI must resolve to the same binary"
        );
    }

    #[test]
    fn canonical_cli_bin_name_is_not_its_package_name() {
        // #5405 的根因：**包名**被当成了**二进制名**。
        //
        //   产出 canonical CLI 的包  = `sqlrustgo-cli`
        //   它的 `[[bin]] name`      = `sqlrustgo`
        //
        // 仓库里**没有**任何 bin 叫 `sqlrustgo-cli`。这条断言把这条
        // 不变量钉死：谁想把包名改回候选名，这条用例立刻红。
        assert_eq!(CANONICAL_CLI_BIN, "sqlrustgo");
        assert_ne!(
            CANONICAL_CLI_BIN, "sqlrustgo-cli",
            "the canonical CLI binary must never be named after its package"
        );
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

    // -----------------------------------------------------------------------
    // #5500 — 外部工具定位
    // -----------------------------------------------------------------------

    #[test]
    fn external_tool_resolution_prefers_the_explicit_env_var() {
        let dir = std::env::temp_dir().join(format!("ext-tool-env-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let fake = dir.join(PSQL_BIN_NAME);
        std::fs::write(&fake, b"#!/bin/sh\nexit 0\n").expect("write fake client");
        let fake = fake.display().to_string();

        // ⚠️ 用**本用例专属**的环境变量名 —— #5423。
        // 之前这里改的是 `SQLRUSTGO_PSQL_BIN`，而下面的
        // `external_tool_resolution_never_invents_an_absolute_path`
        // 会 `remove_var(PSQL_BIN_ENV)` —— 并发时把本用例刚设的值抹掉，
        // 断言于是拿到 PATH 上的真实 `psql` 而不是临时路径 ⇒ 偶发红灯。
        std::env::set_var(SELFTEST_PSQL_ENV, &fake);
        let resolved = resolve_external_tool(SELFTEST_PSQL_ENV, &[PSQL_BIN_NAME]);
        std::env::remove_var(SELFTEST_PSQL_ENV);

        let resolved = resolved.expect("explicit env var must resolve");
        assert_eq!(
            resolved.display().to_string(),
            fake,
            "an explicit {} must win over PATH",
            SELFTEST_PSQL_ENV
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn external_tool_resolution_never_invents_an_absolute_path() {
        // 找不到时返回 None —— 调用方必须把它变成 NOT-RUN 或错误，
        // 绝不能回退到某个开发机的绝对路径。
        //
        // ⚠️ 这里**曾经**有一对 `remove_var(PSQL_BIN_ENV)` /
        // `remove_var(MYSQL_BIN_ENV)` —— 那是批次 11 全量回归里 3 个目标
        // 偶发变红的直接原因（#5423）：进程级环境变量是全局的，
        // 并发用例会把别人的值抹掉。现在每个用例只碰**自己**的变量名，
        // 谁也不动别人的。

        // 环境变量指向一个**不存在的文件**时，不得被当成解析成功。
        let dir = std::env::temp_dir().join(format!("ext-tool-absent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let absent = dir.join("psql-does-not-exist");
        std::env::set_var(SELFTEST_ABSENT_ENV, absent.display().to_string());
        let resolved = resolve_external_tool(SELFTEST_ABSENT_ENV, &[PSQL_BIN_NAME]);
        std::env::remove_var(SELFTEST_ABSENT_ENV);
        let _ = std::fs::remove_dir_all(&dir);

        if let Some(found) = resolved {
            assert_ne!(
                found.display().to_string(),
                absent.display().to_string(),
                "a non-existent file must never be reported as a resolved tool"
            );
        }

        // 一个名字被换成不存在的二进制 ⇒ 一定找不到。
        let missing =
            resolve_external_tool("SQLRUSTGO_DEFINITELY_ABSENT_TOOL", &["__no_such_tool__"]);
        assert!(
            missing.is_none(),
            "an absent tool must resolve to None, got {missing:?}"
        );
    }

    #[test]
    fn missing_external_tool_message_names_the_env_var() {
        let msg = missing_external_tool_message("psql", PSQL_BIN_ENV);
        assert!(msg.contains("psql"), "message must name the tool: {msg}");
        assert!(
            msg.contains(PSQL_BIN_ENV),
            "message must name the env var to set: {msg}"
        );
    }

    // -----------------------------------------------------------------------
    // #5500 — C7 判据的 Rust 侧等价物
    // -----------------------------------------------------------------------

    #[test]
    fn c7_detects_machine_specific_install_paths() {
        // ⚠️ 样例路径**运行期拼接**：C7 判据扫的是文件**内容**，
        // 源码里一旦出现完整形态，这条样例自己就会把门禁打红。
        let homebrew = ["/op", "t/homebrew/bin/mysql"].concat();
        let pg = ["/op", "t/homebrew/opt/postgresql@16/bin/psql"].concat();
        let usr_local = ["/usr", "/local/bin/mysql"].concat();
        let apps = ["/App", "lications/Postgres.app/Contents/MacOS/psql"].concat();

        for sample in [&homebrew, &pg, &usr_local, &apps] {
            let hits = machine_specific_install_paths(sample);
            assert!(
                !hits.is_empty(),
                "C7-R1 must flag a machine-specific install path: {sample}"
            );
        }
    }

    #[test]
    fn c7_accepts_paths_that_are_not_machine_specific() {
        // 仓库内相对路径、运行期变量、通用系统临时目录都**不是**命中。
        for sample in [
            "tests/data/tpch-fixture/lineitem.tbl",
            "CARGO_MANIFEST_DIR",
            "CARGO_TARGET_DIR",
            "/tmp/tpch-fixture",
            "env!(\"CARGO_MANIFEST_DIR\")",
        ] {
            let hits = machine_specific_install_paths(sample);
            assert!(
                hits.is_empty(),
                "C7-R1 must not flag {sample} (got {hits:?})"
            );
        }
    }

    #[test]
    fn c7_detects_literal_client_credentials_but_not_variables() {
        // ⚠️ 样例**运行期拼接**：C7 门禁扫的是文件**内容**，
        // 源码里一旦出现完整形态，这条样例自己就会把门禁打红。
        let flag_u = ["-", "U"].concat();
        let flag_lower_u = ["-", "u"].concat();
        let flag_user = ["--", "user"].concat();
        let flag_password = ["--", "password"].concat();
        let secret = ["some", "secret"].concat();

        // 写死的字面量 ⇒ 命中。
        for (flag, value) in [
            (&flag_u, &secret),
            (&flag_lower_u, &secret),
            (&flag_user, &secret),
            (&flag_password, &secret),
        ] {
            let sample = format!("args([\"{flag}\", \"{value}\"])");
            let hits = hardcoded_client_credentials(&sample);
            assert!(
                !hits.is_empty(),
                "C7-R2 must flag a literal client credential: {sample}"
            );
        }

        // 紧贴写法（`mysql` 的口令形态）⇒ 命中。
        let inline = format!("args([\"-p{secret}\"])");
        let hits = hardcoded_client_credentials(&inline);
        assert!(
            !hits.is_empty(),
            "C7-R2 must flag an inline password: {inline}"
        );

        // 值是**变量** ⇒ 运行期解析，不是写死的 ⇒ 不命中。
        for sample in [
            format!("args([\"{flag_lower_u}\", user])"),
            format!("args([\"{flag_u}\", &pg_user])"),
            format!("args([\"{flag_password}\", password.as_str()])"),
            // `psql` 的 `-p` 是**端口**，`cargo test -p` 是**包选择器**。
            "args([\"-p\", \"5432\"])".to_string(),
            "args([\"-d\", \"tpch_test\"])".to_string(),
        ] {
            let hits = hardcoded_client_credentials(&sample);
            assert!(
                hits.is_empty(),
                "C7-R2 must not flag a runtime-resolved credential: {sample} (got {hits:?})"
            );
        }
    }
}

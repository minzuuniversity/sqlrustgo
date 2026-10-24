//! 测试**输出目录**的唯一规则 —— #5577 / #5389。
//!
//! # 为什么这个模块存在
//!
//! 仓库里有五处测试把产物写进 `docs/` 下的归档目录。每一处的**意图**都是
//! 「把这次运行的产物落到一个地方」，每一处的**实际行为**都是
//! 「**覆写已提交的证据**」。
//!
//! 最直接的一次：`tests/integration/compat/a04_mysql_diff_matrix_test.rs`
//! 把对照结果写进 `docs/releases/v4.1.0/evidence/a04-5226/`。当 PostgreSQL
//! 参考容器不可用时，它**没有跳过**，而是把每个用例写成
//! `ENGINE_UNAVAILABLE: container a04-pg-ref not running`
//! ⇒ **在一台没有 pg 参考容器的机器上跑一次 `cargo test --workspace`，
//! 就把 21 个已归档的对照输出抹成占位符**（#5577）。
//! 同族：#5389（一次跑改写 81 个证据文件）。
//!
//! # 规则（**一份实现**）
//!
//! 1. 设置了环境变量 ⇒ 用它 —— 写到哪里是**显式声明**的；
//! 2. 没设置 ⇒ 写进 Cargo 的**构建产物目录** `target/<name>/`，
//!    **绝不**写进仓库的 `docs/`。
//!
//! # 为什么默认不能是 `docs/`
//!
//! **归档证据是产物，不是日志。**
//!
//! 「测试顺便把证据写好」听起来是省事，但它把两件事混成了一件：
//! *一次普通的测试运行* 和 *一次显式的证据重新生成*。
//! 前者不该有能力改后者 —— 它甚至**不知道**自己跑得对不对
//! （A04 在没有参照引擎时就是在"跑不对"的状态下把归档抹掉的）。
//!
//! 所以规则是：**要写进仓库，必须显式声明**。
//!
//! ```bash
//! SQLRUSTGO_A04_EVIDENCE_DIR=docs/releases/v4.1.0/evidence/a04-5226 \
//!   cargo test --all-features --test a04_mysql_diff_matrix -- --nocapture
//! ```
//!
//! # 守卫
//!
//! 静态守卫：`scripts/gate/check_tests_dont_write_archived_evidence.sh`
//! —— 任何测试源码里出现「写归档证据目录」的形态 ⇒ **FAIL**，并点名文件与行号。
//! 门禁扫的是**规则**（写 + `docs/` 路径同现），不是这份实现的副本；
//! 换成任何别的写法，只要还在写 `docs/`，门禁照样红。
//!
//! 本文件被**每个**用它的测试二进制各自 `#[path]` include，
//! 所以每个编译单元只用得到其中一部分函数
//! （`#5577` 与 `tests/common/mod.rs` 的同一处考量）。
#![allow(dead_code)]

use std::path::PathBuf;

/// Cargo 的构建产物目录。
///
/// `CARGO_TARGET_DIR` 可能是**相对路径**（相对当前工作目录），而集成测试的
/// 工作目录是**包目录**，不是仓库根 ⇒ 相对路径会解析到错误的地方。
/// 所以只接受绝对路径，否则退回 `CARGO_MANIFEST_DIR/target`。
pub fn target_dir() -> PathBuf {
    if let Ok(d) = std::env::var("CARGO_TARGET_DIR") {
        let d = d.trim();
        if !d.is_empty() {
            let p = PathBuf::from(d);
            if p.is_absolute() {
                return p;
            }
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target")
}

/// 解析一个测试产物的输出目录。
///
/// * `env_var` —— 用来**显式声明**输出位置的环境变量名；
/// * `name` —— 默认输出目录在 `target/` 下的名字。
///
/// 返回的路径**默认一定在构建产物目录里**，不在仓库的 `docs/` 下。
pub fn evidence_dir(env_var: &str, name: &str) -> PathBuf {
    if let Ok(dir) = std::env::var(env_var) {
        let dir = dir.trim();
        if !dir.is_empty() {
            return PathBuf::from(dir);
        }
    }
    target_dir().join(name)
}

/// 解析一个测试产物的输出**文件**路径。
///
/// 与 [`evidence_dir`] 的区别：环境变量里给的是**完整文件路径**，不是目录。
/// 用在「产物是单个文件」的场合（例如 soak 的 summary 文本）。
///
/// 语义与旧约定保持一致 —— 已有调用者传的仍然是**完整文件路径**，
/// 只是**默认值**从仓库里的归档路径换成了构建产物目录。
pub fn evidence_file(env_var: &str, name: &str) -> PathBuf {
    if let Ok(path) = std::env::var(env_var) {
        let path = path.trim();
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }
    target_dir().join(name)
}

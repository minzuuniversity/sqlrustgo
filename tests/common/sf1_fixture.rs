//! SF=1 TPC-H 夹具的运行期解析 —— #5406 / #5428。
//!
//! c6-allow-home-paths — 本文件**故意**包含机器相关的样本路径：
//! 它记录的是「被修掉的反例」，必须留着样本才能说清修的是什么。
//!
//! # 为什么这个模块存在
//!
//! 三个 SF=1 用例此前把夹具目录**硬编码**成某台开发机家目录下的绝对路径：
//!
//! ```text
//! /home/openclaw/tpch_baseline/sf1
//! /home/openclaw/sqlrustgo_work/tests/data/tpch-sf01
//! ```
//!
//! 这条路径**只在一台机器上存在**。后果不是「测试失败」，而是更糟的两种：
//!
//! * `expect("read")` / `bulk_load_tbl_file(...).unwrap()` **直接 panic**
//!   —— 目标从「跑过」变成「崩掉」，而且崩在离问题很远的地方；
//! * 或者被「找不到就跳过」的降级路径吞掉 —— **假绿**。
//!
//! # 本模块的规则
//!
//! 解析顺序：
//!
//! 1. `TPCH_SF1_DIR` —— 显式指定（CI 与本地都用它）。
//! 2. `TPCH_DATA_DIR` —— 通用夹具变量；仅当它看起来是 SF=1 时才用。
//! 3. 否则：**缺失**。
//!
//! 缺失时**不得**回退到任何硬编码路径。调用方拿到 [`resolve_sf1_dir`] 的
//! `None` 后，必须走 **NOT-RUN 三态披露**（`report_not_run`），而不是
//! panic、也不是「跳过 ⇒ 通过」。
//!
//! # 为什么不复用 `tpch_fixture.rs` 的 `FixtureState`
//!
//! `tpch_fixture.rs` 面向的是 **SF=0.01** 那套 CI 生成的夹具，并用
//! manifest 里的 `sf` 字段做**档位匹配**（`require_profile`）。SF=1 用例
//! 需要的是另一个档位、另一套变量名（`TPCH_SF1_DIR`），而且 SF=1 夹具
//! 在 CI 内**根本不可得**（#5387）。把两件事混在一起会让「档位不匹配」
//! 和「夹具缺失」这两种不同的 NOT-RUN 原因无法区分。

#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// SF=1 夹具目录的环境变量名（专用）。
pub const SF1_DIR_ENV: &str = "TPCH_SF1_DIR";

/// 通用夹具目录的环境变量名。
pub const DATA_DIR_ENV: &str = "TPCH_DATA_DIR";

/// NOT-RUN 的受控原因码：夹具缺失。
pub const REASON_FIXTURE_MISSING: &str = "fixture_missing";

/// NOT-RUN 的受控原因码：夹具存在，但档位不是 SF=1。
pub const REASON_PROFILE_MISMATCH: &str = "profile_mismatch";

/// 解析 SF=1 夹具目录。
///
/// 返回 `None` 表示「这台机器上没有 SF=1 夹具」—— 调用方**必须**把它
/// 变成 NOT-RUN 披露，绝不能当成「通过」。
pub fn resolve_sf1_dir() -> Option<PathBuf> {
    // 1. 专用变量优先。
    if let Some(dir) = env_dir(SF1_DIR_ENV) {
        return Some(dir);
    }

    // 2. 通用变量：只在它确实像一个 TPC-H 夹具目录时才用
    //    （目录里至少有 region.tbl）。
    if let Some(dir) = env_dir(DATA_DIR_ENV) {
        if dir.join("region.tbl").is_file() {
            return Some(dir);
        }
        // 它指向的是别的档位（比如 CI 的 SF=0.01）。SF=1 用例的期望值是
        // 按 SF=1 校准的，对 SF=0.01 数据断言就是**断言错的东西**。
        // 交给调用方按 profile_mismatch 披露，而不是硬跑。
        return None;
    }

    // 3. 没有变量 ⇒ 没有夹具。绝不回退到硬编码路径。
    None
}

/// 读取一个环境变量里的目录（非空且存在才返回）。
fn env_dir(name: &str) -> Option<PathBuf> {
    let raw = std::env::var(name).ok()?;
    let raw = raw.trim();
    if raw.is_empty() {
        return None;
    }
    let path = PathBuf::from(raw);
    if path.is_dir() {
        Some(path)
    } else {
        None
    }
}

/// 判定一个目录看起来是不是 SF=1 夹具。
///
/// 判据：8 张 TPC-H 表的 `.tbl` 文件都在。
pub fn looks_like_sf1(dir: &Path) -> bool {
    const TABLES: [&str; 8] = [
        "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
    ];
    TABLES
        .iter()
        .all(|t| dir.join(format!("{t}.tbl")).is_file())
}

/// 打印 NOT-RUN 记录（与 `tpch_fixture.rs` 的口径一致）。
///
/// CI 按 `TPCH-NOT-RUN:` 前缀统计这些记录，并把它们从 `passed` 里**扣掉**——
/// 它们永远不会被算作「通过」。
pub fn report_not_run(target: &str, reason: &str, detail: &str) {
    let line = format!("TPCH-NOT-RUN: {target} reason={reason} detail={detail}");
    println!("{line}");
    if let Ok(path) = std::env::var("TPCH_NOT_RUN_FILE") {
        if !path.trim().is_empty() {
            use std::io::Write;
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path.trim())
            {
                let _ = writeln!(f, "{line}");
            }
        }
    }
}

/// 解析 SF=1 夹具；缺失时**披露 NOT-RUN** 并返回 `None`。
///
/// 这是三个 SF=1 用例的统一入口：它们只需要在开头调一次，
/// 拿不到夹具就直接返回（测试以「未运行」结束，而不是崩溃或假绿）。
pub fn require_sf1_dir(target: &str) -> Option<PathBuf> {
    match resolve_sf1_dir() {
        Some(dir) if looks_like_sf1(&dir) => Some(dir),
        Some(dir) => {
            report_not_run(
                target,
                REASON_PROFILE_MISMATCH,
                &format!(
                    "{} points at {:?}, which does not contain all 8 TPC-H .tbl files \
                     (expected an SF=1 fixture)",
                    SF1_DIR_ENV, dir
                ),
            );
            None
        }
        None => {
            report_not_run(
                target,
                REASON_FIXTURE_MISSING,
                &format!(
                    "no SF=1 TPC-H fixture found (set {SF1_DIR_ENV} to a directory with \
                     the 8 .tbl files); this test's expectations are calibrated against \
                     SF=1 and must not run against another scale"
                ),
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sf1_detection_requires_all_eight_tables() {
        let dir = std::env::temp_dir().join(format!("sf1-detect-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        assert!(!looks_like_sf1(&dir), "an empty dir is not an SF=1 fixture");

        for t in [
            "region", "nation", "supplier", "customer", "part", "partsupp", "orders", "lineitem",
        ] {
            std::fs::write(dir.join(format!("{t}.tbl")), b"x").expect("write stub table");
        }
        assert!(looks_like_sf1(&dir), "all 8 tables present => SF=1 fixture");

        std::fs::remove_file(dir.join("lineitem.tbl")).expect("remove one table");
        assert!(
            !looks_like_sf1(&dir),
            "a fixture missing one table is not an SF=1 fixture"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolution_never_falls_back_to_a_hardcoded_path() {
        // 不设任何环境变量时，解析必须返回 None（=「缺失」），
        // 而不是某个开发机的家目录路径。
        let saved_sf1 = std::env::var(SF1_DIR_ENV).ok();
        let saved_data = std::env::var(DATA_DIR_ENV).ok();
        std::env::remove_var(SF1_DIR_ENV);
        std::env::remove_var(DATA_DIR_ENV);

        let resolved = resolve_sf1_dir();

        if let Some(sf1) = saved_sf1 {
            std::env::set_var(SF1_DIR_ENV, sf1);
        }
        if let Some(data) = saved_data {
            std::env::set_var(DATA_DIR_ENV, data);
        }

        assert!(
            resolved.is_none(),
            "with no fixture env var set, resolution must report 'missing', got {:?}",
            resolved
        );
    }
}

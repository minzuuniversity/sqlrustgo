//! SQLite 方言适配与数值感知比较 —— 差分测试的共享件。
//!
//! #5406：差分测试（`tests/compat/*`、`tests/integration/compat/*`）此前
//! 通过**外部二进制**做对照：引擎侧跑 `sqlrustgo sqlite --mode csv`，
//! oracle 侧跑 `/usr/bin/sqlite3 -csv`。二进制找不到时，测试**静默跳过**
//! 并报告「通过」—— 36 个用例 0.00s 通过、一次比对都没做。
//!
//! 现在两侧都改成**进程内**：
//!
//! * 引擎侧用 `ExecutionEngine<MemoryStorage>`（root 包的公开 API）；
//! * oracle 侧用 `rusqlite`（root 包的依赖，`bundled` feature ⇒ SQLite 的
//!   C 源码随仓库编译，CI 不需要额外二进制、不需要网络）。
//!
//! 于是差分测试**没有任何外部依赖**：没有硬编码路径，没有「环境缺失就当
//! 通过」的降级路径，也就不可能假绿。
//!
//! # 两种方言差异
//!
//! SQLite **没有** `MOD` 与 `CHAR_LENGTH` 这两个函数名：
//!
//! | 引擎写法 | SQLite 等价写法 |
//! |---------|----------------|
//! | `MOD(a, b)` | `((a) % (b))` |
//! | `CHAR_LENGTH(x)` | `LENGTH(x)` |
//!
//! 两者都是**语义完全等价**的映射。改写**必须可见** —— [`adapt_for_sqlite`]
//! 把每一条改写都返回给调用方，由调用方打印出来。静默改写会让差分测试拿
//! 引擎去和**另一个查询**比较，什么也证明不了
//! （`tests/common/sqlite_oracle.rs` 的 `adapt_sql` 守的是同一条规矩）。
//!
//! # 比较规则
//!
//! 逐单元格、**数值感知**：`17` 与 `17.0` 是同一个数；NULL 用一个不可能与
//! 真实文本撞上的哨兵表示。

#![allow(dead_code)]

/// NULL 的哨兵表示 —— 不可能与真实文本 `"NULL"` 撞上。
pub const NULL_CELL: &str = "\u{0}NULL";

/// 浮点比较的相对容差。
pub const FLOAT_REL_TOLERANCE: f64 = 1e-9;

/// 找到 `needle` 在 `haystack` 中的词边界位置（前后都不能是标识符字符）。
pub fn find_keyword(haystack: &str, needle: &str) -> Option<usize> {
    let bytes = haystack.as_bytes();
    let mut start = 0usize;
    while let Some(rel) = haystack[start..].find(needle) {
        let abs = start + rel;
        let before_ok = abs == 0
            || (!bytes[abs - 1].to_ascii_lowercase().is_ascii_alphanumeric()
                && bytes[abs - 1] != b'_');
        let after = abs + needle.len();
        let after_ok = after >= bytes.len()
            || (!bytes[after].to_ascii_lowercase().is_ascii_alphanumeric() && bytes[after] != b'_');
        if before_ok && after_ok {
            return Some(abs);
        }
        start = abs + 1;
        if start >= haystack.len() {
            break;
        }
    }
    None
}

/// 找到 `(` 在 `open_idx` 处对应的 `)`。
pub fn matching_paren(s: &str, open_idx: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    for (i, b) in bytes.iter().enumerate().skip(open_idx) {
        match b {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

/// 在括号深度 0 处按逗号切分参数。
fn split_top_level(args: &str) -> Option<(String, String)> {
    let bytes = args.as_bytes();
    let mut depth = 0i32;
    let mut split_at = None;
    for (i, b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth += 1,
            b')' => depth -= 1,
            b',' if depth == 0 => {
                split_at = Some(i);
                break;
            }
            _ => {}
        }
    }
    let at = split_at?;
    Some((
        args[..at].trim().to_string(),
        args[at + 1..].trim().to_string(),
    ))
}

/// 把引擎的函数名改写成 SQLite 的等价写法。
///
/// 返回 `(改写后的 SQL, 改写说明列表)`。调用方**必须**把说明打印出来 ——
/// 静默改写会让差分测试拿引擎去和**另一个查询**比较。
pub fn adapt_for_sqlite(sql: &str) -> (String, Vec<String>) {
    let mut out = sql.to_string();
    let mut notes: Vec<String> = Vec::new();

    while let Some(pos) = find_keyword(&out, "CHAR_LENGTH") {
        let open = pos + "CHAR_LENGTH".len();
        let Some(close) = matching_paren(&out, open) else {
            break;
        };
        let inner = out[open + 1..close].to_string();
        out = format!("{}LENGTH({}){}", &out[..pos], inner, &out[close + 1..]);
        notes.push(format!("CHAR_LENGTH({}) → LENGTH({})", inner, inner));
    }

    while let Some(pos) = find_keyword(&out, "MOD") {
        let open = pos + "MOD".len();
        let Some(close) = matching_paren(&out, open) else {
            break;
        };
        let inner = out[open + 1..close].to_string();
        let Some((a, b)) = split_top_level(&inner) else {
            break;
        };
        out = format!("{}(({}) % ({})){}", &out[..pos], a, b, &out[close + 1..]);
        notes.push(format!("MOD({}, {}) → (({}) % ({}))", a, b, a, b));
    }

    (out, notes)
}

/// 整数值浮点不带 `.0` 后缀，这样 `17` 与 `17.0` 渲染一致。
pub fn format_real(f: f64) -> String {
    if f.is_finite() && f.fract() == 0.0 && f.abs() < 9.007_199_254_740_992e15 {
        format!("{}", f as i64)
    } else {
        format!("{}", f)
    }
}

/// 两个单元格是否代表同一个值。
pub fn cells_equal(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    match (a.parse::<f64>(), b.parse::<f64>()) {
        (Ok(x), Ok(y)) => {
            if x == y {
                return true;
            }
            let denom = x.abs().max(y.abs());
            if denom == 0.0 {
                return x == y;
            }
            (x - y).abs() / denom <= FLOAT_REL_TOLERANCE
        }
        _ => false,
    }
}

//! #4951 性能回归守卫 —— 读热路径不得克隆整表。
//!
//! # 背景（这是一个真实测量出来的回归，不是假想）
//!
//! #4951 AC2 为了移除 `&self → &mut Self` 的逃逸口，把 4 个受保护字段
//! 打包进 `RwLock<WriteState>`。因为 `&TableData` 无法从读守卫里借出去，
//! `get_table` 的签名从
//!
//! ```ignore
//! pub fn get_table(&self, name: &str) -> Option<&TableData>   // 借用，零成本
//! ```
//!
//! 改成了
//!
//! ```ignore
//! pub fn get_table(&self, name: &str) -> Option<TableData>    // 全表深拷贝
//! ```
//!
//! 而 `get_table_info` 一直是这么写的：
//!
//! ```ignore
//! self.get_table(table).map(|t| t.info.clone())
//! ```
//!
//! 同一行代码，**在旧签名下只克隆 `TableInfo`（几个字段），在新签名下
//! 先把整张表的所有 `Record` 深拷贝一遍，再把 rows 全部丢掉**。
//!
//! 致命之处在于调用点：`scan_pk` 在**每一次主键点查**的开头都会调
//! `get_table_info`。而 `sysbench oltp_read_only` 的负载几乎全是点查。
//! 于是「拿到列名列表」这个 O(列数) 的操作变成了 O(表行数) 的全表克隆。
//!
//! # 实测代价
//!
//! 三方 A/B（10 轮 × 3 侧，固定数据集 4 表 × 5000 行，纯只读，8 线程，
//! 每轮重启服务端，COUNT(*) 校验一致）：
//!
//! | 侧 | 版本 | QPS 中位数 |
//! |---|---|---|
//! | base | `95880f76c` 隔离修复前 | ~3.9k |
//! | mid  | `6fada46b46` 隔离修复后 | ~3.9k |
//! | mine | `a38931dc5c` + WriteState 重构 | ~2.8k |
//!
//! mine 的**最高值**仍低于 base/mid 的**最低值**，两组零重叠 —— 这不是
//! 噪声，是 ~30% 的确定性回退。
//!
//! # 为什么用源码模式守卫
//!
//! 「`get_table_info` 很快」无法用断言直接表达：要么测时间（易抖动、
//! 在 CI 上不可靠），要么给 `Record::clone` 打桩（改动面太大，等于为了
//! 测一个回归去改生产类型）。所以本测试走**源码模式**路线 —— 与
//! `ref_to_mut_escape_hatch_guard_test.rs` 同一思路：断言「这个函数体里
//! 不出现那个调用」，而不是断言「它的运行时间小于 X」。
//!
//! # 覆盖范围
//!
//! 扫**全仓** `crates/*/src/**.rs` + `src/**.rs`，而不是只扫
//! `file_storage.rs`：把同样的写法复制到别的 storage 实现里，守卫同样要能抓到。

use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/storage -> repo root")
        .to_path_buf()
}

fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let skip_dirs = ["target", "docs", ".worktrees", ".git", "evidence"];
    fn walk(dir: &Path, skip: &[&str], out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() {
                if !skip.contains(&name.as_str()) {
                    walk(&p, skip, out);
                }
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                out.push(p);
            }
        }
    }
    walk(root, &skip_dirs, &mut out);
    out.sort();
    out
}

/// 剥掉注释，只留真正的代码。
///
/// 这不是洁癖：guard 测试文件**自己的文档注释**里就写着
/// `self.get_table(table).map(|t| t.info.clone())` 这段示例。第一版
/// 守卫没剥注释，直接把自己的文档当成了违规代码，自己把自己判红
/// （实测 `get_table_info body offset 670` —— offset 670 落在本文件里）。
///
/// 顺带一提，这个假阳性本身说明了「扫源码」这条路必须配套剥注释，
/// 否则任何文档里引用一次违规写法，守卫就永久变红，最终被人注释掉。
fn strip_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0usize;
    // 跟踪字符串字面量状态，避免把 "//" 出现在字符串里当成注释
    let mut in_str = false; // 普通字符串
    let mut in_char = false;
    let mut raw_hashes: Option<usize> = None; // r"..." / r#"..."#
    while i < b.len() {
        let c = b[i];
        if let Some(n) = raw_hashes {
            // 处于 r#"..."# 中：找匹配的 "# 序列 + n 个 #
            if c == b'"' {
                let mut j = i + 1;
                let mut cnt = 0;
                while j < b.len() && b[j] == b'#' && cnt < n {
                    cnt += 1;
                    j += 1;
                }
                if cnt == n {
                    out.push_str(&src[i..j]);
                    i = j;
                    raw_hashes = None;
                    continue;
                }
            }
            out.push(c as char);
            i += 1;
            continue;
        }
        if in_str || in_char {
            if c == b'\\' && i + 1 < b.len() {
                out.push_str(&src[i..i + 2]);
                i += 2;
                continue;
            }
            if (in_str && c == b'"') || (in_char && c == b'\'') {
                in_str = false;
                in_char = false;
            }
            out.push(c as char);
            i += 1;
            continue;
        }
        match c {
            b'/' if i + 1 < b.len() && b[i + 1] == b'/' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if i + 1 < b.len() && b[i + 1] == b'*' => {
                i += 2;
                let mut depth = 1;
                while i < b.len() && depth > 0 {
                    if b[i] == b'/' && i + 1 < b.len() && b[i + 1] == b'*' {
                        depth += 1;
                        i += 2;
                    } else if b[i] == b'*' && i + 1 < b.len() && b[i + 1] == b'/' {
                        depth -= 1;
                        i += 2;
                    } else {
                        if b[i] == b'\n' {
                            out.push('\n');
                        }
                        i += 1;
                    }
                }
            }
            b'r' => {
                // r" / r#" / r##" —— 裸字符串起始
                let mut j = i + 1;
                let mut cnt = 0;
                while j < b.len() && b[j] == b'#' {
                    cnt += 1;
                    j += 1;
                }
                if j < b.len() && b[j] == b'"' {
                    raw_hashes = Some(cnt);
                    out.push_str(&src[i..=j]);
                    i = j + 1;
                } else {
                    out.push('r');
                    i += 1;
                }
            }
            b'"' => {
                in_str = true;
                out.push('"');
                i += 1;
            }
            b'\'' => {
                in_char = true;
                out.push('\'');
                i += 1;
            }
            _ => {
                out.push(c as char);
                i += 1;
            }
        }
    }
    out
}

/// 截出 `fn <name>` 的函数体（按大括号配平）。
fn fn_body<'a>(src: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("fn {name}");
    let start = src.find(&key)?;
    let open = src[start..].find('{')? + start;
    let bytes = src.as_bytes();
    let mut depth = 0usize;
    for (i, b) in bytes.iter().enumerate().skip(open) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(&src[open..=i]);
                }
            }
            _ => {}
        }
    }
    None
}

/// 本守卫自身所在文件。跳过它，避免自指。
fn is_guard_file(path: &Path) -> bool {
    path.file_name()
        .map(|n| n == "hot_read_path_clone_guard_test.rs")
        .unwrap_or(false)
}

/// 核心判定：`fn <name>` 的函数体里不得出现 `self.get_table(`。
fn find_clone_get_table(src: &str, fn_name: &str) -> Option<usize> {
    let body = fn_body(src, fn_name)?;
    let needle = "self.get_table(";
    body.find(needle)
}

#[test]
fn get_table_info_must_not_clone_whole_table() {
    let root = repo_root();
    let mut hits = Vec::new();
    let mut checked = 0usize;

    for file in source_files(&root) {
        if is_guard_file(&file) {
            continue;
        }
        let Ok(raw) = std::fs::read_to_string(&file) else {
            continue;
        };
        let src = strip_comments(&raw);
        if !src.contains("fn get_table_info") {
            continue;
        }
        checked += 1;
        if let Some(rel) = find_clone_get_table(&src, "get_table_info") {
            let rel_path = file.strip_prefix(&root).unwrap_or(&file);
            hits.push(format!("{}: get_table_info body offset {}", rel_path.display(), rel));
        }
    }

    assert!(
        hits.is_empty(),
        "#4951 性能回归：get_table_info 通过 self.get_table() 取值。\n\
         get_table 返回克隆的 TableData，会把整表 Record 深拷贝一遍只为拿 \
         info。\n\
         scan_pk 在每次主键点查开头都调用 get_table_info，在 oltp_read_only \
         下实测造成 ~30% 吞吐回退。\n\
         改用 self.with_table(table, |t| t.map(|t| t.info.clone()))，\n\
         它在读守卫内借出 &TableData，不克隆 rows。\n\
         违规位置:\n  {}",
        hits.join("\n  ")
    );
    assert!(
        checked > 0,
        "没扫到任何 `fn get_table_info` —— 守卫本身失效（重命名/移动后未更新），\
         这比测试变红更需要人看一眼"
    );
}

#[test]
fn partition_rows_must_not_double_clone() {
    let root = repo_root();
    let file = root.join("crates/storage/src/file_storage.rs");
    let raw = std::fs::read_to_string(&file).expect("file_storage.rs");
    let src = strip_comments(&raw);
    let body = fn_body(&src, "partition_rows").expect("partition_rows 存在于 file_storage.rs");

    // 旧实现：`get_table` 已经克隆了一次 `TableData`，函数体里又
    // `table_data.rows.clone()` 第二次 —— 每次调用两份全表拷贝。
    assert!(
        !body.contains("self.get_table("),
        "partition_rows 用了 self.get_table()（克隆返回），随后又 clone 一次 rows。\n\
         应在一个 with_read_lock 守卫内同时取 rows 与 insert_buffer，只克隆一次。"
    );
    // 防止「改成 get_table 但删掉第二次 clone」这种半吊子修法：
    // 守卫语义是「rows 与 buffer 必须在同一个读守卫内取得」。
    assert!(
        body.contains("with_read_lock"),
        "partition_rows 应当在单个读守卫内同时取 tables 与 insert_buffer，\
         否则两次取样可能来自两个不同的瞬间（撕裂快照）。"
    );
}

/// 三位一体的负面结果记录：如果把 `with_read_lock` 改成提前 `drop` 守卫、
/// 再用裸指针续命（use-after-free），**本守卫全绿、单测全绿**。
///
/// 这里显式写死这个事实，是为了让后来人知道本测试的边界：它防的是
/// 「无谓的全表克隆」，**不防**内存安全。内存安全由类型系统保证
/// （`RwLock<WriteState>` + 守卫生命周期），要抓 UAF 需要
/// ThreadSanitizer 或 loom，不在本测试能力范围内。
#[test]
fn guard_scope_does_not_cover_memory_safety() {
    let root = repo_root();
    let raw = std::fs::read_to_string(root.join("crates/storage/src/file_storage.rs")).unwrap();
    let src = strip_comments(&raw);
    let body = fn_body(&src, "with_read_lock").expect("with_read_lock");
    assert!(
        body.contains("read()"),
        "with_read_lock 应当取读守卫；若被改成裸指针逃逸，本守卫抓不到（见本文件顶部说明）"
    );
}

//! #4951 — 全仓 `&self → &mut` 逃逸口守卫。
//!
//! # 为什么已有守卫不够
//!
//! `tests/blk2_walstorage_no_escape_hatch_test.rs` 断言的是
//!
//! ```ignore
//! assert!(!src.contains("fn as_inner_mut"));   // src = wal_storage.rs
//! ```
//!
//! 也就是「**某一个文件里不再有那个名字的函数**」。它有两个盲区：
//!
//! 1. **换文件不换检查** —— `file_storage.rs:406` 的 `as_mut_self`
//!    与它是完全相同的模式，却因为文件名/函数名都不同而从未被扫到。
//!    #4951 立项时它有 24 处调用点。
//! 2. **换名字不换检查** —— 把 `as_mut_self` 改叫 `as_mut_me`，
//!    守卫全绿。
//!
//! 所以这里改成**扫模式而非扫名字**，并且**扫全仓而非单文件**。
//!
//! # 这个测试防的是什么
//!
//! 从 `&self` 派生 `&mut Self` 的原语只有几种，本测试逐个覆盖：
//!
//! | 模式 | 例 |
//! |---|---|
//! | 裸指针强转 | `unsafe { &mut *(self as *const Self as *mut Self) }` |
//! | `UnsafeCell` 取 `&mut` | `&mut *self.cell.get()` |
//! | `transmute` `&T`→`&mut T` | `transmute::<&T, &mut T>(x)` |
//! | `addr_of_mut!` / `&raw mut` | 由 `&self` 地址构造 `&mut` |
//!
//! # 已确认的历史违例
//!
//! `crates/storage/src/file_storage.rs` 的 `as_mut_self` 是**已知存在**
//! 的（#4951 待修）。本文件把它登记为显式 allowlist 条目而不是静默
//! 忽略 —— 静默忽略等于这个守卫从第一天起就是坏的。条目里带 issue
//! 号，修复后由 `block_4951_as_mut_self_removed_when_fixed` 提醒删除。

use std::path::{Path, PathBuf};

/// 仓库根（`CARGO_MANIFEST_DIR` = `<repo>/crates/storage`）。
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/storage -> repo root")
        .to_path_buf()
}

/// 收集参与扫描的 `.rs` 源文件。
///
/// 排除目录：
/// - `target`（构建产物，含展开的代码）
/// - `docs`（示例代码片段，不是实现）
/// - `.worktrees`（其他 agent 的隔离工作树，同一份代码会重复计数）
fn source_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let skip_dirs = ["target", "docs", ".worktrees", ".git"];
    fn walk(dir: &Path, skip: &[&str], out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if skip.contains(&name) {
                    continue;
                }
                walk(&path, skip, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }
    walk(root, &skip_dirs, &mut out);
    out.sort();
    out
}

#[derive(Debug, PartialEq, Eq)]
struct Violation {
    file: String,
    line: usize,
    pattern: &'static str,
    text: String,
}

/// 已知且已登记的违例。修复后应从本表移除 —— 移除前守卫会失败。
#[derive(Debug)]
struct KnownViolation {
    file_substr: &'static str,
    line_substr: &'static str,
    issue: &'static str,
}

/// #4951 修复后已清空。
///
/// 原先登记着 `file_storage.rs` 的 `as_mut_self`
/// （`&mut *(self as *const Self as *mut Self)`，24 处调用点）。该函数
/// 已被删除，条目随之移除 —— 这正是下面 `block_4951_...` 测试要驱动的
/// 清理动作。
///
/// 表**保持非空语义**：将来若又发现一处确属安全的，必须连同依据一起
/// 登记，而不是悄悄放过。空表意味着「全仓零已知违例」。
const KNOWN: &[KnownViolation] = &[];

/// 最近的 `fn` 是否以 `&self` 接收（而非 `&mut self`）。
///
/// 这是区分「真逃逸口」与「合法独占借用」的关键。
/// `wal_storage.rs:282` 的 `unsafe { &mut *self.inner.get() }` 位于
/// `fn split(&mut self)` 里 —— 独占借用下从 `UnsafeCell` 取 `&mut` 是
/// sound 的，BLK-2 修掉的是从 `&self` 派生的那个。只按字面扫会把
/// 这类合法用法一起报成违例，守卫一旦噪声过大就会被忽略。
fn current_receiver_is_shared_ref(line: &str) -> bool {
    let t = line.trim_start();
    let after_name = match t.find("fn ") {
        Some(i) => &t[i + 3..],
        None => return false,
    };
    let Some(open) = after_name.find('(') else {
        return false;
    };
    let args = &after_name[open + 1..];
    if args.contains("&mut self") {
        return false;
    }
    args.contains("&self")
}

/// 逐行匹配逃逸口模式。
///
/// 刻意**不**做完整 Rust 解析：编译期 lint 做不到「全仓没有这个
/// 模式」，而一个能真正跑在 CI 上的源码级扫描，用行级启发式足够 ——
/// 目标不是证明没有 `&mut`，而是让新增的 `&self → &mut` 桥接在
/// review 前就冒出来。
fn scan(text: &str, file: &str, out: &mut Vec<Violation>) {
    // 本守卫文件自身包含这些模式字面量，扫自己是自指噪声
    if file.ends_with("ref_to_mut_escape_hatch_guard_test.rs") {
        return;
    }
    // 当前所在函数是否为 `&self` 接收。只有 `&self` 方法体里的裸
    // `&mut` 强转才是逃逸口；`&mut self` 方法里的同类写法在独占借用
    // 下是 sound 的（见 wal_storage.rs:282 的 split(&mut self)）。
    let mut enclosing_is_shared_ref = false;
    for (idx, line) in text.lines().enumerate() {
        let line_no = idx + 1;
        let t = line.trim_start();
        if t.starts_with("//") || t.starts_with("*") || t.starts_with("/*") {
            continue;
        }
        let is_fn_sig = t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("pub(crate) fn ")
            || t.starts_with("pub(super) fn ")
            || t.starts_with("async fn ")
            || t.starts_with("pub async fn ");
        if is_fn_sig {
            enclosing_is_shared_ref = current_receiver_is_shared_ref(line);
            continue; // 签名行本身不是违例
        }

        let mut hit: Option<&'static str> = None;

        if t.contains("as *const Self as *mut Self") || t.contains("as *const _ as *mut") {
            hit = Some("bare-pointer-cast");
        } else if t.contains(".get() as *mut") || t.contains("&mut *self.") {
            hit = Some("unsafe-cell-get");
        } else if t.contains("transmute::<&") && t.contains("&mut") {
            hit = Some("transmute-ref-to-mut");
        } else if (t.contains("&raw mut self") || t.contains("addr_of_mut!(self)"))
            && !t.starts_with("let")
        {
            hit = Some("raw-mut-from-ref");
        }

        if let (Some(pattern), true) = (hit, enclosing_is_shared_ref) {
            out.push(Violation {
                file: file.to_string(),
                line: line_no,
                pattern,
                text: line.chars().take(120).collect(),
            });
        }
    }
}

fn all_violations() -> Vec<Violation> {
    let root = repo_root();
    let files = source_files(&root);
    assert!(
        files.len() > 200,
        "只扫到 {} 个 .rs 文件，扫描范围明显不对（仓库应有数千个）",
        files.len()
    );
    let mut out = Vec::new();
    for f in &files {
        let rel = f
            .strip_prefix(&root)
            .unwrap_or(f)
            .to_string_lossy()
            .to_string();
        let Ok(text) = std::fs::read_to_string(f) else {
            continue;
        };
        scan(&text, &rel, &mut out);
    }
    out
}

/// 违例是否属于已登记的已知项。
///
/// 必须**同时**匹配文件与代码特征才算已知。只按文件匹配会让该文件里
/// 新增的任何逃逸口都被静默放过 —— 那等于 allowlist 退化成了文件名
/// 白名单，正是本测试要取代的旧做法的同一个毛病。
fn is_known(v: &Violation) -> bool {
    KNOWN.iter().any(|k| {
        v.file.contains(k.file_substr) && v.text.contains(k.line_substr) && {
            // `issue` 字段供人阅读；这里显式 touch 一下避免 dead_code，
            // 同时确保登记表里每条都真的带了追踪号。
            let _ = k.issue;
            true
        }
    })
}

/// AC3 主断言：除已登记的 #4951 外，全仓不应有新的 `&self → &mut` 逃逸口。
#[test]
fn no_new_ref_to_mut_escape_hatch_anywhere_in_repo() {
    let violations = all_violations();
    let unknown: Vec<&Violation> = violations.iter().filter(|v| !is_known(v)).collect();

    if !unknown.is_empty() {
        let mut msg = String::from("发现未登记的 `&self -> &mut` 逃逸口：\n");
        for v in &unknown {
            msg.push_str(&format!(
                "  {}:{}  [{}]\n      {}\n",
                v.file, v.line, v.pattern, v.text
            ));
        }
        msg.push_str(
            "\n若确认安全，请把它加进本文件的 KNOWN 表并写明依据（issue 号 + 为什么别名不成立）。\n\
             不要靠改函数名绕过 —— 本守卫扫的是模式，不是名字。",
        );
        panic!("{msg}");
    }
}

/// 自证：这个守卫**确实能**抓到人为加入的逃逸口。
///
/// 变异测试的价值在于证明测试有效，而不是让测试通过。BLK-2 的教训
/// 正是「断言了 wrapper 转发，而转发到的调用点根本没人走」，结果
/// 空转。
#[test]
fn guard_catches_a_planted_escape_hatch() {
    // 与真实 `as_mut_self` 同构的片段（换个名字与类型，模拟「改名绕过」）
    let planted = "\
impl Demo {
    fn as_mut_me(&self) -> &mut Self {
        unsafe { &mut *(self as *const Self as *mut Self) }
    }
}
";
    let mut found = Vec::new();
    scan(planted, "crates/demo/src/planted.rs", &mut found);
    assert!(
        !found.is_empty(),
        "守卫扫不到人为植入的逃逸口 —— 整个守卫是无效的"
    );
    assert_eq!(found[0].pattern, "bare-pointer-cast");
    assert_eq!(found[0].line, 3, "行号定位错误，失败信息会误导人");
}

/// 反向自证：`&mut self` 方法里的同类写法**不应**被报成违例。
///
/// `wal_storage.rs:282` 的 `split(&mut self)` / `:292` 的
/// `recover_split_mut(&mut self)` 就是这一类 —— 独占借用下从
/// `UnsafeCell` 取 `&mut` 是 sound 的。若守卫把它们也报出来，噪声会
/// 大到让人开始忽略它，那比没有守卫更糟。
#[test]
fn guard_allows_same_primitive_under_mut_self() {
    let legit = "\
impl Wal<T> {
    pub fn split(&mut self) -> (&mut T, Guard) {
        let inner = unsafe { &mut *self.inner.get() };
        (inner, self.wal.lock())
    }
}
";
    let mut found = Vec::new();
    scan(legit, "crates/storage/src/wal_storage.rs", &mut found);
    assert!(
        found.is_empty(),
        "`&mut self` 下的合法 UnsafeCell 用法被误报: {found:?}"
    );
}

/// 自证：注释与文档里的同名字样**不**应被误报，否则守卫会被人加
/// 注释绕过。
#[test]
fn guard_ignores_comments_and_docs() {
    let benign = "\
// unsafe { &mut *(self as *const Self as *mut Self) }
/// Previously did: unsafe { &mut *(self as *const Self as *mut Self) }
* a doc line mentioning as *const Self as *mut Self
fn real_code(&self) -> u32 { self.counter }
";
    let mut found = Vec::new();
    scan(benign, "benign.rs", &mut found);
    assert!(found.is_empty(), "注释/文档被误报: {found:?}");
}

/// 自证：`&raw mut` 的**正常**用法（对独立局部变量取地址）不应被误报。
#[test]
fn guard_does_not_flag_legitimate_raw_mut_use() {
    let benign = "\
fn f(&self) {
    let mut n = 0;
    let p = &raw mut n;
    unsafe { *p += 1 };
}
";
    let mut found = Vec::new();
    scan(benign, "ok.rs", &mut found);
    assert!(found.is_empty(), "合法的 &raw mut 被误报: {found:?}");
}

/// #4951 已修复：`as_mut_self` 不得再出现，且 KNOWN 表必须已清空。
///
/// 这条测试原本**故意写成失败**（断言 `as_mut_self` 仍然存在），用来在
/// 修复落地时提醒维护者把 KNOWN 条目一并清掉。现在 `as_mut_self` 已删，
/// tripwire 兑现，反转为正向断言 —— 它从此是回归防线而非一次性提醒。
#[test]
fn block_4951_as_mut_self_stays_removed() {
    let root = repo_root();
    let target = root.join("crates/storage/src/file_storage.rs");
    let text = std::fs::read_to_string(&target)
        .unwrap_or_else(|e| panic!("读不到 {}: {e}", target.display()));
    assert!(
        !text.contains("fn as_mut_self"),
        "#4951 回归：`as_mut_self` 又出现在 file_storage.rs 里了。\n\
         受保护的字段现在住在 `RwLock<WriteState>` 里，理论上不再需要任何\
         `&self -> &mut` 逃逸口。若确有新需求，请先说明为什么 RwLock 方案\
         不适用，并把条目登记进 KNOWN 表。"
    );
    assert!(
        KNOWN.is_empty(),
        "#4951 清理未完成：KNOWN 表里仍有条目，但全仓已无已知违例。\n\
         条目: {KNOWN:?}"
    );
}

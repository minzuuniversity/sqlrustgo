//! 测试清单诚实性回归套件 —— Issue #5449
//!
//! `check_test_target_execution.sh`（#5379 建立）比对的是「**已声明**的目标」与
//! 「**被执行**的目标」。它看不见第三类：**既没被声明、也没被执行**的文件。
//! 而这恰恰是 #5449 的全部内容。
//!
//! 实测（基线 `13471d24cf6e`）：
//!
//! | 类别 | 目标数 | `#[test]` 用例 | 是否执行 |
//! |------|--------|---------------|----------|
//! | 已声明 `[[test]]` | 449 | — | 是 |
//! | 顶层 `tests/*.rs` 靠 cargo 自动发现 | 15 | 72 | **是**，但清单里查不到 |
//! | 顶层 `tests/sequence_test.rs` | 1 | 10 | **否** —— 被同名显式声明静默丢弃 |
//! | 子目录孤儿文件 | 6 | 34 | **否** —— 从未被编译 |
//!
//! `tests/sequence_test.rs` 是最隐蔽的一类：cargo 的自动发现按**文件名**推导
//! 目标名，而清单里已有一条 `[[test]] name = "sequence_test"` 指向
//! `tests/integration/sql/sequence_test.rs`。名字被占 ⇒ cargo **静默丢弃**自动
//! 发现的结果，**没有任何诊断**。`cargo metadata` 是唯一能看见它的入口。
//!
//! 设计取向
//! --------
//! * **判据写成不变量**：「凡含 `#[test]` 的测试文件，要么被声明、要么能从某个
//!   被声明的目标可达」——不是「给这 7 个文件补 7 条声明」。
//! * **能跑的就真跑**：`cargo metadata` 与 cargo 自己的目标表对账交给门禁脚本
//!   （`check_test_manifest_honesty.sh`），这里只断言它的退出码。
//! * **清单是 SSOT**：顶层文件必须显式声明，这样「cargo 自动发现了什么」不再是
//!   一个只能靠读 cargo 源码才知道的事实。

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// 仓库根目录。
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// 一条 `[[test]]` 声明。
#[derive(Debug, Clone)]
struct TestTarget {
    name: String,
    path: String,
}

/// 解析 Cargo.toml 里的 `[[test]]` 段。
///
/// 手写解析而非引入 toml 依赖：CI 镜像不保证 python >= 3.11 / toml 解析器，
/// 而 `[[test]]` 只是一个扁平的 name/path 对。
fn parse_declared_targets(cargo_toml: &str) -> Vec<TestTarget> {
    let mut declared: Vec<TestTarget> = Vec::new();
    let mut in_test = false;
    let mut name: Option<String> = None;
    let mut path: Option<String> = None;

    for line in cargo_toml.lines() {
        let stripped = line.trim();
        if stripped.starts_with('#') {
            continue;
        }
        if stripped == "[[test]]" {
            push_target(&mut declared, &mut name, &mut path);
            in_test = true;
            continue;
        }
        if !in_test {
            continue;
        }
        // 另一个表头结束了当前 `[[test]]` 段。
        if line.trim_start().starts_with('[') {
            push_target(&mut declared, &mut name, &mut path);
            in_test = false;
            continue;
        }
        if let Some(v) = quoted_value(line, "name") {
            name = Some(v);
        } else if let Some(v) = quoted_value(line, "path") {
            path = Some(v);
        }
    }
    push_target(&mut declared, &mut name, &mut path);
    declared
}

/// 把当前累积的 name/path 落成一条声明。
fn push_target(
    declared: &mut Vec<TestTarget>,
    name: &mut Option<String>,
    path: &mut Option<String>,
) {
    if let Some(n) = name.take() {
        let default_path = format!("tests/{}.rs", n);
        declared.push(TestTarget {
            name: n,
            path: path.take().unwrap_or(default_path),
        });
    }
    path.take();
}

/// 从 `key = "value"` 形式里取值。
fn quoted_value(line: &str, key: &str) -> Option<String> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix(key)?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// 文件里的 `#[test]` / `#[tokio::test]` 数量。
fn test_fn_count(src: &str) -> usize {
    src.lines()
        .filter(|l| {
            let t = l.trim();
            t.starts_with("#[test]") || t.starts_with("#[tokio::test]")
        })
        .count()
}

/// 从已声明目标出发，沿 `#[path = "..."]` 与 `mod x;` 求可达集合。
fn reachable_from(declared_paths: &[String], root: &Path) -> HashSet<String> {
    let mut reachable: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = declared_paths.to_vec();

    while let Some(cur) = stack.pop() {
        if !reachable.insert(cur.clone()) {
            continue;
        }
        let full = root.join(&cur);
        let Ok(src) = std::fs::read_to_string(&full) else {
            continue;
        };
        let base = Path::new(&cur)
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_default();

        // `#[path = "../common/x.rs"]`
        for line in src.lines() {
            let t = line.trim();
            let Some(rest) = t.strip_prefix("#[path") else {
                continue;
            };
            let Some(start) = rest.find('"') else {
                continue;
            };
            let after = &rest[start + 1..];
            let Some(end) = after.find('"') else {
                continue;
            };
            stack.push(normalize(&base.join(&after[..end])));
        }

        // `mod x;` —— rust 的默认模块解析：x.rs 或 x/mod.rs
        for line in src.lines() {
            let t = line.trim();
            let t = t.strip_prefix("pub(crate) ").unwrap_or(t);
            let t = t.strip_prefix("pub ").unwrap_or(t);
            let Some(rest) = t.strip_prefix("mod ") else {
                continue;
            };
            // 名字后面跟的是 `;`（或 `; // 注释`），先去掉分隔符。
            let mod_name = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(';');
            if mod_name.is_empty() || !mod_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let cand_a = base.join(format!("{}.rs", mod_name));
            if cand_a.exists() {
                stack.push(normalize(&cand_a));
            }
            let cand_b = base.join(mod_name).join("mod.rs");
            if cand_b.exists() {
                stack.push(normalize(&cand_b));
            }
        }
    }
    reachable
}

/// 把路径规范成仓库相对、`/` 分隔、无前导 `./`、无 `..` 的形式。
///
/// `#[path = "../common/x.rs"]` 里的 `..` 必须在这里解掉 —— 否则
/// `tests/integration/../common/x.rs` 与 `tests/common/x.rs` 会被当成两个
/// 不同的文件，可达性分析就会把真正可达的辅助模块误报成孤儿。
fn normalize(p: &Path) -> String {
    let raw = p.to_string_lossy().replace('\\', "/");
    let mut out: Vec<&str> = Vec::new();
    for seg in raw.split('/') {
        match seg {
            "" | "." => continue,
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    out.join("/")
}

/// 收集 `tests/` 下的全部 `.rs`（仓库相对路径）。
fn collect_test_sources(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![root.join("tests")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                let rel = path
                    .strip_prefix(root)
                    .map(|p| normalize(p))
                    .unwrap_or_else(|_| normalize(&path));
                out.push(rel);
            }
        }
    }
    out.sort();
    out
}

// =============================================================================
// 不变量
// =============================================================================

/// **不变量**：任何含 `#[test]` 的 `tests/**/*.rs`，要么是已声明的 `[[test]]`
/// 目标，要么能从某个已声明目标经 `#[path]` / `mod` 可达。
///
/// 不可达的测试文件 ⇒ 从未被编译 ⇒ 它的每个 `#[test]` 都是一句空话。
#[test]
fn no_orphan_test_file() {
    let root = repo_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declared = parse_declared_targets(&cargo_toml);
    let declared_paths: Vec<String> = declared.iter().map(|t| t.path.clone()).collect();
    let declared_set: HashSet<String> = declared_paths.iter().cloned().collect();
    let reachable = reachable_from(&declared_paths, &root);

    let mut orphans = Vec::new();
    for rel in collect_test_sources(&root) {
        if declared_set.contains(&rel) || reachable.contains(&rel) {
            continue;
        }
        let src = std::fs::read_to_string(root.join(&rel)).unwrap();
        let n = test_fn_count(&src);
        if n > 0 {
            orphans.push(format!("{} ({} 个 #[test])", rel, n));
        }
    }

    assert!(
        orphans.is_empty(),
        "以下测试文件既未被 Cargo.toml 声明、也无法从任何已声明目标可达 ⇒ 从未执行:\n  {}",
        orphans.join("\n  ")
    );
}

/// **不变量**：每个顶层 `tests/*.rs` 都有自己的 `[[test]]` 声明。
///
/// 顶层文件默认由 cargo 自动发现执行，但它们因此**在清单里不可见**——所有基于
/// 清单的审计（以及读 Cargo.toml 的人）都看不到它们。
#[test]
fn every_top_level_test_file_has_explicit_declaration() {
    let root = repo_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declared = parse_declared_targets(&cargo_toml);
    let declared_set: HashSet<String> = declared.iter().map(|t| t.path.clone()).collect();

    let mut missing = Vec::new();
    for rel in collect_test_sources(&root) {
        if rel.contains('/') {
            continue;
        }
        if !declared_set.contains(&rel) {
            missing.push(rel);
        }
    }
    missing.sort();

    assert!(
        missing.is_empty(),
        "以下顶层 tests/*.rs 没有显式 [[test]] 声明（cargo 自动发现 ⇒ 清单不可见）:\n  {}",
        missing.join("\n  ")
    );
}

/// **不变量**：顶层测试文件不会被 cargo 静默丢弃。
///
/// cargo 按**文件名**推导自动发现的目标名。清单里若已有一条同名声明指向别的
/// 文件，cargo 会**静默丢弃**自动发现的结果——没有任何诊断，那 10 个
/// `#[test]` 就此永远不执行。
#[test]
fn no_top_level_file_is_silently_shadowed() {
    let root = repo_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declared = parse_declared_targets(&cargo_toml);

    let mut by_name: HashMap<&str, &TestTarget> = HashMap::new();
    for t in &declared {
        by_name.insert(t.name.as_str(), t);
    }
    let declared_set: HashSet<String> = declared.iter().map(|t| t.path.clone()).collect();

    let mut shadowed = Vec::new();
    for rel in collect_test_sources(&root) {
        if rel.contains('/') {
            continue;
        }
        let stem = Path::new(&rel)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        if let Some(other) = by_name.get(stem.as_str()) {
            if other.path != rel {
                shadowed.push(format!(
                    "{} 的目标名 `{}` 已被 {} 占用 ⇒ cargo 静默丢弃本文件",
                    rel, stem, other.path
                ));
            }
        } else if !declared_set.contains(&rel) {
            shadowed.push(format!("{} 没有自己的 [[test]] 声明", rel));
        }
    }
    shadowed.sort();

    assert!(
        shadowed.is_empty(),
        "以下顶层测试文件会被 cargo 静默丢弃或缺少声明:\n  {}",
        shadowed.join("\n  ")
    );
}

/// **不变量**：已声明的目标路径都真实存在。
#[test]
fn every_declared_target_exists() {
    let root = repo_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declared = parse_declared_targets(&cargo_toml);
    let missing: Vec<String> = declared
        .iter()
        .filter(|t| !root.join(&t.path).exists())
        .map(|t| t.path.clone())
        .collect();
    assert!(
        missing.is_empty(),
        "已声明但文件不存在的 [[test]] 目标:\n  {}",
        missing.join("\n  ")
    );
}

/// **不变量**：目标名唯一。
#[test]
fn no_duplicate_target_names() {
    let root = repo_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declared = parse_declared_targets(&cargo_toml);
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut dupes = Vec::new();
    for t in &declared {
        let e = seen.entry(t.name.as_str()).or_insert(0);
        *e += 1;
        if *e == 2 {
            dupes.push(t.name.clone());
        }
    }
    dupes.sort();
    assert!(dupes.is_empty(), "重复的 [[test]] 目标名: {:?}", dupes);
}

// =============================================================================
// #5449 点名的 7 个文件 —— 逐个钉死，防止回归
// =============================================================================

/// #5449 点名的 6 个「从未被执行」的文件必须都成为已声明目标。
#[test]
fn issue_5449_orphan_files_are_now_declared() {
    let root = repo_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declared = parse_declared_targets(&cargo_toml);
    let declared_set: HashSet<String> = declared.iter().map(|t| t.path.clone()).collect();

    let files = [
        "tests/compat/bustubx_differential_test.rs",
        "tests/compat/differential_framework.rs",
        "tests/compat/semantic_kernel.rs",
        "tests/integration/sql/v312_90_sign_function_test.rs",
        "tests/integration/storage_engine_api_test.rs",
        "tests/soak/v312_mixed_soak.rs",
    ];
    let missing: Vec<&str> = files
        .iter()
        .copied()
        .filter(|f| !declared_set.contains(*f))
        .collect();
    assert!(
        missing.is_empty(),
        "以下 #5449 点名的孤儿文件仍未被声明 ⇒ 仍然从不执行:\n  {}",
        missing.join("\n  ")
    );
}

/// 顶层 `tests/sequence_test.rs`（被同名声明静默丢弃的那一个）必须有自己的目标。
#[test]
fn issue_5449_shadowed_top_level_sequence_test_is_declared() {
    let root = repo_root();
    let cargo_toml = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let declared = parse_declared_targets(&cargo_toml);
    let declared_set: HashSet<String> = declared.iter().map(|t| t.path.clone()).collect();

    assert!(
        declared_set.contains("tests/sequence_test.rs"),
        "tests/sequence_test.rs 仍未被声明 —— 它会被同名目标静默丢弃，10 个 #[test] 永不执行"
    );
}

// =============================================================================
// 门禁脚本本身
// =============================================================================

/// 门禁脚本必须存在、可执行，且在当前仓库上判 PASS。
///
/// 脚本内部的 C5 用 `cargo metadata` 与 cargo 自己的目标表对账 —— 那是
/// 「cargo 认为本仓库有哪些测试目标」的唯一权威来源。
#[test]
fn gate_script_passes_on_current_tree() {
    let root = repo_root();
    let script = root.join("scripts/gate/check_test_manifest_honesty.sh");
    assert!(script.exists(), "门禁脚本缺失: {}", script.display());

    let out = Command::new("bash")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_test_manifest_honesty.sh");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    assert!(
        out.status.success(),
        "check_test_manifest_honesty.sh 判 FAIL:\n--- stdout ---\n{}\n--- stderr ---\n{}",
        stdout,
        stderr
    );
}

/// 门禁脚本必须真的会因为孤儿文件而红 —— 用一个临时仓库副本证明它有判别力。
///
/// 一个只会输出 PASS 的门禁脚本，比没有门禁更糟。
#[test]
fn gate_script_detects_an_injected_orphan() {
    let root = repo_root();
    let tmp = std::env::temp_dir().join(format!(
        "test-manifest-honesty-{}-{}",
        std::process::id(),
        "orphan"
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(tmp.join("tests")).unwrap();
    std::fs::create_dir_all(tmp.join("scripts/gate")).unwrap();

    // 一个孤儿测试文件：不在任何声明里，也不可达。
    std::fs::write(
        tmp.join("tests/orphan_probe.rs"),
        "#[test]\nfn orphan_probe() {}\n",
    )
    .unwrap();
    std::fs::write(tmp.join("Cargo.toml"), "[[package]]\nname = \"probe\"\nversion = \"0.0.0\"\nedition = \"2021\"\n\n[[test]]\nname = \"declared\"\npath = \"tests/declared.rs\"\n").unwrap();
    std::fs::write(tmp.join("tests/declared.rs"), "#[test]\nfn declared() {}\n").unwrap();

    // 把门禁脚本复制过去（C1..C5 都是静态分析，不需要真实 cargo 工程）。
    let src =
        std::fs::read_to_string(root.join("scripts/gate/check_test_manifest_honesty.sh")).unwrap();
    std::fs::write(tmp.join("scripts/gate/check_test_manifest_honesty.sh"), src).unwrap();

    let out = Command::new("bash")
        .arg(tmp.join("scripts/gate/check_test_manifest_honesty.sh"))
        .current_dir(&tmp)
        .output()
        .expect("run gate on probe tree");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        !out.status.success(),
        "门禁脚本对含孤儿文件的仓库判了 PASS —— 它没有判别力:\n{}",
        stdout
    );
    assert!(
        stdout.contains("orphan_probe.rs"),
        "门禁脚本的输出没有点名孤儿文件:\n{}",
        stdout
    );

    let _ = std::fs::remove_dir_all(&tmp);
}

use regex::Regex;
use std::path::Path;
use std::process::exit;
use walkdir::WalkDir;

struct Config {
    check_mainline: bool,
    check_forbidden: bool,
    check_deps: bool,
    check_wal: bool,
    mode: String,
}

impl Config {
    fn new() -> Self {
        let mut cfg = Config {
            check_mainline: false,
            check_forbidden: false,
            check_deps: false,
            check_wal: false,
            mode: "all".to_string(),
        };

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--mode=mainline" => {
                    cfg.mode = "mainline".to_string();
                    cfg.check_mainline = true;
                }
                "--mode=forbidden" => {
                    cfg.mode = "forbidden".to_string();
                    cfg.check_forbidden = true;
                }
                "--mode=wal" => {
                    cfg.mode = "wal".to_string();
                    cfg.check_wal = true;
                }
                "--mode=all" => {
                    cfg.check_mainline = true;
                    cfg.check_forbidden = true;
                    cfg.check_deps = true;
                    cfg.check_wal = true;
                }
                _ => {
                    eprintln!("Unknown argument: {}", arg);
                    eprintln!("Usage: xtask architecture-check [--mode=<mode>]");
                    eprintln!("  Modes: mainline, forbidden, wal, all");
                    exit(1);
                }
            }
        }

        cfg
    }
}

fn main() {
    let cfg = Config::new();
    let mut errors = Vec::new();

    println!("=== Architecture Check ===\n");

    if cfg.check_mainline || cfg.mode == "all" {
        println!("[1/4] Checking mainline path violations...");
        check_dml_without_txn(&mut errors);
    }

    if cfg.check_forbidden || cfg.mode == "all" {
        println!("[2/4] Checking forbidden patterns...");
        check_isolated_in_mainline(&mut errors);
    }

    if cfg.check_deps || cfg.mode == "all" {
        println!("[3/4] Checking dependency rules...");
        check_forbidden_deps(&mut errors);
    }

    if cfg.check_wal || cfg.mode == "all" {
        println!("[4/4] Checking WAL integration...");
        check_wal_in_executor(&mut errors);
    }

    println!("\n=== Results ===\n");

    if errors.is_empty() {
        println!("✅ No architecture violations found.");
        exit(0);
    }

    println!("❌ Found {} violation(s):\n", errors.len());
    for (i, err) in errors.iter().enumerate() {
        println!("{}. {}", i + 1, err);
    }

    exit(1)
}

fn check_dml_without_txn(errors: &mut Vec<String>) {
    let forbidden_re = Regex::new(r"storage\.(insert|update|delete)\s*\(").unwrap();

    for entry in WalkDir::new("crates/executor/src")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if forbidden_re.is_match(&content) {
                let has_txn = content.contains("txn")
                    || content.contains("transaction")
                    || content.contains("wal");
                if !has_txn {
                    errors.push(format!(
                        "F1: DML without txn/wal in {}",
                        entry.path().display()
                    ));
                }
            }
        }
    }
}

fn check_isolated_in_mainline(errors: &mut Vec<String>) {
    let isolated_modules = [
        "parallel_executor",
        "vec_simd",
        "expr-legacy",
        "local_executor_dml",
    ];

    for entry in WalkDir::new("crates")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            for module in &isolated_modules {
                if content.contains(&format!("use sqlrustgo_{}", module))
                    || content.contains(&format!("mod {};", module))
                {
                    errors.push(format!(
                        "F2: Isolated module '{}' used in {}",
                        module,
                        entry.path().display()
                    ));
                }
            }
        }
    }
}

fn check_forbidden_deps(_errors: &mut Vec<String>) {
    // Placeholder - would need cargo tree parsing
    println!("  (dependency check skipped - requires cargo tree)")
}

fn check_wal_in_executor(errors: &mut Vec<String>) {
    let executor_files = [
        "crates/executor/src/executor.rs",
        "crates/executor/src/local_executor.rs",
        "crates/executor/src/transactional_executor.rs",
    ];

    for file in &executor_files {
        let path = Path::new(file);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(path) {
                let has_wal = content.contains("wal")
                    || content.contains("Wal")
                    || content.contains("WalStorage");
                if !has_wal {
                    errors.push(format!(
                        "R1: Executor {} missing WAL integration",
                        path.display()
                    ));
                }
            }
        }
    }
}
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let crates_dir = Path::new("crates");
    if !crates_dir.exists() {
        eprintln!("Error: crates/ directory not found");
        std::process::exit(1);
    }

    let mut isolated = Vec::new();
    let mut deprecated = Vec::new();
    let mut experimental = Vec::new();
    let mut frozen = Vec::new();

    for entry in std::fs::read_dir(crates_dir).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name().to_string_lossy().to_string();

        if name.starts_with('.') || name == "README.md" {
            continue;
        }

        let src_dir = entry.path().join("src");
        if !src_dir.exists() {
            continue;
        }

        if has_direct_storage_call(&entry.path()) {
            continue;
        }

        if is_mainline_module(&name) {
            continue;
        }

        let status = determine_status(&name, &entry.path());
        match status.as_str() {
            "isolated" => isolated.push(name),
            "deprecated" => deprecated.push(name),
            "experimental" => experimental.push(name),
            "frozen" => frozen.push(name),
            _ => {}
        }
    }

    println!("=== Module Status Report ===\n");

    if !isolated.is_empty() {
        println!("ISOLATED modules (not in mainline):");
        for m in &isolated {
            println!("  - {}", m);
        }
        println!();
    }

    if !deprecated.is_empty() {
        println!("DEPRECATED modules (pending deletion):");
        for m in &deprecated {
            println!("  - {}", m);
        }
        println!();
    }

    if !experimental.is_empty() {
        println!("EXPERIMENTAL modules (feature-gated):");
        for m in &experimental {
            println!("  - {}", m);
        }
        println!();
    }

    if !frozen.is_empty() {
        println!("FROZEN modules (development paused):");
        for m in &frozen {
            println!("  - {}", m);
        }
        println!();
    }

    if isolated.len() + deprecated.len() + experimental.len() + frozen.len() == 0 {
        println!("No isolated/deprecated/experimental/frozen modules found.");
        println!("All crates are in mainline.");
    }

    println!("\nFor details, see:");
    println!("  - MAINLINE_COMPONENTS.md");
    println!("  - ISOLATED_MODULES.md");
    println!("  - docs/MODULE_LIFECYCLE.md");
}

fn is_mainline_module(name: &str) -> bool {
    matches!(
        name,
        "parser"
            | "planner"
            | "optimizer"
            | "executor"
            | "types"
            | "storage"
            | "transaction"
            | "catalog"
            | "network"
            | "common"
            | "information-schema"
            | "server"
            | "sql-cli"
            | "bench"
            | "bench-cli"
            | "telemetry"
            | "tools"
    )
}

fn determine_status(name: &str, path: &Path) -> String {
    let cargo_toml = path.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
            if content.contains("# experimental") || content.contains("experimental = true") {
                return "experimental".to_string();
            }
            if content.contains("# frozen") || content.contains("frozen = true") {
                return "frozen".to_string();
            }
        }
    }

    if name.contains("legacy")
        || name.contains("deprecated")
        || name == "expr-legacy"
    {
        return "deprecated".to_string();
    }

    if name == "distributed" || name == "graph" || name == "vector" {
        return "frozen".to_string();
    }

    if name == "vec_simd" || name.contains("simd") {
        return "experimental".to_string();
    }

    "isolated".to_string()
}

fn has_direct_storage_call(path: &Path) -> bool {
    let src_dir = path.join("src");
    if !src_dir.exists() {
        return false;
    }

    for entry in WalkDir::new(&src_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
    {
        if let Ok(content) = std::fs::read_to_string(entry.path()) {
            if content.contains("storage.insert")
                || content.contains("storage.update")
                || content.contains("storage.delete")
            {
                if !content.contains("txn")
                    && !content.contains("transaction")
                    && !content.contains("wal")
                {
                    return true;
                }
            }
        }
    }
    false
}
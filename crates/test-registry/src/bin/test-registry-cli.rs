//! `test-registry-cli` — manage the on-disk `test-registry.toml` manifest.
//!
//! V312-24 activation: this binary drives the on-disk layer of `TestRegistry`.
//! The in-source `TestMetadata` layer is still managed programmatically
//! (see `TestRegistryBuilder` in the library).
//!
//! Subcommands:
//!   - `init <path>` — write a starter manifest with `sqlancer` +
//!     `test-runner` entries; overwrites if exists.
//!   - `list <path>` — print the entries in the manifest as a table.
//!   - `register <path> <name> <binary> [--args a b c] [--timeout-ms N]`
//!     — append a single `[[test]]` entry to the manifest, loading the
//!     existing file first if present.
//!   - `print-schema` — print the TOML schema reference to stdout.
//!
//! Exit codes:
//!   0 — success
//!   1 — I/O / parse / argument error
//!   2 — manifest was missing when required

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use test_registry::{ManagedTest, RegistryError, TestCategory, TestPriority, TestRegistry};

#[derive(Debug)]
enum Command {
    Init {
        path: PathBuf,
    },
    List {
        path: PathBuf,
    },
    Register {
        path: PathBuf,
        name: String,
        binary: String,
        args: Vec<String>,
        timeout_ms: Option<u64>,
        priority: Option<TestPriority>,
        category: Option<TestCategory>,
    },
    PrintSchema,
    Help,
}

struct Args {
    cmd: Command,
}

fn print_help() {
    eprintln!(
        "test-registry-cli — manage the test-registry.toml manifest\n\n\
         USAGE:\n  \
         test-registry-cli <COMMAND> [ARGS]\n\n\
         COMMANDS:\n  \
         init <PATH>             Write a starter manifest (sqlancer + test-runner)\n  \
         list <PATH>             Print registered entries\n  \
         register <PATH> <NAME> <BINARY> [OPTIONS]\n                              \
                                Add one [[test]] entry\n  \
         print-schema            Print the TOML schema reference\n  \
         help                    Show this help\n\n\
         OPTIONS for `register`:\n  \
             --args <A> <B> ...   Arguments to pass to the binary\n  \
             --timeout-ms <MS>    Per-binary wall-clock budget\n  \
             --priority <P0..P4>  Priority class\n  \
             --category <NAME>    unit|integration|anomaly|stress|e2e|ci\n"
    );
}

fn parse_priority(s: &str) -> Result<TestPriority, String> {
    match s.to_ascii_uppercase().as_str() {
        "P0" => Ok(TestPriority::P0),
        "P1" => Ok(TestPriority::P1),
        "P2" => Ok(TestPriority::P2),
        "P3" => Ok(TestPriority::P3),
        "P4" => Ok(TestPriority::P4),
        _ => Err(format!("invalid priority {} (expected P0..P4)", s)),
    }
}

fn parse_category(s: &str) -> Result<TestCategory, String> {
    match s.to_ascii_lowercase().as_str() {
        "unit" => Ok(TestCategory::Unit),
        "integration" => Ok(TestCategory::Integration),
        "anomaly" => Ok(TestCategory::Anomaly),
        "stress" => Ok(TestCategory::Stress),
        "e2e" => Ok(TestCategory::E2E),
        "ci" => Ok(TestCategory::CI),
        _ => Err(format!(
            "invalid category {} (expected unit|integration|anomaly|stress|e2e|ci)",
            s
        )),
    }
}

fn parse_args<I: IntoIterator<Item = String>>(raw: I) -> Result<Args, String> {
    // Materialize once so we can push tokens back during parsing.
    let tokens: Vec<String> = raw.into_iter().collect();
    let mut idx: usize = 0;
    let cmd = {
        let first = tokens.get(idx).cloned().ok_or("missing command")?;
        idx += 1;
        match first.as_str() {
            "help" | "-h" | "--help" => Command::Help,
            "print-schema" => Command::PrintSchema,
            "init" => {
                let path = tokens.get(idx).cloned().ok_or("init: missing <path>")?;
                Command::Init {
                    path: PathBuf::from(path),
                }
            }
            "list" => {
                let path = tokens.get(idx).cloned().ok_or("list: missing <path>")?;
                Command::List {
                    path: PathBuf::from(path),
                }
            }
            "register" => {
                let path = tokens.get(idx).cloned().ok_or("register: missing <path>")?;
                idx += 1;
                let name = tokens.get(idx).cloned().ok_or("register: missing <name>")?;
                idx += 1;
                let binary = tokens
                    .get(idx)
                    .cloned()
                    .ok_or("register: missing <binary>")?;
                idx += 1;
                let mut args: Vec<String> = Vec::new();
                let mut timeout_ms: Option<u64> = None;
                let mut priority: Option<TestPriority> = None;
                let mut category: Option<TestCategory> = None;
                const KNOWN_FLAGS: &[&str] =
                    &["--args", "--timeout-ms", "--priority", "--category"];
                while let Some(flag) = tokens.get(idx).cloned() {
                    idx += 1;
                    if !flag.starts_with("--") {
                        return Err(format!(
                            "register: unexpected positional arg `{}` (use --args to pass through)",
                            flag
                        ));
                    }
                    match flag.as_str() {
                        "--args" => {
                            // Collect args until we hit a recognized flag
                            // or end-of-input. Bare `--` is end-of-args.
                            while let Some(t) = tokens.get(idx).cloned() {
                                if t == "--" {
                                    idx += 1;
                                    break;
                                }
                                if KNOWN_FLAGS.contains(&t.as_str()) {
                                    break; // leave for outer loop
                                }
                                idx += 1;
                                args.push(t);
                            }
                        }
                        "--timeout-ms" => {
                            let v = tokens
                                .get(idx)
                                .cloned()
                                .ok_or("--timeout-ms: missing <ms>")?
                                .parse::<u64>()
                                .map_err(|e| format!("--timeout-ms: {}", e))?;
                            idx += 1;
                            timeout_ms = Some(v);
                        }
                        "--priority" => {
                            let v = tokens
                                .get(idx)
                                .cloned()
                                .ok_or("--priority: missing <P0..P4>")?;
                            idx += 1;
                            priority = Some(parse_priority(&v)?);
                        }
                        "--category" => {
                            let v = tokens
                                .get(idx)
                                .cloned()
                                .ok_or("--category: missing <name>")?;
                            idx += 1;
                            category = Some(parse_category(&v)?);
                        }
                        other => return Err(format!("register: unknown flag {}", other)),
                    }
                }
                Command::Register {
                    path: PathBuf::from(path),
                    name,
                    binary,
                    args,
                    timeout_ms,
                    priority,
                    category,
                }
            }
            other => return Err(format!("unknown command: {} (try `help`)", other)),
        }
    };
    Ok(Args { cmd })
}

/// Build the canonical starter manifest (sqlancer + test-runner).
/// Mirrors the schema in `openspec/changes/v312-24-test-infra-activation/design.md`.
fn starter_manifest() -> TestRegistry {
    let mut r = TestRegistry::new();
    r.register_managed(
        ManagedTest::new("sqlancer", "target/release/sqlancer")
            .with_args(vec![
                "--duration".to_string(),
                "120".to_string(),
                "--out".to_string(),
                "target/sqlancer-report.json".to_string(),
            ])
            .with_timeout_ms(600_000)
            .with_priority(TestPriority::P1)
            .with_category(TestCategory::CI),
    );
    r.register_managed(
        ManagedTest::new("test-runner", "target/release/test-runner")
            .with_args(vec![
                "--out".to_string(),
                "target/test-runner-report.json".to_string(),
            ])
            .with_timeout_ms(300_000)
            .with_priority(TestPriority::P0)
            .with_category(TestCategory::Integration),
    );
    r
}

fn load_or_empty(path: &Path) -> Result<TestRegistry, RegistryError> {
    if !path.exists() {
        return Ok(TestRegistry::new());
    }
    TestRegistry::from_toml(path)
}

fn run_init(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("create_dir_all({}): {}", parent.display(), e))?;
        }
    }
    let r = starter_manifest();
    r.write_toml(path).map_err(|e| e.to_string())?;
    println!(
        "wrote starter manifest at {} ({} entries)",
        path.display(),
        r.managed_len()
    );
    Ok(())
}

fn run_list(path: &Path) -> Result<(), String> {
    let r = load_or_empty(path).map_err(|e| e.to_string())?;
    if r.managed_len() == 0 {
        println!("(no entries in {})", path.display());
        return Ok(());
    }
    println!("name              | binary                       | timeout_ms | priority | category");
    println!(
        "------------------+------------------------------+------------+----------+------------"
    );
    let mut entries: Vec<&ManagedTest> = r.managed().collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    for m in entries {
        println!(
            "{:<17} | {:<29} | {:>10} | {:>8?} | {:?}",
            m.name, m.binary, m.timeout_ms, m.priority, m.category
        );
    }
    println!("\n({} entries total)", r.managed_len());
    Ok(())
}

fn run_register(
    path: &Path,
    name: String,
    binary: String,
    args: Vec<String>,
    timeout_ms: Option<u64>,
    priority: Option<TestPriority>,
    category: Option<TestCategory>,
) -> Result<(), String> {
    let mut r = load_or_empty(path).map_err(|e| e.to_string())?;
    if r.get_managed(&name).is_some() {
        return Err(format!(
            "entry {} already exists in {}; remove it first",
            name,
            path.display()
        ));
    }
    let mut entry = ManagedTest::new(&name, &binary).with_args(args);
    if let Some(t) = timeout_ms {
        entry = entry.with_timeout_ms(t);
    }
    if let Some(p) = priority {
        entry = entry.with_priority(p);
    }
    if let Some(c) = category {
        entry = entry.with_category(c);
    }
    r.register_managed(entry);
    r.write_toml(path).map_err(|e| e.to_string())?;
    println!("registered {} in {}", name, path.display());
    Ok(())
}

fn run_print_schema() {
    println!(
        "# test-registry.toml schema\n\
         #\n\
         # A [[test]] table-array. Each entry describes a managed binary the\n\
         # test-runner can dispatch against. Order is not significant.\n\
         #\n\
         # Required fields:\n\
         #   name     — unique entry id (used as the key in the registry)\n\
         #   binary   — path or command name to invoke\n\
         #\n\
         # Optional fields (defaults shown):\n\
         #   args         = []\n\
         #   timeout_ms   = 120000\n\
         #   priority     = \"p2\"\n\
         #   category     = \"integration\"\n\
         #\n\
         # Example:\n\
         #\n\
         # [[test]]\n\
         # name = \"sqlancer\"\n\
         # binary = \"target/release/sqlancer\"\n\
         # args = [\"--duration\", \"120\", \"--out\", \"target/sqlancer-report.json\"]\n\
         # timeout_ms = 600000\n\
         # priority = \"p1\"\n\
         # category = \"ci\"\n"
    );
}

fn main() -> ExitCode {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    if raw.is_empty() {
        print_help();
        return ExitCode::from(1);
    }
    let args = match parse_args(raw) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("error: {}\n", e);
            print_help();
            return ExitCode::from(1);
        }
    };
    let result = match args.cmd {
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::PrintSchema => {
            run_print_schema();
            Ok(())
        }
        Command::Init { path } => run_init(&path),
        Command::List { path } => run_list(&path),
        Command::Register {
            path,
            name,
            binary,
            args,
            timeout_ms,
            priority,
            category,
        } => run_register(&path, name, binary, args, timeout_ms, priority, category),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {}", e);
            ExitCode::from(1)
        }
    }
}

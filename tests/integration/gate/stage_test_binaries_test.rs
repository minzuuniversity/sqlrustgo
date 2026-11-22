//! `stage_test_binaries.py` 的回归套件 —— 门禁曾**认证过期二进制**
//!
//! 事故
//! ----
//! `check_anti_fabrication.sh`（CHECK 1.5，V312-24）把 `cargo` 编出来的测试
//! 二进制摆到 `test-runner` manifest 指定的路径上，然后跑它们、据结果判
//! 绿红。旧实现在两处会失守：
//!
//! 1. 它**猜**哪个 artifact 是对的（"扫 `deps/`，取 mtime 最新的"）；
//! 2. 目标路径已存在时**直接跳过拷贝**，把旧文件当证据用。
//!
//! 合起来的后果是：**门禁在认证过期代码**。
//!
//! **2026-11-17 实测**：VtuGuard 的事务透传修复（`1f9425134d`，04:14）落地
//! 之后，门禁仍然报
//!
//! ```text
//! vtu_guard::tests::test_vtu_guard_tx_state_via_set_current_tx_id ... FAILED
//! assertion failed: guarded.in_transaction()
//! ```
//!
//! 它跑的是 02:30 那个**修复前**的二进制；cargo 在 05:31 已经编出了含修复
//! 的新二进制，只是没被拷过去。修复本身是好的（新二进制跑那条用例全绿），
//! 是门禁在测上一版。
//!
//! 修法：让 **cargo 自己**说出它这次编出了什么（`--message-format=json`），
//! 照着它给的 `executable` 路径摆，**每次都覆盖**，并且拒绝摆一个**比它
//! 自己的源码还旧**的 artifact（fail closed）。

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn script_path() -> PathBuf {
    repo_root()
        .join("scripts")
        .join("gate")
        .join("stage_test_binaries.py")
}

fn set_mtime(path: &Path, secs_since_epoch: u64) {
    let f = std::fs::File::open(path).expect("open for mtime");
    f.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(secs_since_epoch))
        .expect("set mtime");
}

/// The stager only accepts an executable artifact — the real ones are test
/// binaries. The fixtures must be executable too, or the fixture would be
/// testing the wrong thing.
#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perm = std::fs::metadata(path).expect("stat").permissions();
    perm.set_mode(0o755);
    std::fs::set_permissions(path, perm).expect("chmod");
}

/// A miniature cargo build tree.
///
/// Returns the root of the fixture.
fn fixture(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("sqlrustgo_stage_bins_{}", tag));
    let src = root.join("crates/storage/src");
    std::fs::create_dir_all(&src).expect("create src");
    std::fs::create_dir_all(root.join("target/release/deps")).expect("create target");
    std::fs::write(
        root.join("crates/storage/Cargo.toml"),
        "[package]\nname = \"sqlrustgo-storage\"\n",
    )
    .expect("write Cargo.toml");
    std::fs::write(src.join("lib.rs"), "pub fn f() {}\n").expect("write lib.rs");
    root
}

/// Write a cargo JSON build report with one compiler-artifact message.
fn write_build_json(root: &Path, name: &str, exe: &Path, test_profile: bool) {
    let msg = serde_json_line(
        name,
        exe.to_str().unwrap(),
        root.join("crates/storage/Cargo.toml").to_str().unwrap(),
        root.join("crates/storage/src/lib.rs").to_str().unwrap(),
        test_profile,
    );
    std::fs::write(root.join("build.json"), msg).expect("write build.json");
}

fn serde_json_line(
    name: &str,
    exe: &str,
    manifest_path: &str,
    src_path: &str,
    test_profile: bool,
) -> String {
    format!(
        "{{\"reason\":\"compiler-artifact\",\"executable\":{:?},\
         \"profile\":{{\"test\":{}}},\
         \"target\":{{\"name\":{:?},\"kind\":[\"lib\"],\"src_path\":{:?}}},\
         \"manifest_path\":{:?}}}\n",
        exe, test_profile, name, src_path, manifest_path
    )
}

fn write_manifest(root: &Path, binary_rel: &str) {
    std::fs::write(
        root.join("manifest.toml"),
        format!(
            "[[test]]\nname = \"storage-lib\"\nbinary = \"{}\"\nargs = [\"--test-threads=1\"]\n",
            binary_rel
        ),
    )
    .expect("write manifest");
}

fn run_stager(root: &Path) -> (i32, String, String) {
    let out = Command::new("python3")
        .arg(script_path())
        .arg(root.join("build.json"))
        .arg(root.join("manifest.toml"))
        .current_dir(root)
        .output()
        .expect("failed to spawn python3");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

/// The historical bug, as a test.
///
/// A destination that already exists must be **overwritten** with the binary
/// cargo just built — the old implementation skipped the copy and used the
/// stale file as the evidence.
#[test]
fn test_existing_stale_destination_is_overwritten_with_the_fresh_binary() {
    let root = fixture("overwrite");
    let src = root.join("crates/storage/src/lib.rs");
    let cargo_toml = root.join("crates/storage/Cargo.toml");
    set_mtime(&src, 1_000);
    set_mtime(&cargo_toml, 1_000);

    // The destination left over from an earlier run: OLD content.
    let dst = root.join("target/release/sqlrustgo_storage");
    std::fs::write(&dst, b"OLD-BINARY-FROM-THE-PREVIOUS-REVISION").expect("write old dst");
    make_executable(&dst);
    set_mtime(&dst, 900); // built before the sources changed

    // What cargo just built: NEW content, built after the sources.
    let exe = root.join("target/release/deps/sqlrustgo_storage-new");
    std::fs::write(&exe, b"NEW-BINARY-FROM-THIS-BUILD").expect("write new exe");
    make_executable(&exe);
    set_mtime(&exe, 2_000);

    write_build_json(&root, "sqlrustgo_storage", &exe, true);
    write_manifest(&root, "target/release/sqlrustgo_storage");

    let (code, stdout, stderr) = run_stager(&root);
    assert_eq!(code, 0, "staging should succeed; stderr: {}", stderr);
    assert!(
        stdout.contains("sqlrustgo_storage"),
        "the staged binary must be reported; got:\n{}",
        stdout
    );
    let content = std::fs::read(&dst).expect("read dst");
    assert_eq!(
        content, b"NEW-BINARY-FROM-THIS-BUILD",
        "the destination must be OVERWRITTEN with the binary cargo just built — \
         reusing the existing file is exactly how the gate certified a stale \
         binary on 2026-11-17"
    );
}

/// An artifact older than its own sources must be refused — fail closed.
///
/// This is the **reverse anchor** of the test above: without it, a stager
/// that always copies would pass, and the "never certify the previous
/// revision" promise would be unenforced.
#[test]
fn test_stale_artifact_is_refused() {
    let root = fixture("stale");
    let src = root.join("crates/storage/src/lib.rs");
    let cargo_toml = root.join("crates/storage/Cargo.toml");
    // The sources were changed AFTER the artifact was built.
    set_mtime(&cargo_toml, 5_000);
    set_mtime(&src, 5_000);

    let dst = root.join("target/release/sqlrustgo_storage");
    let exe = root.join("target/release/deps/sqlrustgo_storage-old");
    std::fs::write(&exe, b"OLD").expect("write old exe");
    make_executable(&exe);
    set_mtime(&exe, 1_000); // built long before the sources

    write_build_json(&root, "sqlrustgo_storage", &exe, true);
    write_manifest(&root, "target/release/sqlrustgo_storage");

    let (code, _stdout, stderr) = run_stager(&root);
    assert_ne!(
        code, 0,
        "a stale artifact must not be staged — the gate has to fail closed"
    );
    assert!(
        stderr.contains("STALE"),
        "the failure must name the problem; got:\n{}",
        stderr
    );
    assert!(
        !dst.exists(),
        "nothing may be written when the artifact is stale"
    );
}

/// A manifest binary that this build did not produce must be refused —
/// otherwise the gate would silently fall back to some other artifact.
#[test]
fn test_binary_not_built_by_this_run_is_refused() {
    let root = fixture("notbuilt");
    let src = root.join("crates/storage/src/lib.rs");
    let cargo_toml = root.join("crates/storage/Cargo.toml");
    set_mtime(&src, 1_000);
    set_mtime(&cargo_toml, 1_000);

    // cargo built `sqlrustgo_executor`, but the manifest wants `sqlrustgo_storage`.
    let exe = root.join("target/release/deps/sqlrustgo_executor-new");
    std::fs::write(&exe, b"EXECUTOR").expect("write exe");
    make_executable(&exe);
    set_mtime(&exe, 2_000);
    write_build_json(&root, "sqlrustgo_executor", &exe, true);
    write_manifest(&root, "target/release/sqlrustgo_storage");

    let (code, _stdout, stderr) = run_stager(&root);
    assert_ne!(
        code, 0,
        "a binary this build did not produce must be refused"
    );
    assert!(
        stderr.contains("did not build"),
        "the failure must say the binary was not built; got:\n{}",
        stderr
    );
}

/// A non-test artifact (the library itself) must not be mistaken for the
/// test binary.
#[test]
fn test_non_test_artifact_is_not_staged() {
    let root = fixture("non_test");
    let src = root.join("crates/storage/src/lib.rs");
    let cargo_toml = root.join("crates/storage/Cargo.toml");
    set_mtime(&src, 1_000);
    set_mtime(&cargo_toml, 1_000);

    let exe = root.join("target/release/deps/libsqlrustgo_storage.rlib");
    std::fs::write(&exe, b"RLIB").expect("write rlib");
    set_mtime(&exe, 2_000);
    write_build_json(&root, "sqlrustgo_storage", &exe, false);
    write_manifest(&root, "target/release/sqlrustgo_storage");

    let (code, _stdout, stderr) = run_stager(&root);
    assert_ne!(code, 0, "a non-test artifact must not be staged");
    assert!(
        stderr.contains("did not build"),
        "the failure must say no test binary was built; got:\n{}",
        stderr
    );
}

/// The gate must actually call the standalone stager — a second copy of the
/// logic inline in the gate script is how the two implementations drift.
#[test]
fn test_gate_script_delegates_to_the_stager() {
    let gate = std::fs::read_to_string(
        repo_root()
            .join("scripts")
            .join("gate")
            .join("check_anti_fabrication.sh"),
    )
    .expect("read the anti-fabrication gate");
    assert!(
        gate.contains("stage_test_binaries.py"),
        "the gate must delegate staging to scripts/gate/stage_test_binaries.py"
    );
    assert!(
        !gate.contains("Prefer the newest artifact"),
        "the old 'guess by mtime' staging must not come back"
    );
}

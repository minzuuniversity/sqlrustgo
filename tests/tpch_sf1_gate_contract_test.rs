use std::path::PathBuf;

fn repo_root() -> PathBuf {
    std::env::current_dir().expect("current dir")
}

#[test]
fn tpch_sf1_gate_script_exists_and_delegates_to_baseline() {
    let gate_path = repo_root().join("scripts/gate/check_tpch_sf1.sh");
    assert!(
        gate_path.exists(),
        "scripts/gate/check_tpch_sf1.sh must exist for issue #3732"
    );

    let content = std::fs::read_to_string(&gate_path).expect("read check_tpch_sf1.sh");
    assert!(
        content.contains("scripts/tpch_sf1_baseline.sh")
            || content.contains("tpch_sf1_baseline.sh"),
        "check_tpch_sf1.sh must run the SF=1 baseline wrapper"
    );
}

#[test]
fn tpch_sf1_baseline_uses_canonical_dbgen_lineitem_count() {
    let script_path = repo_root().join("scripts/tpch_sf1_baseline.sh");
    let content = std::fs::read_to_string(&script_path).expect("read tpch_sf1_baseline.sh");
    assert!(
        content.contains("lineitem:6001215"),
        "SF=1 official dbgen lineitem count must be 6,001,215"
    );
    assert!(
        !content.contains("lineitem:6000000"),
        "SF=1 gate must not reject official dbgen lineitem count 6,001,215"
    );
}

#[test]
fn tpch_sf1_test_can_use_operator_supplied_fixture_dir() {
    let test_path = repo_root().join("tests/tpch_sf1_22_vs_3engines_test.rs");
    let content = std::fs::read_to_string(&test_path).expect("read tpch sf1 test");
    assert!(
        content.contains("TPCH_SF1_DIR"),
        "SF=1 test must honor TPCH_SF1_DIR so gate and test use the same verified fixture"
    );
}

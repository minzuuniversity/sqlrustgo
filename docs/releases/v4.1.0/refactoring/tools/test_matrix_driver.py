#!/usr/bin/env python3
"""matrix_driver.py 的单元夹具 —— 证明适配层真的会咬。

WHY（2026-11-23）
------------------
适配层的价值全在两件事上：把 pilot 词汇翻译成契约词汇，以及**不把「没跑」
说成「跑了」**。一个恒绿的适配层比没有适配层更坏 —— 它给下游任务一个假信号。
本夹具对这两件事各做一次变异，要求适配层必须给出对应的结论。

覆盖的用例：

  D1  pilot 词汇 FAILED_RED  → execution_status == "KNOWN_FAIL"
  D2  同上 → red_identity 五元组齐全且非空
  D3  pilot 词汇 COVERED_PASS → execution_status == "PASS" 且带覆盖来源标注
  D4  工厂报告零测试          → execution_status == "NOT_RUN" 且 != "PASS"
  D5  bespoke 探针场景        → NOT_RUN，且理由写进 actual，绝不 PASS
  D6  KNOWN_FAIL 缺 red_identity → INFRA_ERROR（不得静默 PASS/FAIL）
  D7  run_slice 端到端        → 按 (package, target) 聚合，逐条给出结论
  D8  未知状态词              → DriverError（不得静默降级成 PASS）
  D9  命令行入口              → 与 run_slice 同结论（薄包装）

所有用例都在临时目录里做，不碰工作仓库。

用法：python3 docs/releases/v4.1.0/refactoring/tools/test_matrix_driver.py
退出码：0 = 全部通过；1 = 有断言失败（fail-closed）。
"""
from __future__ import annotations

import contextlib
import io
import json
import pathlib
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import matrix_driver as md  # noqa: E402

FAILURES: list[str] = []

MAPPING = dict(md.DEFAULT_STATUS_MAPPING)

RED_IDENTITY = {
    "test_fn": "tf_t4_01_scan_after_insert_sees_rows",
    "package_or_target": "sqlrustgo-storage / tf_t4_stub_silent_success",
    "source_sha256": "35e03d78234e2f680ec1407389c4695a904250a809cc396a2dcebc6241464e0a",
    "rc": 101,
    "attribution": "STUB_SILENT_SUCCESS: insert Ok but scan returned 0 rows",
}


def check(name: str, cond: bool, detail: str = "") -> None:
    if cond:
        print(f"  [OK]   {name}")
    else:
        print(f"  [FAIL] {name} {detail}")
        FAILURES.append(name)


def case(case_id: str, test_name: str, *, identity=None) -> dict:
    return {
        "case_id": case_id,
        "feature": "T4",
        "invariant_ids": ["INV-14"],
        "invariant_mapping_status": "MAPPED",
        "source": {"kind": "EXISTING_PILOT_UNCOMMITTED_OR_LISTED_TEST",
                   "test_name": test_name,
                   "declared_source": "cargo test --list @ e9ff21fc",
                   "claimed_status": "KNOWN_FAIL",
                   "declared_invariant": "后端能力契约"},
        "execution_status": "NOT_RUN",
        "actual": None,
        "artifact_sha256": None,
        "required_for_certification": False,
        **({"red_identity": identity} if identity else {}),
    }


def factory_result(class_: str, tests) -> dict:
    return {"class": class_, "tests": list(tests), "summary": {},
            "exit_code": 0, "note": ""}


# --------------------------------------------------------------------------
# D1 / D2 —— FAILED_RED 必须落成 KNOWN_FAIL，且带完整 RED 五元组
# --------------------------------------------------------------------------

def d1_d2_failed_red() -> None:
    print("A. pilot 词汇 FAILED_RED")
    subject = case("PILOT-T4-101", "tf_t4_01_scan_after_insert_sees_rows",
                   identity=RED_IDENTITY)
    pilot = {"rerun_status": "FAILED_RED", "evidence": "evidence/D001.log",
             "note": "D-001 桩静默成功 RED (CARGO_EXIT=101)"}
    result = factory_result("FAIL", [{"name": "tf_t4_01_scan_after_insert_sees_rows",
                                       "class": "FAIL", "raw_status": "FAILED"}])
    verdict = md.evaluate_case(subject, 0, pilot, MAPPING, result,
                               {"command": "cargo test", "sha256": "ab" * 32})

    check("D1 ⇒ execution_status == KNOWN_FAIL",
          verdict["execution_status"] == "KNOWN_FAIL", verdict["execution_status"])
    check("D1 ⇒ 不是 PASS", verdict["execution_status"] != "PASS")
    check("D1 ⇒ 不是普通 FAIL（RED 身份不得被抹掉）",
          verdict["execution_status"] != "FAIL", verdict["execution_status"])

    identity = verdict["red_identity"]
    check("D2 ⇒ red_identity 是对象", isinstance(identity, dict), repr(identity))
    missing = [f for f in md.RED_IDENTITY_FIELDS
               if not isinstance(identity, dict) or not identity.get(f)]
    check("D2 ⇒ 五元组齐全且非空", not missing, f"missing={missing}")
    check("D2 ⇒ source_sha256 是 64 位 sha256",
          isinstance(identity, dict) and len(str(identity.get("source_sha256"))) == 64)


# --------------------------------------------------------------------------
# D3 —— COVERED_PASS 必须落成 PASS，并带覆盖来源
# --------------------------------------------------------------------------

def d3_covered_pass() -> None:
    print("B. pilot 词汇 COVERED_PASS")
    subject = case("PILOT-T4-001", "append_only_storage::tests::transaction_lifecycle")
    pilot = {"rerun_status": "COVERED_PASS",
             "evidence": "evidence/RERUN-T2T3-20261120T172252.json",
             "note": "covered by the T2/T3 rerun"}
    result = factory_result("PASS", [{"name": "append_only_storage::tests::transaction_lifecycle",
                                       "class": "PASS", "raw_status": "ok"}])
    verdict = md.evaluate_case(subject, 1, pilot, MAPPING, result,
                               {"command": "cargo test", "sha256": "cd" * 32})

    check("D3 ⇒ execution_status == PASS",
          verdict["execution_status"] == "PASS", verdict["execution_status"])
    check("D3 ⇒ 带覆盖来源标注",
          bool(verdict["coverage_source"])
          and "RERUN-T2T3" in verdict["coverage_source"],
          verdict["coverage_source"])


# --------------------------------------------------------------------------
# D4 —— 零测试必须落成 NOT_RUN，绝不 PASS
# --------------------------------------------------------------------------

def d4_zero_tests() -> None:
    print("C. 工厂零测试")
    subject = case("PILOT-T1-001", "append_only_storage::tests::transaction_lifecycle")
    pilot = {"rerun_status": "PASS", "evidence": "evidence/RERUN-T1.json"}
    result = factory_result("NOT_RUN", [])
    verdict = md.evaluate_case(subject, 2, pilot, MAPPING, result,
                               {"command": "cargo test", "sha256": None})

    check("D4 ⇒ execution_status == NOT_RUN",
          verdict["execution_status"] == "NOT_RUN", verdict["execution_status"])
    check("D4 ⇒ execution_status != PASS",
          verdict["execution_status"] != "PASS")
    check("D4 ⇒ actual 写明没有测试行", "no test line" in verdict["actual"],
          verdict["actual"])
    check("D4 ⇒ 没有 artifact 摘要", verdict["artifact_sha256"] is None)

    # 工厂跑了、但这个 case 的测试不在其中：同样不得算 PASS。
    result = factory_result("PASS", [{"name": "other::test", "class": "PASS",
                                       "raw_status": "ok"}])
    verdict = md.evaluate_case(subject, 2, pilot, MAPPING, result,
                               {"command": "cargo test", "sha256": "ef" * 32})
    check("D4 ⇒ 测试不在结果里 ⇒ NOT_RUN",
          verdict["execution_status"] == "NOT_RUN", verdict["execution_status"])
    check("D4 ⇒ 测试不在结果里 ⇒ 不得 PASS",
          verdict["execution_status"] != "PASS")

    # 工厂跑了、但没有给出任何结论：是 INFRA_ERROR，不是 NOT_RUN。
    verdict = md.evaluate_case(subject, 2, pilot, MAPPING, None,
                               {"command": "cargo test", "sha256": None,
                                "stderr": "cargo: command not found"})
    check("D4 ⇒ 工厂无结论 ⇒ INFRA_ERROR",
          verdict["execution_status"] == "INFRA_ERROR", verdict["execution_status"])
    check("D4 ⇒ 工厂无结论 ⇒ 不得 PASS",
          verdict["execution_status"] != "PASS")


# --------------------------------------------------------------------------
# D5 —— bespoke 探针必须 NOT_RUN
# --------------------------------------------------------------------------

def d5_bespoke() -> None:
    print("D. bespoke 探针场景")
    subject = case("PILOT-T6-001B", "(crash-recovery-kill-9)")
    pilot = {"rerun_status": "COVERED_PASS", "evidence": "evidence/d003-red-run-*.log"}
    verdict = md.evaluate_case(subject, 72, pilot, MAPPING, None, None,
                               note="(crash-recovery-kill-9) is a bespoke probe, "
                                    "not a cargo test; the probe was not migrated "
                                    "into the baseline, so the case was not executed")
    check("D5 ⇒ execution_status == NOT_RUN",
          verdict["execution_status"] == "NOT_RUN", verdict["execution_status"])
    check("D5 ⇒ execution_status != PASS", verdict["execution_status"] != "PASS")
    check("D5 ⇒ actual 写明原因", "bespoke probe" in verdict["actual"],
          verdict["actual"])


# --------------------------------------------------------------------------
# D6 —— KNOWN_FAIL 缺 RED 身份必须 fail closed
# --------------------------------------------------------------------------

def d6_missing_identity() -> None:
    print("E. KNOWN_FAIL 缺 RED 身份")
    subject = case("PILOT-T4-101", "tf_t4_01_scan_after_insert_sees_rows")
    pilot = {"rerun_status": "FAILED_RED", "evidence": "evidence/D001.log"}
    result = factory_result("FAIL", [{"name": "tf_t4_01_scan_after_insert_sees_rows",
                                       "class": "FAIL", "raw_status": "FAILED"}])
    verdict = md.evaluate_case(subject, 0, pilot, MAPPING, result,
                               {"command": "cargo test", "sha256": None})
    check("D6 ⇒ execution_status == INFRA_ERROR",
          verdict["execution_status"] == "INFRA_ERROR", verdict["execution_status"])
    check("D6 ⇒ 不得 PASS", verdict["execution_status"] != "PASS")
    check("D6 ⇒ actual 写明缺 RED 身份",
          "RED identity" in verdict["actual"], verdict["actual"])


# --------------------------------------------------------------------------
# D7 / D9 —— 端到端：一个假工作树 + 一个假工厂
# --------------------------------------------------------------------------

FAKE_CASES = [
    ("CASE-UNIT", "fake::tests::fake_unit_case", "PASS"),
    ("CASE-TARGET", "fake_target_case", "PASS"),
    ("CASE-BESPOKE", "(bespoke-probe)", "COVERED_PASS"),
    ("CASE-RED", "fake_red_case", "FAILED_RED"),
]


def build_fake_repo(root: pathlib.Path) -> pathlib.Path:
    repo = root / "repo"
    (repo / "crates/fake/src").mkdir(parents=True)
    (repo / "crates/fake/tests").mkdir(parents=True)
    (repo / "tests").mkdir(parents=True)
    (repo / "evidence").mkdir(parents=True)
    (repo / "Cargo.toml").write_text(
        '[package]\nname = "fake-root"\nversion = "0.1.0"\n', encoding="utf-8")
    (repo / "crates/fake/Cargo.toml").write_text(
        '[package]\nname = "fake-lib"\nversion = "0.1.0"\n', encoding="utf-8")
    (repo / "crates/fake/src/lib.rs").write_text(
        "pub mod fake {\n    pub fn x() {}\n    #[cfg(test)]\n    mod tests {\n"
        "        #[test]\n        fn fake_unit_case() {}\n    }\n}\n", encoding="utf-8")
    (repo / "crates/fake/tests/fake_target.rs").write_text(
        "#[test]\nfn fake_target_case() {}\n", encoding="utf-8")
    (repo / "crates/fake/tests/fake_red.rs").write_text(
        "#[test]\nfn fake_red_case() {}\n", encoding="utf-8")
    (repo / "tests/fake_root.rs").write_text(
        "#[test]\nfn fake_root_case() {}\n", encoding="utf-8")
    records = []
    for index, (case_id, test_name, status) in enumerate(FAKE_CASES):
        records.append({"case_id": case_id, "matrix_index": index,
                        "test_name": test_name, "rerun_status": status,
                        "evidence": f"evidence/fake-{index}.log"})
    (repo / "evidence/case-mapping-20261120.json").write_text(
        json.dumps(records, ensure_ascii=False, indent=1), encoding="utf-8")
    return repo


def build_fake_matrix(path: pathlib.Path) -> None:
    matrix = {
        "version": "C-TF-0.1-PLAN",
        "status_vocabulary_mapping": dict(md.DEFAULT_STATUS_MAPPING),
        "cases": [],
    }
    for case_id, test_name, _status in FAKE_CASES:
        subject = case(case_id, test_name)
        if case_id == "CASE-RED":
            subject["red_identity"] = RED_IDENTITY
        matrix["cases"].append(subject)
    path.write_text(json.dumps(matrix, ensure_ascii=False, indent=2), encoding="utf-8")


def d7_d9_end_to_end() -> None:
    print("F. run_slice 端到端（假工作树 + 假工厂）")
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        root = pathlib.Path(tmp)
        repo = build_fake_repo(root)
        matrix_path = root / "matrix.json"
        build_fake_matrix(matrix_path)
        out_dir = root / "evidence"

        calls = []

        def fake_run_factory(_repo, package, target, filters, suite, _out, timeout=1800):
            calls.append({"package": package, "target": target,
                          "filters": list(filters), "suite": suite})
            tests = []
            for name in filters:
                if "red" in name:
                    tests.append({"name": name, "class": "FAIL",
                                  "raw_status": "FAILED"})
                else:
                    tests.append({"name": name, "class": "PASS",
                                  "raw_status": "ok"})
            return factory_result("FAIL" if tests and any(
                t["class"] == "FAIL" for t in tests) else "PASS", tests), \
                {"command": "cargo test", "sha256": "12" * 32, "path": None}

        original = md.run_factory
        md.run_factory = fake_run_factory
        try:
            results = md.run_slice(repo, matrix_path,
                                   ["0", "1", "2", "3"], out_dir)
            slice_calls = list(calls)
            calls.clear()
            buffer = io.StringIO()
            with contextlib.redirect_stdout(buffer):
                code = md.main(["--matrix", str(matrix_path), "--slice", "0,1,2,3",
                                "--out", str(out_dir), "--repo", str(repo)])
            cli_stdout = buffer.getvalue()
            cli_calls = list(calls)
        finally:
            md.run_factory = original

        by_id = {entry["case_id"]: entry for entry in results}
        check("D7 ⇒ 四条结论", len(results) == 4, str(len(results)))
        check("D7 ⇒ 单位测试 ⇒ PASS",
              by_id["CASE-UNIT"]["execution_status"] == "PASS",
              by_id["CASE-UNIT"]["execution_status"])
        check("D7 ⇒ 集成测试目标被解析",
              by_id["CASE-TARGET"]["execution_status"] == "PASS",
              by_id["CASE-TARGET"]["execution_status"])
        check("D7 ⇒ bespoke ⇒ NOT_RUN",
              by_id["CASE-BESPOKE"]["execution_status"] == "NOT_RUN",
              by_id["CASE-BESPOKE"]["execution_status"])
        check("D7 ⇒ FAILED_RED ⇒ KNOWN_FAIL",
              by_id["CASE-RED"]["execution_status"] == "KNOWN_FAIL",
              by_id["CASE-RED"]["execution_status"])
        check("D7 ⇒ KNOWN_FAIL 带完整 RED 五元组",
              md.complete_identity(by_id["CASE-RED"]["red_identity"]))
        check("D7 ⇒ 结论按 slice 顺序返回",
              [entry["case_id"] for entry in results]
              == [c[0] for c in FAKE_CASES],
              str([entry["case_id"] for entry in results]))

        packages = sorted({call["package"] for call in slice_calls})
        check("D7 ⇒ 按 (package, target) 聚合调用",
              packages == ["fake-lib"], str(slice_calls))
        groups = {(call["package"], call["target"]) for call in slice_calls}
        check("D7 ⇒ 每个 (package, target) 只调用一次",
              len(slice_calls) == len(groups) == 3,
              f"calls={len(slice_calls)} groups={sorted(map(str, groups))}")
        check("D7 ⇒ bespoke 场景不触发工厂调用",
              all("bespoke" not in call["filters"][0] for call in slice_calls),
              str(slice_calls))

        check("D9 ⇒ 命令行入口退出码非 0（fail closed：slice 里有 RED/NOT_RUN）",
              code != 0, str(code))
        check("D9 ⇒ 命令行入口与 run_slice 调用同一组目标",
              {(c["package"], c["target"]) for c in cli_calls} == groups,
              str(cli_calls))
        check("D9 ⇒ 命令行入口打印每条结论",
              all(entry["case_id"] in cli_stdout
                  for entry in results), cli_stdout[-400:])
        check("D9 ⇒ 命令行入口打印统计",
              "4 case(s)" in cli_stdout, cli_stdout[-400:])

        # 一个全绿的 slice 必须退出 0。
        calls.clear()
        buffer = io.StringIO()
        md.run_factory = fake_run_factory
        try:
            with contextlib.redirect_stdout(buffer):
                green_code = md.main(["--matrix", str(matrix_path),
                                      "--slice", "0,1", "--out", str(out_dir),
                                      "--repo", str(repo)])
        finally:
            md.run_factory = original
        check("D9 ⇒ 全绿 slice 退出码 0", green_code == 0, str(green_code))


# --------------------------------------------------------------------------
# D8 —— 未知状态词不得静默降级
# --------------------------------------------------------------------------

def d8_unknown_status() -> None:
    print("G. 未知状态词")
    try:
        md.map_status("SOMETHING_ELSE", MAPPING)
    except md.DriverError:
        check("D8 ⇒ 未知状态词抛 DriverError", True)
    else:
        check("D8 ⇒ 未知状态词抛 DriverError", False, "no exception raised")
    check("D8 ⇒ 契约词自映射",
          md.map_status("KNOWN_FAIL", MAPPING) == "KNOWN_FAIL")
    check("D8 ⇒ RED 词落成 KNOWN_FAIL",
          md.map_status("FAILED_RED", MAPPING) == "KNOWN_FAIL"
          and md.map_status("COVERED_RED", MAPPING) == "KNOWN_FAIL")


def main() -> int:
    d1_d2_failed_red()
    d3_covered_pass()
    d4_zero_tests()
    d5_bespoke()
    d6_missing_identity()
    d7_d9_end_to_end()
    d8_unknown_status()
    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: matrix_driver 夹具全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())

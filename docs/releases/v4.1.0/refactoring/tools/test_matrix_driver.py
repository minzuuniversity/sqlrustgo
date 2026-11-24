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

评审第一轮补的用例（每一条都对应一条「评审指出的洞」）：

  H   选中的 index 没有 pilot 记录 → DriverError（不得把 RED 判成 FAIL）
  I   矩阵自己登记 KNOWN_FAIL、pilot 记录说别的 → 矩阵的信号算数，不是 FAIL
  J   矩阵的 status_vocabulary_mapping 被改坏（FAILED_RED→FAIL）→ DriverError
  K   矩阵的映射块与适配层自带副本不一致 → DriverError（不得漂移）
  L   case 引用 ADR-010 → M4 判红（ADR 引用不是不变量）
  M   bespoke 探针已迁入仓内 → 适配层去跑它，而不是印一句「未迁入」
  N   PILOT-T5-101 的身份：四个元素在、source_sha256 显式为 null、缺口标记在
  O   适配层输出的固定形状：五个键一个不少
  P   工厂把测试判成 KNOWN_FAIL 但 case 没有完整身份 → INFRA_ERROR

Task 10（逐用例重做 invariant 映射）补的用例 —— **M7 必须咬住分桶映射**：

  Q1   某条 case 少了 invariant_mapping_basis        → M7 判红
  Q2   两条 case 共用同一个标签 + 同一组 id + 同一句依据 → M7 判红（分桶签名）
  Q2b  抄一遍再改一个字                             → M7 判红（不能靠改写躲过）
  Q3   某条 case 声明空映射却没登记进缺口提案索引表   → M7 判红
  Q4   某条 case 空映射但没声明 NO_INVARIANT_MATCH   → M1 判红
  Q5   基线矩阵本身过 M7
  Q6   同一个模板给很多条 case 生成依据              → M7 判红（分桶签名的典型形状）
  Q7   一条 id 被三分之一的矩阵以上主张               → M7 判红（分桶映射的规模形状）
  Q8   主张了某条 id，依据里却没有点名它             → M7 判红（共享 id 必须逐案说明）
  Q9   用了词表外的 invariant_mapping_status          → M7 判红
  Q9b  登记表登记的词表与工具的常量一致
  Q10  缺口提案索引表多列了一条不是空映射的 case      → M7 判红（反向一致性）

矩阵门禁的契约已由 6/6 变为 7/7（M1–M6 的**语义目标**不变；M1 的**判定形式**接受「显式声明的空映射」，
M7 是新增的逐用例映射断言）。**M7 只验形式**：一条映射与测试源码是否**实质**相符，
是人（Oracle / 副签人）的事，`verify_matrix.py` 的 docstring 里写明了这一点。

所有用例都在临时目录里做，不碰工作仓库。

用法：python3 docs/releases/v4.1.0/refactoring/tools/test_matrix_driver.py
退出码：0 = 全部通过；1 = 有断言失败（fail-closed）。
"""
from __future__ import annotations

import contextlib
import copy
import io
import json
import pathlib
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))

import matrix_driver as md  # noqa: E402
import verify_matrix as vm  # noqa: E402

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


def case(case_id: str, test_name: str, *, identity=None,
         claimed_status: str = "PASS") -> dict:
    """A minimal matrix case.

    The mapping fields are per case, not per bucket: the fixture's own matrix
    has to survive M7, so the label and the basis carry the case id.  The
    basis is generated from one template on purpose -- that is the shape M7 is
    built to flag in a real matrix (see Q6), so the fixture must not pretend
    the shape is acceptable.
    """
    return {
        "case_id": case_id,
        "feature": "T4",
        "invariant_ids": ["INV-14"],
        "invariant_mapping_status": "MAPPED",
        "invariant_mapping_basis": (
            f"夹具用例 {case_id}（{test_name}）：断言该测试直接检查 "
            f"INV-14 所述的能力不足写前拒绝，故贴 INV-14"),
        "source": {"kind": "EXISTING_PILOT_UNCOMMITTED_OR_LISTED_TEST",
                   "test_name": test_name,
                   "declared_source": "cargo test --list @ e9ff21fc",
                   "claimed_status": claimed_status,
                   "declared_invariant": f"后端能力契约（夹具用例 {case_id}）"},
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
                   identity=RED_IDENTITY, claimed_status="KNOWN_FAIL")
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
    subject = case("PILOT-T4-101", "tf_t4_01_scan_after_insert_sees_rows",
                   claimed_status="KNOWN_FAIL")
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


def build_fake_repo(root: pathlib.Path, *, drop_indices=()) -> pathlib.Path:
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
        if index in drop_indices:
            continue
        records.append({"case_id": case_id, "matrix_index": index,
                        "test_name": test_name, "rerun_status": status,
                        "evidence": f"evidence/fake-{index}.log"})
    (repo / "evidence/case-mapping-20261120.json").write_text(
        json.dumps(records, ensure_ascii=False, indent=1), encoding="utf-8")
    return repo


def build_fake_matrix(path: pathlib.Path, *, with_probe_registry: bool = False) -> None:
    matrix = {
        "version": "C-TF-0.1-PLAN",
        "status_vocabulary_mapping": dict(md.DEFAULT_STATUS_MAPPING),
        "cases": [],
    }
    if with_probe_registry:
        matrix["probe_registry"] = {
            "probes": {"bespoke-probe": {"package": "fake-lib",
                                         "target": "fake_target",
                                         "filter": "fake_target_case"}}}
    for case_id, test_name, _status in FAKE_CASES:
        subject = case(case_id, test_name,
                        claimed_status="KNOWN_FAIL" if case_id == "CASE-RED"
                        else "PASS")
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

        # 输出的固定形状：Task 6 依赖这五个键。
        for key in md.VERDICT_KEYS:
            check(f"O ⇒ 每条结论都带 {key}",
                  all(key in entry for entry in results),
                  str(sorted(set(md.VERDICT_KEYS) - set(results[0]))))
        check("O ⇒ case_id 与矩阵一致",
              all(entry["case_id"] in {c[0] for c in FAKE_CASES}
                  for entry in results))
        check("O ⇒ execution_status 是契约词汇",
              all(entry["execution_status"] in md.CONTRACT_VOCABULARY
                  for entry in results),
              str([e["execution_status"] for e in results]))
        check("O ⇒ actual 非空",
              all(isinstance(entry["actual"], str) and entry["actual"].strip()
                  for entry in results))
        check("O ⇒ artifact_sha256 是 64 位或 null",
              all(entry["artifact_sha256"] is None
                  or len(entry["artifact_sha256"]) == 64
                  for entry in results))

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


# --------------------------------------------------------------------------
# H —— 选中的 index 没有 pilot 记录 ⇒ 必须 fail closed
# --------------------------------------------------------------------------

def h_missing_pilot_record() -> None:
    print("H. 选中的 index 没有 pilot 记录")
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        root = pathlib.Path(tmp)
        # index 3（CASE-RED）没有 pilot 记录。
        repo = build_fake_repo(root, drop_indices=(3,))
        matrix_path = root / "matrix.json"
        build_fake_matrix(matrix_path)
        try:
            md.run_slice(repo, matrix_path, ["0", "1", "2", "3"], root / "out")
        except md.DriverError as exc:
            check("H ⇒ 抛 DriverError", True)
            check("H ⇒ 理由点名缺失的记录", "no record" in str(exc), str(exc))
            check("H ⇒ 理由说明不得把 RED 变成 FAIL",
                  "FAIL" in str(exc), str(exc))
        else:
            check("H ⇒ 抛 DriverError", False, "no exception raised")

    # 反向：记录齐的时候同一条 slice 正常返回，且 RED 仍然是 KNOWN_FAIL。
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        root = pathlib.Path(tmp)
        repo = build_fake_repo(root)
        matrix_path = root / "matrix.json"
        build_fake_matrix(matrix_path)

        def fake_run_factory(_repo, _package, _target, filters, _suite, _out,
                             timeout=1800):
            tests = [{"name": name, "class": "FAIL", "raw_status": "FAILED"}
                     for name in filters]
            return factory_result("FAIL", tests), \
                {"command": "cargo test", "sha256": None, "path": None}

        original = md.run_factory
        md.run_factory = fake_run_factory
        try:
            results = md.run_slice(repo, matrix_path, ["3"], root / "out")
        finally:
            md.run_factory = original
        check("H ⇒ 记录齐时 RED 仍是 KNOWN_FAIL",
              results[0]["execution_status"] == "KNOWN_FAIL",
              results[0]["execution_status"])


# --------------------------------------------------------------------------
# I —— 矩阵自己登记的 KNOWN_FAIL 不依赖 pilot 记录
# --------------------------------------------------------------------------

def i_matrix_signal_wins() -> None:
    print("I. 矩阵的 RED 登记与 pilot 记录不一致")
    subject = case("PILOT-T4-101", "tf_t4_01_scan_after_insert_sees_rows",
                   identity=RED_IDENTITY, claimed_status="KNOWN_FAIL")
    # pilot 记录说 PASS，矩阵说 KNOWN_FAIL：矩阵的信号必须算数。
    pilot = {"rerun_status": "PASS", "evidence": "evidence/stale.log"}
    result = factory_result("FAIL", [{"name": "tf_t4_01_scan_after_insert_sees_rows",
                                       "class": "FAIL", "raw_status": "FAILED"}])
    verdict = md.evaluate_case(subject, 0, pilot, MAPPING, result,
                               {"command": "cargo test", "sha256": None})
    check("I ⇒ 不是普通 FAIL", verdict["execution_status"] != "FAIL",
          verdict["execution_status"])
    check("I ⇒ 落成 KNOWN_FAIL",
          verdict["execution_status"] == "KNOWN_FAIL",
          verdict["execution_status"])
    check("I ⇒ 带完整 RED 五元组",
          md.complete_identity(verdict["red_identity"]))


# --------------------------------------------------------------------------
# J / K —— 映射表被改坏 / 与适配层副本漂移
# --------------------------------------------------------------------------

def j_k_mapping_guards() -> None:
    print("J. 映射表被改坏")
    laundered = dict(MAPPING)
    laundered["FAILED_RED"] = "FAIL"
    try:
        md.status_mapping({"status_vocabulary_mapping": laundered})
    except md.DriverError as exc:
        check("J ⇒ 抛 DriverError", True)
        check("J ⇒ 理由点名 KNOWN_FAIL", "KNOWN_FAIL" in str(exc), str(exc))
    else:
        check("J ⇒ 抛 DriverError", False, "no exception raised")

    print("K. 映射表与适配层副本漂移")
    drifted = dict(MAPPING)
    drifted["KNOWN_FAIL"] = "KNOWN_FAIL"  # 值合法，但多出一个键
    try:
        md.status_mapping({"status_vocabulary_mapping": drifted})
    except md.DriverError as exc:
        check("K ⇒ 抛 DriverError", True)
        check("K ⇒ 理由点名漂移", "diverge" in str(exc), str(exc))
    else:
        check("K ⇒ 抛 DriverError", False, "no exception raised")

    # 自带副本与基线矩阵的块必须一致 —— 这一条直接读仓内矩阵。
    baseline = md.load_matrix(md.ROOT / md.MATRIX_REL)
    try:
        md.status_mapping(baseline)
        check("K ⇒ 基线矩阵的映射块与适配层副本一致", True)
    except md.DriverError as exc:
        check("K ⇒ 基线矩阵的映射块与适配层副本一致", False, str(exc))


# --------------------------------------------------------------------------
# L —— ADR 引用不是不变量
# --------------------------------------------------------------------------

def l_adr_is_not_an_invariant() -> None:
    print("L. ADR 引用不能冒充不变量")
    corpus = vm.corpus()
    check("L ⇒ 语料里没有 ADR-010", "ADR-010" not in corpus,
          str(sorted(corpus)[:5]))
    check("L ⇒ 语料里没有 ADR-410-006", "ADR-410-006" not in corpus)
    check("L ⇒ 语料就是那 18 条不变量",
          sorted(corpus) == [f"INV-{n:02d}" for n in range(1, 19)],
          str(sorted(corpus)))

    # 语料必须是**结构上**取自登记表的 id 列，而不是「正碰巧被一个宽正则扫到」。
    # 把一句散文里的 ADR 引用塞进存储契约的副本：它不得进语料。
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        copy_path = pathlib.Path(tmp) / "STORAGE_CAPABILITY_CONTRACT.md"
        original = (md.ROOT / vm.STORAGE_REL).read_text(encoding="utf-8")
        copy_path.write_text(original + "\n格式细节见 ADR-010 的决定。\n",
                             encoding="utf-8")
        ids = vm.declared_ids(copy_path)
        check("L ⇒ 散文里的 ADR 引用不进语料", "ADR-010" not in ids,
              str(sorted(ids)))
        check("L ⇒ 散文副本不产生任何 id（存储契约没有 id 列）",
              ids == set(), str(sorted(ids)))

    cases = []
    for index in range(vm.EXPECTED_CASES):
        ids = ["ADR-010"] if index == 7 else ["INV-01"]
        cases.append({"case_id": f"C{index:03d}", "invariant_ids": ids})
    matrix = {"cases": cases}
    try:
        vm.m4(matrix)
    except vm.MatrixError as exc:
        check("L ⇒ M4 判红", True)
        check("L ⇒ 理由点名 ADR-010", "ADR-010" in str(exc), str(exc))
    else:
        check("L ⇒ M4 判红", False, "no exception raised")


# --------------------------------------------------------------------------
# M —— bespoke 探针已迁入 ⇒ 适配层必须去跑它
# --------------------------------------------------------------------------

def m_migrated_probe_runs() -> None:
    print("M. bespoke 探针已迁入仓内")
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        root = pathlib.Path(tmp)
        repo = build_fake_repo(root)
        matrix_path = root / "matrix.json"
        build_fake_matrix(matrix_path, with_probe_registry=True)
        calls = []

        def fake_run_factory(_repo, package, target, filters, suite, _out, timeout=1800):
            calls.append({"package": package, "target": target,
                          "filters": list(filters)})
            tests = [{"name": name, "class": "PASS", "raw_status": "ok"}
                     for name in filters]
            return factory_result("PASS", tests), \
                {"command": "cargo test", "sha256": "34" * 32, "path": None}

        original = md.run_factory
        md.run_factory = fake_run_factory
        try:
            results = md.run_slice(repo, matrix_path, ["2"], root / "out")
        finally:
            md.run_factory = original

        verdict = results[0]
        check("M ⇒ 探针被执行了（工厂被调用）", bool(calls), str(calls))
        check("M ⇒ 探针的过滤器是注册表里的那个",
              calls and calls[0]["filters"] == ["fake_target_case"], str(calls))
        check("M ⇒ 结论不是「未迁入」的 NOT_RUN",
              "not migrated" not in verdict["actual"], verdict["actual"])
        check("M ⇒ 结论是 PASS",
              verdict["execution_status"] == "PASS", verdict["execution_status"])

    # 反向：没有注册表 ⇒ NOT_RUN，且理由说的是「没有迁入的探针」。
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        root = pathlib.Path(tmp)
        repo = build_fake_repo(root)
        matrix_path = root / "matrix.json"
        build_fake_matrix(matrix_path)
        results = md.run_slice(repo, matrix_path, ["2"], root / "out")
        verdict = results[0]
        check("M ⇒ 未迁入 ⇒ NOT_RUN",
              verdict["execution_status"] == "NOT_RUN", verdict["execution_status"])
        check("M ⇒ 未迁入的理由点名注册表",
              "probe_registry" in verdict["actual"], verdict["actual"])


# --------------------------------------------------------------------------
# N —— PILOT-T5-101 的身份：四个元素在、sha256 显式为 null、缺口标记在
# --------------------------------------------------------------------------

def n_odku_identity() -> None:
    print("N. PILOT-T5-101 携带的身份")
    matrix = md.load_matrix(md.ROOT / md.MATRIX_REL)
    subject = next((c for c in matrix["cases"]
                    if c["case_id"] == "PILOT-T5-101"), None)
    check("N ⇒ 矩阵里有这条", subject is not None)
    if subject is None:
        return
    identity = subject.get("red_identity")
    check("N ⇒ 带了 red_identity", isinstance(identity, dict), repr(identity))
    if not isinstance(identity, dict):
        return
    for field in ("test_fn", "package_or_target", "rc", "attribution"):
        check(f"N ⇒ {field} 在且非空",
              bool(identity.get(field)) and str(identity.get(field)).strip(),
              repr(identity.get(field)))
    check("N ⇒ source_sha256 显式为 null",
          "source_sha256" in identity and identity["source_sha256"] is None,
          repr(identity.get("source_sha256")))
    check("N ⇒ 缺口标记在且非空",
          bool(identity.get(md.GAP_FIELD)), repr(identity.get(md.GAP_FIELD)))
    state, problems = vm.identity_state(identity)
    check("N ⇒ M6 认得这是一个声明过的缺口", state == "gap", f"{state} {problems}")
    check("N ⇒ 缺口让身份不算完整（适配层因此只报 NOT_RUN）",
          not md.complete_identity(identity))
    check("N ⇒ 缺口里点名了不可复原的原因",
          "unrecoverable" in str(identity.get(md.GAP_FIELD)).lower()
          or "no digest" in str(identity.get(md.GAP_FIELD)).lower(),
          str(identity.get(md.GAP_FIELD))[:200])


# --------------------------------------------------------------------------
# P —— 工厂判成 KNOWN_FAIL 但没有完整身份 ⇒ 必须 fail closed
# --------------------------------------------------------------------------

def p_factory_known_fail_without_identity() -> None:
    print("P. 工厂判成 KNOWN_FAIL 但身份不全")
    subject = case("PILOT-T9-001", "some::test", claimed_status="PASS")
    pilot = {"rerun_status": "PASS", "evidence": "evidence/x.log"}
    result = factory_result("FAIL", [{"name": "some::test", "class": "KNOWN_FAIL",
                                       "raw_status": "FAILED"}])
    verdict = md.evaluate_case(subject, 0, pilot, MAPPING, result,
                               {"command": "cargo test", "sha256": None})
    check("P ⇒ 落成 INFRA_ERROR",
          verdict["execution_status"] == "INFRA_ERROR", verdict["execution_status"])
    check("P ⇒ 不得 PASS", verdict["execution_status"] != "PASS")

    # 身份齐的时候，工厂判的 KNOWN_FAIL 才是 KNOWN_FAIL。
    subject = case("PILOT-T9-001", "some::test", identity=RED_IDENTITY,
                   claimed_status="PASS")
    verdict = md.evaluate_case(subject, 0, pilot, MAPPING, result,
                               {"command": "cargo test", "sha256": None})
    check("P ⇒ 身份齐 ⇒ KNOWN_FAIL",
          verdict["execution_status"] == "KNOWN_FAIL",
          verdict["execution_status"])


# --------------------------------------------------------------------------
# Q —— M7 必须咬住「分桶映射」，M1 必须咬住「未声明的空映射」
# --------------------------------------------------------------------------

def q_m7_bites_bucket_mapping() -> None:
    print("Q. M7 必须咬住分桶映射")
    baseline = md.load_matrix(md.ROOT / md.MATRIX_REL)

    # Q5: 基线矩阵本身过 M7。
    try:
        vm.m7(baseline)
        check("Q5 ⇒ 基线矩阵过 M7", True)
    except vm.MatrixError as exc:
        check("Q5 ⇒ 基线矩阵过 M7", False, str(exc))

    # Q1: 某条 case 少了逐用例依据。
    mutated = copy.deepcopy(baseline)
    del mutated["cases"][0]["invariant_mapping_basis"]
    try:
        vm.m7(mutated)
        check("Q1 ⇒ 缺依据 ⇒ M7 判红", False, "no exception raised")
    except vm.MatrixError as exc:
        check("Q1 ⇒ 缺依据 ⇒ M7 判红", True)
        check("Q1 ⇒ 理由点名依据字段", vm.BASIS_FIELD in str(exc), str(exc))

    # Q2: 分桶签名 —— 同一个标签 + 同一组 id + 同一句依据。
    mutated = copy.deepcopy(baseline)
    first, second = mutated["cases"][0], mutated["cases"][1]
    for subject in (first, second):
        subject["source"]["declared_invariant"] = "同一个标签"
        subject["invariant_ids"] = ["INV-01"]
        subject["invariant_mapping_basis"] = "同一句依据"
    try:
        vm.m7(mutated)
        check("Q2 ⇒ 分桶签名 ⇒ M7 判红", False, "no exception raised")
    except vm.MatrixError as exc:
        check("Q2 ⇒ 分桶签名 ⇒ M7 判红", True)
        check("Q2 ⇒ 理由点名近似依据", "near-identical basis" in str(exc), str(exc))

    # Q2b: 抄一遍再改一个字 —— 必须仍然判红（不能靠改写躲过）。
    mutated = copy.deepcopy(baseline)
    first, second = mutated["cases"][0], mutated["cases"][1]
    first["invariant_mapping_basis"] = "断言的是回滚之后行数回到事务前的那一条不变量的恢复语义"
    second["invariant_mapping_basis"] = "断言的是回滚之后行数回到事务前的那一条不变量的恢复语义。"
    try:
        vm.m7(mutated)
        check("Q2b ⇒ 改一个字也判红", False, "no exception raised")
    except vm.MatrixError as exc:
        check("Q2b ⇒ 改一个字也判红", True)

    # Q6: 同一个模板给很多条 case 生成依据 —— 分桶签名的典型形状。
    mutated = copy.deepcopy(baseline)
    for subject in mutated["cases"]:
        subject["invariant_mapping_basis"] = "断言该测试直接检查 INV-14 所述的能力不足写前拒绝"
    try:
        vm.m7(mutated)
        check("Q6 ⇒ 模板化依据 ⇒ M7 判红", False, "no exception raised")
    except vm.MatrixError as exc:
        check("Q6 ⇒ 模板化依据 ⇒ M7 判红", True)
        check("Q6 ⇒ 理由点名近似依据", "near-identical basis" in str(exc), str(exc))

    # Q7: 一条 id 被三分之一的矩阵以上主张 —— 分桶映射的规模形状。
    mutated = copy.deepcopy(baseline)
    for subject in mutated["cases"][:40]:
        subject["invariant_ids"] = ["INV-01"]
    try:
        vm.m7(mutated)
        check("Q7 ⇒ 一条 id 铺满矩阵 ⇒ M7 判红", False, "no exception raised")
    except vm.MatrixError as exc:
        check("Q7 ⇒ 一条 id 铺满矩阵 ⇒ M7 判红", True)
        check("Q7 ⇒ 理由点名覆盖上限", "third of the matrix" in str(exc), str(exc))

    # Q8: 主张了某条 id，依据里却没有点名它 —— 共享 id 必须逐案说明。
    mutated = copy.deepcopy(baseline)
    mapped = next(c for c in mutated["cases"] if c["invariant_ids"])
    mapped["invariant_mapping_basis"] = "断言与源码一致，故贴一条已登记的不变量。"
    try:
        vm.m7(mutated)
        check("Q8 ⇒ 依据没点名所主张的 id ⇒ M7 判红", False, "no exception raised")
    except vm.MatrixError as exc:
        check("Q8 ⇒ 依据没点名所主张的 id ⇒ M7 判红", True)
        check("Q8 ⇒ 理由点名逐案说明", "justified per case" in str(exc), str(exc))

    # Q9: 词表外的映射状态。
    mutated = copy.deepcopy(baseline)
    mutated["cases"][0]["invariant_mapping_status"] = "SOMETHING_ELSE"
    try:
        vm.m7(mutated)
        check("Q9 ⇒ 词表外的状态 ⇒ M7 判红", False, "no exception raised")
    except vm.MatrixError as exc:
        check("Q9 ⇒ 词表外的状态 ⇒ M7 判红", True)
        check("Q9 ⇒ 理由点名登记表词表", "registered vocabulary" in str(exc), str(exc))

    # Q9b: 登记表里登记的词表与工具的常量必须一致。
    registered = vm.mapping_status_vocabulary(md.ROOT / vm.TX_REL)
    check("Q9b ⇒ 登记表登记了词表", registered == set(vm.MAPPING_STATUSES),
          str(sorted(registered)))
    check("Q9b ⇒ 登记表词表含 NO_INVARIANT_MATCH",
          vm.NO_MATCH_STATUS in registered, str(sorted(registered)))

    # Q3: 声明了空映射，却没有登记进缺口提案的索引表。
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        gap = pathlib.Path(tmp) / "gap.md"
        gap.write_text("| 缺口 | 用例 |\n|---|---|\n| GAP-00 | PILOT-T1-001 |\n",
                       encoding="utf-8")
        original = vm.GAP_REL
        vm.GAP_REL = str(gap)
        try:
            mutated = copy.deepcopy(baseline)
            orphan = mutated["cases"][0]
            orphan["case_id"] = "PILOT-XX-999"
            orphan["invariant_ids"] = []
            orphan["invariant_mapping_status"] = vm.NO_MATCH_STATUS
            orphan["invariant_mapping_basis"] = "该断言不对应任何已登记不变量"
            try:
                vm.m7(mutated)
                check("Q3 ⇒ 空映射未登记 ⇒ M7 判红", False, "no exception raised")
            except vm.MatrixError as exc:
                check("Q3 ⇒ 空映射未登记 ⇒ M7 判红", True)
                check("Q3 ⇒ 理由点名缺口提案索引表", "index table" in str(exc), str(exc))
        finally:
            vm.GAP_REL = original

    # Q10: 反向 —— 索引表里列了一条矩阵里不是空映射的 case。
    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        empty_ids = [c["case_id"] for c in baseline["cases"]
                     if not c["invariant_ids"]]
        mapped_id = next(c["case_id"] for c in baseline["cases"]
                         if c["invariant_ids"])
        gap = pathlib.Path(tmp) / "gap.md"
        gap.write_text("| 缺口 | 用例 |\n|---|---|\n| GAP-00 | "
                       + "、".join(empty_ids + [mapped_id]) + " |\n",
                       encoding="utf-8")
        original = vm.GAP_REL
        vm.GAP_REL = str(gap)
        try:
            try:
                vm.m7(baseline)
                check("Q10 ⇒ 索引表多列了一条 ⇒ M7 判红", False, "no exception raised")
            except vm.MatrixError as exc:
                check("Q10 ⇒ 索引表多列了一条 ⇒ M7 判红", True)
                check("Q10 ⇒ 理由点名反向不一致",
                      "do not declare an empty mapping" in str(exc), str(exc))
        finally:
            vm.GAP_REL = original

    # Q4: 空映射没有显式声明（状态仍是 MAPPED）。
    mutated = copy.deepcopy(baseline)
    mapped = next(c for c in mutated["cases"] if c["invariant_ids"])
    mapped["invariant_ids"] = []
    try:
        vm.m1(mutated)
        check("Q4 ⇒ 未声明的空映射 ⇒ M1 判红", False, "no exception raised")
    except vm.MatrixError as exc:
        check("Q4 ⇒ 未声明的空映射 ⇒ M1 判红", True)
        check("Q4 ⇒ 理由点名 NO_INVARIANT_MATCH",
              vm.NO_MATCH_STATUS in str(exc), str(exc))


def main() -> int:
    d1_d2_failed_red()
    d3_covered_pass()
    d4_zero_tests()
    d5_bespoke()
    d6_missing_identity()
    d7_d9_end_to_end()
    d8_unknown_status()
    h_missing_pilot_record()
    i_matrix_signal_wins()
    j_k_mapping_guards()
    l_adr_is_not_an_invariant()
    m_migrated_probe_runs()
    n_odku_identity()
    p_factory_known_fail_without_identity()
    q_m7_bites_bucket_mapping()
    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: matrix_driver 夹具全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())

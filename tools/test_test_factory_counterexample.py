"""反例 #1 回归（contract-observations 2026-11-20）：
零测试 + exit 0 不得判 PASS；summary 与解析行不一致不得判 PASS。
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))
import test_factory as tf  # noqa: E402


class _Proc:
    def __init__(self, stdout: str, returncode: int = 0):
        self.stdout = stdout
        self.stderr = ""
        self.returncode = returncode


def _run_with(monkeypatch, raw: str, rc: int = 0):
    monkeypatch.setattr(tf.subprocess, "run",
                        lambda *a, **k: _Proc(raw, rc))
    return tf.run_suite(Path("."), "pkg", ["some_filter"], target_dir=None, timeout=10)


def test_zero_tests_exit0_is_not_run(monkeypatch):
    result = _run_with(monkeypatch, "running 0 tests\n\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n")
    assert result["class"] == "NOT_RUN", result
    assert result["tests"] == []


def test_zero_tests_exit0_no_summary_is_not_run(monkeypatch):
    result = _run_with(monkeypatch, "")
    assert result["class"] == "NOT_RUN", result


def test_summary_mismatch_is_infra_error(monkeypatch):
    # cargo 声称 3 passed，但解析只得到 2 行 → 不可信
    raw = ("test pkg::a ... ok\n"
           "test pkg::b ... ok\n"
           "test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n")
    result = _run_with(monkeypatch, raw)
    assert result["class"] == "INFRA_ERROR", result


def test_normal_pass_still_pass(monkeypatch):
    raw = ("test pkg::a ... ok\n"
           "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n")
    result = _run_with(monkeypatch, raw)
    assert result["class"] == "PASS", result


def test_real_failure_still_fail(monkeypatch):
    raw = ("test pkg::a ... FAILED\n"
           "test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out\n")
    result = _run_with(monkeypatch, raw)
    assert result["class"] == "FAIL", result


def test_known_fail_beats_pass(monkeypatch):
    raw = ("test pkg::a ... ok\n"
           "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n")
    result = _run_with(monkeypatch, raw)
    assert result["tests"][0]["class"] == "PASS"
    r2 = tf.run_suite(Path("."), "pkg", ["x"], target_dir=None, timeout=10) if False else \
        tf.classify("ok", {"pkg::a"}, "pkg::a")
    assert r2 == "KNOWN_FAIL"


def test_should_panic_lines_parsed(monkeypatch):
    # 4 条 should panic 测试此前被解析器漏掉（214 vs 218 差异的根因）
    raw = ("test vtug::a - should panic ... ok\n"
           "test vtug::b ... ok\n"
           "test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n")
    result = _run_with(monkeypatch, raw)
    assert result["class"] == "PASS", result
    assert len(result["tests"]) == 2
    assert result["tests"][0]["name"] == "vtug::a"


def test_raw_preserved_for_record(monkeypatch):
    result = _run_with(monkeypatch, "test x::a ... ok\n")
    assert "raw" in result and "x::a" in result["raw"]

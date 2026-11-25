#!/usr/bin/env python3
"""verify_sha_scope.py 的变异夹具 —— 证明分界校验器真的会咬。

WHY（2026-11-23）
------------------
「前向绑定统一、历史绑定未变」这条判据的全部价值在于：校验器能区分
「核过了」与「没核」。一个恒绿的校验器比没有校验器更坏 —— 它给下游任务
一个假信号。本夹具对每条断言各做一次变异，要求校验器必须变红。

第一轮评审抓到两个假绿的洞，都在这里钉死：

  * 仓内被跟踪的 `evidence/rerun-summary-20261120.json` 曾无人看守 ——
    S6 指向仓外那台机器上的副本，于是改坏仓内副本校验器照样绿。
  * 历史记录集合由工作树 glob 推出 —— 删掉一份历史 manifest，glob 变小，
    集合变小，校验器把「少了一份」当成「本来就是这么多」报绿。

所有变异都在临时克隆里做，不碰工作仓库。

用法：python3 docs/releases/v4.1.0/refactoring/tools/test_verify_sha_scope.py
退出码：0 = 全部通过；1 = 有断言失败（fail-closed）。

环境变量 VERIFY_SHA_SCOPE_UNDER_TEST
    指向另一个校验器文件：夹具会把它拷进临时克隆再跑。用来证明「修之前不咬、
    修之后才咬」—— 对着修前的版本跑，相应探针会失败。
"""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[4]
VERIFIER_REL = "docs/releases/v4.1.0/refactoring/tools/verify_sha_scope.py"

DAG_REL = "docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json"
MATRIX_REL = "docs/releases/v4.1.0/refactoring/TEST_COVERAGE_MATRIX.json"
TASK_MANIFEST_REL = "docs/refactor/v4.1.0-c/handoff/03_TASK_MANIFEST.jsonl"
HANDOFF_REL = "docs/releases/v4.1.0/refactoring/test-factory-design/14_HANDOFF_MANIFEST.json"
PILOT_REL = "evidence/rerun-summary-20261120.json"
HISTORICAL_MANIFEST_REL = "audit/v4.1.0/runs/20261105-lb1-006-mcode/manifest.yaml"
# A historical record that was ADDED after the freeze and is therefore listed
# in BASELINE_FREEZE.json -> historical_set_additions.
REGISTERED_ADDITION_REL = ("audit/v4.1.0/runs/20261124-task8-alpha-negfix-001"
                        "/manifest.yaml")
UNRELATED_REL = "docs/releases/v4.1.0/refactoring/README.md"

PRE_FREEZE_BASE_SHA = "e9ff21fca3aa945f6fe543ce8f91dfc19e6aaed0"
FROZEN_BASE_SHA = "fd849ae20541901d5e59f45cd3ba2c51093f5d5e"

FAILURES: list[str] = []


def check(name: str, cond: bool, detail: str = "") -> None:
    if cond:
        print(f"  [OK]   {name}")
    else:
        print(f"  [FAIL] {name} {detail}")
        FAILURES.append(name)


def git_out(cwd, *args: str) -> str:
    return subprocess.check_output(["git", *args], cwd=str(cwd), text=True).strip()


def source_repo() -> Path:
    """The main checkout (a worktree's .git is a file, not a directory)."""
    common = Path(git_out(REPO_ROOT, "rev-parse", "--path-format=absolute",
                           "--git-common-dir"))
    return common.parent


def clone_shared(dest: Path, branch: str) -> None:
    """Fast clone that keeps every ref (the source commits stay reachable)."""
    subprocess.run(["git", "clone", "--quiet", "--shared", "--branch", branch,
                    str(source_repo()), str(dest)],
                   check=True, capture_output=True, text=True)
    # No auto-gc: a background gc writing into the clone while the temporary
    # directory is being removed makes the cleanup fail with ENOTEMPTY.
    subprocess.run(["git", "config", "gc.auto", "0"], cwd=str(dest),
                   check=True, capture_output=True)


def run_verifier(repo: Path, *args: str, python_flags: list[str] | None = None,
                 env: dict[str, str] | None = None):
    cmd = [sys.executable, *(python_flags or []), str(Path(repo) / VERIFIER_REL), *args]
    environment = dict(os.environ)
    if env:
        environment.update(env)
    return subprocess.run(cmd, cwd=str(repo), capture_output=True, text=True,
                          env=environment)


def git_reset(repo: Path, pristine: str) -> None:
    subprocess.run(["git", "reset", "--hard", "--quiet", pristine], cwd=str(repo),
                   check=True, capture_output=True)
    subprocess.run(["git", "clean", "-fdq"], cwd=str(repo), check=True, capture_output=True)


def commit_all(repo: Path, message: str) -> None:
    subprocess.run(["git", "add", "-A"], cwd=str(repo), check=True, capture_output=True)
    subprocess.run(["git", "-c", "user.name=probe", "-c", "user.email=probe@example.invalid",
                    "commit", "--quiet", "-m", message],
                   cwd=str(repo), check=True, capture_output=True)


def probe_red(repo: Path, pristine: str, name: str, expect_in_output: str,
              mutate, commit: bool = True) -> None:
    """A mutation must turn the verifier red, mentioning `expect_in_output`."""
    mutate(repo)
    if commit:
        commit_all(repo, f"probe: {name}")
    try:
        proc = run_verifier(repo)
    finally:
        git_reset(repo, pristine)
    check(f"{name} ⇒ 退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
    check(f"{name} ⇒ 报出原因", expect_in_output in proc.stdout,
          f"stdout={proc.stdout.strip()[-400:]}")


def probe_green(repo: Path, pristine: str, name: str, mutate) -> None:
    """A legitimate state must NOT turn the verifier red."""
    mutate(repo)
    commit_all(repo, f"probe: {name}")
    try:
        proc = run_verifier(repo)
    finally:
        git_reset(repo, pristine)
    check(f"{name} ⇒ 退出码 0", proc.returncode == 0, proc.stdout.strip()[-400:])
    check(f"{name} ⇒ 判定行 6/6 PASS", "6/6 PASS" in proc.stdout, proc.stdout.strip()[-400:])


def probe_red_with_env(repo: Path, name: str, expect_in_output: str,
                       env: dict[str, str]) -> None:
    """A run under a given environment must turn the verifier red."""
    proc = run_verifier(repo, env=env)
    check(f"{name} ⇒ 退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
    check(f"{name} ⇒ 报出原因", expect_in_output in proc.stdout,
          f"stdout={proc.stdout.strip()[-400:]}")


def probe_skip_with_env(repo: Path, name: str, expect_in_output: str,
                        env: dict[str, str]) -> None:
    """A run that cannot check something must say so -- never report it as verified."""
    proc = run_verifier(repo, env=env)
    check(f"{name} ⇒ 退出码 0", proc.returncode == 0, proc.stdout.strip()[-400:])
    check(f"{name} ⇒ 报出原因", expect_in_output in proc.stdout,
          f"stdout={proc.stdout.strip()[-400:]}")
    proc = run_verifier(repo, "--strict", env=env)
    check(f"{name} ⇒ --strict 退出码非 0", proc.returncode != 0, f"got {proc.returncode}")


def edit_json(repo: Path, rel: str, mutate) -> None:
    path = Path(repo) / rel
    data = json.loads(path.read_text())
    mutate(data)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


def edit_text(repo: Path, rel: str, old: str, new: str) -> None:
    path = Path(repo) / rel
    text = path.read_text()
    assert old in text, f"{rel} does not contain {old!r}"
    path.write_text(text.replace(old, new))


def main() -> int:
    branch = git_out(REPO_ROOT, "rev-parse", "--abbrev-ref", "HEAD")
    print(f"分支：{branch}")
    under_test = os.environ.get("VERIFY_SHA_SCOPE_UNDER_TEST")
    if under_test:
        print(f"被测校验器：{under_test}（覆盖克隆内的版本）")

    with tempfile.TemporaryDirectory(ignore_cleanup_errors=True) as tmp:
        tmp = Path(tmp)
        shared = tmp / "shared"
        clone_shared(shared, branch)
        if under_test:
            # Commit the verifier under test: every probe resets the clone back
            # to `pristine`, so an uncommitted copy would be restored away after
            # the first probe and the rest would silently test the committed
            # verifier instead.
            shutil.copyfile(under_test, Path(shared) / VERIFIER_REL)
            commit_all(shared, "probe: the verifier under test")
        pristine = git_out(shared, "rev-parse", "HEAD")

        print("A. 干净树")
        proc = run_verifier(shared)
        check("退出码 0", proc.returncode == 0, proc.stdout.strip()[-400:])
        check("输出 6/6 PASS", "6/6 PASS" in proc.stdout, proc.stdout.strip()[-400:])
        proc = run_verifier(shared, "--strict")
        check("--strict 退出码 0", proc.returncode == 0, proc.stdout.strip()[-400:])

        print("B. 前向绑定被改回旧 SHA ⇒ 必须判红")

        def dag_task(repo: Path) -> None:
            def mutate(data):
                data["tasks"][7]["base_sha"] = PRE_FREEZE_BASE_SHA
            edit_json(repo, DAG_REL, mutate)

        probe_red(shared, pristine, "DAG 某一个 task 的 base_sha 回退",
                  "do not point at the baseline", dag_task)

        def matrix_case(repo: Path) -> None:
            def mutate(data):
                data["cases"][41]["base_sha"] = PRE_FREEZE_BASE_SHA
            edit_json(repo, MATRIX_REL, mutate)

        probe_red(shared, pristine, "矩阵某一个 case 的 base_sha 回退",
                  "do not point at the baseline", matrix_case)

        def task_manifest(repo: Path) -> None:
            path = Path(repo) / TASK_MANIFEST_REL
            lines = path.read_text().splitlines()
            record = json.loads(lines[3])
            record["base_commit"] = PRE_FREEZE_BASE_SHA
            lines[3] = json.dumps(record, ensure_ascii=False)
            path.write_text("\n".join(lines) + "\n")

        probe_red(shared, pristine, "task manifest 某一行 base_commit 回退",
                  "do not point at the baseline", task_manifest)

        def task_manifest_order(repo: Path) -> None:
            path = Path(repo) / TASK_MANIFEST_REL
            lines = path.read_text().splitlines()
            lines[0], lines[1] = lines[1], lines[0]
            path.write_text("\n".join(lines) + "\n")

        probe_red(shared, pristine, "task manifest 两行对调（同序检查）",
                  "does not match the DAG", task_manifest_order)

        def handoff_base(repo: Path) -> None:
            edit_text(repo, HANDOFF_REL, FROZEN_BASE_SHA, PRE_FREEZE_BASE_SHA)

        probe_red(shared, pristine, "14_HANDOFF 的 base_sha 回退",
                  "base_sha is", handoff_base)

        def adr_base(repo: Path) -> None:
            edit_text(repo, HANDOFF_REL,
                      "45da6968ae2baaade6a31206eeee5854a8514ee2", FROZEN_BASE_SHA)

        probe_red(shared, pristine, "adr_base_commit 被重指",
                  "adr_base_commit is", adr_base)

        def dropped_task(repo: Path) -> None:
            def mutate(data):
                data["tasks"].pop()
            edit_json(repo, DAG_REL, mutate)

        probe_red(shared, pristine, "DAG 少了一个 task（计数守卫）",
                  "expected 18", dropped_task)

        print("C. 历史绑定被改写 ⇒ 必须判红")

        def frozen_commit_rewritten(repo: Path) -> None:
            edit_text(repo, HISTORICAL_MANIFEST_REL,
                      "c1a5b0ba9dc34eeb24eefae22e4f3aa7af7e815e", "0" * 40)

        probe_red(shared, pristine, "历史 manifest 的 frozen_commit 被改写",
                  "changed its frozen_commit binding", frozen_commit_rewritten)

        print("D. 评审抓到的两个假绿洞")

        def pilot_evidence_corrupted(repo: Path) -> None:
            path = Path(repo) / PILOT_REL
            data = json.loads(path.read_text())
            data["base_sha"] = "0" * 40
            path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")

        probe_red(shared, pristine, "仓内被跟踪的 pilot 证据被改坏",
                  f"{PILOT_REL} points at", pilot_evidence_corrupted)

        def historical_manifest_deleted(repo: Path) -> None:
            subprocess.run(["git", "rm", "--quiet", HISTORICAL_MANIFEST_REL],
                           cwd=str(repo), check=True, capture_output=True)

        probe_red(shared, pristine, "历史 manifest 被删除",
                  "are gone from the worktree", historical_manifest_deleted)

        def unregistered_manifest_added(repo: Path) -> None:
            # A new audit run manifest that is NOT listed in the freeze record's
            # historical_set_additions must still be rejected: the set may only
            # widen by an entry that is registered.
            path = Path(repo) / "audit/v4.1.0/runs/20991231-unregistered/manifest.yaml"
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("schema_version: 1\n"
                            "run_id: 20991231-unregistered\n"
                            "frozen_commit: " + "0" * 40 + "\n")

        probe_red(shared, pristine, "没有登记在冻结记录里的新 manifest",
                  "not part of the frozen historical set", unregistered_manifest_added)

        def registered_addition_binding_removed(repo: Path) -> None:
            # A registered addition must still declare its own frozen_commit
            # binding -- otherwise the binding comparison would be vacuous for
            # it, which is exactly what the frozen entries are checked for.
            path = Path(repo) / REGISTERED_ADDITION_REL
            text = path.read_text()
            path.write_text(re.sub(r"^frozen_commit:.*\n", "", text, flags=re.M))

        probe_red(shared, pristine, "已登记增补项的 frozen_commit 绑定被抹掉",
                  "declares no frozen_commit binding",
                  registered_addition_binding_removed)

        def artifact_context_repointed(repo: Path) -> None:
            def mutate(data):
                data["artifact_context"]["source_code_base_sha"] = FROZEN_BASE_SHA
            edit_json(repo, DAG_REL, mutate)

        probe_red(shared, pristine, "artifact_context.source_code_base_sha 被重指",
                  "artifact_context.source_code_base_sha is", artifact_context_repointed)

        def binding_key_renamed(repo: Path) -> None:
            pattern = re.compile(r"^(\s*)frozen_commit:", re.M)
            for manifest in sorted((Path(repo) / "audit/v4.1.0/runs").glob("*/manifest.yaml")):
                text = manifest.read_text()
                manifest.write_text(pattern.sub(r"\1frozen_commit_x:", text))

        probe_red(shared, pristine, "frozen_commit 绑定键被改名（不得空过）",
                  "the binding comparison would be vacuous", binding_key_renamed)

        print("E. 下游任务的合法改动不得判红")

        def matrix_case_rewritten(repo: Path) -> None:
            # Task 5 会重写矩阵用例的判定字段 —— 本门禁只管 SHA，不该拦。
            def mutate(data):
                data["cases"][0]["note"] = "Task 5 rewrote this case"
            edit_json(repo, MATRIX_REL, mutate)

        probe_green(shared, pristine, "矩阵用例的非 SHA 字段被合法改写（Task 5 形态）",
                    matrix_case_rewritten)

        def adr_refs_filled(repo: Path) -> None:
            # Task 4 会把 ADR 批准填进 tasks[].approved_adr_refs。
            def mutate(data):
                data["tasks"][0]["approved_adr_refs"] = ["ADR-410-001"]
            edit_json(repo, DAG_REL, mutate)

        probe_green(shared, pristine, "DAG 里填了 approved_adr_refs（Task 4 形态）",
                    adr_refs_filled)

        print("F. 工作树必须干净（判定绑定到提交）")

        def dirty_tree(repo: Path) -> None:
            path = Path(repo) / UNRELATED_REL
            path.write_text(path.read_text() + "\n<!-- dirty -->\n")

        probe_red(shared, pristine, "工作树有未提交改动",
                  "the working tree is not clean", dirty_tree, commit=False)

        print("G. python -O 必须拒绝运行")
        proc = run_verifier(shared, python_flags=["-O"])
        check("退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
        check("不打印 6/6 PASS", "6/6 PASS" not in proc.stdout, proc.stdout.strip()[-400:])
        check("说明拒绝原因", "python -O" in proc.stdout, proc.stdout.strip()[-400:])

        print("H. 仓外 pilot 副本的三个分支")
        fake_missing = tmp / "fake-pilot-missing"
        (fake_missing / "evidence").mkdir(parents=True, exist_ok=True)
        probe_red_with_env(shared, "pilot checkout 在但文件没了 ⇒ 硬失败",
                           "cannot have been deleted",
                           {"SQLRUSTGO_PILOT_ROOT": str(fake_missing)})

        fake_divergent = tmp / "fake-pilot-divergent"
        (fake_divergent / "evidence").mkdir(parents=True, exist_ok=True)
        (fake_divergent / "evidence/rerun-summary-20261120.json").write_text(
            json.dumps({"base_sha": "0" * 40}, ensure_ascii=False, indent=2) + "\n")
        probe_red_with_env(shared, "仓外 pilot 副本与仓内不一致 ⇒ 判红",
                           "differs from the tracked",
                           {"SQLRUSTGO_PILOT_ROOT": str(fake_divergent)})

        probe_skip_with_env(shared, "整台机器没有 pilot checkout ⇒ 显式记为未核",
                            "cross-check(s) skipped",
                            {"SQLRUSTGO_PILOT_ROOT": str(tmp / "fake-pilot-absent")})

    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: verify_sha_scope 变异夹具全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())

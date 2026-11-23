#!/usr/bin/env python3
"""verify_baseline.py 的变异夹具 —— 证明基线校验器真的会咬。

WHY（2026-11-23）
------------------
冻结记录的全部价值在于：校验器能区分「核过了」与「没核」。一个恒绿的校验器
比没有校验器更坏 —— 它给下游任务一个假信号。本夹具对每条断言各做一次变异，
要求校验器必须变红；并单独验证「浅克隆下核不了的交叉核对必须被显式报告，
且 --strict 必须因此失败」以及「python -O 下校验器必须拒绝运行」。

所有变异都在临时克隆里做，不碰工作仓库。

用法：python3 docs/releases/v4.1.0/refactoring/tools/test_verify_baseline.py
退出码：0 = 全部通过；1 = 有断言失败（fail-closed）。
"""
from __future__ import annotations

import json
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[4]
VERIFIER_REL = "docs/releases/v4.1.0/refactoring/tools/verify_baseline.py"
RECORD_REL = "docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json"
MATERIAL_FILE = "docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json"
ALREADY_PRESENT_FILE = "scripts/soak/soak_steady_window.py"

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


def clone_single_branch(dest: Path, branch: str) -> None:
    """Clone that carries only this branch: the source commits are unreachable."""
    subprocess.run(["git", "clone", "--quiet", "--single-branch", "--no-local",
                    "--branch", branch, f"file://{source_repo()}", str(dest)],
                   check=True, capture_output=True, text=True)


def run_verifier(repo: Path, *args: str, python_flags: list[str] | None = None):
    cmd = [sys.executable, *(python_flags or []), str(Path(repo) / VERIFIER_REL), *args]
    return subprocess.run(cmd, cwd=str(repo), capture_output=True, text=True)


def patch_record(repo: Path, mutate) -> str:
    """Apply `mutate` to the freeze record; return the original text."""
    path = Path(repo) / RECORD_REL
    original = path.read_text()
    data = json.loads(original)
    mutate(data)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")
    return original


def entry_for(data, prefix: str) -> dict:
    for entry in data["cherry_picked"]:
        if entry["sha"].startswith(prefix):
            return entry
    raise KeyError(prefix)


def expect_red(repo: Path, name: str, expect_in_output: str, mutate) -> None:
    """A mutation must turn the verifier red, mentioning `expect_in_output`."""
    original = patch_record(repo, mutate)
    try:
        proc = run_verifier(repo)
    finally:
        (Path(repo) / RECORD_REL).write_text(original)
    check(f"{name} ⇒ 退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
    check(f"{name} ⇒ 报出原因", expect_in_output in proc.stdout,
          f"stdout={proc.stdout.strip()[-400:]}")


def commit_tampering(repo: Path, path: str, extra: bytes, message: str):
    """Tamper with a file, commit it, and return a record mutation that
    re-points `base_sha` at the tampered commit."""
    target = Path(repo) / path
    target.write_bytes(target.read_bytes() + extra)
    subprocess.run(["git", "add", path], cwd=str(repo), check=True, capture_output=True)
    subprocess.run(["git", "-c", "user.name=probe", "-c", "user.email=probe@example.invalid",
                    "commit", "--quiet", "-m", message],
                   cwd=str(repo), check=True, capture_output=True)
    head = git_out(repo, "rev-parse", "HEAD")
    tree = git_out(repo, "rev-parse", "HEAD^{tree}")

    def repoint(data):
        data["base_sha"] = head
        data["tree_sha"] = tree

    return repoint


def main() -> int:
    branch = git_out(REPO_ROOT, "rev-parse", "--abbrev-ref", "HEAD")
    print(f"分支：{branch}")

    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        shared = tmp / "shared"
        clone_shared(shared, branch)
        pristine = git_out(shared, "rev-parse", "HEAD")

        print("A. 干净树")
        proc = run_verifier(shared)
        check("退出码 0", proc.returncode == 0, proc.stdout.strip()[-400:])
        check("输出 4/4 PASS", "4/4 PASS" in proc.stdout)
        check("没有未核对的交叉核对", "skipped:" not in proc.stdout, proc.stdout.strip()[-400:])
        proc = run_verifier(shared, "--strict")
        check("--strict 在完整克隆下退出码 0", proc.returncode == 0)

        print("B. 台账与导入的变异必须让校验器变红")

        def wrong_state(data):
            entry_for(data, "cd8cefe623")["state"] = "applied"

        expect_red(shared, "state 分类错误（cd8cefe623 谎称 applied）",
                   "expected 1 'applied' entries", wrong_state)

        def empty_applied(data):
            # 谎称由一个空提交承载 —— 净增量为空却标成 applied
            entry_for(data, "45da6968")["cherry_pick"] = \
                "fd849ae20541901d5e59f45cd3ba2c51093f5d5e"

        expect_red(shared, "applied 条目的净增量为空", "carries no change", empty_applied)

        def wrong_count(data):
            entry_for(data, "b681b74ca3")["files"] = 62

        expect_red(shared, "path-import 目录的冻结计数与记录不符",
                   "the record says 62", wrong_count)

        print("C. 冻结基线被篡改（blob 级）")
        repoint = commit_tampering(shared, MATERIAL_FILE, b"\n<!-- tampered -->\n",
                                   "probe: tampered material")
        expect_red(shared, "材料文件的 blob 与源提交不符",
                   "was altered during import", repoint)
        subprocess.run(["git", "reset", "--hard", "--quiet", pristine], cwd=str(shared),
                       check=True, capture_output=True)

        repoint = commit_tampering(shared, ALREADY_PRESENT_FILE, b"\n# tampered\n",
                                   "probe: tampered already-present file")
        expect_red(shared, "already-present 条目与冻结 parent 实际有差异",
                   "differs between the frozen parent and the baseline", repoint)
        subprocess.run(["git", "reset", "--hard", "--quiet", pristine], cwd=str(shared),
                       check=True, capture_output=True)

        print("D. 工作树漂移")
        drift = Path(shared) / ALREADY_PRESENT_FILE
        original_bytes = drift.read_bytes()
        drift.write_bytes(original_bytes + b"\n# drift\n")
        proc = run_verifier(shared)
        check("工作树偏离冻结基线 ⇒ 退出码非 0", proc.returncode != 0,
              f"got {proc.returncode}")
        check("报出漂移", "drifted away from the frozen baseline" in proc.stdout,
              proc.stdout.strip()[-400:])
        drift.write_bytes(original_bytes)

        print("E. python -O 必须拒绝运行")
        proc = run_verifier(shared, python_flags=["-O"])
        check("退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
        check("不打印 4/4 PASS", "4/4 PASS" not in proc.stdout, proc.stdout.strip()[-400:])
        check("说明拒绝原因", "python -O" in proc.stdout, proc.stdout.strip()[-400:])

        print("F. 单分支克隆：核不了的交叉核对必须被显式报告")
        single = tmp / "single"
        clone_single_branch(single, branch)
        for sha in ("45da6968ae2baaade6a31206eeee5854a8514ee2",
                    "b681b74ca3c0373cb2b7c44ff2cb032cf1a1c540"):
            check(f"源提交 {sha[:10]} 在单分支克隆里不可达",
                  subprocess.run(["git", "cat-file", "-e", f"{sha}^{{commit}}"],
                                 cwd=str(single), capture_output=True).returncode != 0)
        proc = run_verifier(single)
        check("正常模式退出码 0", proc.returncode == 0, proc.stdout.strip()[-400:])
        check("输出 4/4 PASS", "4/4 PASS" in proc.stdout)
        check("显式列出跳过的交叉核对", "skipped:" in proc.stdout, proc.stdout.strip()[-400:])
        check("判定行写明跳过数量", "cross-check(s) skipped" in proc.stdout,
              proc.stdout.strip()[-400:])
        proc = run_verifier(single, "--strict")
        check("--strict 退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
        check("--strict 判定为 STRICT FAIL", "STRICT FAIL" in proc.stdout,
              proc.stdout.strip()[-400:])

    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: verify_baseline 变异夹具全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())

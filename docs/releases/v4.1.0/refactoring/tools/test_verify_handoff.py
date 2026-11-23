#!/usr/bin/env python3
"""verify_handoff.py 的变异夹具 —— 证明这个交接门禁真的会咬。

WHY（2026-11-23）
------------------
第一轮评审抓到六个洞：一个恒绿的断言组、一个把「NOT APPROVED」读成「APPROVED」
的档位判定、一个从没被真正解析过的 Approvals 字段、一个名不副实的变异臂、
两个可以被自己重签掉的锚点。这些都在这里钉死：

  * 档位判定必须是整词匹配：`NOT APPROVED` / `NOT_APPROVED` / `UNAPPROVED`
    以及「散文里提到 APPROVED」都必须判成 PROPOSED；
  * `Approvals:` 字段必须真的被解析，字段里的每一项都必须指向真实存在的
    ADR —— 不认识的项要判红，不能悄悄过滤掉；
  * 「档位越权」变异臂必须自己提供档位表，不能靠 ADR 正文当前的措辞；
  * freeze 记录必须在 manifest 的覆盖范围内：改它就要判红；
  * HARA manifest 的摘要锚点必须来自冻结基线提交的内容：改完 HARA 再
    `--finalize` 重签也必须判红；
  * ADR 目录的期望状态必须钉死在 freeze 记录里：把 011 挪出去、重签，
    也必须判红。

所有变异都在临时克隆里做，不碰工作仓库。

用法：python3 docs/releases/v4.1.0/refactoring/tools/test_verify_handoff.py
退出码：0 = 全部通过；1 = 有断言失败（fail-closed）。

依赖：jsonschema==4.23.0（见同目录 requirements.txt）。夹具用 sys.executable
起子进程，所以要用装了该依赖的解释器跑本夹具。

环境变量 VERIFY_HANDOFF_UNDER_TEST
    指向另一个校验器文件：夹具会把它拷进临时克隆再跑。用来证明「修之前不咬、
    修之后才咬」—— 对着修前的版本跑，相应探针会失败。
"""
from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parents[4]
VERIFIER_REL = "docs/releases/v4.1.0/refactoring/tools/verify_handoff.py"

FREEZE_REL = "docs/refactor/v4.1.0-c/takeover/BASELINE_FREEZE.json"
HARA_REL = "docs/releases/v4.1.0/refactoring/audit/HARA-410/evidence/MANIFEST.json"
ADR_REL = "docs/releases/v4.1.0/refactoring/adrs/ADR-410-001.md"
DAG_REL = "docs/releases/v4.1.0/refactoring/REFACTOR_TASK_DAG.json"
BASELINE_REL = "docs/refactor/v4.1.0-c/handoff/01_ARCHITECTURE_BASELINE.json"
MANIFEST_REL = "docs/refactor/v4.1.0-c/handoff/10_HANDOFF_MANIFEST.json"
ADR011_SRC_REL = ("docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/"
                  "ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md")
ADR011_README_REL = ("docs/releases/v4.1.0/refactoring/runtime-contract/20261120-pr5915/"
                     "00_DECISION_README.md")
ADR011_DST_REL = "docs/releases/v4.1.0/refactoring/adrs/ADR-410-011.md"

# The `Status:` line every ADR in the shipped corpus carries.
CORPUS_STATUS = "Status: PROPOSED / OWNER_REVIEW_REQUIRED。Approvals: []。本轮无正式接口实现。"

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


def run_verifier(repo: Path, *args: str, python_flags: list[str] | None = None):
    cmd = [sys.executable, *(python_flags or []), str(Path(repo) / VERIFIER_REL), *args]
    return subprocess.run(cmd, cwd=str(repo), capture_output=True, text=True)


def git_reset(repo: Path, pristine: str) -> None:
    subprocess.run(["git", "reset", "--hard", "--quiet", pristine], cwd=str(repo),
                   check=True, capture_output=True)
    subprocess.run(["git", "clean", "-fdq"], cwd=str(repo), check=True, capture_output=True)


def commit_all(repo: Path, message: str) -> None:
    subprocess.run(["git", "add", "-A"], cwd=str(repo), check=True, capture_output=True)
    subprocess.run(["git", "-c", "user.name=probe", "-c", "user.email=probe@example.invalid",
                    "commit", "--quiet", "-m", message],
                   cwd=str(repo), check=True, capture_output=True)


def edit_text(repo: Path, rel: str, old: str, new: str) -> None:
    path = Path(repo) / rel
    text = path.read_text()
    assert old in text, f"{rel} does not contain {old!r}"
    path.write_text(text.replace(old, new))


def edit_json(repo: Path, rel: str, mutate) -> None:
    path = Path(repo) / rel
    data = json.loads(path.read_text())
    mutate(data)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n")


def set_adr_status(repo: Path, status_line: str) -> None:
    """Replace ADR-410-001's `Status:` line with `status_line`."""
    path = Path(repo) / ADR_REL
    text = path.read_text()
    assert CORPUS_STATUS in text, "the corpus status line changed"
    path.write_text(text.replace(CORPUS_STATUS, status_line))


def cite_adr(repo: Path, adr_id: str) -> None:
    """Make task C01-01 cite `adr_id`."""
    def mutate(data):
        data["tasks"][0]["approved_adr_refs"] = [adr_id]
    edit_json(repo, DAG_REL, mutate)


def probe_red(repo: Path, pristine: str, name: str, expect_in_output: str,
              mutate, resign: bool = True, commit: bool = True) -> None:
    """A mutation must turn the verifier red, mentioning `expect_in_output`.

    `resign` runs `--finalize` before `--check`: several probes are about the
    gate's own verdict, and re-signing removes the manifest's digest as a
    confounder, so that only the probed rule can turn the gate red.
    """
    mutate(repo)
    if commit:
        commit_all(repo, f"probe: {name}")
    try:
        if resign:
            fin = run_verifier(repo, "--finalize")
            if fin.returncode != 0:
                # The gate refused to re-sign: that is already a red verdict.
                check(f"{name} ⇒ 退出码非 0", True)
                check(f"{name} ⇒ 报出原因", expect_in_output in fin.stdout,
                      f"stdout={fin.stdout.strip()[-400:]}")
                return
        proc = run_verifier(repo, "--check")
    finally:
        git_reset(repo, pristine)
    check(f"{name} ⇒ 退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
    check(f"{name} ⇒ 报出原因", expect_in_output in proc.stdout,
          f"stdout={proc.stdout.strip()[-400:]}")


def probe_green(repo: Path, pristine: str, name: str, mutate, resign: bool = True) -> None:
    """A legitimate state must NOT turn the verifier red."""
    mutate(repo)
    commit_all(repo, f"probe: {name}")
    try:
        if resign:
            fin = run_verifier(repo, "--finalize")
            check(f"{name} ⇒ 重签退出码 0", fin.returncode == 0, fin.stdout.strip()[-400:])
        proc = run_verifier(repo, "--check")
    finally:
        git_reset(repo, pristine)
    check(f"{name} ⇒ 退出码 0", proc.returncode == 0, proc.stdout.strip()[-400:])


def summary_of(repo: Path) -> dict:
    proc = run_verifier(repo, "--check")
    if proc.returncode != 0:
        return {}
    try:
        return json.loads(proc.stdout)
    except ValueError:
        return {}


def main() -> int:
    branch = git_out(REPO_ROOT, "rev-parse", "--abbrev-ref", "HEAD")
    print(f"分支：{branch}")
    under_test = os.environ.get("VERIFY_HANDOFF_UNDER_TEST")
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
            # Re-sign with the verifier under test, so the clone's manifest
            # matches its own verifier.  Otherwise every probe would trip the
            # manifest's digest of the verifier file and the probe would be
            # testing the wrong thing.
            fin = run_verifier(shared, "--finalize")
            if fin.returncode == 0:
                commit_all(shared, "probe: re-signed with the verifier under test")
            else:
                print(f"  [WARN] 被测校验器无法重签：{fin.stdout.strip()[-300:]}")
        pristine = git_out(shared, "rev-parse", "HEAD")

        print("A. 干净树")
        proc = run_verifier(shared, "--check")
        check("退出码 0", proc.returncode == 0, proc.stdout.strip()[-400:])
        summary = summary_of(shared)
        arms = summary.get("mutation_rejections", [])
        check("十个变异臂全部被拒", len(arms) == 10, f"got {arms}")
        for arm in ("tier_overreach", "unapproved_adr_still_proposed", "unapproved_adr"):
            check(f"{arm} 在被拒名单里", arm in arms, f"got {arms}")

        print("B. 档位判定必须整词匹配（NOT APPROVED 不得读成 APPROVED）")
        for label, status in (
            ("NOT APPROVED", "Status: NOT APPROVED。Approvals: []。"),
            ("NOT_APPROVED", "Status: NOT_APPROVED。Approvals: []。"),
            ("UNAPPROVED", "Status: UNAPPROVED。Approvals: []。"),
            ("散文里的 APPROVED", "Status: 本轮不得视为 APPROVED。Approvals: []。"),
        ):
            def mutate(repo: Path, status=status) -> None:
                set_adr_status(repo, status)
                cite_adr(repo, "ADR-410-001")
            probe_red(shared, pristine, f"档位 {label} ⇒ 引用它的 task 被拒",
                      "still PROPOSED", mutate)

        print("C. Approvals 字段必须被解析，非 ADR 项必须判红")
        def approvals_probe(label: str, approvals: str, expect: str) -> None:
            def mutate(repo: Path) -> None:
                set_adr_status(repo,
                               f"Status: APPROVED（依 ADR-DECISION-RECORD.md:35-47，2026-11-21）。"
                               f"Approvals: {approvals}。")
            probe_red(shared, pristine, f"Approvals {label} ⇒ 判红", expect, mutate)
        approvals_probe("含非 ADR 项 OWNER-DECISION-C-ROUTE", "[OWNER-DECISION-C-ROUTE]",
                        "which is not an ADR reference")
        approvals_probe("含不存在的 ADR-410-999", "[ADR-410-999]", "which is not a real ADR id")

        def mutate_real_approval(repo: Path) -> None:
            set_adr_status(repo,
                           "Status: APPROVED（依 ADR-DECISION-RECORD.md:35-47，2026-11-21）。"
                           "Approvals: [ADR-410-002]。")
        probe_green(shared, pristine, "Approvals 含真实 ADR-410-002 ⇒ 合法", mutate_real_approval)
        # The catalogue must carry the parsed reference -- not an empty list.
        set_adr_status(shared, "Status: APPROVED（依 ADR-DECISION-RECORD.md:35-47，2026-11-21）。"
                                "Approvals: [ADR-410-002]。")
        commit_all(shared, "probe: approvals parsed")
        try:
            run_verifier(shared, "--finalize")
            baseline = json.loads((Path(shared) / BASELINE_REL).read_text())
            entry = next((e for e in baseline["adrs"] if e["id"] == "ADR-410-001"), {})
            check("目录里 ADR-410-001 的 approval_refs 真的被解析出来",
                  entry.get("approval_refs") == ["ADR-410-002"], f"got {entry.get('approval_refs')}")
        finally:
            git_reset(shared, pristine)

        print("D. 「档位越权」臂必须自己咬自己")
        summary = summary_of(shared)
        reason = summary.get("mutation_rejection_reasons", {}).get("tier_overreach", "")
        check("档位越权臂的理由是授权执行引用了非 APPROVED 档位",
              "authorised for execution" in reason, f"got {reason!r}")
        check("档位越权臂的理由落在 APPROVED_DIRECTION 档位上",
              "APPROVED_DIRECTION" in reason, f"got {reason!r}")
        check("档位越权臂的理由不是 PROPOSED 分支",
              "still PROPOSED" not in reason, f"got {reason!r}")
        reason = summary.get("mutation_rejection_reasons", {}).get("unapproved_adr_still_proposed", "")
        check("PROPOSED 分支另有独立的一臂",
              "still PROPOSED" in reason, f"got {reason!r}")

        print("E. freeze 记录必须在 manifest 的覆盖范围内")
        manifest = json.loads((Path(shared) / MANIFEST_REL).read_text())
        check("manifest 覆盖 freeze 记录", FREEZE_REL in manifest["files"],
              f"files={len(manifest['files'])}")
        check("manifest 覆盖交接包 README", "docs/refactor/v4.1.0-c/README.md" in manifest["files"])
        check("manifest 不覆盖 takeover 顶层的其它文档",
              not any(p.startswith("docs/refactor/v4.1.0-c/takeover/")
                      and "dsh-b" not in p and p != FREEZE_REL for p in manifest["files"]),
              "an active takeover document leaked into the hash range")
        check("manifest 覆盖 takeover/dsh-b/",
              any("/takeover/dsh-b/" in p for p in manifest["files"]))

        def mutate_base_sha(repo: Path) -> None:
            def mutate(data):
                data["base_sha"] = "0" * 40
            edit_json(repo, FREEZE_REL, mutate)
        probe_red(shared, pristine, "改 freeze 记录的 base_sha ⇒ 判红",
                  "is bound to base_sha", mutate_base_sha, resign=False)

        def mutate_frozen_at(repo: Path) -> None:
            def mutate(data):
                data["frozen_at"] = "2000-01-01T00:00:00+00:00"
            edit_json(repo, FREEZE_REL, mutate)
        # Only the manifest's coverage can catch this one: base_sha and tree_sha
        # are untouched, so nothing else in the gate can see the edit.
        probe_red(shared, pristine, "改 freeze 记录的 frozen_at ⇒ 判红（只有覆盖才看得到）",
                  FREEZE_REL, mutate_frozen_at, resign=False)

        print("F. HARA 摘要锚点不能被 --finalize 重写")
        def mutate_hara(repo: Path) -> None:
            path = Path(repo) / HARA_REL
            path.write_text(path.read_text() + "\n")
        # The mutation is re-signed on purpose: only an anchor that --finalize
        # cannot rewrite can still catch it.
        probe_red(shared, pristine, "改 HARA manifest 再重签 ⇒ 仍判红",
                  "no longer matches the frozen base commit's copy", mutate_hara, resign=True)

        print("G. ADR 目录的期望状态必须钉死")
        def merge_adr_011(repo: Path) -> None:
            """Simulate the O7 merge: relocate ADR-410-011 and record it.

            The relocation also has to repair the link the runtime contract's
            README carries to the ADR -- the gate checks markdown links, so a
            merge that leaves a dangling link is not a merge that passes.
            """
            src = Path(repo) / ADR011_SRC_REL
            dst = Path(repo) / ADR011_DST_REL
            dst.write_text(src.read_text())
            src.unlink()
            readme = Path(repo) / ADR011_README_REL
            text = readme.read_text()
            readme.write_text(text.replace("](ADR-410-011-UNIFIED_RUNTIME_CONTRACT.md)",
                                            "](../../adrs/ADR-410-011.md)"))
            def mutate(data):
                expectation = data["adr_catalogue"]
                expectation["expected_ids"] = sorted(
                    set(expectation["expected_ids"]) | {"ADR-410-011"})
                expectation["expected_size"] = len(expectation["expected_ids"])
                expectation["adr_410_011_in_adrs"] = True
            edit_json(repo, FREEZE_REL, mutate)
        probe_green(shared, pristine, "O7 合并 011 并记录期望 ⇒ 绿", merge_adr_011)

        def merge_then_drop(repo: Path) -> None:
            merge_adr_011(repo)
            (Path(repo) / ADR011_DST_REL).unlink()
        probe_red(shared, pristine, "合并后把 011 拿掉再重签 ⇒ 判红",
                  "expects", merge_then_drop)

        print("H. CLI 的三条边界")
        proc = run_verifier(shared, python_flags=["-O"])
        check("python -O 退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
        check("python -O 说明拒绝原因", "python -O" in proc.stdout, proc.stdout.strip()[-400:])
        proc = run_verifier(shared)
        check("不带旗标退出码非 0", proc.returncode != 0, f"got {proc.returncode}")
        check("不带旗标说明要做什么", "nothing to do" in proc.stderr + proc.stdout,
              (proc.stderr + proc.stdout).strip()[-400:])

    print()
    if FAILURES:
        print(f"FAILED ({len(FAILURES)}): {', '.join(FAILURES)}")
        return 1
    print("OK: verify_handoff 变异夹具全部通过")
    return 0


if __name__ == "__main__":
    sys.exit(main())

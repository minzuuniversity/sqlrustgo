#!/usr/bin/env python3
"""`check_audit_manifest.sh` / `audit_run.py verify` 的**反向夹具**（P0-6 / AMAN-1①）。

不是门禁（不叫 check_*，不进 STAGE_CONFIG 阶段清单）。它回答两件事：

1. 新判据是**承重的**：把健康记录的命令行抹掉 ⇒ 必须变红。
2. 新判据**不误伤**：健康记录必须保持绿；`cmd` 换成同义键 `command` 也必须绿
   （判据盯的是"有没有留下命令行"，不是某个拼写）。

夹具自己造一条**合法的** iaia410-manifest/v1 记录（v2 的写入端早已不在仓里），
所有散列用被测工具自己的函数算，避免夹具与被测实现各写一套口径。

AMAN-1① 的修正（**只改测试侧**）：
    原夹具**从不建 git 提交**，而 `check_audit_manifest.sh` 枚举的是**已提交的树**
    （`git ls-tree -r --name-only HEAD -- audit/v4.1.0`，不是工作区目录）⇒
    沙箱里没有 HEAD ⇒ 门禁在 `git rev-parse --verify HEAD` 失败处早退
    `PASS: no committed audit runs yet` + `exit 0` ⇒ 门禁那条判据**从未被测过**
    （M5 臂恒红，其余臂因为走 `audit_run.py verify`、不看 git 而绿）。
    ⇒ 现在沙箱是**真 git 仓库**，run 目录建好后立即提交。

    另一个坑：**git 不跟踪空目录**。M5 的「无清单 run 目录」必须**含文件**
    （真实案例形态是「只有 findings/」），否则它压根不在已提交树里，
    门禁看不到，M5 依然测不到那条判据。

每条臂再配一条**反向变异**（撤掉对应判据 ⇒ 该臂必须**变绿**），
用来证明这条臂真的压在它声称的那条判据上，而不是碰巧红。
"""
from __future__ import annotations

import hashlib
import importlib.util
import json
import pathlib
import shutil
import subprocess
import sys
import tempfile

REPO = pathlib.Path(__file__).resolve().parents[2]
TOOL = REPO / "audit" / "v4.1.0" / "audit_run.py"
GATE = REPO / "scripts" / "gate" / "check_audit_manifest.sh"
RUN_ID = "iaia410-fixture-healthy"


def _sha(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


def _load_tool(sb_tool: pathlib.Path):
    spec = importlib.util.spec_from_file_location("audit_run_under_test", sb_tool)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


# ---------------------------------------------------------------------------
# git 沙箱：门禁枚举的是**已提交树**，所以沙箱必须是真仓库。
# ---------------------------------------------------------------------------


def _git(sb: pathlib.Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        ["git", "-C", str(sb), *args], capture_output=True, text=True, check=True
    )


def _commit_all(sb: pathlib.Path, msg: str) -> None:
    _git(sb, "add", "-A")
    _git(sb, "commit", "-q", "--allow-empty", "-m", msg)


def _init_repo(sb: pathlib.Path) -> None:
    _git(sb, "init", "-q")
    _git(sb, "config", "user.email", "fixture@example.invalid")
    _git(sb, "config", "user.name", "audit-manifest-fixture")
    _git(sb, "config", "commit.gpgsign", "false")
    _commit_all(sb, "fixture baseline")


def _mutate(path: pathlib.Path, old: str, new: str, *, tag: str) -> None:
    """在**沙箱副本**上做一次**唯一**文本替换。

    替换点命中数不等于 1 ⇒ 直接失败：命中 0 次说明锚点漂了（改动的判据
    已经不在那儿，变异臂就变成了「什么都没改」的假承重），命中 2 次说明
    锚点不唯一（可能改到别的判据上去）。两种情况都必须让整套件红，
    不能默默放过。
    """
    txt = path.read_text(encoding="utf-8")
    n = txt.count(old)
    if n != 1:
        raise AssertionError(f"{tag}: 变异锚点命中 {n} 次（必须恰好 1 次）")
    path.write_text(txt.replace(old, new), encoding="utf-8")


def build_run(sb: pathlib.Path, tool_mod, *, cmd_key: str = "cmd", drop_cmd: bool = False,
              empty_commands: bool = False, drop_log: bool = False) -> pathlib.Path:
    run_dir = sb / "audit" / "v4.1.0" / RUN_ID
    cmds_dir = run_dir / "commands"
    cmds_dir.mkdir(parents=True)
    out = b"hello-stdout\n"
    err = b"hello-stderr\n"
    (cmds_dir / "001.stdout.log").write_bytes(out)
    (cmds_dir / "001.stderr.log").write_bytes(err)
    (cmds_dir / "001.meta.yaml").write_text(
        "schema_version: iaia410-command-meta/v1\nid: 1\nexit_code: 0\n", encoding="utf-8"
    )
    entry = {
        "id": "001",
        "name": "fixture",
        "cwd": ".",
        "timeout_sec": 30,
        "exit_code": 0,
        "stdout": "commands/001.stdout.log",
        "stderr": "commands/001.stderr.log",
        "stdout_sha256": _sha(out),
        "stderr_sha256": _sha(err),
    }
    if not drop_cmd:
        entry[cmd_key] = "echo hello-stdout"
    m = {
        "schema_version": "iaia410-manifest/v1",
        "run_id": RUN_ID,
        "work_package": "FIXTURE",
        "source_agent": "mcode-macmini",
        "repository": {"commit": "0" * 40},
        "commands": [] if empty_commands else [entry],
    }
    m["manifest_sha256"] = tool_mod.manifest_sha256_of(m)
    import yaml
    (run_dir / "manifest.yaml").write_text(
        yaml.safe_dump(m, allow_unicode=True, sort_keys=False), encoding="utf-8"
    )
    subprocess.run([sys.executable, str(sb / "audit" / "v4.1.0" / "audit_run.py"),
                    "sha256sums", "--run-id", RUN_ID], cwd=sb, check=True,
                   capture_output=True, text=True)
    if drop_log:
        (cmds_dir / "001.stdout.log").unlink()
    # run 目录必须在**已提交树**里，否则门禁枚举不到它。
    _commit_all(sb, "fixture run")
    return run_dir


def fresh_sandbox() -> pathlib.Path:
    sb = pathlib.Path(tempfile.mkdtemp(prefix="audit_integrity_"))
    (sb / "audit" / "v4.1.0").mkdir(parents=True)
    shutil.copy(TOOL, sb / "audit" / "v4.1.0" / "audit_run.py")
    (sb / "scripts" / "gate").mkdir(parents=True)
    shutil.copy(GATE, sb / "scripts" / "gate" / "check_audit_manifest.sh")
    _init_repo(sb)
    return sb


def verify(sb: pathlib.Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(sb / "audit" / "v4.1.0" / "audit_run.py"), "verify", "--run-id", RUN_ID],
        cwd=sb, capture_output=True, text=True, timeout=120,
    )


def gate(sb: pathlib.Path) -> subprocess.CompletedProcess:
    return subprocess.run(["bash", "scripts/gate/check_audit_manifest.sh"], cwd=sb,
                          capture_output=True, text=True, timeout=300)


def tool_in(sb: pathlib.Path) -> pathlib.Path:
    return sb / "audit" / "v4.1.0" / "audit_run.py"


def gate_in(sb: pathlib.Path) -> pathlib.Path:
    return sb / "scripts" / "gate" / "check_audit_manifest.sh"


def main() -> int:
    fails: list[str] = []
    mod = _load_tool(TOOL)

    def case(tag, expect_red, **kw):
        sb = fresh_sandbox()
        build_run(sb, mod, **kw)
        r = verify(sb)
        red = r.returncode != 0
        ok = (red if expect_red else not red)
        if not ok:
            fails.append(f"{tag}: 期望 {'红' if expect_red else '绿'}，实际 rc={r.returncode} "
                         f"{(r.stdout + r.stderr).strip()[:120]}")
        shutil.rmtree(sb, ignore_errors=True)
        return sb

    # ---- 正臂：判据承重 + 不误伤 -------------------------------------------
    case("CTRL 健康 v2 记录", expect_red=False)
    case("M1 抹掉命令行(cmd)", expect_red=True, drop_cmd=True)
    case("M2 同义键 command 不误伤", expect_red=False, cmd_key="command")
    case("M3 commands 为空", expect_red=True, empty_commands=True)
    case("M4 删掉日志(既有判据仍承重)", expect_red=True, drop_log=True)

    # M5 有目录无清单 ⇒ 门禁必须点名（改前：静默跳过）
    sb = fresh_sandbox()
    build_run(sb, mod)
    # 无清单的 run 目录**必须含文件**：git 不跟踪空目录，不含文件它就不在
    # 已提交树里 ⇒ 门禁枚举不到 ⇒ M5 依然测不到那条判据。
    # 真实案例形态是「只有 findings/」。
    nom = sb / "audit" / "v4.1.0" / "iaia410-fixture-nomanifest"
    (nom / "findings").mkdir(parents=True)
    (nom / "findings" / "summary.txt").write_text("fixture findings\n", encoding="utf-8")
    _commit_all(sb, "fixture no-manifest run")
    g = gate(sb)
    if g.returncode == 0 or "without manifest" not in (g.stderr + g.stdout):
        fails.append(f"M5 无清单运行目录未被点名: rc={g.returncode} "
                     f"{(g.stdout + g.stderr).strip()[:160]}")
    shutil.rmtree(sb, ignore_errors=True)

    # M6 控制组：沙箱里**有提交**但没有任何 run 目录 ⇒ 门禁不该凭空报错
    sb = fresh_sandbox()
    g = gate(sb)
    if g.returncode != 0:
        fails.append(f"M6 空仓控制组应绿: rc={g.returncode} {(g.stdout + g.stderr).strip()[:120]}")
    shutil.rmtree(sb, ignore_errors=True)

    # ---- 反向变异臂：撤掉对应判据 ⇒ 该臂必须**变绿** ----------------------
    # 没有这一组，「正臂是红的」只能证明「跑出了红」，不能证明「红在它声称的
    # 那条判据上」。每条变异臂都在**沙箱副本**上改，不碰仓库里的真文件。

    # M1R：抹掉命令行（该红）⇒ 再撤掉「缺命令行」判据 ⇒ 必须变绿
    sb = fresh_sandbox()
    build_run(sb, mod, drop_cmd=True)
    try:
        _mutate(
            tool_in(sb),
            '        if not (c.get("cmd") or c.get("command")):',
            '        if False:  # MUTATED: 撤掉「缺命令行」判据',
            tag="M1R",
        )
    except AssertionError as e:
        fails.append(f"M1R 变异失败: {e}")
    r = verify(sb)
    if r.returncode != 0:
        fails.append(f"M1R 撤掉判据后应变绿，实际 rc={r.returncode} "
                     f"{(r.stdout + r.stderr).strip()[:160]}")
    shutil.rmtree(sb, ignore_errors=True)

    # M3R：commands 为空（该红）⇒ 再撤掉「commands 为空」判据 ⇒ 必须变绿
    sb = fresh_sandbox()
    build_run(sb, mod, empty_commands=True)
    try:
        _mutate(
            tool_in(sb),
            "    if not cmds:",
            "    if False:  # MUTATED: 撤掉「commands 为空」判据",
            tag="M3R",
        )
    except AssertionError as e:
        fails.append(f"M3R 变异失败: {e}")
    r = verify(sb)
    if r.returncode != 0:
        fails.append(f"M3R 撤掉判据后应变绿，实际 rc={r.returncode} "
                     f"{(r.stdout + r.stderr).strip()[:160]}")
    shutil.rmtree(sb, ignore_errors=True)

    # M4R：删掉日志（该红）⇒ 再撤掉「日志缺失」检查与 SHA256SUMS 校验 ⇒ 必须变绿。
    # 两个变异点缺一不可：只撤「missing log」的话，随后 `sha256_file(p)` 读不到
    # 文件会直接抛异常（照样非零退出），M4R 就测不出承重。
    sb = fresh_sandbox()
    build_run(sb, mod, drop_log=True)
    try:
        _mutate(
            tool_in(sb),
            '    for c in m.get("commands", []):\n'
            '        cid = str(c["id"])\n'
            '        for key in ("stdout", "stderr"):',
            '    for c in []:  # MUTATED: 撤掉「日志缺失/散列不符」检查\n'
            '        cid = str(c["id"])\n'
            '        for key in ("stdout", "stderr"):',
            tag="M4R-日志检查",
        )
        _mutate(
            tool_in(sb),
            "    for line in sums.read_text().splitlines():",
            "    for line in []:  # MUTATED: 撤掉 SHA256SUMS 逐行校验",
            tag="M4R-SHA256SUMS",
        )
    except AssertionError as e:
        fails.append(f"M4R 变异失败: {e}")
    r = verify(sb)
    if r.returncode != 0:
        fails.append(f"M4R 撤掉判据后应变绿，实际 rc={r.returncode} "
                     f"{(r.stdout + r.stderr).strip()[:160]}")
    shutil.rmtree(sb, ignore_errors=True)

    # M5R：有目录无清单（该红且被点名）⇒ 再撤掉门禁的「点名」判据 ⇒ 必须变绿
    sb = fresh_sandbox()
    build_run(sb, mod)
    nom = sb / "audit" / "v4.1.0" / "iaia410-fixture-nomanifest"
    (nom / "findings").mkdir(parents=True)
    (nom / "findings" / "summary.txt").write_text("fixture findings\n", encoding="utf-8")
    _commit_all(sb, "fixture no-manifest run")
    try:
        _mutate(
            gate_in(sb),
            '        echo "FAIL: run dir without manifest: ${id}" >&2\n'
            "        fail=1",
            '        echo "MUTATED: run dir without manifest: ${id}" >&2\n'
            "        fail=0",
            tag="M5R",
        )
    except AssertionError as e:
        fails.append(f"M5R 变异失败: {e}")
    g = gate(sb)
    if g.returncode != 0:
        fails.append(f"M5R 撤掉判据后应变绿，实际 rc={g.returncode} "
                     f"{(g.stdout + g.stderr).strip()[:160]}")
    shutil.rmtree(sb, ignore_errors=True)

    if fails:
        for f in fails:
            print("FIXTURE FAIL:", f)
        return 1
    print("test_audit_manifest_integrity: OK "
          "(CTRL/M1..M6 + 反向变异 M1R/M3R/M4R/M5R all held)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

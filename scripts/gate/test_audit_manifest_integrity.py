#!/usr/bin/env python3
"""`check_audit_manifest.sh` / `audit_run.py verify` 的**反向夹具**（P0-6）。

不是门禁（不叫 check_*，不进 STAGE_CONFIG 阶段清单）。它回答两件事：

1. 新判据是**承重的**：把健康记录的命令行抹掉 ⇒ 必须变红。
2. 新判据**不误伤**：健康记录必须保持绿；`cmd` 换成同义键 `command` 也必须绿
   （判据盯的是"有没有留下命令行"，不是某个拼写）。

夹具自己造一条**合法的** iaia410-manifest/v1 记录（v2 的写入端早已不在仓里），
所有散列用被测工具自己的函数算，避免夹具与被测实现各写一套口径。
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
    return run_dir


def fresh_sandbox() -> pathlib.Path:
    sb = pathlib.Path(tempfile.mkdtemp(prefix="audit_integrity_"))
    (sb / "audit" / "v4.1.0").mkdir(parents=True)
    shutil.copy(TOOL, sb / "audit" / "v4.1.0" / "audit_run.py")
    (sb / "scripts" / "gate").mkdir(parents=True)
    shutil.copy(GATE, sb / "scripts" / "gate" / "check_audit_manifest.sh")
    return sb


def verify(sb: pathlib.Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(sb / "audit" / "v4.1.0" / "audit_run.py"), "verify", "--run-id", RUN_ID],
        cwd=sb, capture_output=True, text=True, timeout=120,
    )


def gate(sb: pathlib.Path) -> subprocess.CompletedProcess:
    return subprocess.run(["bash", "scripts/gate/check_audit_manifest.sh"], cwd=sb,
                          capture_output=True, text=True, timeout=300)


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

    case("CTRL 健康 v2 记录", expect_red=False)
    case("M1 抹掉命令行(cmd)", expect_red=True, drop_cmd=True)
    case("M2 同义键 command 不误伤", expect_red=False, cmd_key="command")
    case("M3 commands 为空", expect_red=True, empty_commands=True)
    case("M4 删掉日志(既有判据仍承重)", expect_red=True, drop_log=True)

    # M5 有目录无清单 ⇒ 门禁必须点名（改前：静默跳过）
    sb = fresh_sandbox()
    build_run(sb, mod)
    (sb / "audit" / "v4.1.0" / "iaia410-fixture-nomanifest").mkdir()
    g = gate(sb)
    if g.returncode == 0 or "without manifest" not in (g.stderr + g.stdout):
        fails.append(f"M5 无清单运行目录未被点名: rc={g.returncode}")
    shutil.rmtree(sb, ignore_errors=True)

    # M6 控制组：整个沙箱里没有运行记录时，门禁不该凭空报错
    sb = fresh_sandbox()
    g = gate(sb)
    if g.returncode != 0:
        fails.append(f"M6 空仓控制组应绿: rc={g.returncode} {(g.stdout + g.stderr).strip()[:120]}")
    shutil.rmtree(sb, ignore_errors=True)

    if fails:
        for f in fails:
            print("FIXTURE FAIL:", f)
        return 1
    print("test_audit_manifest_integrity: OK (CTRL/M1..M6 all held)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

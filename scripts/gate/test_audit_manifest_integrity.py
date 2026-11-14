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
    git_commit(sb)
    return run_dir


LEGACY_NEW_ID = "20261105-lb1-006-mcode"
LEGACY_OLD_ID = "20261105-lb1-006"


def build_legacy_run(
    sb: pathlib.Path,
    tool_mod,
    *,
    with_canonical: bool = True,
    canonical_id: str = LEGACY_OLD_ID,
    run_id: str = LEGACY_NEW_ID,
    tamper_exit: int | None = None,
) -> pathlib.Path:
    """造一条 schema_version=1 的 run：hash 按 OLD id 算，目录/清单用 NEW id。

    这正是 #5698 的真实形态：20261106 撞车处置把目录改名，但 evidence_hash
    仍是按原名算的。`canonical_run_id` 是修复开关。
    """
    import yaml

    run_dir = sb / "audit" / "v4.1.0" / "runs" / run_id
    logs = run_dir / "logs"
    logs.mkdir(parents=True)
    out, err = b"legacy-stdout\n", b""
    (logs / "001-x.stdout").write_bytes(out)
    (logs / "001-x.stderr").write_bytes(err)
    base = {
        "schema_version": 1,
        "run_id": run_id,
        "frozen_commit": "0" * 40,
        "artifacts": {"cargo_lock_sha256": "0" * 64, "dataset_hashes": {}},
        "commands": [{"id": "001", "name": "fixture", "cwd": ".", "timeout_sec": 30,
                      "cmd": "echo legacy-stdout"}],
        "results": [{
            "id": "001", "exit_code": 0,
            "stdout_sha256": _sha(out), "stderr_sha256": _sha(err),
            "stdout_path": "logs/001-x.stdout", "stderr_path": "logs/001-x.stderr",
        }],
    }
    old = dict(base)
    old["run_id"] = canonical_id
    base["evidence_hash"] = tool_mod.evidence_hash_of(old)
    if with_canonical:
        base["canonical_run_id"] = canonical_id
    if tamper_exit is not None:
        # 只在 hash 计算之后改受保护字段 ⇒ verify 必须发现 hash mismatch。
        base["results"][0]["exit_code"] = tamper_exit
    (run_dir / "manifest.yaml").write_text(
        yaml.safe_dump(base, allow_unicode=True, sort_keys=False), encoding="utf-8"
    )
    git_commit(sb)
    return run_dir


def verify_id(sb: pathlib.Path, run_id: str) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(sb / "audit" / "v4.1.0" / "audit_run.py"),
         "verify", "--run-id", run_id],
        cwd=sb, capture_output=True, text=True, timeout=120,
    )


def check_aliases(sb: pathlib.Path) -> subprocess.CompletedProcess:
    return subprocess.run(
        [sys.executable, str(sb / "audit" / "v4.1.0" / "audit_run.py"), "check-aliases"],
        cwd=sb, capture_output=True, text=True, timeout=120,
    )


def git_commit(sb: pathlib.Path) -> None:
    """AMAN-1：门禁枚举的是 **已提交的树**（`git ls-tree HEAD`）。

    旧夹具只建目录、不建 git 提交，于是门禁永远看到空树 ⇒ M5 的
    “无清单目录必须点名”从未真正被判过。这里把沙箱变成真 git 仓库并提交。
    """
    if not (sb / ".git").exists():
        subprocess.run(["git", "init", "-q"], cwd=sb, check=True)
        subprocess.run(["git", "config", "user.email", "fixture@local"], cwd=sb, check=True)
        subprocess.run(["git", "config", "user.name", "fixture"], cwd=sb, check=True)
    subprocess.run(["git", "add", "-A"], cwd=sb, check=True)
    # --allow-empty：M6 的空仓控制组也要有一个真实 HEAD。
    subprocess.run(["git", "commit", "-q", "--allow-empty", "-m", "fixture"], cwd=sb, check=True)


def fresh_sandbox() -> pathlib.Path:
    sb = pathlib.Path(tempfile.mkdtemp(prefix="audit_integrity_"))
    (sb / "audit" / "v4.1.0").mkdir(parents=True)
    shutil.copy(TOOL, sb / "audit" / "v4.1.0" / "audit_run.py")
    (sb / "scripts" / "gate").mkdir(parents=True)
    shutil.copy(GATE, sb / "scripts" / "gate" / "check_audit_manifest.sh")
    alias_src = TOOL.parent / "canonical_run_aliases.json"
    if alias_src.is_file():
        shutil.copy(alias_src, sb / "audit" / "v4.1.0" / "canonical_run_aliases.json")
    git_commit(sb)
    return sb


def write_alias_file(sb: pathlib.Path, entries: list[dict]) -> None:
    (sb / "audit" / "v4.1.0" / "canonical_run_aliases.json").write_text(
        json.dumps({"schema_version": 1, "aliases": entries}, ensure_ascii=False),
        encoding="utf-8",
    )
    git_commit(sb)


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
    nm = sb / "audit" / "v4.1.0" / "iaia410-fixture-nomanifest"
    nm.mkdir()
    (nm / "findings.txt").write_text("no manifest here\n", encoding="utf-8")
    git_commit(sb)
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

    # M7 #5698：改名后的 legacy run 带 canonical_run_id ⇒ 必须重算一致（绿）
    sb = fresh_sandbox()
    build_legacy_run(sb, mod, with_canonical=True)
    r = verify_id(sb, LEGACY_NEW_ID)
    if r.returncode != 0:
        fails.append("M7 canonical_run_id 存在时仍判红: "
                     f"{ (r.stdout + r.stderr).strip()[:160] }")
    shutil.rmtree(sb, ignore_errors=True)

    # M8 #5698 反向臂：同样的改名记录但去掉 canonical_run_id ⇒ 必须判红
    # （证明 M7 的绿来自修复，而不是判据被放松）
    sb = fresh_sandbox()
    build_legacy_run(sb, mod, with_canonical=False)
    r = verify_id(sb, LEGACY_NEW_ID)
    if r.returncode == 0:
        fails.append("M8 去掉 canonical_run_id 后仍判绿 ⇒ canonical 判据失去鉴别力")
    shutil.rmtree(sb, ignore_errors=True)

    # M9：canonical_run_id 未登记（hash 本身能对上）⇒ 必须拒绝，不得成为逃生门
    sb = fresh_sandbox()
    build_legacy_run(sb, mod, with_canonical=True, canonical_id="evil-2026")
    r = verify_id(sb, LEGACY_NEW_ID)
    if r.returncode == 0:
        fails.append("M9 未登记的 canonical_run_id 仍判绿 ⇒ 别名可任意指向")
    shutil.rmtree(sb, ignore_errors=True)

    # M10：两个 run 共用同一个 canonical id ⇒ 登记表本身必须拒绝（身份歧义）
    sb = fresh_sandbox()
    build_legacy_run(sb, mod)
    write_alias_file(sb, [
        {"current_run_id": LEGACY_NEW_ID, "canonical_run_id": LEGACY_OLD_ID},
        {"current_run_id": "20261105-lb1-006-other", "canonical_run_id": LEGACY_OLD_ID},
    ])
    r = verify_id(sb, LEGACY_NEW_ID)
    if r.returncode == 0:
        fails.append("M10 重复 canonical id 仍判绿 ⇒ 身份唯一性未校验")
    shutil.rmtree(sb, ignore_errors=True)

    # M11：只改当前 run_id、不登记 alias ⇒ hash 必须对不上
    sb = fresh_sandbox()
    build_legacy_run(sb, mod, with_canonical=False, run_id="20261105-lb1-006-renamed2")
    r = verify_id(sb, "20261105-lb1-006-renamed2")
    if r.returncode == 0:
        fails.append("M11 改 run_id 未登记 alias 仍判绿")
    shutil.rmtree(sb, ignore_errors=True)

    # M12：alias 合法，但篡改受 hash 保护的 results.exit_code ⇒ 必须红
    sb = fresh_sandbox()
    build_legacy_run(sb, mod, with_canonical=True, tamper_exit=1)
    r = verify_id(sb, LEGACY_NEW_ID)
    if r.returncode == 0:
        fails.append("M12 篡改 results.exit_code 后仍判绿 ⇒ hash 未覆盖受保护字段")
    shutil.rmtree(sb, ignore_errors=True)

    # M13：alias 文件自身有重复 canonical id（即使没有 run 使用）⇒ check-aliases 必须红
    sb = fresh_sandbox()
    build_run(sb, mod)
    write_alias_file(sb, [
        {"current_run_id": "run-a", "canonical_run_id": "same-canonical"},
        {"current_run_id": "run-b", "canonical_run_id": "same-canonical"},
    ])
    r = check_aliases(sb)
    if r.returncode == 0:
        fails.append("M13 重复 canonical 的 alias 文件仍判绿 ⇒ check-aliases 不承重")
    shutil.rmtree(sb, ignore_errors=True)

    if fails:
        for f in fails:
            print("FIXTURE FAIL:", f)
        return 1
    print("test_audit_manifest_integrity: OK (CTRL/M1..M13 all held)")
    return 0


if __name__ == "__main__":
    sys.exit(main())

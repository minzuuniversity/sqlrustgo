#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""反向变异：board_appendix_check.py 的五条判据是不是承重的（AGENTS 2.5）。

每个臂做两件事：
  1) 造一个**现场缺陷**（看板缺附录 / 附录正文被手改 / 字节口径漂了 / 哈希漂了 / API 不通）；
  2) 同一个现场，先跑在仓核对器（必须判红），再跑「把那条判据撤掉」的核对器副本（必须变绿）。
只有「撤掉判据 ⇒ 现场缺陷不再被发现」，那条判据才算真在挡事；否则它就是摆设。
在仓核对器与看板正文都不被改动，副本只在临时目录里活。

退出码：0 = 五臂全部判别成功；1 = 有臂判别失败（判据不承重）；3 = 变异施加不上（源漂了，证据过期）。
  python3 audit/v4.1.0/board_appendix_check_mutations.py
"""
import hashlib
import pathlib
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
CHECKER = HERE / "board_appendix_check.py"
SRC = HERE.parents[1] / "docs/knowledge-loop/handoffs/HANDOFF_2026-11-05_4.1.0_OMP_ONBOARDING.md"
HEADER = "## 附录 A：交接正文全文"
DEAD_URL = "http://127.0.0.1:9"

ARMS = [
    # name, 现场构造器, 撤掉判据的变异 (old, new), 这条判据挡的是什么
    ("m1-appendix-content-drift", "world_content_tampered",
     ('        if src_text[:200] not in sec:', '        if False:'),
     "附录正文被手改/源文件改了但看板没重刷"),
    ("m2-appendix-missing", "world_appendix_missing",
     ('        findings.append("看板 #%s 没有附录 A 段" % BOARD)', '        pass'),
     "看板附录整段缺失"),
    ("m3-byte-metric-drift", "world_bytes_tampered",
     ('                findings.append("字节口径漂移：看板标 %d/%d，仓内实测 %d/%d"\n'
      '                                % (got_file, got_inj, src_bytes, inj_bytes))',
      '                pass'),
     "附录头的字节口径与仓内不一致"),
    ("m4-sha-drift", "world_sha_tampered",
     ('                findings.append("哈希漂移：看板附录 sha256 %s…%s，仓内源文件 %s…%s"\n'
      '                                % (got_sha[:12], got_sha[-6:], sha[:12], sha[-6:]))',
      '                pass'),
     "附录正文哈希与仓内源文件不一致"),
    ("m5-network-must-not-be-green", "world_api_dead",
     ('        print("INCOMPLETE: 读不到 ISSUE 正文（%s）—— 没能核对，不算通过" % type(e).__name__)\n'
      '        return 2',
      '        print("MUTATED: 把没能核对写成通过")\n'
      '        return 0'),
     "API 不可达时必须 INCOMPLETE(rc2)，不许算成通过(rc0)"),
]


def _fixtures(tmp, src_text):
    src = tmp / "src.md"
    src.write_text(src_text, encoding="utf-8")
    hf = tmp / "handoff.md"
    hf.write_text(src_text + "\n\n## 发布元数据\n- fixture\n", encoding="utf-8")
    return src, hf


def _gen_board(src_path):
    sys.path.insert(0, str(HERE))
    import gen_kanban_issue as g
    return g.main(metrics_line="FIXTURE", appendix_path=str(src_path))


def world_content_tampered(tmp, src, hf, src_text):
    body = _gen_board(src).replace("Owner 只要让接手者读", "Owner 只要让接手者细读", 1)
    bf = tmp / "board.md"
    bf.write_text(body, encoding="utf-8")
    return str(src), str(bf), str(hf), []


def world_appendix_missing(tmp, src, hf, src_text):
    body = _gen_board(src).split(HEADER, 1)[0]
    bf = tmp / "board.md"
    bf.write_text(body, encoding="utf-8")
    return str(src), str(bf), str(hf), []


def world_bytes_tampered(tmp, src, hf, src_text):
    body = _gen_board(src).replace("文件 17905 字节", "文件 17900 字节", 1)
    bf = tmp / "board.md"
    bf.write_text(body, encoding="utf-8")
    return str(src), str(bf), str(hf), []


def world_sha_tampered(tmp, src, hf, src_text):
    body = _gen_board(src)
    sha = hashlib.sha256(src_text.strip().encode("utf-8")).hexdigest()
    bad = sha[:-1] + ("0" if sha[-1] != "0" else "1")
    bf = tmp / "board.md"
    bf.write_text(body.replace(sha, bad, 1), encoding="utf-8")
    return str(src), str(bf), str(hf), []


def world_api_dead(tmp, src, hf, src_text):
    return str(src), None, None, ["--base-url", DEAD_URL]


def main() -> int:
    base = CHECKER.read_text(encoding="utf-8")
    src_text = SRC.read_text(encoding="utf-8")
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="board-appx-mut-"))
    not_disc, drifted = [], []
    try:
        for name, world, (old, new), guard in ARMS:
            if base.count(old) != 1:
                drifted.append(name)
                print("DRIFT %s: 变异锚点命中 %d 次 != 1" % (name, base.count(old)))
                continue
            src, hf = _fixtures(tmp, src_text)
            src_arg, board_arg, handoff_arg, extra = globals()[world](tmp, src, hf, src_text)
            mut = tmp / (name + ".py")
            mut.write_text(base.replace(old, new), encoding="utf-8")

            def run(tool):
                cmd = [sys.executable, str(tool), "--src", src_arg]
                if board_arg:
                    cmd += ["--board-file", board_arg, "--handoff-file", handoff_arg]
                cmd += extra
                return subprocess.run(cmd, capture_output=True, text=True, timeout=120)

            real = run(CHECKER)
            mutated = run(mut)
            real_red, mut_green = real.returncode != 0, mutated.returncode == 0
            ok = real_red and mut_green
            print("%s %s: 在仓核对器 rc=%d(应非零) 撤掉判据后 rc=%d(应为零) 挡的是=%s"
                  % ("DISC" if ok else "NOT-DISC", name, real.returncode, mutated.returncode, guard))
            sighting = next((l for l in (real.stdout + "\n" + real.stderr).splitlines()
                             if l.startswith(("NOT-OK", "INCOMPLETE"))), "")
            if sighting:
                print("     首见: " + sighting[:160])
            if not ok:
                not_disc.append(name)
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    n = len(ARMS)
    print("arms=%d discriminating=%d not_discriminating=%d drift=%d"
          % (n, n - len(not_disc) - len(drifted), len(not_disc), len(drifted)))
    if drifted:
        return 3
    return 1 if not_disc else 0


if __name__ == "__main__":
    raise SystemExit(main())

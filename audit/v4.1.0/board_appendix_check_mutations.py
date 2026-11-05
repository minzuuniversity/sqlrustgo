#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""反向变异：board_appendix_check.py 的六条判据是不是承重的（AGENTS 2.5）。

每个臂做两件事：
  1) 造一个**现场缺陷**（缺附录 / 正文被手改 / 字节口径漂 / 哈希漂 / API 不通 / 交接 ISSUE 过期）；
  2) 同一现场先跑在仓核对器（必须判红，且**恰好一条** NOT-OK），
     再跑「把那条判据撤掉」的副本（必须变绿）。
只有「撤掉判据 ⇒ 缺陷不再被发现」，那条判据才算真在挡事。

frozen 口径：判据锚点**自动定位**（只认整行且唯一命中的判据行），命中不了就 exit 3；
**注入空操作**（现场缺陷没改动任何字节）一律记为 drift ⇒ exit 3。
理由见 docs/releases/v4.1.0/HO2_APPENDIX_RULES_2026-11-05.md：
写死的字节数/句子在文档长胖后会静默脱靶，2026-11-05 实测就是这样让一臂形同虚设的。
"""
import hashlib
import pathlib
import re
import shutil
import subprocess
import sys
import tempfile


HERE = pathlib.Path(__file__).resolve().parent
CHECKER = HERE / "board_appendix_check.py"
SRC = HERE.parents[1] / "docs/knowledge-loop/handoffs/HANDOFF_2026-11-05_4.1.0_OMP_ONBOARDING.md"
HEADER = "## 附录 A：交接正文全文"
BULLET_MARK = "- 本节由 `gen_kanban_issue.py --appendix` 原样注入"
DEAD_URL = "http://127.0.0.1:9"

# 换行用 chr(10) 而不是转义字面量：本仓判据脚本常经 heredoc 生成，转义会被吞。
NL = chr(10)

# (name, 现场构造器, 判据行定位关键字, 撤掉判据后替换成什么, 挡的是什么, 该臂应改动的对象)
ARMS = [
    ("m1-appendix-content-drift", "world_content_tampered", "看板附录正文",
     "pass", "附录正文被手改 / 源文件改了但看板没重刷（含只在末尾动手的改动）", "board"),
    ("m2-appendix-missing", "world_appendix_missing", "没有附录 A 段",
     "pass", "看板附录整段缺失", "board"),
    ("m3-byte-metric-drift", "world_bytes_tampered", "字节口径漂移",
     "pass", "附录头声明的字节口径与仓内实测不一致", "board"),
    ("m4-sha-drift", "world_sha_tampered", "哈希漂移",
     "pass", "附录头声明的正文哈希与仓内源文件不一致", "board"),
    ("m5-network-must-not-be-green", "world_api_dead", "读不到 ISSUE 正文",
     "return 0", "API 不可达必须 INCOMPLETE(rc2)，不许算成通过(rc0)", "none"),
    ("m6-handoff-body-stale", "world_handoff_stale", "交接 ISSUE 正文",
     "pass", "交接 ISSUE #5599 正文与仓内源文件分叉", "handoff"),
]


def locate(src_text, key):
    """按关键字找到**唯一一整行**判据，返回 (缩进, 整行)。命不中或多次命中 = 源漂了。"""
    hits = [ln for ln in src_text.split(NL) if key in ln]
    if len(hits) != 1:
        raise SystemExit("ANCHOR-DRIFT: %r 命中 %d 次（应为 1）" % (key, len(hits)))
    line = hits[0]
    return line[:len(line) - len(line.lstrip())], line


def _fixtures(tmp, src_text):
    src = tmp / "src.md"
    src.write_text(src_text, encoding="utf-8")
    hf = tmp / "handoff.md"
    hf.write_text(pristine_hf(src_text), encoding="utf-8")
    return src, hf


def pristine_hf(src_text):
    return src_text.strip() + (NL + NL * 2) + "## 发布元数据" + (NL + "- fixture" + NL)


def _gen_board(src_path):
    sys.path.insert(0, str(HERE))
    import gen_kanban_issue as g
    return g.main(metrics_line="FIXTURE", appendix_path=str(src_path))


def _write_board(tmp, body):
    bf = tmp / "board.md"
    bf.write_text(body, encoding="utf-8")
    return str(bf)


def world_content_tampered(tmp, src, hf, src_text):
    # 在附录正文**末尾**追加一个字。刻意不碰开头，也不写死正文里的句子：
    # 前 200 字口径下这种改动不会被发现，写死的句子改版后会静默脱靶。
    return str(src), _write_board(tmp, _gen_board(src) + "改"), str(hf), []


def world_appendix_missing(tmp, src, hf, src_text):
    return str(src), _write_board(tmp, _gen_board(src).split(HEADER, 1)[0]), str(hf), []



def world_bytes_tampered(tmp, src, hf, src_text):
    # 两个字节数各自减一点。**数字从看板正文里读**，不写死。
    body = _gen_board(src)
    m = re.search("文件 ([0-9]+) 字节｜注入正文 ([0-9]+) 字节", body)
    if not m:
        raise SystemExit("FIXTURE-DRIFT: 看板正文里找不到字节口径")
    bad = "文件 %d 字节｜注入正文 %d 字节" % (int(m.group(1)) - 5, int(m.group(2)) - 3)
    return str(src), _write_board(tmp, body[:m.start()] + bad + body[m.end():]), str(hf), []


def world_sha_tampered(tmp, src, hf, src_text):
    body = _gen_board(src)
    sha = hashlib.sha256(src_text.strip().encode("utf-8")).hexdigest()
    if body.count(sha) != 1:
        raise SystemExit("FIXTURE-DRIFT: 看板正文里哈希命中 %d 次" % body.count(sha))
    bad = sha[:-1] + ("0" if sha[-1] != "0" else "1")
    return str(src), _write_board(tmp, body.replace(sha, bad, 1)), str(hf), []


def world_api_dead(tmp, src, hf, src_text):
    return str(src), None, None, ["--base-url", DEAD_URL]


def world_handoff_stale(tmp, src, hf, src_text):
    # 交接 ISSUE 的正文停在旧版：末尾一字之差就足以说明「看板没重刷」。
    bad = tmp / "handoff_stale.md"
    st = src_text.strip()
    bad.write_text(pristine_hf(st[:-1] + "旧"), encoding="utf-8")
    return str(src), _write_board(tmp, _gen_board(src)), str(bad), []


def main() -> int:
    base = CHECKER.read_text(encoding="utf-8")
    src_text = SRC.read_text(encoding="utf-8")
    tmp = pathlib.Path(tempfile.mkdtemp(prefix="board-appx-mut-"))
    not_disc, drifted = [], []
    try:
        for name, world, key, repl, guard, target in ARMS:
            try:
                indent, anchor = locate(base, key)
            except SystemExit as e:
                drifted.append(name)
                print("DRIFT %s: %s" % (name, e))
                continue
            src, hf = _fixtures(tmp, src_text)
            src_arg, board_arg, handoff_arg, extra = globals()[world](tmp, src, hf, src_text)
            if target == "board" and pathlib.Path(board_arg).read_text(encoding="utf-8") == _gen_board(src):
                drifted.append(name)
                print("DRIFT %s: MUTATION-NOOP 注入没有改动看板任何字节 ⇒ 该臂形同虚设" % name)
                continue
            if target == "handoff" and pathlib.Path(handoff_arg).read_text(encoding="utf-8") == pristine_hf(src_text):
                drifted.append(name)
                print("DRIFT %s: MUTATION-NOOP 注入没有改动交接 ISSUE 任何字节 ⇒ 该臂形同虚设" % name)
                continue
            mut = tmp / (name + ".py")
            mut.write_text(base.replace(anchor, indent + repl), encoding="utf-8")


            def run(tool):
                cmd = [sys.executable, str(tool), "--src", src_arg]
                if board_arg:
                    cmd += ["--board-file", board_arg, "--handoff-file", handoff_arg]
                cmd += extra
                return subprocess.run(cmd, capture_output=True, text=True, timeout=120)

            real = run(CHECKER)
            mutated = run(mut)
            reds = [l for l in (real.stdout + NL + real.stderr).splitlines() if l.startswith("NOT-OK")]
            if board_arg and len(reds) != 1:
                drifted.append(name)
                print("DRIFT %s: 现场只该注入一个缺陷，实际触发 %d 条判据（夹具串味）" % (name, len(reds)))
                continue
            ok = real.returncode != 0 and mutated.returncode == 0
            print("%s %s: 在仓核对器 rc=%d(应非零) 撤掉判据后 rc=%d(应为零) 挡的是=%s"
                  % ("DISC" if ok else "NOT-DISC", name, real.returncode, mutated.returncode, guard))
            if reds:
                print("     首见: " + reds[0][:160])
            if real.returncode == 2:
                print("     首见: " + (real.stdout.strip().splitlines() or [""])[-1][:160])
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

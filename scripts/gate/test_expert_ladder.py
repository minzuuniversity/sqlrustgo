#!/usr/bin/env python3
# scripts/gate/test_expert_ladder.py
#
# 机器核对「咨询专家裁定制度」(docs/governance/EXPERT_ADJUDICATION.md) 的 R1/R2/R3。
#
# 形状遵循 AGENTS §2.5「每条红灯必须做反向变异」：每个 rev_* 用例把某条规则的
# 条件撤掉，断言聚合结果随之改变。若撤掉条件后结果不变，说明该规则是装饰。
#
# 夹具不得触碰真实仓库（BidSystem lesson 18 同源纪律）：证据一律写 tmp 目录，
# 席位一律指向 127.0.0.1 上的假端点，绝不真调 9655/8002/250/252。
#
# Usage: python3 scripts/gate/test_expert_ladder.py
# Exit 0 = 全部核对通过; 非 0 = 至少一条失败（调用方不得写成 PASS）。

from __future__ import annotations

import http.server
import importlib.util
import json
import sys
import tempfile
import threading
import time
import yaml
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
LADDER_PATH = REPO_ROOT / "scripts" / "gate" / "expert_ladder.py"


def load_ladder():
    spec = importlib.util.spec_from_file_location("expert_ladder", LADDER_PATH)
    mod = importlib.util.module_from_spec(spec)
    assert spec and spec.loader
    # Register before exec: @dataclass resolves its owning module via sys.modules
    # (fails on Python 3.14 otherwise).
    sys.modules[spec.name] = mod
    spec.loader.exec_module(mod)
    return mod


# --------------------------------------------------------------------------
# fake seat endpoint
# --------------------------------------------------------------------------
class FakeHub:
    """Configurable OpenAI-ish endpoint. Records max observed concurrency."""

    def __init__(self):
        self.responses: dict[str, dict] = {}
        # prompt-substring -> canned answer, checked before the per-seat table.
        # Lets a fixture distinguish "reachable" from "answers correctly", which is
        # what R4 calibration is about.
        self.prompt_map: list[tuple[str, str]] = []
        self.in_flight = 0
        self.max_in_flight = 0
        self.lock = threading.Lock()
        self.calls: list[str] = []
        outer = self

        class H(http.server.BaseHTTPRequestHandler):
            def log_message(self, *a):  # silence
                pass

            def do_POST(self):
                key = self.path.strip("/")
                n = int(self.headers.get("Content-Length", "0"))
                body = json.loads(self.rfile.read(n) or b"{}")
                with outer.lock:
                    outer.in_flight += 1
                    outer.max_in_flight = max(outer.max_in_flight, outer.in_flight)
                    outer.calls.append(key)
                try:
                    spec = None
                    msgs = body.get("messages") or []
                    last = msgs[-1].get("content", "") if msgs else ""
                    for needle, content in outer.prompt_map:
                        if needle in last:
                            spec = {"content": content}
                            break
                    if spec is None:
                        spec = outer.responses.get(key)
                    if spec is None:
                        self.send_response(404)
                        self.end_headers()
                        self.wfile.write(b"{}")
                        return
                    delay = spec.get("delay", 0)
                    if delay:
                        time.sleep(delay)
                    if spec.get("http_status"):
                        self.send_response(spec["http_status"])
                        self.end_headers()
                        self.wfile.write(b'{"error":"boom"}')
                        return
                    payload = {
                        "choices": [{
                            "message": {"role": "assistant",
                                        "content": spec.get("content", ""),
                                        "reasoning_content": spec.get("reasoning", "")},
                            "finish_reason": spec.get("finish_reason", "stop"),
                        }],
                        "usage": {"completion_tokens": spec.get("ct", 10), "prompt_tokens": 20},
                    }
                    raw = json.dumps(payload).encode()
                    self.send_response(200)
                    self.send_header("Content-Type", "application/json")
                    self.send_header("Content-Length", str(len(raw)))
                    self.end_headers()
                    self.wfile.write(raw)
                finally:
                    with outer.lock:
                        outer.in_flight -= 1

        self.srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), H)
        self.port = self.srv.server_address[1]
        self.thread = threading.Thread(target=self.srv.serve_forever, daemon=True)
        self.thread.start()

    @property
    def base(self) -> str:
        return f"http://127.0.0.1:{self.port}"

    def seat(self, sid: str, family: str, tier: int = 1, roles=("adjudicate",),
             single_session: bool = True, reasoning_effort=None, lineage: str | None = None,
             calibration_min: float = 0.5) -> dict:
        return {
            "id": sid, "family": family, "lineage": lineage or family,
            "endpoint": f"{self.base}/{sid}", "model": f"fake-{sid}",
            "tier": tier, "roles": list(roles), "enabled": True, "max_tokens_default": 128,
            "timeout_s": 10, "single_session": single_session,
            "reasoning_effort": reasoning_effort,
            "calibration_min": calibration_min,
        }

    def set(self, key: str, **spec):
        self.responses[key] = spec

    def stop(self):
        self.srv.shutdown()
        self.srv.server_close()


def seats_yaml(seats: list[dict]) -> Path:
    f = tempfile.NamedTemporaryFile("w", suffix=".yaml", delete=False, encoding="utf-8")
    yaml.safe_dump({"schema_version": "expert-seats/v1", "seats": seats}, f, allow_unicode=True)
    f.close()
    return Path(f.name)


def V(v: str, conf: float = 0.9) -> str:
    return json.dumps({"verdict": v, "confidence": conf, "reason": "fixture"}, ensure_ascii=False)


# --------------------------------------------------------------------------
# checks
# --------------------------------------------------------------------------
FAILURES: list[str] = []


def check(name: str, cond: bool, detail: str = ""):
    print(("  OK   " if cond else "  FAIL ") + name + (f" — {detail}" if detail and not cond else ""))
    if not cond:
        FAILURES.append(name)


def main() -> int:
    L = load_ladder()

    # --- 1. verdict parsing contract --------------------------------------
    print("[1] parse_verdict")
    check("plain json", L.parse_verdict(V("REAL"))["verdict"] == "REAL")
    check("fenced json", L.parse_verdict("prefix\n```json\n" + V("FALSE_POSITIVE") + "\n```")["verdict"] == "FALSE_POSITIVE")
    check("prose-wrapped json", L.parse_verdict("我判断如下: " + V("INCOMPLETE") + " 完毕")["verdict"] == "INCOMPLETE")
    check("garbage => None (abstain)", L.parse_verdict("我觉得这是误报") is None)
    check("wrong enum => None", L.parse_verdict('{"verdict":"PASS","confidence":1}') is None)

    hub = FakeHub()
    tmp = Path(tempfile.mkdtemp(prefix="srg-expert-fixtures-"))
    try:
        s_ds = hub.seat("fake-deepseek", "deepseek")
        s_qw = hub.seat("fake-qwen", "qwen-strata")
        s_qw2 = hub.seat("fake-qwen-9b", "qwen-llamacpp", tier=3, roles=("triage",))
        # second seat in the SAME family as s_ds: probing the same vendor twice
        # must not be counted as independent corroboration (R1).
        s_ds2 = hub.seat("fake-deepseek-2", "deepseek-bridge2", lineage="deepseek")
        reg = seats_yaml([s_ds, s_qw, s_qw2, s_ds2])
        seats = L.load_seats(reg)

        # --- 2. R1 family independence --------------------------------------
        print("[2] R1 family independence")
        hub.set("fake-deepseek", content=V("REAL"))
        hub.set("fake-deepseek-2", content=V("REAL"))
        hub.set("fake-qwen", content=V("REAL"))
        hub.set("fake-qwen-9b", content=V("REAL"))
        recs = [L.ask(seats["fake-deepseek"], "q"), L.ask(seats["fake-deepseek-2"], "q")]
        agg = L.aggregate(recs, required_lineages=2)
        check("two same-lineage seats must NOT constitute independent corroboration",
              agg["aggregate"] == "INCOMPLETE" and agg["rule"].startswith("R1"), json.dumps(agg["rule"]))
        # reverse mutation of R1: relax the requirement -> the block disappears
        agg_mut = L.aggregate(recs, required_lineages=1)
        check("rev_R1: relaxing family count changes the outcome",
              agg_mut["aggregate"] == "REAL", json.dumps(agg_mut["rule"]))
        agg2 = L.aggregate([L.ask(seats["fake-deepseek"], "q"), L.ask(seats["fake-qwen"], "q")],
                           required_lineages=2)
        check("two distinct families agree REAL => REAL",
              agg2["aggregate"] == "REAL", json.dumps(agg2["rule"]))
        # strata vs llama.cpp: two different serving stacks, SAME lineage (qwen).
        s_str = hub.seat("fake-qwen-strata2", "qwen-strata", lineage="qwen")
        s_llc = hub.seat("fake-qwen-llamacpp2", "qwen-llamacpp", lineage="qwen")
        hub.set("fake-qwen-strata2", content=V("REAL"))
        hub.set("fake-qwen-llamacpp2", content=V("REAL"))
        agg3 = L.aggregate([L.ask(L.Seat(**s_str), "q"), L.ask(L.Seat(**s_llc), "q")],
                           required_lineages=2)
        check("two qwen serving stacks must NOT count as two independent experts",
              agg3["aggregate"] == "INCOMPLETE", agg3["rule"])

        # --- 3. R2 abstention cannot be outvoted -----------------------------
        print("[3] R2 abstention")
        hub.set("fake-qwen", content=V("INCOMPLETE"))
        agg = L.aggregate([L.ask(seats["fake-deepseek"], "q"), L.ask(seats["fake-qwen"], "q")],
                          required_lineages=2)
        check("a seat voting INCOMPLETE blocks the aggregate",
              agg["aggregate"] == "INCOMPLETE" and agg["rule"].startswith("R2"), agg["rule"])
        # reverse mutation: drop the abstaining seat -> conclusion reappears
        agg_mut = L.aggregate([L.ask(seats["fake-deepseek"], "q")], required_lineages=1)
        check("rev_R2: removing the abstaining seat restores a conclusion",
              agg_mut["aggregate"] == "REAL", agg_mut["rule"])
        # transport failure counts as abstention, never as agreement
        dead = L.Seat(id="dead", family="ghost", endpoint="http://127.0.0.1:1/v1/chat/completions",
                      model="x", tier=1, roles=["adjudicate"], timeout_s=3)
        agg = L.aggregate([L.ask(seats["fake-deepseek"], "q"), L.ask(dead, "q")], required_lineages=2)
        check("transport fault => abstention => INCOMPLETE (never REAL/FALSE_POSITIVE)",
              agg["aggregate"] == "INCOMPLETE", json.dumps(agg["abstained"]))
        # unparseable output = abstention
        hub.set("fake-qwen", content="这是一大段中文分析但没有 JSON")
        agg = L.aggregate([L.ask(seats["fake-deepseek"], "q"), L.ask(seats["fake-qwen"], "q")],
                          required_lineages=2)
        check("unparseable verdict => INCOMPLETE", agg["aggregate"] == "INCOMPLETE")
        # thinking model with exhausted budget: empty content + finish=length
        hub.set("fake-qwen", content="", finish_reason="length", reasoning="x" * 500, ct=200)
        rec = L.ask(seats["fake-qwen"], "q")
        check("empty content w/ finish_reason=length is an abstention",
              rec["ok"] is False and "thinking budget" in (rec.get("error") or ""), rec.get("error", ""))
        # reverse mutation: same payload but budget finished normally => still abstention (empty is empty)
        hub.set("fake-qwen", content="", finish_reason="stop", ct=5)
        check("empty content w/ finish_reason=stop is also an abstention",
              L.ask(seats["fake-qwen"], "q")["ok"] is False)
        # EX-7: a bridge can answer HTTP 200 / finish_reason=stop while the page body
        # is still a UI error placeholder ("无法加载此回复"). That is an abstention.
        hub.set("fake-qwen", content="无法加载此回复", finish_reason="stop", ct=9)
        rec = L.ask(seats["fake-qwen"], "q")
        check("ui-placeholder body w/ finish_reason=stop is an abstention",
              rec["ok"] is False and (rec.get("error") or "").startswith("placeholder-content"),
              json.dumps({"ok": rec["ok"], "error": rec.get("error")}))
        # near-miss placeholder variants are caught too
        hub.set("fake-qwen", content="Oops! Something went wrong.", finish_reason="stop")
        check("english placeholder variant is an abstention",
              L.ask(seats["fake-qwen"], "q")["ok"] is False)
        # reverse mutation of EX-7: a LONG answer that merely *mentions* the phrase
        # must stay ok=true, proving the detector is not permanently red.
        long_answer = V("REAL") + " 说明：此前一次调用曾返回「无法加载此回复」，本轮已重试成功，" \
                      "以下是基于证据的结论，并给出复现命令与反例夹具路径以备核对。"
        hub.set("fake-qwen", content=long_answer, finish_reason="stop")
        rec = L.ask(seats["fake-qwen"], "q")
        check("CONTROL: long answer quoting the placeholder stays ok",
              rec["ok"] is True, json.dumps({"ok": rec["ok"], "error": rec.get("error")}))

        # --- 4. R3 false-positive needs machine evidence ---------------------
        print("[4] R3 false-positive requires a passing reverse-mutation fixture")
        hub.set("fake-deepseek", content=V("FALSE_POSITIVE"))
        hub.set("fake-qwen", content=V("FALSE_POSITIVE"))
        pair = [L.ask(seats["fake-deepseek"], "q"), L.ask(seats["fake-qwen"], "q")]
        agg = L.aggregate(pair, required_lineages=2, machine_evidence=None)
        check("unanimous FALSE_POSITIVE without fixture must NOT greenlight the gate",
              agg["aggregate"] == "REAL" and agg.get("substate") == "FALSE_POSITIVE_UNVERIFIED", agg["rule"])
        agg = L.aggregate(pair, required_lineages=2,
                          machine_evidence={"reverse_mutation_passed": False})
        check("a FAILING fixture does not count as machine evidence",
              agg["aggregate"] == "REAL", agg["rule"])
        # reverse mutation of R3: supply a PASSING fixture -> FALSE_POSITIVE becomes admissible
        agg_mut = L.aggregate(pair, required_lineages=2,
                             machine_evidence={"reverse_mutation_passed": True})
        check("rev_R3: passing fixture flips the outcome",
              agg_mut["aggregate"] == "FALSE_POSITIVE", agg_mut["rule"])

        # disagreement => INCOMPLETE (needs arbitration)
        hub.set("fake-qwen", content=V("REAL"))
        agg = L.aggregate([L.ask(seats["fake-deepseek"], "q"), L.ask(seats["fake-qwen"], "q")],
                          required_lineages=2)
        check("seats disagree => INCOMPLETE", agg["aggregate"] == "INCOMPLETE", agg["rule"])

        # --- 5. single-session lock ------------------------------------------
        print("[5] single-session serialization")
        hub.set("fake-slow-a", content=V("REAL"), delay=0.4)
        hub.set("fake-slow-b", content=V("REAL"), delay=0.4)
        sa = hub.seat("fake-slow-a", "famA")
        sb = hub.seat("fake-slow-b", "famB")
        sa, sb = L.Seat(**sa), L.Seat(**sb)
        hub.max_in_flight = 0
        ts = [threading.Thread(target=lambda: L.ask(sa, "q")) for _ in range(4)]
        [t.start() for t in ts]
        [t.join() for t in ts]
        check("same endpoint is serialized (max concurrency == 1)",
              hub.max_in_flight == 1, f"max_in_flight={hub.max_in_flight}")
        # reverse mutation: two distinct endpoints must NOT be globally serialized
        hub.max_in_flight = 0
        ts = [threading.Thread(target=lambda s=sb: L.ask(s, "q")) for _ in range(2)]
        ts += [threading.Thread(target=lambda s=sa: L.ask(s, "q")) for _ in range(2)]
        [t.start() for t in ts]
        [t.join() for t in ts]
        check("rev_LOCK: distinct endpoints run in parallel (not a global mutex)",
              hub.max_in_flight >= 2, f"max_in_flight={hub.max_in_flight}")

        # --- 6. evidence manifest (AFP 5 fields + tamper detection) ----------
        print("[6] evidence contract")
        ev = L.RunEvidence("fixture", base=tmp)
        ev.append("asks", {"seat": "fake", "content": "x"})
        d = ev.finalize(extra={"kind": "fixture"})
        man = (d / "MANIFEST.yaml").read_text(encoding="utf-8")
        for field in ("source_agent", "source_run", "timestamp_started", "conflict_resolution"):
            check(f"MANIFEST carries {field}", field in man)
        check("artifacts hashed", "asks.jsonl" in man and "sha256:" in man)
        before = yaml.safe_load(man)["artifacts"]["asks.jsonl"]["sha256"]
        (d / "asks.jsonl").write_text('{\"seat\":\"fake\",\"content\":\"tampered\"}\n', encoding="utf-8")
        after = L.sha256_file(d / "asks.jsonl")
        check("tampering an artifact is detectable via MANIFEST hash", before != after,
              f"{before} == {after}")

        # --- 7. aggregate never emits PASS ----------------------------------
        print("[7] no PASS vocabulary")
        hub.set("fake-deepseek", content='{"verdict":"PASS","confidence":1}')
        r = L.ask(seats["fake-deepseek"], "q")
        check("a seat claiming PASS is rejected (PASS is not a legal verdict)",
              L.parse_verdict(r.get("content", "")) is None)
        for bad in ("PASS", "pass", "DONE"):
            a = L.aggregate([{"seat": "x", "family": "f", "ok": True, "content": bad}])
            check(f"aggregate refuses to conclude on vocabulary {bad!r}",
                  a["aggregate"] == "INCOMPLETE")

        # --- 8. R4 calibration: reachable != trustworthy --------------------
        print("[8] R4 seat calibration")
        hub.prompt_map = [("17*23", "391"),
                          ('{"verdict":"INCOMPLETE"', V("INCOMPLETE")),
                          ("exited with code 2", '{"verdict":"NOT_TRUSTWORTHY"}'),
                          ("len([x for x in range(10)", "5")]
        good = seats_yaml([hub.seat("cal-good", "deepseek")])
        cal_good = L.run_calibration(L.load_seats(good)["cal-good"], lock_wait_s=5)
        check("a correct seat calibrates 4/4", cal_good["ratio"] == 1.0,
              json.dumps(cal_good["items"], ensure_ascii=False)[:200])
        # reverse mutation of R4: a confidently wrong seat must score zero,
        # i.e. the scorer really discriminates instead of always returning high.
        hub.prompt_map = [("17*23", "390"),
                          ('{"verdict":"INCOMPLETE"', "not json at all"),
                          ("exited with code 2", '{"verdict":"TRUSTWORTHY"}'),
                          ("len([x for x in range(10)", "9")]
        cal_bad = L.run_calibration(L.load_seats(good)["cal-good"], lock_wait_s=5)
        check("rev_R4: a confidently wrong seat scores 0/4",
              cal_bad["ratio"] == 0.0, json.dumps(cal_bad["items"], ensure_ascii=False)[:240])
        # --- 8b. EX-6: empty answer != wrong answer (transport vs competence) ---
        hub.prompt_map = [("17*23", "391"),
                          ('{"verdict":"INCOMPLETE"', V("INCOMPLETE")),
                          ("exited with code 2", '{"verdict":"NOT_TRUSTWORTHY"}'),
                          ("len([x for x in range(10)", "")]
        cal_empty = L.run_calibration(L.load_seats(good)["cal-good"], lock_wait_s=5)
        check("an empty answer is booked as unanswered, not wrong",
              cal_empty["unanswered"] == ["code-truth-len"] and cal_empty["wrong"] == []
              and cal_empty["answered"] == 3,
              json.dumps({k: cal_empty[k] for k in ("answered", "unanswered", "wrong")}))
        check("credit math stays fail-closed (3/4, not 3/3)",
              cal_empty["ratio"] == 0.75 and cal_empty["score"] == 3)
        check("competence is reported separately from connectivity",
              cal_empty["competence_ratio"] == 1.0,
              "competence_ratio=%s" % cal_empty["competence_ratio"])
        hub.prompt_map = [("17*23", "391"),
                          ('{"verdict":"INCOMPLETE"', V("INCOMPLETE")),
                          ("exited with code 2", '{"verdict":"NOT_TRUSTWORTHY"}'),
                          ("len([x for x in range(10)", "5")]
        cal_fixed = L.run_calibration(L.load_seats(good)["cal-good"], lock_wait_s=5)
        check("rev_EX6: filling that one item in clears unanswered (the flag discriminates)",
              cal_fixed["unanswered"] == [] and cal_fixed["ratio"] == 1.0,
              json.dumps({k: cal_fixed[k] for k in ("answered", "unanswered", "ratio")}))
        check("retry happens before an item is called unanswered",
              all(d["retried"] for d in cal_empty["items"] if not d["answered"]),
              json.dumps([d["retried"] for d in cal_empty["items"]]))

        rows_ok = [
            {"seat": "ds", "lineage": "deepseek", "roles": ["adjudicate"],
             "ok": True, "parseable_contract": True, "calibrated": True},
            {"seat": "qw", "lineage": "qwen", "roles": ["adjudicate"],
             "ok": True, "parseable_contract": True, "calibrated": True},
        ]
        s_ok = L.summarize_health(rows_ok, required_lineages=2, calibrate=True)
        check("two calibrated distinct lineages => capable", s_ok["adjudication_capable"])
        rows_no = [dict(r, calibrated=False) for r in rows_ok]
        s_no = L.summarize_health(rows_no, required_lineages=2, calibrate=True)
        check("same seats un-calibrated => NOT capable", not s_no["adjudication_capable"])
        # A seat that failed calibration stays disqualified even when the CLI
        # is run in transport-only mode; only *un-probed* seats default in.
        rows_unprobed = [{k: v for k, v in r.items() if k != "calibrated"} for r in rows_ok]
        s_failclosed = L.summarize_health(rows_no, required_lineages=2, calibrate=False)
        check("a seat that failed calibration stays disqualified in transport-only mode",
              not s_failclosed["adjudication_capable"])
        s_rev = L.summarize_health(rows_unprobed, required_lineages=2, calibrate=False)
        check("rev_R4: dropping calibration flips capability (calibration is load-bearing)",
              s_rev["adjudication_capable"])
        check("weaker mode is labelled, not silently reported as trust",
              "NOT trust" in s_rev["capability_basis"])
        rows_triage = [
            {"seat": "qw-triage", "lineage": "qwen", "roles": ["triage"],
             "ok": True, "parseable_contract": True, "calibrated": True},
        ] + [dict(r, calibrated=False) for r in rows_ok]
        s_tri = L.summarize_health(rows_triage, required_lineages=2, calibrate=True)
        check("a triage-only seat is never credited for R1 lineages",
              s_tri["lineages_credited"] == [], json.dumps(s_tri["lineages_credited"]))
        check("reachable lineages are still listed separately (transport truth)",
              s_tri["lineages_reachable"] == ["deepseek", "qwen"])
    finally:
        hub.stop()

    print()
    if FAILURES:
        print(f"FAILED {len(FAILURES)}: " + ", ".join(FAILURES))
        return 1
    print("ALL EXPERT-LADDER CHECKS PASS (fixture run, no real seat contacted)")
    return 0


if __name__ == "__main__":
    import sys
    sys.exit(main())

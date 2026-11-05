#!/usr/bin/env python3
# scripts/gate/expert_ladder.py
#
# 咨询专家梯 (Expert Ladder) —— 本机可用大模型的分级调用与**失败关闭**裁定聚合。
#
# 设计前提（来自本仓 governance，不是发明）：
#   - AFP §5.1/5.3：AI 生成的结论是 **Soft Gate**，无决策权威，"只有 Hard Gate 可以决定
#     pass/fail"。因此本脚本**永远不把模型意见写成 PASS**；它只产出带 provenance 的
#     **建议**，并把"误报"结论压给机器夹具（反向变异）去判定。
#   - AFP §5.4 / ADR-014 D-06：每条 claim 必须带
#     source_agent + source_run + timestamp + evidence_hash，引用链每跳降一级置信。
#   - AGENTS §8-5 (ADR-008)：缺位/超时/弃权 **不得**计 PASS ⇒ 弃权即 INCOMPLETE。
#
# 三条硬规矩（可被 gate/tests/test_expert_ladder.py 机器核对）：
#   R1 家族独立：裁定至少 2 个**不同模型家族**的有效席位；不足 ⇒ INCOMPLETE。
#   R2 弃权不可被否决：任一有效席位返回 INCOMPLETE ⇒ 聚合结果 INCOMPLETE。
#   R3 误报需机器证据：全体判 FALSE_POSITIVE 时，若没有通过的反向变异机器证据，
#      聚合结论仍是 REAL（记 FALSE_POSITIVE_UNVERIFIED），模型意见不能把红门禁变绿。
#
# 单会话机制：本地 llama.cpp / strata 都是单会话服务，故每个端点用文件锁串行化，
# 避免与 omp / hermes 的在用请求互相踩（实测 250:8011 因此出现过 120s 超时）。
#
# 用法:
#   python3 scripts/gate/expert_ladder.py health                 # 实测席位能力矩阵
#   python3 scripts/gate/expert_ladder.py ask --seat <id> -m "..."
#   python3 scripts/gate/expert_ladder.py adjudicate --id <qid> --question <file> \
#          [--machine-evidence <file.json>] [--required-families 2]
#
# 退出码: 0 = 聚合结论为 REAL 或 FALSE_POSITIVE（两者都是"有结论"）
#         2 = INCOMPLETE（席位不足/弃权/超时/解析失败）
#         1 = 用法或配置错误
#   注意：REAL/FALSE_POSITIVE 都不是"门禁通过"，门禁权威只来自机器判据。

from __future__ import annotations

import argparse
import contextlib
import fcntl
import hashlib
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request
import uuid
from dataclasses import dataclass, field, asdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parents[2]
SEATS_YAML = REPO_ROOT / "docs" / "governance" / "EXPERT_SEATS.yaml"
AGENT_ID = os.environ.get("SQLRUSTGO_AGENT_ID", "mcode-macmini")

VALID_VERDICTS = ("REAL", "FALSE_POSITIVE", "INCOMPLETE")


def utcnow() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def sha256_text(s: str) -> str:
    return "sha256:" + hashlib.sha256(s.encode("utf-8")).hexdigest()


def sha256_file(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


# --------------------------------------------------------------------------
# seat registry
# --------------------------------------------------------------------------
@dataclass
class Seat:
    id: str
    family: str
    endpoint: str
    model: str
    tier: int
    roles: list[str]
    # lineage = model lineage/vendor (deepseek|qwen|gpt). Defaults to family when unset.
    # R1 independence counts lineages, so two stacks of one lineage cannot corroborate.
    lineage: str = ""
    enabled: bool = True
    max_tokens_default: int = 512
    timeout_s: int = 120
    reasoning_effort: str | None = None
    single_session: bool = True
    calibration_min: float = 0.5
    note: str = ""

    @property
    def lock_path(self) -> Path:
        hostport = re.sub(r"[^A-Za-z0-9._-]", "_", self.endpoint.split("://", 1)[-1])
        return Path("/tmp") / f"sqlrustgo-expert-{hostport}.lock"


def load_seats(path: Path = SEATS_YAML) -> dict[str, Seat]:
    import yaml  # repo already depends on pyyaml tooling

    raw = yaml.safe_load(path.read_text(encoding="utf-8"))
    seats: dict[str, Seat] = {}
    for s in raw.get("seats", []):
        seat = Seat(
            id=s["id"],
            family=s["family"],
            lineage=(s.get("lineage") or s["family"]),
            endpoint=s["endpoint"],
            model=s["model"],
            tier=int(s.get("tier", 9)),
            roles=list(s.get("roles", [])),
            enabled=bool(s.get("enabled", True)),
            max_tokens_default=int(s.get("max_tokens_default", 512)),
            timeout_s=int(s.get("timeout_s", 120)),
            reasoning_effort=s.get("reasoning_effort"),
            single_session=bool(s.get("single_session", True)),
            calibration_min=float(s.get("calibration_min", 0.5)),
            note=s.get("note", ""),
        )
        seats[seat.id] = seat
    return seats


def seats_for_role(seats: dict[str, Seat], role: str) -> list[Seat]:
    return sorted(
        [s for s in seats.values() if s.enabled and role in s.roles],
        key=lambda s: (s.tier, s.id),
    )


# --------------------------------------------------------------------------
# transport
# --------------------------------------------------------------------------
@contextlib.contextmanager
def seat_lock(seat: Seat, max_wait_s: int = 90):
    """Serialize calls to a single-session endpoint."""
    seat.lock_path.parent.mkdir(parents=True, exist_ok=True)
    with open(seat.lock_path, "a+", encoding="utf-8") as fh:
        deadline = time.time() + max_wait_s
        while True:
            try:
                fcntl.flock(fh.fileno(), fcntl.LOCK_EX | fcntl.LOCK_NB)
                break
            except BlockingIOError:
                if time.time() > deadline:
                    raise TimeoutError(f"seat {seat.id} busy: lock held > {max_wait_s}s")
                time.sleep(2.0)
        try:
            yield
        finally:
            fcntl.flock(fh.fileno(), fcntl.LOCK_UN)


PLACEHOLDER_MARKERS = (
    "无法加载此回复",
    "无法回答该问题",
    "出了点问题",
    "服务暂时不可用",
    "couldn't load this response",
    "failed to load this response",
    "oops! something went wrong",
)
PLACEHOLDER_MAX_CHARS = 64


def placeholder_content(content):
    """Return a reason string when content is a UI error placeholder, else None.

    A bridge can answer HTTP 200 with finish_reason=stop while the page body is
    still an error placeholder. Length guard: a genuine answer may *discuss* one
    of these phrases, so only short replies count as placeholders.
    """
    text = (content or "").strip()
    if not text or len(text) > PLACEHOLDER_MAX_CHARS:
        return None
    low = text.lower()
    for marker in PLACEHOLDER_MARKERS:
        m = marker.lower()
        if low == m or low.startswith(m) or (m in low and len(low) <= len(m) + 24):
            return f"placeholder-content: {marker}"
    return None


def http_chat(
    seat: Seat,
    prompt: str,
    max_tokens: int | None = None,
    temperature: float = 0.1,
    timeout_s: int | None = None,
    system: str | None = None,
) -> dict[str, Any]:
    """POST /v1/chat/completions. Returns a record dict; never raises for network faults."""
    mt = max_tokens or seat.max_tokens_default
    to = timeout_s or seat.timeout_s
    msgs = []
    if system:
        msgs.append({"role": "system", "content": system})
    msgs.append({"role": "user", "content": prompt})
    payload: dict[str, Any] = {
        "model": seat.model,
        "messages": msgs,
        "max_tokens": mt,
        "temperature": temperature,
    }
    if seat.reasoning_effort:
        payload["reasoning_effort"] = seat.reasoning_effort
    req = urllib.request.Request(
        seat.endpoint,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    t0 = time.time()
    rec: dict[str, Any] = {
        "seat": seat.id,
        "family": seat.family,
        "lineage": seat.lineage or seat.family,
        "endpoint": seat.endpoint,
        "model": seat.model,
        "requested_at": utcnow(),
        "prompt_sha256": sha256_text(prompt),
        "max_tokens": mt,
    }
    try:
        with urllib.request.urlopen(req, timeout=to) as resp:
            data = json.loads(resp.read().decode("utf-8"))
    except Exception as exc:  # noqa: BLE001 - transport faults become abstentions, not passes
        rec.update(ok=False, error=f"{type(exc).__name__}: {exc}"[:300], latency_s=round(time.time() - t0, 2))
        return rec
    latency = time.time() - t0
    try:
        choice = data["choices"][0]
        msg = choice.get("message") or {}
        content = (msg.get("content") or "").strip()
        reasoning = (msg.get("reasoning_content") or "").strip()
    except Exception as exc:  # noqa: BLE001
        rec.update(ok=False, error=f"malformed response: {exc}"[:200], latency_s=round(latency, 2))
        return rec
    usage = data.get("usage") or {}
    # Thinking models (qwen35-9b / 35b-a3b) can exhaust the budget inside reasoning_content
    # and leave content empty. Treat that as an abstention, never as an empty answer.
    truncated_empty = bool(content == "" and choice.get("finish_reason") == "length")
    placeholder = placeholder_content(content)
    rec.update(
        ok=bool(content) and not truncated_empty and placeholder is None,
        content=content,
        reasoning_chars=len(reasoning),
        finish_reason=choice.get("finish_reason"),
        completion_tokens=usage.get("completion_tokens"),
        prompt_tokens=usage.get("prompt_tokens"),
        latency_s=round(latency, 2),
        output_sha256=sha256_text(content) if content else None,
    )
    if truncated_empty:
        rec["error"] = "empty content with finish_reason=length (thinking budget exhausted)"
    if not content and not truncated_empty:
        rec["error"] = "empty content"
    if placeholder:
        rec["error"] = placeholder
    return rec


def ask(seat: Seat, prompt: str, **kw) -> dict[str, Any]:
    try:
        with seat_lock(seat, max_wait_s=kw.pop("lock_wait_s", 90)):
            return http_chat(seat, prompt, **kw)
    except TimeoutError as exc:
        return {
            "seat": seat.id,
            "family": seat.family,
            "endpoint": seat.endpoint,
            "model": seat.model,
            "requested_at": utcnow(),
            "ok": False,
            "error": str(exc),
            "lineage": seat.lineage or seat.family,
            "prompt_sha256": sha256_text(prompt),
        }


# --------------------------------------------------------------------------
# verdict parsing (strict; unparseable == abstention)
# --------------------------------------------------------------------------
_JSON_RE = re.compile(r"\{.*\}", re.S)


CALIBRATION_ITEMS: list[dict[str, Any]] = [
    {
        "id": "arith-17x23",
        "prompt": "Compute 17*23. Reply with only the number, nothing else.",
        "expect": lambda t: "391" in t and "390" not in t and "392" not in t,
    },
    {
        "id": "json-contract",
        "prompt": 'Reply with exactly this JSON and no prose: {"verdict":"INCOMPLETE","confidence":0,"reason":"probe"}',
        "expect": lambda t: parse_verdict(t) is not None,
    },
    {
        "id": "gate-trust-two-state",
        "prompt": (
            "A gate runner exited with code 2, which means INCOMPLETE. The report line "
            "for that gate says PASS. Is the report trustworthy? Reply with a single JSON "
            "object whose verdict field is either TRUSTWORTHY or NOT_TRUSTWORTHY."
        ),
        "expect": lambda t: "NOT_TRUSTWORTHY" in t.upper(),
    },
    {
        "id": "code-truth-len",
        "prompt": "What number does this print? print(len([x for x in range(10) if x%2])) Reply with only the number.",
        "expect": lambda t: "5" in t and not any(x in t for x in ("4", "6", "10")),
    },
]


def run_calibration(seat: Seat, **kw) -> dict[str, Any]:
    """R4 seat calibration. A reachable-but-wrong seat is noise, not corroboration.

    A seat only counts toward adjudication when it answers ground-truth probes
    correctly. This is what stops a second lineage from being a lie amplifier
    instead of an independent check.
    """
    got = 0
    detail = []
    for item in CALIBRATION_ITEMS:
        rec = ask(seat, item["prompt"], max_tokens=64, **kw)
        text = rec.get("content") or ""
        try:
            ok = bool(item["expect"](text))
        except Exception:
            ok = False
        if not rec.get("ok"):
            ok = False
        detail.append({"id": item["id"], "ok": ok, "answer": text[:80].replace(chr(10), " ")})
        got += 1 if ok else 0
    total = len(CALIBRATION_ITEMS)
    ratio = round(got / total, 3) if total else 0.0
    return {"score": got, "total": total, "ratio": ratio, "items": detail}


def parse_verdict(text: str) -> dict[str, Any] | None:
    if not text:
        return None
    candidate = text.strip()
    fence = re.search(r"```(?:json)?\s*(.*?)```", candidate, re.S)
    if fence:
        candidate = fence.group(1).strip()
    for chunk in (candidate,) + tuple(_JSON_RE.findall(candidate)):
        try:
            obj = json.loads(chunk)
        except Exception:
            continue
        if not isinstance(obj, dict):
            continue
        v = str(obj.get("verdict", "")).strip().upper()
        if v not in VALID_VERDICTS:
            continue
        try:
            conf = float(obj.get("confidence", 0.0))
        except Exception:
            conf = 0.0
        return {
            "verdict": v,
            "confidence": round(max(0.0, min(1.0, conf)), 3),
            "reason": str(obj.get("reason", ""))[:240],
        }
    return None


# --------------------------------------------------------------------------
# fail-closed aggregation  (R1 / R2 / R3)
# --------------------------------------------------------------------------
def aggregate(
    seat_records: list[dict[str, Any]],
    required_lineages: int = 2,
    machine_evidence: dict[str, Any] | None = None,
) -> dict[str, Any]:
    """Machine-decided aggregation of expert opinions. Fail-closed by construction."""
    # Independence is measured by **lineage** (model lineage / vendor), not by serving
    # stack. qwen-strata and qwen-llamacpp are different stacks but the same lineage,
    # so they must not corroborate each other (that would be self-sign-off, ADR-014 D-06).
    decided: list[dict[str, Any]] = []
    abstained: list[dict[str, Any]] = []
    for rec in seat_records:
        if not rec.get("ok"):
            abstained.append({"seat": rec.get("seat"), "why": rec.get("error", "transport fault")})
            continue
        parsed = parse_verdict(rec.get("content", ""))
        if parsed is None:
            abstained.append({"seat": rec.get("seat"), "why": "unparseable verdict"})
            continue
        decided.append({**rec, **{"parsed": parsed}})

    families = sorted({d["family"] for d in decided})
    lineages = sorted({d.get("lineage") or d["family"] for d in decided})
    verdicts = {d["seat"]: d["parsed"]["verdict"] for d in decided}
    machine_ok = bool(machine_evidence and machine_evidence.get("reverse_mutation_passed") is True)

    out: dict[str, Any] = {
        "decided_seats": sorted(verdicts),
        "verdicts": verdicts,
        "abstained": abstained,
        "families": families,
        "lineages": lineages,
        "required_lineages": required_lineages,
        "machine_evidence_present": bool(machine_evidence),
        "machine_evidence_passed": machine_ok,
        "soft_gate_note": "[SOFT GATE] 模型意见仅供参考，不作为门禁依据（AFP §5.1/5.3）",
    }

    # R2 first: an abstaining *decided* seat (INCOMPLETE) can never be outvoted.
    if any(v == "INCOMPLETE" for v in verdicts.values()):
        out["aggregate"] = "INCOMPLETE"
        out["rule"] = "R2: a seat abstained inside the verdict set"
        return out

    # R1: lineage independence
    if len(lineages) < required_lineages:
        out["aggregate"] = "INCOMPLETE"
        out["rule"] = (f"R1: only {len(lineages)} independent lineage(s) {lineages} "
                       f"< required {required_lineages}")
        return out

    distinct = set(verdicts.values())
    if distinct == {"REAL"}:
        out["aggregate"] = "REAL"
        out["rule"] = "all independent seats agree REAL"
        return out

    if distinct == {"FALSE_POSITIVE"}:
        if machine_ok:
            out["aggregate"] = "FALSE_POSITIVE"
            out["rule"] = "R3 satisfied: unanimous FALSE_POSITIVE + passing reverse-mutation fixture"
            return out
        # R3: opinion alone cannot greenlight a red gate.
        out["aggregate"] = "REAL"
        out["rule"] = "R3: unanimous FALSE_POSITIVE but no passing reverse-mutation fixture -> stays REAL"
        out["substate"] = "FALSE_POSITIVE_UNVERIFIED"
        return out

    out["aggregate"] = "INCOMPLETE"
    out["rule"] = "seats disagree; needs arbitration (conflict_resolution required)"
    return out


# --------------------------------------------------------------------------
# evidence writer
# --------------------------------------------------------------------------
class RunEvidence:
    def __init__(self, kind: str, run_id: str | None = None, base: Path | None = None):
        self.ts = datetime.now(timezone.utc)
        self.run_id = run_id or f"{kind}-{self.ts.strftime('%Y%m%dT%H%M%SZ')}-{AGENT_ID}-{uuid.uuid4().hex[:6]}"
        self.dir = (base or (REPO_ROOT / "evidence" / "expert-runs")) / self.run_id
        self.dir.mkdir(parents=True, exist_ok=True)
        self.started_at = self.ts.strftime("%Y-%m-%dT%H:%M:%SZ")
        self.records: dict[str, list[dict[str, Any]]] = {}

    def append(self, name: str, rec: dict[str, Any]) -> None:
        self.records.setdefault(name, []).append(rec)
        with open(self.dir / f"{name}.jsonl", "a", encoding="utf-8") as fh:
            fh.write(json.dumps(rec, ensure_ascii=False) + "\n")

    def finalize(self, extra: dict[str, Any] | None = None) -> Path:
        manifest: dict[str, Any] = {
            "schema_version": "expert-manifest/v1",
            "run_id": self.run_id,
            "source_agent": AGENT_ID,
            "source_run": f"{AGENT_ID}-{self.started_at}",
            "timestamp_started": self.started_at,
            "timestamp_finished": utcnow(),
            "conflict_resolution": "none",
            "repository": {
                "canonical_remote": "http://192.168.0.252:3000/openclaw/sqlrustgo.git",
                "commit": _git("rev-parse", "HEAD"),
                "branch": _git("branch", "--show-current") or "detached",
            },
            "artifacts": {},
        }
        for p in sorted(self.dir.glob("*")):
            if p.name == "MANIFEST.yaml" or not p.is_file():
                continue
            manifest["artifacts"][p.name] = {
                "sha256": sha256_file(p),
                "bytes": p.stat().st_size,
                "lines": sum(1 for _ in p.open("r", encoding="utf-8", errors="replace")),
            }
        if extra:
            manifest.update(extra)
        text = _dump_yaml(manifest)
        (self.dir / "MANIFEST.yaml").write_text(text, encoding="utf-8")
        return self.dir


class NullEvidence:
    """与 RunEvidence 同接口，但不落盘。

    用在"由 audit_run.py 承载证据"的场合：stdout 就是产物，
    不允许本脚本另外发明一套 evidence/ 布局（会与 IAIA-410 SSOT 的证据链打架）。
    """

    def append(self, name: str, rec: dict[str, Any]) -> None:
        return None

    def finalize(self, extra: dict[str, Any] | None = None):
        return None


def _git(*args: str) -> str:
    import subprocess

    try:
        return subprocess.run(["git", "-C", str(REPO_ROOT), *args], capture_output=True, text=True, timeout=20).stdout.strip()
    except Exception:
        return ""


def _dump_yaml(obj: Any, indent: int = 0) -> str:
    import yaml

    return yaml.safe_dump(obj, allow_unicode=True, sort_keys=False, default_flow_style=False)


# --------------------------------------------------------------------------
# prompt builders
# --------------------------------------------------------------------------
VERDICT_CONTRACT = (
    "输出必须是**单个 JSON 对象**，不要任何其他文字：\n"
    '{"verdict":"REAL|FALSE_POSITIVE|INCOMPLETE","confidence":0.0-1.0,"reason":"不超过60字的中文理由"}\n'
    "判定含义：REAL=判据指出的问题真实存在；FALSE_POSITIVE=判据自身有缺陷导致误报；"
    "INCOMPLETE=现有信息不足以判定（宁可选这个，也不要猜）。"
)


def build_adjudication_prompt(question: str, artifacts: list[str]) -> str:
    art = "\n".join(f"--- {name} ---\n{body}" for name, body in artifacts)
    return (
        "你是数据库引擎项目的门禁判据评审专家。只依据给出的材料判定，材料不足则输出 INCOMPLETE。\n\n"
        f"待裁定问题：\n{question}\n\n材料：\n{art}\n\n{VERDICT_CONTRACT}"
    )


# --------------------------------------------------------------------------
# CLI
# --------------------------------------------------------------------------
def summarize_health(rows: list[dict[str, Any]], required_lineages: int = 2,
                     calibrate: bool = True) -> dict[str, Any]:
    """Capability matrix from health rows. Shared by health CLI and fixtures.

    A lineage counts only if some seat of it is reachable, contract-parseable,
    role-eligible for adjudication, AND (when calibrating) ground-truth calibrated.
    Without calibration this reports *reachability*, which must never be quoted
    as the ladder being able to corroborate a truth claim.
    """
    def adjudicable(r: dict[str, Any]) -> bool:
        if not (r.get("ok") and r.get("parseable_contract")):
            return False
        if "adjudicate" not in (r.get("roles") or []):
            return False
        return bool(r.get("calibrated", not calibrate))

    reachable = sorted({r["lineage"] for r in rows if r.get("ok") and r.get("parseable_contract")})
    credited = sorted({r["lineage"] for r in rows if adjudicable(r)})
    return {
        "seats_total": len(rows),
        "seats_ok": sum(1 for r in rows if r.get("ok")),
        "seats_parseable": sum(1 for r in rows if r.get("parseable_contract")),
        "lineages_reachable": reachable,
        "lineages_credited": credited,
        "adjudication_capable": len(credited) >= required_lineages,
        "capability_basis": "calibrated+role-eligible" if calibrate else "reachability only (NOT trust)",
    }


def cmd_health(args: argparse.Namespace) -> int:
    seats = load_seats()
    ev = (NullEvidence() if args.no_evidence_dir
           else RunEvidence("health", base=args.evidence_base))
    probe_q = '只输出 JSON：{"verdict":"INCOMPLETE","confidence":0,"reason":"probe"}\n' + VERDICT_CONTRACT
    rows = []
    for seat in seats.values():
        if args.seat and seat.id not in args.seat:
            continue
        rec = ask(seat, probe_q, max_tokens=args.max_tokens, timeout_s=seat.timeout_s, lock_wait_s=args.lock_wait)
        parsed = parse_verdict(rec.get("content", "")) if rec.get("ok") else None
        row = {
            "seat": seat.id,
            "family": seat.family,
            "lineage": seat.lineage or seat.family,
            "tier": seat.tier,
            "roles": seat.roles,
            "endpoint": seat.endpoint,
            "reasoning_effort": seat.reasoning_effort,
            "ok": rec.get("ok"),
            "latency_s": rec.get("latency_s"),
            "completion_tokens": rec.get("completion_tokens"),
            "tps": (round(rec["completion_tokens"] / rec["latency_s"], 2)
                    if rec.get("completion_tokens") and rec.get("latency_s") else None),
            "finish_reason": rec.get("finish_reason"),
            "reasoning_chars": rec.get("reasoning_chars"),
            "parseable_contract": parsed is not None,
            "error": rec.get("error"),
            "probed_at": rec.get("requested_at"),
        }
        # Calibration gates only the seats that can vote on a truth claim.
        # Triage-only seats stay advisory and are never counted for R1 lineages.
        if args.calibrate and rec.get("ok") and "adjudicate" in seat.roles:
            cal = run_calibration(seat, timeout_s=seat.timeout_s, lock_wait_s=args.lock_wait)
            row["calibration"] = cal
            row["calibrated"] = cal["ratio"] >= seat.calibration_min
            ev.append("calibration", {"seat": seat.id, **cal,
                                      "min": seat.calibration_min, "pass": row["calibrated"]})
        rows.append(row)
        ev.append("health", row)
        print(json.dumps(row, ensure_ascii=False), flush=True)
    summary = summarize_health(rows, required_lineages=args.required_lineages,
                               calibrate=args.calibrate)
    ev.append("health_summary", summary)
    print(json.dumps(summary, ensure_ascii=False))
    d = ev.finalize(extra={"kind": "seat-health", "summary": summary})
    if d is not None:
        print(f"EVIDENCE: {d}")
    return 0 if summary["adjudication_capable"] else 2


def cmd_ask(args: argparse.Namespace) -> int:
    seats = load_seats()
    seat = seats.get(args.seat)
    if not seat:
        print(f"unknown seat: {args.seat}", file=sys.stderr)
        return 1
    prompt = sys.stdin.read() if args.question == "-" else args.question
    rec = ask(seat, prompt, max_tokens=args.max_tokens, timeout_s=args.timeout, lock_wait_s=args.lock_wait)
    print(json.dumps(rec, ensure_ascii=False, indent=1))
    return 0 if rec.get("ok") else 2


def cmd_adjudicate(args: argparse.Namespace) -> int:
    seats = load_seats()
    chosen = [seats[s] for s in args.seat] if args.seat else seats_for_role(seats, "adjudicate")
    if not chosen:
        print("no seats enabled for role=adjudicate", file=sys.stderr)
        return 1
    question = Path(args.question).read_text(encoding="utf-8")
    artifacts = []
    for ref in args.artifact or []:
        p = Path(ref)
        artifacts.append((p.name, p.read_text(encoding="utf-8", errors="replace")[: args.max_artifact_chars]))
    prompt = build_adjudication_prompt(question, artifacts)
    ev = (NullEvidence() if args.no_evidence_dir
           else RunEvidence("adjudicate", run_id=args.run_id, base=args.evidence_base))
    recs = []
    for seat in chosen:
        rec = ask(seat, prompt, max_tokens=args.max_tokens, lock_wait_s=args.lock_wait)
        rec["question_id"] = args.id
        recs.append(rec)
        ev.append("asks", rec)
        print(f"[{seat.id}/{seat.family}] ok={rec.get('ok')} latency={rec.get('latency_s')}s "
              f"verdict={(parse_verdict(rec.get('content','')) or {}).get('verdict','-')}", flush=True)
    machine_evidence = None
    if args.machine_evidence:
        machine_evidence = json.loads(Path(args.machine_evidence).read_text(encoding="utf-8"))
    agg = aggregate(recs, required_lineages=args.required_lineages, machine_evidence=machine_evidence)
    agg.update(question_id=args.id, artifacts=[a[0] for a in artifacts],
               prompt_sha256=sha256_text(prompt), adjudicated_at=utcnow())
    ev.append("adjudications", agg)
    d = ev.finalize(extra={"kind": "adjudication", "question_id": args.id, "aggregate": agg["aggregate"],
                          "rule": agg["rule"]})
    print(json.dumps(agg, ensure_ascii=False, indent=1))
    if d is not None:
        print(f"EVIDENCE: {d}")
    return 0 if agg["aggregate"] in ("REAL", "FALSE_POSITIVE") else 2


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description="sqlrustgo expert ladder (fail-closed soft-gate advisory)")
    ap.add_argument("--evidence-base", type=Path, default=None,
                    help="override evidence run root (used by fixtures; never point at the real repo in tests)")
    sub = ap.add_subparsers(dest="cmd", required=True)

    h = sub.add_parser("health", help="probe every seat and write the capability matrix")
    h.add_argument("--seat", action="append",
                   help="restrict to this seat id (repeatable)")
    # 改前默认 256：思考型席位会把预算全花在 reasoning 上，content 为空 +
    #     finish_reason=length => 席位被 health 误判为不可用（实测 gmp35b-250）。
    # 改后 1500：同一席位在同一条命令下 finish_reason=stop、parseable_contract=true。
    # 反向变异：把 --max-tokens 压回 256，该席位必须重新变红（见 run 归档里的两条 health 记录）。
    h.add_argument("--max-tokens", type=int, default=1500)
    h.add_argument("--lock-wait", type=int, default=90)
    h.add_argument("--required-lineages", type=int, default=2)
    h.add_argument("--no-calibrate", dest="calibrate", action="store_false",
                   help="transport-only probe; capability then reflects reachability, NOT trust")
    h.set_defaults(calibrate=True)
    h.set_defaults(func=cmd_health)

    a = sub.add_parser("ask", help="single call to one seat")
    a.add_argument("--seat", required=True)
    a.add_argument("--question", required=True, help="literal text, or '-' to read stdin")
    a.add_argument("--max-tokens", type=int, default=None)
    a.add_argument("--timeout", type=int, default=None)
    a.add_argument("--lock-wait", type=int, default=90)
    a.set_defaults(func=cmd_ask)

    j = sub.add_parser("adjudicate", help="multi-seat fail-closed verdict on one question")
    j.add_argument("--id", required=True, help="question id, e.g. issue-5473")
    j.add_argument("--question", required=True, help="path to a file holding the question")
    j.add_argument("--artifact", action="append", help="file to quote as evidence material (repeatable)")
    j.add_argument("--seat", action="append", help="seat id (repeatable); default = role adjudicate")
    j.add_argument("--required-lineages", type=int, default=2,
                   help="independent model lineages required (R1)")
    j.add_argument("--machine-evidence", help="JSON with {reverse_mutation_passed: true|false}")
    j.add_argument("--max-tokens", type=int, default=400)
    j.add_argument("--max-artifact-chars", type=int, default=6000)
    j.add_argument("--lock-wait", type=int, default=120)
    j.add_argument("--run-id")
    j.set_defaults(func=cmd_adjudicate)

    # 挂在子命令上，而不是顶层：顶层可选参数必须写在子命令名之前，
    # `health --no-evidence-dir` 会被 argparse 判成"未知参数"并以 2 退出——
    # 那种退出码和"席位不具备裁定能力"的 2 撞车，会把用法错误误读成能力结论。
    for _sub in (h, j):
        _sub.add_argument("--no-evidence-dir", action="store_true",
                          help="write nothing; stdout is the artifact. Use when audit_run.py "
                               "captures this run's logs (repo evidence SSOT), so the tool "
                               "does not create a parallel evidence/ layout")

    args = ap.parse_args(argv)
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())

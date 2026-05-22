#!/usr/bin/env python3
"""
GMP Platform 最小闭环验证脚本
Closed-Loop Verification: 工单→扫描→语音→DSL→规则引擎→签名→审计链
"""

import hashlib
import json
import time
import uuid
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent
import sys
sys.path.insert(0, str(REPO_ROOT))

OLLAMA_URL = "http://192.168.0.252:11434"
DSL_MODEL  = "qwen2.5:14b-instruct-q4_K_M"

class AuditEntry:
    def __init__(self, step: str, input_data: dict, output_data: dict, status: str, latency_ms: float):
        self.step = step
        self.input_data = input_data
        self.output_data = output_data
        self.status = status
        self.latency_ms = latency_ms
        self.timestamp = datetime.utcnow().isoformat() + "Z"
        self.input_hash = hashlib.sha256(json.dumps(input_data, sort_keys=True).encode()).hexdigest()
        self.output_hash = hashlib.sha256(json.dumps(output_data, sort_keys=True).encode()).hexdigest()
        self.prev_hash = ""
        self.self_hash = ""

    def to_dict(self):
        d = {
            "step": self.step,
            "timestamp": self.timestamp,
            "status": self.status,
            "latency_ms": round(self.latency_ms, 2),
            "input_hash": self.input_hash,
            "output_hash": self.output_hash,
            "prev_hash": self.prev_hash,
            "self_hash": self.self_hash,
        }
        d.update(self.output_data)
        return d


class AuditChain:
    def __init__(self):
        self.entries = []
        self.prev_hash = "GENESIS"

    def add(self, entry: AuditEntry):
        entry.prev_hash = self.prev_hash
        entry.self_hash = hashlib.sha256(
            (entry.input_hash + entry.output_hash + entry.prev_hash).encode()
        ).hexdigest()
        self.entries.append(entry)
        self.prev_hash = entry.self_hash

    def seal(self) -> str:
        if not self.entries:
            return self.prev_hash
        chain_data = "|".join(e.self_hash for e in self.entries)
        return hashlib.sha256(chain_data.encode()).hexdigest()

    def verify(self) -> tuple:
        errors = []
        for i, e in enumerate(self.entries):
            if i == 0 and e.prev_hash != "GENESIS":
                errors.append(f"Entry {i}: prev_hash != GENESIS")
            elif i > 0 and e.prev_hash != self.entries[i-1].self_hash:
                errors.append(f"Entry {i}: chain broken")
        return len(errors) == 0, errors

    def summary(self):
        return [e.to_dict() for e in self.entries]


# ─── Step 1: Mobile L0 ────────────────────────────────────────────────────

class MobileSimulator:
    def scan_label(self, material_code: str, batch_no: str) -> tuple:
        time.sleep(0.05)
        result = {
            "material_code": material_code,
            "batch_no": batch_no,
            "confidence": 0.97,
            "local_processing": True,
        }
        return result, 50.0

    def voice_input(self, text: str) -> tuple:
        time.sleep(0.03)
        import re
        m = re.search(r"([\d.]+)\s*(公斤|kg)", text)
        value = float(m.group(1)) if m else 50.0
        unit = m.group(2) if m else "公斤"
        result = {"text": text, "extracted_value": value, "unit": unit, "confidence": 0.94, "local_processing": True}
        return result, 30.0

    def sign(self, signer_id: str, signer_name: str, role: str, record_hash: str) -> tuple:
        sig_image = "OPERATOR_SIG_BASE64_DATA_OVER_100_CHARS_FOR_21CFR_COMPLIANCE_CHECK"
        signed_at = datetime.utcnow().isoformat() + "Z"
        block = f"{signer_id}|{signer_name}|{role}|{record_hash}|{sig_image}|{signed_at}|PREV"
        self_hash = hashlib.sha256(block.encode()).hexdigest()
        sig = {
            "signer_id": signer_id, "signer_name": signer_name, "role": role,
            "record_hash": record_hash, "signature_image_b64": sig_image,
            "signed_at": signed_at, "self_hash": self_hash,
        }
        return sig, 10.0


# ─── Step 2: DSL Generator ──────────────────────────────────────────────────

class OllamaClient:
    def __init__(self, base_url: str = OLLAMA_URL):
        self.base_url = base_url

    def generate(self, model: str, prompt: str) -> str:
        import urllib.request
        payload = {"model": model, "prompt": prompt, "stream": False,
                   "options": {"temperature": 0.1, "num_predict": 300}}
        req = urllib.request.Request(
            f"{self.base_url}/api/generate",
            data=json.dumps(payload).encode(),
            headers={"Content-Type": "application/json"},
            method="POST",
        )
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                return json.loads(resp.read())["response"]
        except Exception as e:
            raise RuntimeError(f"Ollama error: {e}")


SYSTEM_PROMPT = """你是一个GMP称量操作助手。你的输出必须是有效的JSON格式。
JSON格式: {"action": "...", "params": {...}, "confidence": 0.0-1.0, "requires_human_confirm": true|false, "reasoning": "..."}
只输出JSON，不要有markdown代码块包裹。"""


class DslGenerator:
    def __init__(self):
        self.client = OllamaClient()

    def generate(self, user_input: str, ocr_data: dict, voice_data: dict) -> tuple:
        t0 = time.time()
        context = f"用户: {user_input}\nOCR: {json.dumps(ocr_data, ensure_ascii=False)}\n语音: {json.dumps(voice_data, ensure_ascii=False)}"
        prompt = f"{SYSTEM_PROMPT}\n\n{context}"
        raw = self.client.generate(DSL_MODEL, prompt).strip()
        if raw.startswith("```"):
            raw = raw.split("```")[1]
            if raw.startswith("json"):
                raw = raw[4:]
        dsl = json.loads(raw.strip())
        dsl["_latency_ms"] = round((time.time() - t0) * 1000, 1)
        dsl["_raw"] = raw[:100]
        return dsl, dsl["_latency_ms"]


# ─── Step 3: L3 Validator ──────────────────────────────────────────────────

class DslValidator:
    VALID_ACTIONS = {"weighing_operation", "weigh_material", "compliance_check", "deviation_summary", "capa_status", "generate_record"}

    def validate(self, dsl: dict) -> tuple:
        errors = []
        if "action" not in dsl or dsl["action"] not in self.VALID_ACTIONS:
            errors.append(f"invalid action: {dsl.get('action')}")
        if "params" not in dsl or not isinstance(dsl["params"], dict):
            errors.append("missing/invalid params")
        if "confidence" in dsl and not (0 <= dsl["confidence"] <= 1):
            errors.append(f"confidence out of range: {dsl['confidence']}")
        dsl_str = json.dumps(dsl).upper()
        for p in ["SELECT", "INSERT", "DROP", "UNION", "../"]:
            if p in dsl_str:
                errors.append(f"security: pattern '{p}'")
        return len(errors) == 0, errors


# ─── Step 4: Rule Engine ───────────────────────────────────────────────────

class RuleEngine:
    def evaluate(self, dsl: dict, context: dict) -> dict:
        t0 = time.time()
        action = dsl.get("action", "")
        params = dsl.get("params", {})
        results = {
            "action": action, "params": params,
            "violations": [], "decisions": [],
            "capa_required": False, "blocked": False,
        }

        if action in ("weighing_operation", "weigh_material"):
            mat = params.get("material_code", "")
            val = params.get("weighing_value", 0.0)
            target = context.get("target_weight", 50.0)
            tol = context.get("tolerance_pct", 0.5)

            if not mat:
                results["violations"].append({"rule": "WEI-001", "severity": "critical", "description": "物料编码为空"})
                results["blocked"] = True
            elif not mat.startswith("MAT-"):
                results["violations"].append({"rule": "WEI-001", "severity": "critical", "description": f"物料编码不在工单: {mat}"})
                results["blocked"] = True

            if val > 0:
                dev = abs(val - target) / target * 100
                if dev > tol:
                    sev = "major" if dev < 2.0 else "critical"
                    results["violations"].append({"rule": "WEI-002", "severity": sev, "description": f"偏差 {dev:.2f}% 超过允差 {tol}%"})
                    results["capa_required"] = True

        if results["violations"] or results["capa_required"]:
            max_sev = max(results["violations"], key=lambda v: {"minor": 1, "major": 2, "critical": 3}.get(v["severity"], 0)).get("severity", "minor") if results["violations"] else "minor"
            results["decisions"].append({"rule": "DEV-001", "classification": max_sev, "description": f"分类: {max_sev}"})

        results["_latency_ms"] = round((time.time() - t0) * 1000, 1)
        return results


# ─── Main ───────────────────────────────────────────────────────────────────

def run_closed_loop(
    work_order_id="WO-2026-001",
    material_code="MAT-ABC123",
    batch_no="BATCH-2024-042",
    voice_text="称取 A 物料 50.12 公斤",
    target_weight=50.0,
    tolerance_pct=0.5,
    skip_ollama=False,
    verbose=False,
):
    chain = AuditChain()
    t_loop = time.time()

    print("=" * 70)
    print(" GMP Platform 最小闭环验证")
    print("=" * 70)
    print(f" 工单: {work_order_id}  物料: {material_code}  批号: {batch_no}")
    print(f" 语音: {voice_text}  目标: {target_weight}kg \u00b1{tolerance_pct}%")
    print("=" * 70)

    # Step 1: Mobile L0 — OCR
    print("\n[Step 1/6] Mobile L0 \u2014 OCR 扫描")
    mobile = MobileSimulator()
    ocr_data, lat = mobile.scan_label(material_code, batch_no)
    chain.add(AuditEntry("mobile_ocr", {}, ocr_data, "pass", lat))
    print(f"  物料: {ocr_data['material_code']}  批号: {ocr_data['batch_no']}  \u2713 L0本地处理")

    # Step 2: Mobile L0 — ASR
    print("\n[Step 2/6] Mobile L0 \u2014 语音录入")
    voice_data, lat = mobile.voice_input(voice_text)
    chain.add(AuditEntry("mobile_asr", {"text": voice_text}, voice_data, "pass", lat))
    print(f"  值: {voice_data['extracted_value']} {voice_data['unit']}  \u2713 L0本地处理")

    # Step 3: L2 Cloud — DSL
    print("\n[Step 3/6] L2 Cloud \u2014 DSL 生成 (Ollama qwen2.5:14b)")
    if skip_ollama:
        print("  \u23e9 Skip Ollama")
        dsl = {"action": "weighing_operation", "params": {"work_order_id": work_order_id, "material_code": ocr_data["material_code"], "weighing_value": voice_data["extracted_value"], "unit": voice_data["unit"]}, "confidence": 0.95, "requires_human_confirm": False}
        lat_dsl = 0
    else:
        try:
            gen = DslGenerator()
            dsl, lat_dsl = gen.generate(voice_text, ocr_data, voice_data)
        except Exception as e:
            print(f"  \u274c Ollama失败: {e} \u2014 回退")
            dsl = {"action": "weighing_operation", "params": {"work_order_id": work_order_id, "material_code": ocr_data["material_code"], "weighing_value": voice_data["extracted_value"], "unit": voice_data["unit"]}, "confidence": 0.95, "requires_human_confirm": False}
            lat_dsl = 0

    chain.add(AuditEntry("dsl_generation", {"ocr": ocr_data, "voice": voice_data}, dsl, "pass", lat_dsl))
    print(f"  动作: {dsl.get('action')}  置信度: {dsl.get('confidence')}  延迟: {lat_dsl:.0f}ms")

    # Step 4: L3 Validator
    print("\n[Step 4/6] L3 Validator \u2014 DSL 安全校验")
    validator = DslValidator()
    valid, errors = validator.validate(dsl)
    if valid:
        chain.add(AuditEntry("dsl_validation", dsl, {"valid": True}, "pass", 5.0))
        print(f"  \u2705 DSL校验通过")
    else:
        chain.add(AuditEntry("dsl_validation", dsl, {"errors": errors}, "reject", 5.0))
        print(f"  \u274c 校验失败: {errors} \u2014 闭环终止")
        return

    # Step 5: Rule Engine
    print("\n[Step 5/6] L0 Rule Engine \u2014 规则执行")
    context = {"target_weight": target_weight, "tolerance_pct": tolerance_pct}
    engine = RuleEngine()
    rule_result = engine.evaluate(dsl, context)
    chain.add(AuditEntry("rule_engine", {"dsl": dsl, "context": context}, rule_result, "pass", rule_result["_latency_ms"]))
    violations = rule_result.get("violations", [])
    print(f"  违规: {len(violations)}项  CAPA: {'需要' if rule_result.get('capa_required') else '不需要'}  阻断: {'是' if rule_result.get('blocked') else '否'}")
    if violations:
        for v in violations:
            print(f"    - [{v['severity']}] {v['rule']}: {v['description']}")

    # Step 6: E-Signature + Audit Chain
    print("\n[Step 6/6] L0 E-Signature \u2014 审计链封闭")
    record_data = {"work_order_id": work_order_id, "material_code": ocr_data["material_code"], "batch_no": batch_no, "weighing_value": voice_data["extracted_value"], "unit": voice_data["unit"], "rule_result": rule_result}
    record_hash = hashlib.sha256(json.dumps(record_data, sort_keys=True).encode()).hexdigest()

    op_sig, lat = mobile.sign("OP-001", "张三", "OPERATOR", record_hash)
    chain.add(AuditEntry("signature_operator", {"record_hash": record_hash}, op_sig, "pass", lat))
    print(f"  操作员签名: {op_sig['signer_id']} self_hash={op_sig['self_hash'][:12]}...")

    re_sig, lat = mobile.sign("RE-001", "李四", "REVIEWER", record_hash)
    chain.add(AuditEntry("signature_reviewer", {"record_hash": record_hash}, re_sig, "pass", lat))
    print(f"  复核人签名: {re_sig['signer_id']} self_hash={re_sig['self_hash'][:12]}...")

    seal_hash = chain.seal()
    chain_ok, chain_errors = chain.verify()

    total_ms = (time.time() - t_loop) * 1000

    print("\n" + "=" * 70)
    print(" 闭环结果")
    print("=" * 70)
    print(f" 总延迟:    {total_ms:.0f}ms")
    print(f" 审计链:    {'OK' if chain_ok else 'BREAK'}")
    print(f" Chain Seal: {seal_hash[:32]}...")
    print(f" 违规项:    {len(violations)}")
    print(f" CAPA:      {'需要' if rule_result.get('capa_required') else '不需要'}")

    print("\n 审计链:")
    for e in chain.summary():
        icon = "\u2705" if e["status"] == "pass" else "\u274c"
        print(f"  {icon} {e['step']:28s} {e['status']:6s} {e['latency_ms']:7.1f}ms")

    print(f"\n  Chain Seal: {seal_hash}")
    print("=" * 70)
    return {"chain_ok": chain_ok, "seal_hash": seal_hash, "rule_result": rule_result, "total_ms": total_ms}


if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser()
    p.add_argument("--verbose", "-v", action="store_true")
    p.add_argument("--skip-ollama", action="store_true")
    p.add_argument("--wo", default="WO-2026-001")
    p.add_argument("--mat", default="MAT-ABC123")
    p.add_argument("--batch", default="BATCH-2024-042")
    p.add_argument("--voice", default="称取 A 物料 50.12 公斤")
    args = p.parse_args()
    result = run_closed_loop(args.wo, args.mat, args.batch, args.voice, skip_ollama=args.skip_ollama, verbose=args.verbose)

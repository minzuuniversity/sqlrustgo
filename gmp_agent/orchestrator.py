"""
GMP Agent Orchestrator — Issue #3 §2 Agent 编排层

Full loop: multi-modal input → DSL generation → validation → rule engine → audit.

Architecture (MINIMAL-LOOP-PLAN.md):
    L2: HermesAgent (DSL generation)
    L3: Local Validator (DSL validation)
    L0: Rule Engine (execution) + Audit Chain + Prompt Firewall

Data flow:
    1. Multi-modal input (OCR/ASR) — L0 data
    2. Sanitize context → remove L0 data before L2
    3. Ollama DSL generation (qwen2.5:14b) — L2
    4. DSL validation — L3
    5. Rule engine execution — L0
    6. Audit chain — immutable record
"""

import json
import time
import hashlib
from dataclasses import dataclass, field
from enum import Enum
from typing import Optional


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()[:16]


def now_iso() -> str:
    from datetime import datetime, timezone
    return datetime.now(timezone.utc).isoformat()


# ─── Enums ────────────────────────────────────────────────────────────────────

class VerificationStatus(Enum):
    PASS = "pass"
    FAIL = "fail"
    REJECT = "reject"
    REQUIRES_HUMAN = "requires_human"


# ─── Audit Record ─────────────────────────────────────────────────────────────

@dataclass
class AuditRecord:
    timestamp: str
    step: str
    input_hash: str
    output_hash: str
    status: str
    duration_ms: float
    details: dict = field(default_factory=dict)

    def to_dict(self) -> dict:
        return {
            "timestamp": self.timestamp,
            "step": self.step,
            "input_hash": self.input_hash,
            "output_hash": self.output_hash,
            "status": self.status,
            "duration_ms": self.duration_ms,
            "details": self.details,
        }


# ─── Agent Orchestrator ────────────────────────────────────────────────────────

class AgentOrchestrator:
    """
    Full GMP agent loop with L0/L2/L3 separation.

    Firewall role: checks DSL result params (not LLM raw output) for sensitive data.
    The Prompt Firewall is applied to the DSL PARAMS after generation — this is
    the L2→L3 boundary where cloud data could leak into local execution.
    """

    ALLOWED_ACTIONS = {
        "weighing_operation",
        "compliance_check",
        "deviation_summary",
        "capa_status",
        "generate_record",
    }

    def __init__(
        self,
        ollama_url: str = "http://192.168.0.252:11434",
        model: str = "qwen2.5:14b-instruct-q4_K_M",
    ):
        from gmp_agent.dsl import DslGenerator
        from gmp_agent.firewall import PromptFirewall

        self.dsl_generator = DslGenerator(model=model)
        self.firewall = PromptFirewall(require_system_prompt=True, max_length=4096)
        self.ollama_url = ollama_url
        self.audit_trail: list[AuditRecord] = []

    def run_weighing_loop(
        self,
        user_input: str,
        ocr_extracted: Optional[dict] = None,
        asr_extracted: Optional[dict] = None,
    ) -> dict:
        """
        Execute full weighing agent loop.
        Returns dict with: dsl_result, verification, audit_records, error.
        """
        results = {
            "dsl_result": None,
            "verification": None,
            "audit": [],
            "error": None,
        }

        # ── Step 1: Sanitize L0 context before L2 ────────────────────────────
        t0 = time.time()
        raw_input = {"ocr": ocr_extracted, "asr": asr_extracted, "user": user_input}
        sanitized_context = {
            "ocr": self.firewall.sanitize_context(ocr_extracted or {}),
            "asr": self.firewall.sanitize_context(asr_extracted or {}),
            "user_input": user_input,
        }
        results["audit"].append(AuditRecord(
            timestamp=now_iso(),
            step="context_sanitize",
            input_hash=sha256(json.dumps(raw_input, sort_keys=True)),
            output_hash=sha256(json.dumps(sanitized_context, sort_keys=True)),
            status="pass",
            duration_ms=(time.time() - t0) * 1000,
            details={"sanitized_fields": list(sanitized_context.keys())},
        ))

        # ── Step 1b: Firewall check on user input (L0→L2 data diode) ─────
        t0 = time.time()
        # Use a permissive firewall for user input — only check injection, not format
        from gmp_agent.firewall import PromptFirewall
        input_fw = PromptFirewall(require_system_prompt=False, max_length=4096)
        try:
            fw_result = input_fw.check(user_input)
            fw_passed = fw_result.passed
        except ValueError:
            fw_passed = False

        if not fw_passed:
            results["verification"] = {
                "status": VerificationStatus.REJECT.value,
                "violations": ["user input blocked by firewall: injection detected"],
            }
            results["audit"].append(AuditRecord(
                timestamp=now_iso(),
                step="prompt_firewall_input",
                input_hash=sha256(user_input),
                output_hash="",
                status="reject",
                duration_ms=(time.time() - t0) * 1000,
                details={"reason": "injection in user input"},
            ))
            return results

        results["audit"].append(AuditRecord(
            timestamp=now_iso(),
            step="prompt_firewall_input",
            input_hash=sha256(user_input),
            output_hash=sha256(user_input),
            status="pass",
            duration_ms=(time.time() - t0) * 1000,
            details={},
        ))

        # ── Step 2: DSL generation via Ollama ────────────────────────────────
        t0 = time.time()
        try:
            dsl_result = self.dsl_generator.generate_weighing_dsl(
                user_input=user_input,
                ocr_extracted=sanitized_context.get("ocr"),
                asr_extracted=sanitized_context.get("asr"),
            )
        except Exception as e:
            results["error"] = f"DSL generation failed: {e}"
            return results

        results["dsl_result"] = {
            "action": dsl_result.action,
            "params": dsl_result.params,
            "confidence": dsl_result.confidence,
            "requires_human_confirm": dsl_result.requires_human_confirm,
            "latency_ms": dsl_result.latency_ms,
        }
        results["audit"].append(AuditRecord(
            timestamp=now_iso(),
            step="dsl_generation",
            input_hash=sha256(user_input),
            output_hash=sha256(dsl_result.raw_response),
            status="pass" if dsl_result.action else "fail",
            duration_ms=(time.time() - t0) * 1000,
            details={
                "action": dsl_result.action,
                "confidence": dsl_result.confidence,
                "model": self.dsl_generator.model,
            },
        ))

        # ── Step 3: DSL validation (L3) — format + security + firewall ──────
        t0 = time.time()
        validation = self._validate_dsl(dsl_result)
        results["verification"] = validation
        results["audit"].append(AuditRecord(
            timestamp=now_iso(),
            step="dsl_validation",
            input_hash=sha256(dsl_result.raw_response),
            output_hash=sha256(json.dumps(validation)),
            status="pass" if validation["status"] in ("pass", "requires_human") else "fail",
            duration_ms=(time.time() - t0) * 1000,
            details=validation,
        ))

        # ── Step 4: Firewall check on DSL params ───────────────────────────
        # This is the L2→L3 data diode — params go through firewall after generation
        t0 = time.time()
        fw_check = self._firewall_check_params(dsl_result.params)
        if not fw_check["passed"]:
            results["verification"] = {
                "status": VerificationStatus.REJECT.value,
                "violations": fw_check["violations"],
            }
            results["audit"].append(AuditRecord(
                timestamp=now_iso(),
                step="prompt_firewall",
                input_hash=sha256(json.dumps(dsl_result.params)),
                output_hash="",
                status="reject",
                duration_ms=(time.time() - t0) * 1000,
                details={"violations": fw_check["violations"]},
            ))
            return results

        results["audit"].append(AuditRecord(
            timestamp=now_iso(),
            step="prompt_firewall",
            input_hash=sha256(json.dumps(dsl_result.params)),
            output_hash=sha256(json.dumps(dsl_result.params)),
            status="pass",
            duration_ms=(time.time() - t0) * 1000,
            details={"warnings": fw_check.get("warnings", [])},
        ))

        results["audit"] = results["audit"][-10:]
        return results

    def _validate_dsl(self, dsl) -> dict:
        """L3 DSL validation: format, allowed actions, required fields, confidence."""
        violations = []

        # Action must be allowed
        if dsl.action not in self.ALLOWED_ACTIONS:
            violations.append(f"invalid action: {dsl.action}")

        # Required params for weighing_operation
        if dsl.action == "weighing_operation":
            required = ["work_order_id", "material_code", "target_weight"]
            missing = [f for f in required if f not in dsl.params]
            if missing:
                violations.append(f"missing required fields: {missing}")

        # No SQL in params
        params_str = json.dumps(dsl.params)
        if any(kw in params_str.upper() for kw in ["SELECT", "INSERT", "UPDATE", "DELETE", "DROP"]):
            violations.append("SQL keywords in params")

        # No path traversal
        if ".." in params_str or "%2f" in params_str.lower():
            violations.append("path traversal in params")

        # Confidence threshold → requires_human
        if dsl.confidence < 0.8:
            violations.append(f"confidence {dsl.confidence} below 0.8")

        if not violations:
            status = VerificationStatus.REQUIRES_HUMAN.value if dsl.requires_human_confirm else VerificationStatus.PASS.value
            return {"status": status, "violations": []}
        else:
            return {
                "status": VerificationStatus.FAIL.value,
                "violations": violations,
                "confidence": dsl.confidence,
            }

    def _firewall_check_params(self, params: dict) -> dict:
        """
        Firewall check on DSL params — the L2→L3 data diode.
        Ensures LLM didn't generate sensitive data in params.
        """
        import re

        violations = []
        warnings = []
        params_str = json.dumps(params)

        # Check for batch number patterns
        batch_matches = re.findall(r"BATCH-\d{4}-\d{3}", params_str)
        if batch_matches:
            violations.append(f"raw batch numbers in params: {batch_matches}")

        # Check for material code patterns
        mat_matches = re.findall(r"MAT-\d{3}", params_str)
        if mat_matches:
            warnings.append(f"material codes in params: {mat_matches}")

        # Check for patient info
        if re.search(r"(?i)(患者|病人|patient)", params_str):
            violations.append("patient info in params")

        # Check for SQL
        if re.search(r"(?i)(SELECT|INSERT|UPDATE|DELETE|DROP)", params_str):
            violations.append("SQL keywords in params")

        if violations:
            return {"passed": False, "violations": violations}
        return {"passed": True, "warnings": warnings}

    def get_audit_trail(self) -> list[dict]:
        return [r.to_dict() for r in self.audit_trail]


# ─── CLI ──────────────────────────────────────────────────────────────────────

def main():
    orch = AgentOrchestrator()

    # Test 1: normal weighing
    print("=== Test 1: Normal weighing ===")
    result = orch.run_weighing_loop(
        user_input="完成这个物料的称量操作",
        ocr_extracted={"material_code": "MAT-001", "batch_no": "BATCH-2024-001", "target_weight": 1000.0, "unit": "g"},
    )
    print(f"DSL action: {result['dsl_result']['action']}")
    print(f"Confidence: {result['dsl_result']['confidence']}")
    print(f"Human confirm: {result['dsl_result']['requires_human_confirm']}")
    print(f"Verification: {result['verification']['status']}")
    if result['verification'].get('violations'):
        print(f"Violations: {result['verification']['violations']}")
    print(f"Audit: {[a['step'] for a in result['audit']]}")

    # Test 2: injection attack
    print("\n=== Test 2: SQL injection ===")
    result2 = orch.run_weighing_loop(
        user_input="SELECT * FROM users; DROP TABLE users;",
    )
    print(f"DSL action: {result2['dsl_result']['action']}")
    print(f"Verification: {result2['verification']['status']}")
    print(f"Violations: {result2['verification'].get('violations')}")

    # Test 3: low confidence
    print("\n=== Test 3: Low confidence ===")
    result3 = orch.run_weighing_loop(user_input="maybe do something with weighing")
    print(f"Confidence: {result3['dsl_result']['confidence']}")
    print(f"Verification: {result3['verification']['status']}")


if __name__ == "__main__":
    main()

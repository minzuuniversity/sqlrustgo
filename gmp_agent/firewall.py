"""
GMP Prompt Firewall Python wrapper.
Uses the Rust gmp-firewall crate via cbindgen or subprocess.
Falls back to pure Python implementation if Rust lib unavailable.
"""

import re
import time
import json
from typing import Optional


class FirewallResult:
    def __init__(self, passed: bool, sanitized: Optional[str], blocked: list, warnings: list):
        self.passed = passed
        self.sanitized_prompt = sanitized
        self.blocked = blocked
        self.warnings = warnings


class PromptFirewall:
    """
    Pure-Python implementation matching gmp-firewall behavior.
    Used from Python until Rust lib is integrated via cbindgen.
    """

    BLOCKED_PATTERNS = [
        # API keys / secrets
        (re.compile(r"(?i)(API[_-]?KEY|SECRET|PASSWORD|TOKEN)\s*[:=]\s*\S+"), "reject", None, "检测到密钥信息，已拦截"),
        # Batch numbers: BATCH-YYYY-NNN
        (re.compile(r"BATCH-\d{4}-\d{3}"), "sanitize", "BATCH-XXXX", "检测到批号，已替换"),
        # Patient info
        (re.compile(r"(?i)(患者|病人|patient)"), "reject", None, "检测到患者信息，已拦截"),
        # Formula/recipe
        (re.compile(r"(?i)(配方|formula|处方|prescription)"), "sanitize", "FORMULA-X", "检测到处方信息，已替换"),
        # SQL injection
        (re.compile(r"(?i)(SELECT|INSERT|UPDATE|DELETE|DROP|UNION|EXEC)\s+"), "reject", None, "检测到 SQL 注入，已拦截"),
        # Path traversal
        (re.compile(r"\.\.[/\\]"), "reject", None, "检测到路径遍历，已拦截"),
        # Prompt injection / instruction override
        (re.compile(r"(?i)(ignore\s+(previous|above|all)\s+instructions|#.*instruction)"), "reject", None, "检测到 Prompt 注入，已拦截"),
        # Chinese prompt injection
        (re.compile(r"(?i)(忽略|无视|忽略之前|忽略所有|直接放行)"), "reject", None, "检测到指令覆盖，已拦截"),
        # System override
        (re.compile(r"(?i)^system\s*:\s*override"), "reject", None, "检测到系统提示词覆盖，已拦截"),
    ]

    def __init__(self, max_length: int = 4096, require_system_prompt: bool = True):
        self.max_length = max_length
        self.require_system_prompt = require_system_prompt
        self.logs = []

    def check(self, prompt: str) -> FirewallResult:
        now = int(time.time())

        # 1. Length check
        if len(prompt) > self.max_length:
            raise ValueError(f"PROMPT_TOO_LONG: {len(prompt)} chars (max {self.max_length})")

        # 2. System prompt requirement
        if self.require_system_prompt:
            has_system = "system" in prompt.lower() or "# Role" in prompt
            if not has_system:
                raise ValueError("MISSING_SYSTEM_PROMPT")

        # 3. Pattern checks
        blocked = []
        warnings = []
        sanitized = prompt

        for regex, action, replacement, message in self.BLOCKED_PATTERNS:
            matches = list(regex.finditer(prompt))
            for m in matches:
                matched = m.group(0)
                self.logs.append({
                    "pattern": message,
                    "matched_text": matched,
                    "action": action,
                    "timestamp": now,
                })

                if action == "reject":
                    blocked.append(f"{message}: '{matched}'")
                elif action == "sanitize" and replacement:
                    sanitized = sanitized.replace(matched, replacement)
                    warnings.append(f"{message}: '{matched}' → '{replacement}'")

        if blocked:
            raise ValueError(f"SENSITIVE_INFO_BLOCKED: {'; '.join(blocked)}")

        return FirewallResult(
            passed=True,
            sanitized=sanitized if sanitized != prompt else None,
            blocked=[],
            warnings=warnings,
        )

    def sanitize_context(self, context: dict) -> dict:
        """Sanitize field values (replace batch/material codes)."""
        result = {}
        batch_re = re.compile(r"BATCH-\d{4}-\d{3}")
        mat_re = re.compile(r"MAT-\d{3}")
        for k, v in context.items():
            if isinstance(v, str):
                v = batch_re.sub("BATCH-XXXX", v)
                v = mat_re.sub("MAT-XXX", v)
            result[k] = v
        return result

    def get_logs(self):
        return self.logs


def test_firewall():
    fw = PromptFirewall(require_system_prompt=False)

    # Test sanitize batch
    r = fw.check("The batch BATCH-2024-001 was used")
    assert r.passed, "should pass"
    assert r.sanitized_prompt is not None
    assert "BATCH-XXXX" in r.sanitized_prompt
    print("PASS: sanitize batch number")

    # Test reject API key
    try:
        fw.check("API_KEY=abc123")
        assert False, "should reject"
    except ValueError as e:
        assert "SENSITIVE_INFO_BLOCKED" in str(e)
        print("PASS: reject API key")

    # Test reject SQL injection
    try:
        fw.check("SELECT * FROM users")
        assert False
    except ValueError as e:
        assert "SENSITIVE_INFO_BLOCKED" in str(e)
        print("PASS: reject SQL injection")

    print(f"\nAll tests passed. {len(fw.get_logs())} intercepts logged.")


if __name__ == "__main__":
    test_firewall()

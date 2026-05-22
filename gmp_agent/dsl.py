"""
GMP Agent DSL Generator — Issue #3 §2.5
Calls Ollama (qwen2.5:14b) to generate DSL from multi-modal input.
"""

import json
import sys
import time
import os
from dataclasses import dataclass
from typing import Optional

OLLAMA_URL = "http://192.168.0.252:11434"
DEFAULT_MODEL = "qwen2.5:14b-instruct-q4_K_M"

WEIGHING_AGENT_PROMPT = """# Role
你是 GMP 制药企业的称量操作助手。你的职责是辅助操作工完成合规的称量操作。

# Capabilities
1. 从照片中 OCR 提取物料标签信息（物料编码、批号、供应商）
2. 从语音中提取称量值和物料信息
3. 根据规则判定称量操作是否合规
4. 在不合规时自动触发偏差流程
5. 生成结构化称量记录

# Rules
- 你只能生成 DSL 格式的操作指令，不能直接执行任何操作
- 你不能访问原始数据库，只能通过 DSL 查询
- 你不能看到真实的工艺参数值，只能看到占位符
- 你不能存储或记忆任何敏感数据
- 所有判定结果必须经过本地验证器确认后才能执行

# DSL Format
你的输出必须是以下格式之一：
- weighing_operation: 称量操作指令
- compliance_check: 合规检查请求
- deviation_summary: 偏差查询
- generate_record: 记录生成

# Response Format
```json
{"action": "<dsl_action>", "params": {...}, "reasoning": "<你的推理过程>", "confidence": 0.0-1.0, "requires_human_confirm": true/false}
```

# Constraints
- 禁止输出真实批号、物料名、供应商名
- 禁止输出 SQL 语句
- 禁止输出文件路径
- 禁止输出 API Key
- 当 confidence < 0.8 时，必须设置 requires_human_confirm = true
"""


@dataclass
class DslResult:
    action: str
    params: dict
    reasoning: str
    confidence: float
    requires_human_confirm: bool
    raw_response: str
    latency_ms: float


class OllamaClient:
    def __init__(self, base_url: str = OLLAMA_URL):
        self.base_url = base_url

    def generate(
        self,
        model: str,
        prompt: str,
        system: Optional[str] = None,
        temperature: float = 0.1,
    ) -> tuple[str, float]:
        import urllib.request
        import urllib.error

        payload = {
            "model": model,
            "prompt": prompt,
            "temperature": temperature,
            "stream": False,
        }
        if system:
            payload["system"] = system

        data = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            f"{self.base_url}/api/generate",
            data=data,
            headers={"Content-Type": "application/json"},
            method="POST",
        )

        start = time.time()
        try:
            with urllib.request.urlopen(req, timeout=120) as resp:
                result = json.loads(resp.read().decode("utf-8"))
                latency = (time.time() - start) * 1000
                return result.get("response", ""), latency
        except urllib.error.HTTPError as e:
            raise RuntimeError(f"Ollama HTTP {e.code}: {e.read().decode()}") from e
        except Exception as e:
            raise RuntimeError(f"Ollama request failed: {e}") from e


class DslGenerator:
    """Generates DSL from natural language using Ollama qwen2.5:14b."""

    ALLOWED_ACTIONS = {
        "weighing_operation",
        "compliance_check",
        "deviation_summary",
        "capa_status",
        "generate_record",
    }

    def __init__(self, model: str = DEFAULT_MODEL):
        self.ollama = OllamaClient()
        self.model = model

    def generate_weighing_dsl(
        self,
        user_input: str,
        ocr_extracted: Optional[dict] = None,
        asr_extracted: Optional[dict] = None,
    ) -> DslResult:
        context_parts = []
        if ocr_extracted:
            context_parts.append(f"OCR 提取: {json.dumps(ocr_extracted, ensure_ascii=False)}")
        if asr_extracted:
            context_parts.append(f"ASR 提取: {json.dumps(asr_extracted, ensure_ascii=False)}")

        context = "\n".join(context_parts) if context_parts else "无多模态输入"
        user_prompt = f"{context}\n\n用户输入: {user_input}"

        raw, latency = self.ollama.generate(
            model=self.model,
            prompt=user_prompt,
            system=WEIGHING_AGENT_PROMPT,
            temperature=0.1,
        )

        action, params, reasoning, confidence, requires_human = self._parse_response(raw)

        return DslResult(
            action=action or "weighing_operation",
            params=params or {},
            reasoning=reasoning or "",
            confidence=confidence or 0.0,
            requires_human_confirm=requires_human if requires_human is not None else True,
            raw_response=raw,
            latency_ms=latency,
        )

    def _parse_response(self, raw: str):
        import re

        json_match = re.search(r"```json\s*(.*?)\s*```", raw, re.DOTALL)
        json_str = json_match.group(1) if json_match else raw.strip()

        try:
            obj = json.loads(json_str)
            action = obj.get("action", "")
            params = obj.get("params", {})
            reasoning = obj.get("reasoning", "")
            confidence = float(obj.get("confidence", 0.0))
            requires_human = obj.get("requires_human_confirm")
            if requires_human is not None:
                requires_human = bool(requires_human)
            return action, params, reasoning, confidence, requires_human
        except (json.JSONDecodeError, ValueError, TypeError):
            return "", {}, "", 0.0, True

    def generate_compliance_check(
        self, entity_type: str, entity_code: str, check_type: str
    ) -> DslResult:
        prompt = f"生成 compliance_check DSL:\nentity_type={entity_type}\nentity_code={entity_code}\ncheck_type={check_type}"
        raw, latency = self.ollama.generate(self.model, prompt, system=WEIGHING_AGENT_PROMPT, temperature=0.1)
        action, params, reasoning, confidence, requires_human = self._parse_response(raw)

        if not action:
            action = "compliance_check"
            params = {"entity_type": entity_type, "entity_code": entity_code, "check_type": check_type}

        return DslResult(
            action=action, params=params, reasoning=reasoning,
            confidence=confidence, requires_human_confirm=requires_human if requires_human is not None else True,
            raw_response=raw, latency_ms=latency,
        )


def main():
    """CLI test."""
    user_input = sys.argv[1] if len(sys.argv) > 1 else "请帮我完成称量操作"
    gen = DslGenerator()
    result = gen.generate_weighing_dsl(user_input)
    print(f"Action: {result.action}")
    print(f"Confidence: {result.confidence}")
    print(f"Latency: {result.latency_ms:.0f}ms")
    print(f"Params: {json.dumps(result.params, ensure_ascii=False, indent=2)}")
    print(f"Reasoning: {result.reasoning[:200]}")


if __name__ == "__main__":
    main()

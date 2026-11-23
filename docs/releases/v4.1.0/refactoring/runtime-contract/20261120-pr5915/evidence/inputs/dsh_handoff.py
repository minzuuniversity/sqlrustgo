"""Handoff 契约：Controller ↔ Worker 之间**允许外发**的最小字段集与校验器。

裁决 §四.2 要求：必须区分"允许外发"与"禁止外发"的字段。
本模块只做**契约校验**，不改变现有 Handoff 流程；调用方决定何时强制。

设计原则：
- 白名单（allowlist）：只有列出的字段可外发；
- 每个字段有**形状约束**（类型/长度/是否哈希）；
- 任何未在白名单中的字段 ⇒ 拒绝；
- 校验失败抛出 `HandoffViolation`，**不静默降级**。
"""
from __future__ import annotations

import hashlib
import re
from dataclasses import dataclass, field
from typing import Any


class HandoffViolation(ValueError):
    """Handoff 契约违规：携带字段名与原因，便于审计。"""


# --- 白名单：允许外发给云端 Controller 的字段 -------------------------------
# 形状: (类型, 约束)
ALLOWED_FIELDS: dict[str, tuple[type, dict[str, Any]]] = {
    "task_id":        (str, {"pattern": r"^[A-Za-z0-9._-]{1,64}$"}),
    "project":        (str, {"pattern": r"^[a-z0-9][a-z0-9_-]{0,38}$"}),
    "objective_digest": (str, {"sha256_hex": True}),          # 目标文本的哈希，非原文
    "acceptance_count": (int, {"min": 0, "max": 100}),
    "acceptance_digest": (str, {"sha256_hex": True}),
    "allowed_paths_glob": (list, {"max_items": 20, "item_max_len": 128}),
    "forbidden_paths_glob": (list, {"max_items": 20, "item_max_len": 128}),
    "risk":           (str, {"enum": ["L0", "L1", "L2", "L3", "L4"]}),
    "file_count":     (int, {"min": 0, "max": 100000}),
    "line_stats":     (dict, {"keys": ["added", "removed"], "int_values": True}),
    "base_sha":       (str, {"git_sha": True}),
    "execution_mode": (str, {"enum": ["CLOUD_ASSISTED", "HYBRID_CONTROL_LOCAL_EXECUTION", "LOCAL_ONLY"]}),
    "budget_seconds": (int, {"min": 1, "max": 86400}),
    "budget_attempts": (int, {"min": 1, "max": 5}),
}

# --- 明令禁止外发（即使调用方传入也拒绝，并在异常中列出） --------------------
DENIED_FIELDS = frozenset({
    "source_code", "code", "diff", "patch", "file_content", "contents",
    "build_log", "stderr", "stdout", "test_output", "tool_output",
    "env", "environment", "credentials", "api_key", "token",
    "history", "context", "messages", "prompt", "conversation",
})

# --- 正则：检测"看起来像源码/日志"的字符串，防止伪装成允许字段 --------------
_SOURCE_PATTERNS = (
    re.compile(r"\bfn\s+\w+\s*\("),          # Rust 函数
    re.compile(r"\bpub\s+(struct|enum|trait)\b"),
    re.compile(r"\bimpl\b.*\bfor\b"),
    re.compile(r"\b(def|class)\s+\w+.*:"),   # Python
    re.compile(r"^\s*(use|import|use crate::|mod )", re.M),
    re.compile(r"error\[E\d{4}\]"),          # 编译错误
    re.compile(r"^\s*\+\s+\w+.*$", re.M),    # diff 新增行
)


@dataclass
class HandoffEnvelope:
    """通过校验的信封：只包含白名单字段。"""
    fields: dict[str, Any] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return dict(self.fields)


def sha256_hex(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def _check_field(name: str, value: Any) -> None:
    typ, cons = ALLOWED_FIELDS[name]
    if not isinstance(value, typ):
        raise HandoffViolation(f"{name}: expected {typ.__name__}, got {type(value).__name__}")
    if "pattern" in cons and typ is str and not re.fullmatch(cons["pattern"], value):
        raise HandoffViolation(f"{name}: value does not match {cons['pattern']}")
    if cons.get("sha256_hex") and typ is str and not re.fullmatch(r"[0-9a-f]{64}", value):
        raise HandoffViolation(f"{name}: not a sha256 hex digest")
    if cons.get("git_sha") and typ is str and not re.fullmatch(r"[0-9a-f]{40}", value):
        raise HandoffViolation(f"{name}: not a 40-char git sha")
    if "enum" in cons and value not in cons["enum"]:
        raise HandoffViolation(f"{name}: value not in {cons['enum']}")
    if typ is int:
        if "min" in cons and value < cons["min"]:
            raise HandoffViolation(f"{name}: below min {cons['min']}")
        if "max" in cons and value > cons["max"]:
            raise HandoffViolation(f"{name}: above max {cons['max']}")
    if typ is list:
        if len(value) > cons.get("max_items", 10**9):
            raise HandoffViolation(f"{name}: too many items ({len(value)})")
        for it in value:
            if not isinstance(it, str):
                raise HandoffViolation(f"{name}: items must be str")
            if len(it) > cons.get("item_max_len", 10**9):
                raise HandoffViolation(f"{name}: item too long")
    if typ is dict and cons.get("keys"):
        extra = set(value) - set(cons["keys"])
        if extra:
            raise HandoffViolation(f"{name}: unexpected keys {sorted(extra)}")
        if cons.get("int_values") and not all(isinstance(v, int) for v in value.values()):
            raise HandoffViolation(f"{name}: values must be int")


def _looks_like_sensitive_content(value: Any) -> str | None:
    """在允许字段中检测伪装的源码/日志（防御性；返回命中的模式描述）。

    支持 str 与 list[str]（如 allowed_paths_glob 的每一项）。
    """
    items: list[str] = []
    if isinstance(value, str):
        items = [value]
    elif isinstance(value, list):
        items = [x for x in value if isinstance(x, str)]
    else:
        return None
    for text in items:
        for pat in _SOURCE_PATTERNS:
            if pat.search(text):
                return pat.pattern
    return None


def validate(payload: dict[str, Any], *, strict: bool = True) -> HandoffEnvelope:
    """校验并返回只含白名单字段的信封。

    strict=True（默认）：危险。
      - 出现 DENIED_FIELDS 中的键 ⇒ 立即抛出
      - 出现未在白名单中的键 ⇒ 立即抛出
      - 允许字段的值"看起来像源码" ⇒ 抛出
      - 全部通过 ⇒ 才返回信封
    """
    if not isinstance(payload, dict):
        raise HandoffViolation(f"payload must be dict, got {type(payload).__name__}")

    denied = sorted(set(payload) & DENIED_FIELDS)
    if denied and strict:
        raise HandoffViolation(f"denied fields present: {denied}")

    unknown = sorted(set(payload) - set(ALLOWED_FIELDS) - DENIED_FIELDS)
    if unknown and strict:
        raise HandoffViolation(f"fields not in allowlist: {unknown}")

    env = HandoffEnvelope()
    for k, v in payload.items():
        if k in DENIED_FIELDS:
            continue          # 非 strict 时静默剔除（但仍不进入信封）
        if k not in ALLOWED_FIELDS:
            continue
        _check_field(k, v)
        hit = _looks_like_sensitive_content(v)
        if hit and strict:
            raise HandoffViolation(f"{k}: value looks like source/log (pattern {hit!r})")
        env.fields[k] = v
    return env


def describe_contract() -> dict[str, Any]:
    """供文档和测试引用：当前契约的机器可读描述。"""
    return {
        "allowed": {k: {"type": v[0].__name__, **v[1]} for k, v in ALLOWED_FIELDS.items()},
        "denied": sorted(DENIED_FIELDS),
        "strict_default": True,
    }

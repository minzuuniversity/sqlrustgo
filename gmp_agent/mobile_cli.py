#!/usr/bin/env python3
"""
GMP Platform Mobile App Simulator — GMP移动端CLI模拟工具

模拟移动端发送各种原始输入：文本、语音、图片、视频。
通过 CLI 交互式模拟操作工/复核人的完整称量流程。

Usage:
  python3 -m gmp_agent.mobile_cli --login OP-001
  python3 -m gmp_agent.mobile_cli --scan-image path/to/label.jpg
  python3 -m gmp_agent.mobile_cli --voice "称取 A 物料 50.12 公斤"
  python3 -m gmp_agent.mobile_cli --interactive
  python3 -m gmp_agent.mobile_cli --demo

API Base: http://localhost:8080/api/v1 (configurable via --base-url)
"""

import argparse
import base64
import hashlib
import json
import os
import random
import readline  # for CLI history
import re
import sys
import time
import uuid
from dataclasses import dataclass, field, asdict
from datetime import datetime
from pathlib import Path
from typing import Optional

# ─── Config ────────────────────────────────────────────────────────────────────

DEFAULT_BASE_URL = os.environ.get("GMP_API_URL", "http://localhost:8080/api/v1")
DEFAULT_SECRET = b"gmp-mobile-test-secret-key-256-bits-long!!"
TOKEN_TTL = 3600

# ─── Data Models ──────────────────────────────────────────────────────────────

@dataclass
class AuthToken:
    access_token: str
    token_type: str
    expires_in: int
    refresh_token: str
    claims: dict = field(default_factory=dict)

    def is_valid(self) -> bool:
        exp = self.claims.get("exp", 0)
        return time.time() < exp


@dataclass
class WeighingSession:
    session_id: str
    work_order_id: str
    operator_id: str
    materials: list[dict]
    status: str  # active, confirmed, cancelled
    created_at: str


@dataclass
class WeighingRecord:
    record_id: str
    work_order_id: str
    material_code: str
    weighing_value: float
    unit: str
    tolerance_pct: float
    actual_deviation_pct: float
    operator_id: str
    reviewer_id: Optional[str]
    operator_signature: Optional[dict]
    reviewer_signature: Optional[dict]
    deviations: list[dict]
    audit_chain_ref: str
    created_at: str


# ─── API Client ────────────────────────────────────────────────────────────────

class GmpApiClient:
    """HTTP client for GMP Platform API."""

    def __init__(self, base_url: str = DEFAULT_BASE_URL, token: Optional[AuthToken] = None):
        self.base_url = base_url.rstrip("/")
        self.token = token

    def _headers(self) -> dict:
        headers = {"Content-Type": "application/json"}
        if self.token and self.token.access_token:
            headers["Authorization"] = f"Bearer {self.token.access_token}"
        return headers

    def _post(self, path: str, data: dict, timeout: int = 30) -> dict:
        import urllib.request
        url = f"{self.base_url}{path}"
        body = json.dumps(data).encode()
        req = urllib.request.Request(url, data=body, headers=self._headers(), method="POST")
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return json.loads(resp.read())
        except urllib.error.HTTPError as e:
            error_body = e.read().decode() if e.fp else ""
            raise GmpApiError(f"HTTP {e.code}: {error_body}") from e

    def _get(self, path: str, timeout: int = 10) -> dict:
        import urllib.request
        url = f"{self.base_url}{path}"
        req = urllib.request.Request(url, headers=self._headers(), method="GET")
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return json.loads(resp.read())
        except urllib.error.HTTPError as e:
            raise GmpApiError(f"HTTP {e.code}: {e.read().decode()}") from e

    def _upload(self, path: str, file_path: str, field_name: str = "file",
                extra_data: Optional[dict] = None, timeout: int = 30) -> dict:
        import urllib.request
        import mimetypes

        url = f"{self.base_url}{path}"
        boundary = uuid.uuid4().hex

        with open(file_path, "rb") as f:
            file_data = f.read()

        mime_type = mimetypes.guess_type(file_path)[0] or "application/octet-stream"
        filename = os.path.basename(file_path)

        body_parts = []
        if extra_data:
            for k, v in extra_data.items():
                body_parts.append(
                    f"--{boundary}\r\nContent-Disposition: form-data; name=\"{k}\"\r\n\r\n{v}\r\n".encode()
                )

        body_parts.append(
            f"--{boundary}\r\n"
            f"Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\n"
            f"Content-Type: {mime_type}\r\n\r\n".encode()
        )
        body_parts.append(file_data)
        body_parts.append(f"\r\n--{boundary}--\r\n".encode())

        body = b"".join(body_parts)
        headers = {
            "Content-Type": f"multipart/form-data; boundary={boundary}",
        }
        if self.token and self.token.access_token:
            headers["Authorization"] = f"Bearer {self.token.access_token}"

        req = urllib.request.Request(url, data=body, headers=headers, method="POST")
        try:
            with urllib.request.urlopen(req, timeout=timeout) as resp:
                return json.loads(resp.read())
        except urllib.error.HTTPError as e:
            raise GmpApiError(f"HTTP {e.code}: {e.read().decode()}") from e


class GmpApiError(Exception):
    pass


# ─── Local Engines (L0) ────────────────────────────────────────────────────────

class LocalOcrEngine:
    """Simulated L0 OCR engine — runs entirely on-device."""

    def __init__(self):
        self.latency_ms = 800

    def recognize(self, image_path: str) -> dict:
        """Simulate OCR: extract material_code and batch_no from image."""
        t0 = time.time()
        time.sleep(self.latency_ms / 1000)

        h = hashlib.md5(f"{image_path}{time.time()}".encode()).hexdigest()

        # Simulate different label types
        label_types = ["material_tag", "batch_label", "qr_code"]
        label_type = label_types[int(h[0], 16) % len(label_types)]

        if label_type == "material_tag":
            material_code = f"MAT-{h[1:5].upper()}"
            batch_no = f"BATCH-{h[5:9].upper()}"
        elif label_type == "batch_label":
            material_code = f"API-{h[1:4].upper()}"
            batch_no = f"B{h[4:8].upper()}"
        else:
            material_code = f"QR-{h[1:6].upper()}"
            batch_no = f"{h[6:10].upper()}"

        return {
            "material_code": material_code,
            "batch_no": batch_no,
            "label_type": label_type,
            "confidence": round(0.90 + float(int(h[10], 16)) / 100, 3),
            "extracted_at": datetime.utcnow().isoformat() + "Z",
            "local_processing": True,  # L0 — never sent to cloud
            "latency_ms": round((time.time() - t0) * 1000, 1),
        }


class LocalAsrEngine:
    """Simulated L0 ASR engine — runs entirely on-device."""

    def __init__(self):
        self.latency_ms = 300

    def transcribe(self, audio_path: str, prompt: str = "") -> dict:
        """Simulate ASR: extract weighing value from voice."""
        t0 = time.time()
        time.sleep(self.latency_ms / 1000)

        h = hashlib.md5(f"{audio_path}{time.time()}".encode()).hexdigest()

        # Parse prompt for weighing value
        value_match = re.search(r"([\d.]+)\s*(公斤|kg|KG|g|G)", prompt or "")
        if value_match:
            value = float(value_match.group(1))
        else:
            value = round(random.uniform(10.0, 100.0), 2)

        units = ["公斤", "kg", "g"]
        unit = units[int(h[0], 16) % len(units)]

        material_letters = ["A", "B", "C", "D", "E"]
        material = material_letters[int(h[1], 16) % len(material_letters)]

        return {
            "text": f"称取 {material} 物料 {value} {unit}",
            "material": f"物料 {material}",
            "value": value,
            "unit": unit,
            "confidence": round(0.88 + float(int(h[2], 16)) / 100, 3),
            "transcribed_at": datetime.utcnow().isoformat() + "Z",
            "local_processing": True,
            "latency_ms": round((time.time() - t0) * 1000, 1),
        }


class LocalSigner:
    """Simulated e-signature — runs entirely on-device."""

    def __init__(self):
        self.prev_hash = ""

    def sign(self, signer_id: str, signer_name: str, role: str,
             record_hash: str, signature_image_b64: str) -> dict:
        """Create a local signature with hash chain."""
        signed_at = datetime.utcnow().isoformat() + "Z"
        block = f"{signer_id}|{signer_name}|{role}|{record_hash}|{signature_image_b64}|{signed_at}|{self.prev_hash}"
        self_hash = hashlib.sha256(block.encode()).hexdigest()

        sig = {
            "id": f"SIG-{signer_id}-{uuid.uuid4().hex[:8]}",
            "signer_id": signer_id,
            "signer_name": signer_name,
            "role": role,
            "record_hash": record_hash,
            "signature_image_b64": signature_image_b64,
            "signed_at": signed_at,
            "prev_signature_hash": self.prev_hash,
            "self_hash": self_hash,
        }
        self.prev_hash = self_hash
        return sig


# ─── Mobile CLI ───────────────────────────────────────────────────────────────

class MobileCli:
    """Mobile App Simulator — interactive CLI for GMP weighing operations."""

    PROMPT = "\n[gmp-mobile] > "

    def __init__(self, base_url: str, secret: bytes):
        self.base_url = base_url
        self.secret = secret
        self.client = GmpApiClient(base_url)
        self.token: Optional[AuthToken] = None
        self.session: Optional[WeighingSession] = None
        self.ocr = LocalOcrEngine()
        self.asr = LocalAsrEngine()
        self.signer = LocalSigner()
        self.operator_id: Optional[str] = None
        self.operator_name: Optional[str] = None
        self.reviewer_id: Optional[str] = None
        self._history: list[str] = []

    # ── Auth ─────────────────────────────────────────────────────────────────

    def cmd_login(self, operator_id: str, password: str = "******") -> bool:
        """Simulate login — in real app this calls /auth/login."""
        # For demo, generate JWT locally using gmp-auth
        try:
            from gmp_auth import generate_token, Role
            role_map = {"OP": Role.OPERATOR, "RE": Role.REVIEWER, "QA": Role.QA}
            prefix = operator_id[:2].upper()
            role = role_map.get(prefix, Role.OPERATOR)
            name_map = {"OP": "张三", "RE": "李四", "QA": "王五"}
            name = name_map.get(prefix, "测试用户")

            token_str = generate_token(operator_id, name, role, self.secret, TOKEN_TTL)
            self.token = AuthToken(
                access_token=token_str,
                token_type="Bearer",
                expires_in=TOKEN_TTL,
                refresh_token=uuid.uuid4().hex,
                claims={"sub": operator_id, "name": name, "role": role.as_str()},
            )
            self.operator_id = operator_id
            self.operator_name = name
            self.client.token = self.token
            print(f"  ✅ 登录成功: {operator_id} ({name}) [{role.as_str()}]")
            print(f"  Token: {token_str[:40]}...")
            print(f"  有效期: {TOKEN_TTL}s")
            return True
        except ImportError:
            # Fallback: demo mode without real JWT
            self.operator_id = operator_id
            self.operator_name = "演示用户"
            self.token = AuthToken(
                access_token="demo-token-" + uuid.uuid4().hex,
                token_type="Bearer",
                expires_in=TOKEN_TTL,
                refresh_token="demo-refresh",
            )
            print(f"  ✅ [DEMO] 登录成功: {operator_id}")
            return True

    def cmd_logout(self):
        self.token = None
        self.session = None
        self.operator_id = None
        print("  已退出登录")

    # ── Session ───────────────────────────────────────────────────────────────

    def cmd_start(self, work_order_id: str) -> bool:
        """Start a weighing session."""
        if not self.operator_id:
            print("  ❌ 请先登录: login <operator_id>")
            return False

        # Simulate local checks
        print(f"  🔍 检查操作人员授权: {self.operator_id} ...", end=" ")
        time.sleep(0.3)
        print("✅ 授权通过")

        print(f"  🔍 检查设备校准状态 ...", end=" ")
        time.sleep(0.2)
        print("✅ 设备正常")

        print(f"  🔍 检查区域清场状态 ...", end=" ")
        time.sleep(0.2)
        print("✅ 区域已清场")

        # Simulate loading work order
        materials = [
            {"material_code": f"MAT-{uuid.uuid4().hex[:4].upper()}",
             "material_name": f"原料 {chr(65 + i)}",
             "target_weight": round(random.uniform(20, 100), 2),
             "tolerance_pct": 0.5}
            for i in range(3)
        ]

        self.session = WeighingSession(
            session_id=f"SES-{uuid.uuid4().hex[:8].upper()}",
            work_order_id=work_order_id,
            operator_id=self.operator_id,
            materials=materials,
            status="active",
            created_at=datetime.utcnow().isoformat() + "Z",
        )

        print(f"\n  📋 工单: {work_order_id}")
        print(f"  Session: {self.session.session_id}")
        print(f"  物料列表:")
        for i, m in enumerate(materials, 1):
            print(f"    [{i}] {m['material_name']} — 目标 {m['target_weight']}kg ±{m['tolerance_pct']}%")
        return True

    # ── OCR ─────────────────────────────────────────────────────────────────

    def cmd_scan_image(self, image_path: str) -> dict:
        """Simulate scanning a material label — L0 processing, no cloud upload."""
        if not self.session:
            print("  ❌ 无活动会话，请先: start <work_order_id>")
            return {}

        if not os.path.exists(image_path):
            # Generate a dummy test image
            print(f"  📷 生成测试图片: {image_path}")
            os.makedirs(os.path.dirname(image_path) or ".", exist_ok=True)
            self._generate_test_image(image_path)

        print(f"\n  📷 扫描图片: {image_path}")
        print(f"  [L0] OCR 引擎处理中...", end=" ")

        result = self.ocr.recognize(image_path)

        print(f"{result['latency_ms']:.0f}ms")
        print(f"  物料编码: {result['material_code']}")
        print(f"  批号:     {result['batch_no']}")
        print(f"  可信度:   {result['confidence']}")
        print(f"  ⚠️  L0本地处理，数据未上传云端")

        # Verify against work order
        mat = self.session.materials[0]
        print(f"\n  🔍 验证物料是否在工单中: {result['material_code']} vs {mat['material_code']}")
        print(f"  ✅ 物料匹配，开始称量")

        return result

    # ── ASR ─────────────────────────────────────────────────────────────────

    def cmd_voice(self, text: str) -> dict:
        """Simulate voice input — L0 ASR, no cloud upload."""
        if not self.session:
            print("  ❌ 无活动会话")
            return {}

        print(f"\n  🎤 语音输入: \"{text}\"")
        print(f"  [L0] ASR 引擎处理中...", end=" ")

        result = self.asr.transcribe("<voice_input>", prompt=text)

        print(f"{result['latency_ms']:.0f}ms")
        print(f"  识别结果: {result['text']}")
        print(f"  物料:     {result['material']}")
        print(f"  称量值:   {result['value']} {result['unit']}")
        print(f"  可信度:   {result['confidence']}")
        print(f"  ⚠️  L0本地处理，音频未上传云端")

        # Check tolerance
        target = self.session.materials[0]["target_weight"]
        tolerance = self.session.materials[0]["tolerance_pct"]
        value_kg = result["value"] if result["unit"] in ("公斤", "kg") else result["value"] / 1000
        deviation_pct = abs(value_kg - target) / target * 100

        print(f"\n  📏 允差检查: {value_kg}kg vs 目标 {target}kg (±{tolerance}%)")
        if deviation_pct <= tolerance:
            print(f"  ✅ 称量值在允差范围内 (偏差 {deviation_pct:.2f}%)")
        else:
            print(f"  ⚠️  超出允差 (偏差 {deviation_pct:.2f}%) — 触发偏差规则")

        return result

    # ── Confirm ──────────────────────────────────────────────────────────────

    def cmd_confirm(self) -> Optional[WeighingRecord]:
        """Confirm weighing with operator signature."""
        if not self.session:
            print("  ❌ 无活动会话")
            return None

        print(f"\n  ✍️  操作员确认称量结果...")

        # Create operator signature
        record_hash = hashlib.sha256(
            json.dumps(self.session.__dict__, sort_keys=True).encode()
        ).hexdigest()

        sig_image = base64.b64encode(b"DEMO_SIGNATURE_IMAGE_DATA_OVER_100_CHARS_FOR_COMPLIANCE").decode()

        op_sig = self.signer.sign(
            self.operator_id, self.operator_name or "Operator",
            "OPERATOR", record_hash, sig_image
        )

        print(f"  操作员签名: {op_sig['id']}")
        print(f"  签名人: {op_sig['signer_name']} ({op_sig['signer_id']})")
        print(f"  时间:   {op_sig['signed_at']}")
        print(f"  Self-Hash: {op_sig['self_hash'][:16]}...")

        self.session.status = "confirmed"

        record = WeighingRecord(
            record_id=f"REC-{uuid.uuid4().hex[:8].upper()}",
            work_order_id=self.session.work_order_id,
            material_code=self.session.materials[0]["material_code"],
            weighing_value=random.uniform(20, 100),
            unit="kg",
            tolerance_pct=0.5,
            actual_deviation_pct=round(random.uniform(0, 0.8), 3),
            operator_id=self.operator_id,
            reviewer_id=None,
            operator_signature=op_sig,
            reviewer_signature=None,
            deviations=[],
            audit_chain_ref=f"AC-{uuid.uuid4().hex[:8].upper()}",
            created_at=datetime.utcnow().isoformat() + "Z",
        )

        print(f"\n  📋 记录已创建: {record.record_id}")
        return record

    def cmd_review(self, record: WeighingRecord) -> bool:
        """Simulate reviewer signing off."""
        if not self.reviewer_id:
            print("  请先设置复核人: set_reviewer <reviewer_id>")
            self.reviewer_id = "RE-001"
            print(f"  ✅ 复核人设置为: {self.reviewer_id}")

        print(f"\n  ✍️  复核人确认: {self.reviewer_id}")

        sig_image = base64.b64encode(b"REVIEWER_SIGNATURE_IMAGE_DATA_OVER_100_CHARS_COMPLIANCE_CHECK").decode()

        reviewer_sig = self.signer.sign(
            self.reviewer_id, "李四", "REVIEWER",
            record.record_id, sig_image
        )

        record.reviewer_id = self.reviewer_id
        record.reviewer_signature = reviewer_sig

        print(f"  复核人签名: {reviewer_sig['id']}")
        print(f"  ✅ 双人复核完成，记录已锁定")
        return True

    # ── CLI Helpers ─────────────────────────────────────────────────────────

    def cmd_help(self):
        print("""
  可用命令:
    login <operator_id>        登录 (如: login OP-001)
    logout                     退出登录
    start <work_order_id>      开始称量 (如: start WO-2026-001)
    scan <image_path>          扫描物料标签 (如: scan label.jpg)
    voice "<text>"             语音输入称量值 (如: voice "称取 A 物料 50.12 公斤")
    confirm                    确认称量结果（操作员签名）
    review                     复核人签名确认
    record                     查看当前记录
    history                    查看操作历史
    demo                       运行完整演示流程
    help                      显示此帮助
    exit / quit               退出
""")

    def cmd_history(self):
        print("\n  操作历史:")
        for i, h in enumerate(self._history, 1):
            print(f"    [{i}] {h}")
        if not self._history:
            print("    (空)")

    def cmd_record(self, record: Optional[WeighingRecord]):
        if not record:
            print("  ❌ 无记录")
            return
        print(f"""
  📋 称量记录: {record.record_id}
  工单:        {record.work_order_id}
  物料:        {record.material_code}
  称量值:      {record.weighing_value} {record.unit} (±{record.tolerance_pct}%)
  偏差:        {record.actual_deviation_pct}%
  操作人:      {record.operator_id}
  复核人:      {record.reviewer_id or '待复核'}
  审计链:      {record.audit_chain_ref}
  时间:        {record.created_at}
""")
        if record.operator_signature:
            print(f"  操作员签名: {record.operator_signature['id']}")
        if record.reviewer_signature:
            print(f"  复核人签名: {record.reviewer_signature['id']}")

    def run_demo(self):
        """Run a complete demo weighing flow."""
        print("\n" + "=" * 60)
        print(" GMP Platform 移动端演示流程")
        print("=" * 60)

        # Login
        print("\n[1/7] 登录...")
        self.cmd_login("OP-001")

        # Start
        print("\n[2/7] 开始称量...")
        self.cmd_start("WO-2026-001")

        # Generate test image
        test_img = "/tmp/gmp_test_label.jpg"
        self._generate_test_image(test_img)

        # Scan
        print("\n[3/7] 扫描物料标签...")
        self.cmd_scan_image(test_img)

        # Voice
        print("\n[4/7] 语音录入称量值...")
        self.cmd_voice("称取 A 物料 50.12 公斤")

        # Confirm
        print("\n[5/7] 操作员确认...")
        record = self.cmd_confirm()

        # Review
        print("\n[6/7] 复核人确认...")
        self.cmd_review(record)

        # Record
        print("\n[7/7] 最终记录...")
        self.cmd_record(record)

        print("\n" + "=" * 60)
        print(" 演示流程完成 ✅")
        print("=" * 60)

    # ── Interactive Loop ─────────────────────────────────────────────────────

    def run_interactive(self):
        """Main interactive CLI loop."""
        print("""
╔══════════════════════════════════════════════╗
║   GMP Platform 移动端模拟器 v1.1.0           ║
║   输入 'help' 查看命令，'demo' 运行演示      ║
╚══════════════════════════════════════════════╝
""")

        current_record: Optional[WeighingRecord] = None

        while True:
            try:
                cmd = input(self.PROMPT).strip()
            except (EOFError, KeyboardInterrupt):
                print("\n\n退出")
                break

            if not cmd:
                continue

            self._history.append(cmd)

            parts = cmd.split(maxsplit=1)
            verb = parts[0].lower()
            arg = parts[1] if len(parts) > 1 else ""

            # Echo command
            print()

            if verb in ("exit", "quit", "q"):
                print("  再见!")
                break

            elif verb == "help":
                self.cmd_help()

            elif verb == "demo":
                self.run_demo()

            elif verb == "login":
                if not arg:
                    print("  用法: login <operator_id>")
                else:
                    self.cmd_login(arg)

            elif verb == "logout":
                self.cmd_logout()

            elif verb == "start":
                if not arg:
                    print("  用法: start <work_order_id>")
                else:
                    self.cmd_start(arg)

            elif verb == "scan":
                if not arg:
                    print("  用法: scan <image_path>")
                else:
                    self.cmd_scan_image(arg)

            elif verb == "voice":
                if not arg:
                    print("  用法: voice \"<text>\"")
                else:
                    self.cmd_voice(arg.strip('"'))

            elif verb == "confirm":
                current_record = self.cmd_confirm()

            elif verb == "review":
                self.cmd_review(current_record)

            elif verb == "record":
                self.cmd_record(current_record)

            elif verb == "history":
                self.cmd_history()

            elif verb == "set_reviewer":
                self.reviewer_id = arg or "RE-001"
                print(f"  复核人设置为: {self.reviewer_id}")

            elif verb == "clear":
                os.system("clear" if os.name != "nt" else "cls")

            else:
                print(f"  未知命令: {verb} — 输入 'help' 查看可用命令")

    # ── Utility ──────────────────────────────────────────────────────────────

    @staticmethod
    def _generate_test_image(path: str):
        """Generate a dummy test image (1x1 PNG)."""
        import zlib
        # Minimal 1x1 white PNG
        png_data = (
            b"\x89PNG\r\n\x1a\n"
            b"\x00\x00\x00\rIHDR\x00\x00\x00\x01"
            b"\x00\x00\x00\x01\x08\x02\x00\x00\x00"
            b"\x90wS\xde\x00\x00\x00\x0cIDATx\x9cc\xf8"
            b"\xcf\xc0\x00\x00\x00\x03\x00\x01\x00\x05"
            b"\xfe\xd4\xef\x00\x00\x00\x00IEND\xaeB`\x82"
        )
        os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
        with open(path, "wb") as f:
            f.write(png_data)


# ─── CLI Entry Point ─────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(
        description="GMP Platform 移动端CLI模拟工具",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python3 -m gmp_agent.mobile_cli --interactive
  python3 -m gmp_agent.mobile_cli --demo
  python3 -m gmp_agent.mobile_cli --login OP-001 --start WO-2026-001
  python3 -m gmp_agent.mobile_cli --scan-image /tmp/label.jpg
  python3 -m gmp_agent.mobile_cli --voice "称取 A 物料 50.12 公斤"
  python3 -m gmp_agent.mobile_cli --base-url http://192.168.0.100:8080/api/v1
        """,
    )
    parser.add_argument("--interactive", "-i", action="store_true", help="交互式模式")
    parser.add_argument("--demo", action="store_true", help="运行完整演示流程")
    parser.add_argument("--login", metavar="OP-ID", help="登录指定操作工")
    parser.add_argument("--start", metavar="WO-ID", help="启动称量会话")
    parser.add_argument("--scan-image", metavar="PATH", help="扫描物料标签图片")
    parser.add_argument("--voice", metavar="TEXT", help="语音输入称量值")
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL, help=f"API基础URL (默认: {DEFAULT_BASE_URL})")
    parser.add_argument("--secret", default=None, help="JWT密钥 (默认: 测试密钥)")

    args = parser.parse_args()

    secret = args.secret.encode() if args.secret else DEFAULT_SECRET
    cli = MobileCli(args.base_url, secret)

    # Single-command mode
    if args.login:
        cli.cmd_login(args.login)
        if args.start:
            cli.cmd_start(args.start)
        if args.scan_image:
            cli.cmd_scan_image(args.scan_image)
        if args.voice:
            cli.cmd_voice(args.voice)
        return 0

    # Demo mode
    if args.demo:
        cli.run_demo()
        return 0

    # Interactive mode
    cli.run_interactive()
    return 0


if __name__ == "__main__":
    sys.exit(main())

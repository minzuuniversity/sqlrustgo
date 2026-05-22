#!/usr/bin/env python3
"""
GMP-Platform Web Dashboard Server
Self-contained HTTP server - no external dependencies beyond stdlib.
Serves static HTML + proxies to gmp-retrieve CLI.
"""

import json
import os
import re
import subprocess
import time
import urllib.parse
from http.server import HTTPServer, BaseHTTPRequestHandler
from pathlib import Path

# Config
PORT = 8080
GMP_DIR = Path("/home/openclaw/GMP-Platform")
GMP_CLI = GMP_DIR / "target/release/gmp-retrieve"
GMP_DB = GMP_DIR / "data/gmp.db"
INDEX_HTML = Path(__file__).parent / "index.html"

# In-memory stats
STATS = {
    "total_queries": 0,
    "total_time_ms": 0,
    "db_size_mb": 0.0,
    "start_time": time.time(),
}

def update_db_size():
    try:
        if GMP_DB.exists():
            STATS["db_size_mb"] = round(GMP_DB.stat().st_size / 1024 / 1024, 2)
    except:
        pass
update_db_size()


class DashboardHandler(BaseHTTPRequestHandler):
    """Custom HTTP handler with routing."""

    def log_message(self, fmt, *args):
        """Override to suppress default logging."""
        pass

    def send_json(self, data, code=200):
        body = json.dumps(data, ensure_ascii=False).encode("utf-8")
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", len(body))
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        self.wfile.write(body)

    def send_html(self, html_bytes, code=200):
        self.send_response(code)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Content-Length", len(html_bytes))
        self.send_header("Cache-Control", "no-cache")
        self.end_headers()
        self.wfile.write(html_bytes)

    def send_static(self, path, content_type):
        try:
            file_path = Path(__file__).parent / path
            if not file_path.is_file():
                self.send_error(404, f"Not Found: {path}")
                return
            body = file_path.read_bytes()
            self.send_response(200)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", len(body))
            self.send_header("Cache-Control", "public, max-age=3600")
            self.end_headers()
            self.wfile.write(body)
        except Exception as e:
            self.send_error(500, str(e))

    def parse_query(self):
        """Parse query string from URL."""
        parsed = urllib.parse.urlparse(self.path)
        return dict(urllib.parse.parse_qsl(parsed.query))

    def do_GET(self):
        parsed = urllib.parse.urlparse(self.path)

        # API: /api/search
        if parsed.path == "/api/search":
            self.handle_search()
            return

        # API: /api/stats
        if parsed.path == "/api/stats":
            self.handle_stats()
            return

        # API: /api/audit
        if parsed.path == "/api/audit":
            self.handle_audit()
            return

        # Serve index.html for root
        if parsed.path in ("/", "/index.html", "/dashboard"):
            self.send_html(INDEX_HTML.read_bytes())
            return

        # Static assets
        if parsed.path.startswith("/static/"):
            ext = os.path.splitext(parsed.path)[1]
            ct = {"css": "text/css", "js": "application/javascript",
                  "png": "image/png", "svg": "image/svg+xml"}.get(ext.lstrip("."), "text/plain")
            self.send_static(parsed.path.lstrip("/"), ct)
            return

        # Not found
        self.send_error(404)

    def do_POST(self):
        self.send_error(405)

    def handle_search(self):
        params = self.parse_query()
        query = params.get("q", "").strip()
        channel = params.get("channel", "all")
        rerank = params.get("rerank", "false").lower() == "true"

        if not query:
            self.send_json({"success": False, "error": "Empty query"})
            return

        start = time.time()

        # Build CLI command
        cmd = [
            str(GMP_CLI),
            "search",
            "--channel", channel,
            "--db", str(GMP_DB),
        ]
        if rerank:
            cmd.append("--rerank")
        cmd.extend(["--", query])

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True,
                timeout=30,
                cwd=str(GMP_DIR)
            )
            elapsed_ms = int((time.time() - start) * 1000)

            # Update stats
            STATS["total_queries"] += 1
            STATS["total_time_ms"] += elapsed_ms

            output_lines = result.stdout.strip().split("\n") if result.stdout else []

            self.send_json({
                "success": True,
                "query": query,
                "channel": channel,
                "rerank": rerank,
                "elapsed_ms": elapsed_ms,
                "returncode": result.returncode,
                "stdout": result.stdout[:3000],
                "stderr": result.stderr[:500],
                "lines": output_lines[:20],
            })

        except subprocess.TimeoutExpired:
            self.send_json({"success": False, "error": "Timeout (30s)"}, code=504)
        except FileNotFoundError:
            self.send_json({"success": False, "error": "gmp-retrieve CLI not found"}, code=500)
        except Exception as e:
            self.send_json({"success": False, "error": str(e)}, code=500)

    def handle_stats(self):
        update_db_size()
        uptime = int(time.time() - STATS["start_time"])
        avg_ms = (
            STATS["total_time_ms"] // STATS["total_queries"]
            if STATS["total_queries"] > 0 else 0
        )

        self.send_json({
            "total_queries": STATS["total_queries"],
            "avg_response_ms": avg_ms,
            "db_size_mb": STATS["db_size_mb"],
            "uptime_seconds": uptime,
            "platform_version": "v1.1.0",
            "target_version": "v1.2.0",
        })

    def handle_audit(self):
        # Return demo audit trail - in v1.2.0 this will read from actual audit DB
        self.send_json({
            "audit_enabled": True,
            "message": "Audit API v1.2.0 - integrate with gmp-auth audit chain",
            "entries": [
                {
                    "id": "WE-20260522-001",
                    "time": "2026-05-22T14:32:18+08:00",
                    "action": "工单创建",
                    "user": "operator_zhang",
                    "detail": "称量任务 #WE-001 创建成功",
                    "signature": "ZhangSan#sha256:abc123..."
                },
                {
                    "id": "WE-20260522-001",
                    "time": "2026-05-22T14:33:12+08:00",
                    "action": "规则校验",
                    "user": "system",
                    "detail": "DSL: weigh_material | 校验通过",
                    "signature": None
                },
                {
                    "id": "WE-20260522-001",
                    "time": "2026-05-22T14:33:45+08:00",
                    "action": "主管审批",
                    "user": "supervisor_wang",
                    "detail": "审批通过 | 双签第一级",
                    "signature": "WangWu#sha256:def456..."
                },
            ]
        })


def main():
    server = HTTPServer(("0.0.0.0", PORT), DashboardHandler)
    print(f"GMP-Platform Dashboard starting on http://0.0.0.0:{PORT}")
    print(f"Serving: {INDEX_HTML}")
    print(f"CLI: {GMP_CLI}")
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        print("\nShutting down...")
        server.shutdown()


if __name__ == "__main__":
    main()

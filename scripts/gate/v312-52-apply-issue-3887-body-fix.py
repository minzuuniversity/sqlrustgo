#!/usr/bin/env python3
"""安全更新 #3887 body：通过 git credential helper 获取 token，修改 3 处 line，然后 PATCH。"""
import json
import subprocess
import sys
import urllib.request
import urllib.error
import base64

# 1. 从 git credential 获取密码
proc = subprocess.run(
    ["git", "credential", "fill"],
    input="url=http://192.168.0.252:3000/openclaw/sqlrustgo.git\n",
    capture_output=True, text=True, check=True
)
creds = {}
for line in proc.stdout.strip().split("\n"):
    if "=" in line:
        k, v = line.split("=", 1)
        creds[k] = v

username = creds.get("username")
password = creds.get("password")
if not username or not password:
    print("ERROR: cannot get credentials", file=sys.stderr)
    sys.exit(1)

# 2. 获取当前 body
auth_header = "Basic " + base64.b64encode(f"{username}:{password}".encode()).decode()
req = urllib.request.Request(
    "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/3887",
    headers={"Authorization": auth_header, "Accept": "application/json"}
)
try:
    with urllib.request.urlopen(req) as resp:
        issue = json.loads(resp.read())
except urllib.error.HTTPError as e:
    print(f"GET error: {e.code} {e.reason}", file=sys.stderr)
    sys.exit(1)

body = issue["body"]
print(f"当前 body: {len(body)} chars, {len(body.splitlines())} lines")

# 3. 验证需要修改的行存在
required_lines = [
    "V312 主任务已关闭：16 项",
    "V312 主任务仍 open：8 项",
    "[ ] #3911 [V312-24] Test Infrastructure Activation",
]
for required in required_lines:
    if required not in body:
        print(f"ERROR: required line not found: {required!r}", file=sys.stderr)
        sys.exit(1)

# 4. 精确修改 3 处
old_body = body

# L29: 16 项 -> 17 项，添加 #3911
body = body.replace(
    "V312 主任务已关闭：16 项（#3888-#3897、#3899、#3900、#3901、#3902、#3906、#3908）。",
    "V312 主任务已关闭：17 项（#3888-#3897、#3899、#3900、#3901、#3902、#3906、#3908、#3911）。"
)

# L30: 8 项 -> 7 项，从 open 列表移除 #3911
body = body.replace(
    "V312 主任务仍 open：8 项（#3898、#3903、#3904、#3905、#3907、#3909、#3910、#3911）。",
    "V312 主任务仍 open：7 项（#3898、#3903、#3904、#3905、#3907、#3909、#3910）。"
)

# L59: [ ] #3911 -> [x] #3911 + 更新描述（带 PR #3950 实际 merge 信息）
body = body.replace(
    "- [ ] #3911 [V312-24] Test Infrastructure Activation — 当前 open；V312-24 工具链已能实跑通过，但 Issue body、#3887 勾选和 anti-fab CHECK 1.5 声明仍需同步修正后再关闭。",
    "- [x] #3911 [V312-24] Test Infrastructure Activation — Gitea 当前 closed via PR #3950 (merged 2026-08-09T14:33:16Z)；V312-50 完成 codex #88807 4-item reconciliation（#3911 body + anti-fab CHECK 1.5 + known list cleanup）。"
)

# L23: 更新最后同步时间
body = body.replace(
    "## 严格复核结论（2026-08-10T12:25+08:00，同步 252 Gitea 实际状态）",
    "## 严格复核结论（2026-08-10T12:35+08:00，同步 252 Gitea 实际状态；#3911 V312-24 已 closed by PR #3950 merge）"
)

# 5. 显示 diff
print("\n=== Diff (before -> after) ===")
old_lines = old_body.splitlines()
new_lines = body.splitlines()
for i, (o, n) in enumerate(zip(old_lines, new_lines)):
    if o != n:
        print(f"L{i+1}:")
        print(f"  - {o}")
        print(f"  + {n}")
if len(old_lines) != len(new_lines):
    print(f"WARNING: line count changed: {len(old_lines)} -> {len(new_lines)}")

# 6. PATCH 上传
if "--apply" in sys.argv:
    print("\n=== Applying via PATCH API ===")
    patch_data = json.dumps({"body": body}).encode()
    req = urllib.request.Request(
        "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/issues/3887",
        data=patch_data,
        method="PATCH",
        headers={
            "Authorization": auth_header,
            "Content-Type": "application/json",
            "Accept": "application/json"
        }
    )
    try:
        with urllib.request.urlopen(req) as resp:
            result = json.loads(resp.read())
            print(f"✓ PATCH 成功: {result.get('html_url', '?')}")
            print(f"  Updated body length: {len(result.get('body', ''))} chars")
    except urllib.error.HTTPError as e:
        err_body = e.read().decode()
        print(f"✗ PATCH 失败: {e.code} {e.reason}", file=sys.stderr)
        print(f"  Body: {err_body[:500]}", file=sys.stderr)
        sys.exit(1)
else:
    print("\n=== Dry run (use --apply to push) ===")

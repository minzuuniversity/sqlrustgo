#!/usr/bin/env bash
# Sprint 5 v6/v7/v8 Q21 + Q5/Q15/Q16 + Q7/Q8/Q9 — push to Gitea when servers recover.
# Run this after the Gitea 252/250 servers are back online.

set -euo pipefail
cd /Users/liying/workspace/dev/openheart/sqlrustgo/.worktrees/v390-merge-q21

# Check both Giteas
for host in 252 250; do
  if curl -sf -o /dev/null -w "%{http_code}" "http://192.168.0.${host}:3000/" --max-time 3 2>/dev/null | grep -q "200"; then
    echo "Gitea $host is UP"
  else
    echo "Gitea $host is DOWN, skipping"
  fi
done

# Push to Gitea 252 (origin)
if curl -sf -o /dev/null "http://192.168.0.252:3000/" --max-time 3 2>/dev/null; then
  echo "Pushing to origin (Gitea 252)..."
  git push origin release/v3.9.0-q21-merge 2>&1
  git push origin fix/v390-q21-multi-col-index 2>&1
  git push origin v3.9.0-q21-gate 2>&1
fi

# Push to Gitea 250 (backup)
if curl -sf -o /dev/null "http://192.168.0.250:3000/" --max-time 3 2>/dev/null; then
  echo "Pushing to gitea (Gitea 250)..."
  git push gitea release/v3.9.0-q21-merge 2>&1
  git push gitea fix/v390-q21-multi-col-index 2>&1
  git push gitea v3.9.0-q21-gate 2>&1
fi

# Create PR if origin is up
if curl -sf -o /dev/null "http://192.168.0.252:3000/" --max-time 3 2>/dev/null; then
  echo "Creating PR on Gitea 252..."
  curl -s -u "openclaw:details8848" -X POST \
    "http://192.168.0.252:3000/api/v1/repos/openclaw/sqlrustgo/pulls" \
    -H "Content-Type: application/json" \
    -d '{
      "head": "release/v3.9.0-q21-merge",
      "base": "develop/v3.9.0",
      "title": "[v390] Sprint 5 v6/v7/v8: Q21 + Q5/Q15/Q16 + Q7/Q8/Q9 fixes → 22/22 PASS, 0 FAIL, 0 SKIP",
      "body": "Merges all 5 Sprint 5 bug fixes (Q21 perf, Q5/Q15/Q16 derived-table, Q7-Q9 EXTRACT, Q8 re-projection) into develop/v3.9.0. See docs/audit/status/2026-06-09-sprint5-merge-report-v390.md for full details.\n\nTag: v3.9.0-q21-gate\nCloses: #2977 (Q21 perf)"
    }' | python3 -c "import json, sys; d=json.load(sys.stdin); print(f'PR #{d.get(\"number\",\"?\")} created: {d.get(\"html_url\",\"err\")}')"
fi

echo "DONE"

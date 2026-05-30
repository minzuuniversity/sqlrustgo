#!/usr/bin/env python3
"""
scripts/gate/check_freshness.py — G-06 Evidence Freshness Enforcement

Every verified claim must have execution evidence within 30 days.
This script checks all claims in GOVERNANCE_FRAMEWORK.md and degrades
stale claims to 'stale' status.

Usage:
    python3 scripts/gate/check_freshness.py --version v3.8.0

Output:
    - Lists all verified claims with their age
    - Identifies claims older than 30 days
    - Reports stale claims count
    - Exit code = number of stale claims (0 = all fresh)
"""

import sys
import re
import json
from datetime import datetime, timedelta, timezone
from pathlib import Path

DAYS_WINDOW = 30

def parse_timestamp(ts_str):
    """Parse ISO timestamp from string."""
    ts_str = ts_str.strip()
    for fmt in ["%Y-%m-%dT%H:%M:%S%z", "%Y-%m-%dT%H:%M+08:00", "%Y-%m-%d"]:
        try:
            return datetime.strptime(ts_str, fmt).replace(tzinfo=timezone(timedelta(hours=8)))
        except ValueError:
            continue
    return None

def extract_claims_from_governance(governance_path):
    """Extract claim entries from GOVERNANCE_FRAMEWORK.md."""
    content = governance_path.read_text()
    claims = []
    
    # Find claim tables and entries
    # Pattern: | CLAIM-XXX | title | status | evidence |
    claim_pattern = re.compile(
        r'\| (CLAIM-\d+) \| ([^|]+) \| \*\*([^*]+)\*\* \| ([^|]+) \|',
        re.MULTILINE
    )
    
    for match in claim_pattern.finditer(content):
        claim_id = match.group(1).strip()
        title = match.group(2).strip()
        status = match.group(3).strip()
        evidence = match.group(4).strip()
        
        # Extract timestamp from evidence if present
        # Pattern: commit `abc123` at YYYY-MM-DD or similar
        ts_match = re.search(r'\d{4}-\d{2}-\d{2}', evidence)
        ts = parse_timestamp(ts_match.group()) if ts_match else None
        
        claims.append({
            'id': claim_id,
            'title': title,
            'status': status,
            'evidence': evidence,
            'timestamp': ts
        })
    
    return claims

def check_claim_freshness(claim, now):
    """Check if a claim's evidence is fresh."""
    if claim['status'] != 'verified':
        return 'not_verified'
    
    if claim['timestamp'] is None:
        return 'unknown'
    
    age = (now - claim['timestamp']).days
    
    if age > DAYS_WINDOW:
        return f'stale_{age}d'
    
    return 'fresh'

def main():
    version = 'v3.8.0'
    
    # Parse args
    args = sys.argv[1:]
    if '--version' in args:
        idx = args.index('--version')
        version = args[idx + 1]
    
    governance_path = Path(f'docs/releases/{version}/GOVERNANCE_FRAMEWORK.md')
    
    if not governance_path.exists():
        print(f"ERROR: {governance_path} not found")
        sys.exit(1)
    
    claims = extract_claims_from_governance(governance_path)
    now = datetime.now(timezone(timedelta(hours=8)))
    
    print(f"=== G-06 Evidence Freshness Check — {version} ===")
    print(f"Window: {DAYS_WINDOW} days")
    print(f"Checked: {len(claims)} claims")
    print()
    
    stale_count = 0
    fresh_count = 0
    
    for claim in claims:
        freshness = check_claim_freshness(claim, now)
        
        if freshness == 'fresh':
            fresh_count += 1
            status_icon = '✅'
        elif freshness == 'stale_':
            stale_count += 1
            status_icon = '❌'
            age = claim.get('_age', '?')
            print(f"{status_icon} {claim['id']}: {claim['title']} — STALE ({age})")
            print(f"   Evidence: {claim['evidence'][:80]}...")
            print(f"   → Degrade to stale, trigger governance review")
            print()
        elif freshness == 'unknown':
            print(f"⚠️  {claim['id']}: {claim['title']} — UNKNOWN (no timestamp)")
            print(f"   Evidence: {claim['evidence'][:80]}...")
            print()
        else:
            print(f"➖ {claim['id']}: {claim['title']} — {claim['status']} (not verified)")
    
    print()
    print(f"=== Summary ===")
    print(f"Fresh (verified, < {DAYS_WINDOW}d): {fresh_count}")
    print(f"Stale (verified, > {DAYS_WINDOW}d): {stale_count}")
    print(f"Unverified or unknown: {len(claims) - fresh_count - stale_count}")
    print()
    
    if stale_count > 0:
        print(f"⚠️  {stale_count} stale claim(s) detected — release blocked")
        print(f"   Stale claims must be refreshed or degraded")
        sys.exit(stale_count)
    else:
        print(f"✅ All verified claims are fresh")
        sys.exit(0)

if __name__ == '__main__':
    main()
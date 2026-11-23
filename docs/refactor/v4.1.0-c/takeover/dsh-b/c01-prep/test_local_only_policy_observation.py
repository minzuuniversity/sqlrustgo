'''C01-03 · LOCAL_ONLY 路由观察测试（DSH-B staged 预备版，待授权后迁入正式位置）。

观察三处（只读，不执行任何云端调用）：
  O1 deepseek-schedule 路由策略：active_strategy 存在、escalation 阶梯存在、
     router.py 具备 cloud- 硬过滤（LOCAL_FIRST 策略下的云端拒载能力）。
  O2 DSH-B 控制面域策略：DEFAULT_LOCAL_ENDPOINTS 全部为局域网端点（无 WAN/付费域）；
     RoutePolicy 在 SQLRUSTGO_LOCAL_ONLY 域内拒绝云端端点（发起网络调用前的确定性拒绝）。
  O3 调用账本诚实性：每条记录的 classification 必须与端点真值一致（反洗白属性），
     且 remote 条目只允许出现在 CONTROL_BOOTSTRAP 域。

语义：前置缺失 ⇒ 显式 skip（NOT_RUN），绝不静默 PASS（反伪造）。
证据：写入 evidence/local-only-review.json（绑定输入 sha256）。
'''
from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path

import pytest

SCHEDULE_REPO = Path(os.environ.get(
    'DSH_SCHEDULE_REPO',
    '/Users/liying/dev/workspaces/deepseek-schedule/deepseek-schedule'))
CONTROL_PKG = Path(os.environ.get(
    'DSH_CONTROL_PKG',
    '/Volumes/workspace/dev/deepseek-harness/python/control'))
LEDGER = Path(os.environ.get(
    'DSH_CALL_LEDGER',
    '/Volumes/workspace/dev/sqlrustgo/docs/refactor/v4.1.0-c/takeover/control-plane/call_ledger.jsonl'))
EVIDENCE_OUT = Path(os.environ.get(
    'C01_03_EVIDENCE_OUT',
    '/Volumes/workspace/dev/sqlrustgo/docs/refactor/v4.1.0-c/takeover/dsh-b/c01-prep/local-only-review.json'))


def _sha(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def test_o1_routing_policy_observation():
    routing = SCHEDULE_REPO / 'config' / 'routing.json'
    router = SCHEDULE_REPO / 'scheduler' / 'router.py'
    if not routing.exists() or not router.exists():
        pytest.skip('DSH-A schedule repo not found (NOT_RUN, not PASS)')
    cfg = json.loads(routing.read_text())
    # 能力在位：路由策略定义 + 云端硬过滤机制
    assert cfg.get('active_strategy'), 'active_strategy missing in routing.json'
    strategies = cfg.get('strategies', {})
    assert strategies.get('LOCAL_ONLY', {}).get('local_only') is True, \
        'LOCAL_ONLY strategy (cloud fully excluded) not defined in routing.json'
    src = router.read_text()
    assert 'local_only' in src and 'cloud-' in src, \
        'router lacks cloud hard-filter (LOCAL_ONLY enforcement)'
    # 如实记录现状（不断言）：active 策略的 local_only 状态
    active = cfg['active_strategy']
    active_local_only = strategies.get(active, {}).get('local_only')
    _record('O1', {'active_strategy': active,
                   'active_local_only': active_local_only,
                   'sqlrustgo_readiness_gap': active_local_only is not True,
                   'router_sha256': _sha(router)[:16]})
    # 就绪缺口显式化：SQLRustGo 派发前 active 策略必须 local_only=True。
    # 当前 LOCAL_FIRST=False ⇒ 记录为观察事实（由 DSH-A/属主切换策略），本测试不掩盖。


def test_o2_domain_policy_rejects_cloud():
    domains = CONTROL_PKG / 'dsh_control' / 'domains.py'
    if not domains.exists():
        pytest.skip('dsh_control package not found (NOT_RUN, not PASS)')
    src = domains.read_text()
    assert 'DEFAULT_LOCAL_ENDPOINTS' in src
    # 局域网端点：非 https、非公网域
    m = None
    import re
    mm = re.search(r'DEFAULT_LOCAL_ENDPOINTS\s*=\s*frozenset\(\{(.*?)\}\)', src, re.S)
    assert mm, 'DEFAULT_LOCAL_ENDPOINTS not found'
    endpoints = [e.strip().strip(" '\"") for e in mm.group(1).split(',') if e.strip()]
    assert endpoints, 'local endpoint set empty'
    for ep in endpoints:
        assert ep.startswith('http://192.168.') or ep.startswith('http://127.'), \
            f'non-LAN endpoint in local set: {ep}'
    # 确定性拒绝（纯配置驱动，无网络 I/O）
    sys_path_added = str(CONTROL_PKG) not in __import__('sys').path
    if sys_path_added:
        __import__('sys').path.insert(0, str(CONTROL_PKG))
    from dsh_control.domains import (  # noqa: E402
        SQLRUSTGO_LOCAL_ONLY, CloudRouteRejected, RoutePolicy)
    policy = RoutePolicy()
    with pytest.raises(CloudRouteRejected):
        policy.route(domain=SQLRUSTGO_LOCAL_ONLY, provider='openai',
                     endpoint='https://api.openai.com/v1', model='gpt-6.1')
    _record('O2', {'local_endpoints': endpoints,
                   'cloud_rejection': 'verified (pre-network, deterministic)'})


def test_o3_ledger_honesty():
    if not LEDGER.exists():
        pytest.skip('call ledger not found (NOT_RUN, not PASS)')
    entries = [json.loads(l) for l in LEDGER.read_text().splitlines() if l.strip()]
    assert entries, 'ledger empty'
    local_eps = {'http://192.168.0.252:8001', 'http://192.168.0.250:8011', 'http://192.168.0.250:8021'}
    for e in entries:
        expected = 'local' if e['endpoint'] in local_eps else 'remote'
        assert e['classification'] == expected, \
            f"ledger laundering: {e['endpoint']} recorded as {e['classification']} (expected {expected})"
        if e['classification'] == 'remote':
            assert e['domain'] == 'CONTROL_BOOTSTRAP', \
                f"remote call outside bootstrap domain: {e['task_id']} in {e['domain']}"
    _record('O3', {'entries_checked': len(entries),
                   'local_eps': sorted(local_eps)})


_RECORDED: list = []


def _record(observation: str, facts: dict) -> None:
    _RECORDED.append({'observation': observation, 'facts': facts,
                      'input_sha256': {
                          'routing.json': _sha(SCHEDULE_REPO / 'config' / 'routing.json') if (SCHEDULE_REPO / 'config' / 'routing.json').exists() else None,
                          'domains.py': _sha(CONTROL_PKG / 'dsh_control' / 'domains.py') if (CONTROL_PKG / 'dsh_control' / 'domains.py').exists() else None,
                          'ledger': _sha(LEDGER) if LEDGER.exists() else None}})
    try:
        EVIDENCE_OUT.parent.mkdir(parents=True, exist_ok=True)
        EVIDENCE_OUT.write_text(json.dumps(
            {'source_agent': 'DSH-B (opencode-go/glm-5.3-flash)',
             'source_run': 'c01-03-staged-prep',
             'timestamp': __import__('time').strftime('%Y-%m-%dT%H:%M:%S%z'),
             'observations': _RECORDED}, ensure_ascii=False, indent=1))
    except OSError:
        pass  # 证据写失败不掩盖断言结果

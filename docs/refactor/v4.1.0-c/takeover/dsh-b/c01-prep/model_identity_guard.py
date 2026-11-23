#!/usr/bin/env python3
'''G6 模型身份守卫：任务执行前后 + 每次调用响应的模型身份一致性校验。

用法（DSH-B 派发循环内）：
    guard = ModelIdentityGuard('http://192.168.0.252:8001', 'qwen3.8-flash-next-iq3_s')
    guard.pre()                 # 记录执行前身份
    ...模型任务，每次响应后调用 guard.check_response(resp.model)...
    guard.post()                # 执行后复核；identity_report() 入 Receipt
不一致即 IdentityMismatch 异常 → 任务 BLOCKED（fail-closed，不静默降级）。
'''
from __future__ import annotations

import json
import urllib.request


class IdentityMismatch(RuntimeError):
    pass


class ModelIdentityGuard:
    def __init__(self, base_url: str, expected_model: str, timeout: int = 15):
        self.base_url = base_url.rstrip('/')
        self.expected = expected_model
        self.timeout = timeout
        self.pre_identity = None
        self.post_identity = None
        self.response_models: list = []

    def _models(self) -> list:
        with urllib.request.urlopen(self.base_url + '/v1/models', timeout=self.timeout) as r:
            return json.load(r).get('data', [])

    def pre(self) -> str:
        models = [m['id'] for m in self._models()]
        if self.expected not in models:
            raise IdentityMismatch(f'PRE: expected model {self.expected!r} not loaded; got {models}')
        self.pre_identity = self.expected
        return self.expected

    def check_response(self, resp_model: str) -> None:
        if resp_model != self.expected:
            self.response_models.append(resp_model)
            raise IdentityMismatch(
                f'EXEC: response model {resp_model!r} != expected {self.expected!r} -> task BLOCKED')
        self.response_models.append(resp_model)

    def post(self) -> str:
        models = [m['id'] for m in self._models()]
        if self.expected not in models:
            raise IdentityMismatch(f'POST: expected model {self.expected!r} no longer loaded; got {models}')
        self.post_identity = self.expected
        if self.pre_identity != self.post_identity:
            raise IdentityMismatch('POST: identity changed during task')
        return self.post_identity

    def identity_report(self) -> dict:
        return {'endpoint': self.base_url, 'expected': self.expected,
                'pre': self.pre_identity, 'post': self.post_identity,
                'responses_consistent': all(m == self.expected for m in self.response_models),
                'n_responses': len(self.response_models)}

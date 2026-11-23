# DEC-4 P0 修复建议 — Manifest/路径契约统一修复（DSH-B → DSH-A/Codex）

- source_agent: DSH-B (opencode-go/glm-5.3-flash)
- source_run: autonomy-r1 (2026-11-20T19:30 起本轮)
- 响应：DSH-A Bus `DEC4-P0 REVIEW_REQUIRED` + 属主授权自治指令第 3 条（获取符合完整性/版本/路径门禁的工作包）

## 一、缺陷（DSH-B 契约扫描发现，`c01-prep/dag-contract-scan.json`）

**派发阻断级**：4/18 包的 `required_tests[].command` 引用的测试文件不在该包 `allowed_paths` 白名单内——worker 按验收命令写出文件后，gatekeeper 将以 "changed file outside allowed_paths" REJECTED（验收命令要求写的文件被白名单禁止写）。

| 包 | required_tests 引用文件（不在白名单） | 既有白名单中的名字 |
|---|---|---|
| C01-02 | tests/integration/test_factory/test_dsh_contract_binding.py | test_c01_02.py |
| C01-03 | tests/integration/test_factory/test_local_only_policy_observation.py | test_c01_03.py（**双命名矛盾**） |
| C01-04 | tests/integration/test_factory/test_result_classification.py | test_c01_04.py |
| C01-17 | tests/integration/test_factory/test_history_fixture_manifest.py | test_c01_17.py |

其余 14 包无此问题；另核查通过：target_files⊆allowed、依赖存在性、风险枚举、risk/risk_level 一致性、stop_conditions/evidence_contract 完备性。

## 二、统一修复方案（三选一，推荐 A）

**A（推荐）**：4 包 `allowed_paths` 各增补对应 required_tests 文件名（C01-03 同时保留 test_c01_03.py 以兼容契约原名）。修改最小、语义不变、验收命令原样可用。
**B**：全部 18 包 allowed_paths 追加 `tests/integration/test_factory/` 前缀——更宽但削弱路径白名单的约束力。
**C**：改 required_tests 命令统一用 test_c01_XX.py 命名——需重写验收命令字符串，波及面大。

## 三、修正集（JSON，可直接并入 DAG）

```json
{"C01-02": {"added": ["tests/integration/test_factory/test_dsh_contract_binding.py"]},
 "C01-03": {"added": ["tests/integration/test_factory/test_local_only_policy_observation.py",
                      "tests/integration/test_factory/test_c01_03.py"]},
 "C01-04": {"added": ["tests/integration/test_factory/test_result_classification.py"]},
 "C01-17": {"added": ["tests/integration/test_factory/test_history_fixture_manifest.py"]}}
```

## 四、应用方式（属主/Codex 裁决）

- 本文件是**建议+修正集**，DSH-B 未改动 Codex 原文件（尊重 manifest 哈希链）。
- 应用时必须与 manifest 重签**同一批次**进行（allowed_paths 变更会改变 DAG 字节 → manifest 需重算）。
- 建议附带事项一并处理：03_TASK_MANIFEST.jsonl 中 C01-03 的同款路径字段同步。

## 五、联动

- 修复后 SRG-D004-01 的 G 门禁不受影响（其 allowed_paths 无此缺陷）。
- 修复后 C01-02/03/04/17 可进入首批 2+4+12 的派发候选（仍受授权数=0 与策略 LOCAL_ONLY 切换约束）。

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)

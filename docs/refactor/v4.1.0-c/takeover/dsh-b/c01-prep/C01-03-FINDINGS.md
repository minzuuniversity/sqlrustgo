# C01-03 预备观察结论（LOCAL_ONLY 路由补证）

- source_run: goal-R9
- staged 测试：`c01-prep/test_local_only_policy_observation.py`（**3/3 绿**，授权后迁入 `tests/integration/test_factory/`）
- 证据：`c01-prep/local-only-review.json`（输入 sha256 绑定）

## 三项观察结果

| # | 观察 | 结果 |
|---|---|---|
| O1 | DSH-A 路由策略 | ✅ 能力在位：LOCAL_ONLY 策略已定义（云端全排除）、router.py 具备 cloud- 硬过滤 |
| O2 | DSH-B 域策略 | ✅ 本地端点集全 LAN；SQLRUSTGO_LOCAL_ONLY 域云端拒绝（网络调用前，确定性） |
| O3 | 账本诚实性 | ✅ 17 条记录 classification 与端点真值全一致；remote 只出现在 CONTROL_BOOTSTRAP |

## ⚠️ 就绪缺口（需 DSH-A/属主处理）

**`active_strategy = LOCAL_FIRST`，其 `local_only: false`** —— 当前路由对 L2+ 任务允许云端资源（L2 execute 池含 cloud-minimax-build，escalate 含 cloud-opencode-go）。

**SQLRustGo 任何任务派发前必须切换到 `LOCAL_ONLY` 策略**（能力不足任务走 WAITING_RESOURCE 而非云端回退）。此为 C01-03 的核心产出发现；切换动作属 DSH-A 配置权限，DSH-B 已把缺口写入证据并在此显式上报。

## 附带发现（Codex 契约不一致，归入漂移清单）

C01-03 契约内部矛盾：`allowed_paths` 写 `tests/integration/test_factory/test_c01_03.py`，而 `required_tests[0].command` 指向 `test_local_only_policy_observation.py`——文件名二选一需 Codex 重签时统一（已计入漂移清单）。

## R10 系统扫描：Codex 重签时必须一并修复的契约缺陷（4/18 包）

`dag-contract-scan.json`：C01-02、C01-03、C01-04、C01-17 的 `required_tests[].command` 引用的测试文件（`tests/integration/test_factory/test_*.py`）**不在该包 allowed_paths 白名单内**——worker 按命令写出文件后，gatekeeper 将以 "changed file outside allowed_paths" REJECTED。**这是派发阻断级缺陷**（先有鸡还是先有蛋：验收命令要求写的文件被路径白名单禁止写）。

修复建议（Codex 重签时统一）：allowed_paths 增加 `tests/integration/test_factory/`（或逐包列出对应 test 文件名），并把 required_tests 命令与 allowed_paths 的文件名对齐（C01-03 现有两套名字：test_c01_03.py vs test_local_only_policy_observation.py）。

扫描口径：required_test 文件归属、target_files⊆allowed、依赖存在性、风险枚举、关键字段完备、risk/risk_level 一致性。其余 14 包无此类问题。

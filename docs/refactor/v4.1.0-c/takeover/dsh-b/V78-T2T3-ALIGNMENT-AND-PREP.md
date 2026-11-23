# T2 新旧任务基线对齐矩阵 + T3 实施准备（v7.8-B）

- source_run: v78-takeover-T2T3
- timestamp: 2026-11-20T22:45+08:00
- 前置：HANDOFF_PREFLIGHT_RECEIPT.json（Manifest 121/121 VERIFIED，1 项 INVALID 命名错位）

## 一、新旧 18 包对齐（同一集合，Codex 重签版）

旧版（07:34 manifest）与新版（14:04 重签）18 包 task_id 完全相同，**继承而非替换**；差异：

| 变更 | 内容 |
|---|---|
| allowed_paths | C01-02/03/04/17 增补 test_c01_XX.py（但 required_tests 命令仍引用描述名——**命名错位残留**，唯一 INVALID 项） |
| 03_TASK_MANIFEST.jsonl | 重签（哈希一致）；原生 Schema 18/18 PASS（DSH-B 独立复验） |
| validation.json | 新增（未绑定，良性） |

## 二、DSH-B 自治成果 → 新基线的继承关系

| 已有资产 | 映射 | 处置 |
|---|---|---|
| PR #5912（Test Factory 反例清除批次） | 服务 C01-04（结果分类）/C01-15（SQLLogicTest 基础） | **继承**：工具修复是 C01-04 的前置事实，dispatch 时为验证步骤 |
| PR #5915（C01-05+C01-06 契约测试，4 RED） | **直接就是 C01-05/C01-06 的交付物** | **继承**：dispatch 这两包 = 注册/复跑验收，不是重新编写（避免重复派发） |
| Issue #5913/#5914 | 对应 DAG 内 issue_refs（#5235 等之外的新增事实单） | 关联补充，不冲突 |
| SRG-D004-01 | 新包（BLOCKED_GATES，G5） | 保持，等 DSH-A 门禁解除；**不重复派发** |
| RED-LEDGER 10 项 | INV-01/INV-07 新反例直接支撑 ADR 待审项 | 作为 M0/M1 架构裁决材料提交 |
| 75 场景基线 70 PASS/5 RED | 与新包 75 pilot_input_cases 同源 | 冻结基线沿用 |
| ca01 观察测试 staged（C01-03） | C01-03 交付物已备 | dispatch 时迁入正式位置 |
| Standard/PTC A/B、OpenCode/OMP 评测 | 部分完成（Task A-E） | 按需补充 |

**不叠加、不重复派发、不取消在途**：SRG-D004-01 保持 PLANNED/BLOCKED_GATES 原状；PR #5912/#5915 保持 OPEN。

## 三、十份 ADR 待决清单（审批权：属主）

| ADR | 主题域 | DSH-B 实证支撑 |
|---|---|---|
| ADR-410-001~005 | C 路线核心契约（Storage/Transaction 边界等） | RED 台账 #1-#5（桩族/ODKU）提供现状基线 |
| ADR-410-006~010 | 恢复/隔离/能力契约 | **INV-01 双重撤销（PR #5915）+ INV-07 flush 泄漏未提交数据** 为 #5909 族裁决材料；脏读双引擎固化 |

全部 ADR 不得自动批准（human_gate）。

## 四、M-Pre 阻断清单

1. **命名错位**（唯一 INVALID）：C01-02/03/04/17 命令文件名 vs 白名单——Codex 修订或属主授权 DSH-B 按命令名为准增补白名单
2. **G5 出口控制**（OD-002）：运行时 LOCAL_ONLY 端到端前提
3. **ADR 审批**：10 份全待审
4. **authorized_dispatch_count=0**：首个真实派发授权待属主
5. SRG-D004-01 BLOCKED_GATES（等 G5）

## 五、首批候选工作包与分批预算建议（待授权）

| 批 | 包 | 理由 | 预算（占 2.57M） |
|---|---|---|---|
| 1 | C01-01（L0 校验）+ C01-03（LOCAL_ONLY 观察，测试已备）+ C01-04（结果分类，工具修复已备） | 基础闭环，DSH-B 已有大量前置事实 | ~0.4M |
| 2 | C01-02 + C01-05 + C01-06（测试已备，dispatch=复跑验收） | 继承已交付资产，Token 消耗极低 | ~0.3M |
| 3 | C01-07~C01-14（契约族，含 2 个人工门包） | 依赖批 1-2 证据 | ~1.2M |
| 4 | C01-15~C01-17 + C01-18 终验 | 收口 | ~0.67M |

Reviewer 独立性要求：OMP(252) 开发 / 8021 过程独立审 / 8011 异血统终审（核心契约包必配）；Codex 不做最终独立签发（其自定契约已声明）。

## 六、LOCAL_ONLY 与资源需求

- 契约层 ✅（LOCAL_ONLY×18）；运行时层缺 G5 → 需 DSH-A/属主完成 OD-002（pf 白名单）
- 本地资源：252:8001 主推理 / 250:8021 审阅 / 250:8011 GMP 专用禁占
- cloud_calls=0、cloud_tokens=0、无 fallback；PRE/EXEC/POST 身份守卫强制

---

[![](https://img.shields.io/badge/powered_by-dsh-4D6BFE?style=flat-square&logo=deepseek&logoColor=white)](https://github.com/deepseek-ai/deepseek-harness)

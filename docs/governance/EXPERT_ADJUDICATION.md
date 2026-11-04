# 咨询专家裁定制度 (Expert Adjudication) —— 规则 SSOT

> 适用范围：所有需要"由模型判断"的场合（门禁判据真伪、Issue 归因、计划评审、发布争议）。
> 本文件把"问模型"变成一条**可机器核对**的流程，而不是一句口头承诺。
> 实现：`scripts/gate/expert_ladder.py`；席位注册表：`docs/governance/EXPERT_SEATS.yaml`。

---

## 1. 为什么要有这套制度

v4.1.0 的实测基线（2026-11-04：`python3 scripts/gate/gate_executability.py` + Gitea API 只读实测）：
168 道 `check_*` 里只有 23 道自带反例夹具 ⇒ **145 道不可证伪**；Gitea Actions 全历史 3911 次 run 里
`success` 只有 **70 次（1.8%）**，failure 1839、cancelled 2002。也就是说：判据本身经常是坏的，
而机器又很少真的跑过。

这种局面下最容易发生的错误不是"没人检查"，而是**用模型的一句"这是误报"把红门禁说成绿的**。
因此本制度的第一性目的：让"引入专家"不降低整体强度。

---

## 2. 权威边界（继承既有 governance，不新设权威）

| 本制度组件 | 权威 | 依据 |
|---|---|---|
| 席位（任意模型）的单条输出 | **无**（Soft Gate） | `ANTI_FABRICATION_POLICY.md` §5.1「只有 Hard Gate 可以决定 pass/fail」、§5.3 Soft Gate 标注格式 |
| 聚合器 `aggregate()` 的结论 | 有，但只覆盖"判据真伪"这一事实问题，不覆盖阶段跃迁 | AGENTS §2.7 阶段跃迁需人批准；AFP §5.2 必须为 Hard Gate 的项 |
| 反向变异夹具（机器证据） | **决定性**：没有它 FALSE_POSITIVE 不成立 | 本文件 R3 + 「每条红灯必须做反向变异」纪律 |
| 人（owner / architect） | 最终批准 | ADR-014、`docs/governance/STAGE_CONFIG.yaml` |

每条模型输出在落盘与引用时必须带 `[SOFT GATE]` 前缀，格式沿用 AFP §5.3。

---

## 3. 席位与血统（lineage）

席位登记在 `EXPERT_SEATS.yaml`。表里的数字是**某次实测快照**，当前事实由
`python3 scripts/gate/expert_ladder.py health` 现场探测决定；两者冲突以现场探测为准。

| 席位 | lineage | 角色 | 快照延迟 | 状态（2026-11-04 晚实测） |
|---|---|---|---|---|
| `chatgpt-web` (127.0.0.1:9656) | gpt | adjudicate | 40.05 s | **已恢复**。此前"找不到输入框"的真因是 headless UA 触发 Cloudflare 拦截页（可输入元素=0），非未登录；`cf_clearance` 绑 UA+IP |
| `deepseek-chat` (127.0.0.1:9655) | deepseek | adjudicate | 1.36 s | **不可用**：上游 `create_pow_challenge` 返回 `40003 Authorization Failed (invalid token)` ⇒ token 过期；重新认证需人工登录，Agent 不得代做 |
| `flash-next-252` (192.168.0.252:8001) | qwen | adjudicate / triage | 12–71 s | 可用（须 `reasoning_effort=none`；与他人共用单会话时排队严重） |
| `local-qwen35-9b` (127.0.0.1:8002) | qwen | triage | 10–15 s | 可用；alias 与实际权重不符（别名 `qwen35-9b-mac`，实际加载 `Qwen3-14B-Q4_K_M`），登记时按实测文件名为准 |
| `gmp35b-250` (192.168.0.250:8011) | qwen | triage | 165 s | 可用但慢，且与 omp 抢同一单会话服务；`reasoning_effort` 不被遵守 |

**血统独立**是硬要求，不是"服务栈独立"：`qwen-strata` 与 `qwen-llamacpp` 是两个部署、
同一个血统，让它们互相"投票"等于 ADR-014 D-06 禁止的 Claim 传播（自己给自己背书）。
2026-11-04 的教训：一度用 `family` 计独立性，得出"2 家族可用"的假充足。

**不新部署模型**：owner 指示「不要去部署大模型，只要检查能否使用」。曾把 gemma-4-E4B
拷到 250 并试起 8012（加载成功、`/health`=200），随即撤销：进程停止、4.98 GB GGUF 删除、
250 恢复原状。缺血统时报告缺位并交给人，不去自建推理服务。

---

## 4. 四条硬规矩（可证伪，由 `scripts/gate/test_expert_ladder.py` 核对）

**R1 血统独立**：裁定（`role=adjudicate`）必须拿到 **≥2 个不同 lineage** 的有效席位输出；
不足 ⇒ 聚合 `INCOMPLETE`。

**R2 弃权不可被否决**：任一有效席位返回 `INCOMPLETE`，或任一席位超时/不可解析
⇒ 聚合 `INCOMPLETE`。多数票不能压倒弃权（ADR-008：SKIP/TIMEOUT 不计 PASS）。

**R3 误报需要机器证据**：全体席位判 `FALSE_POSITIVE` 时，若没有一条**通过的反向变异**
（把问题条件去掉后该判据必须变绿）作为 `machine_evidence`，聚合结论仍是 `REAL`，
记 `substate: FALSE_POSITIVE_UNVERIFIED`，并生成"给该判据补夹具"的工作项。

**R4 席位必须先过标定**：能参与裁定的席位必须答对 ground-truth 探针
（`CALIBRATION_ITEMS`：算术、JSON 契约、"退出码 2 却写 PASS 是否可信"、代码真值），
`ratio >= calibration_min`（默认 0.5）才计入 R1 的血统数。
理由：**连得上但答错的席位不是佐证，是噪声放大器**——本项目"报告都好、一用就崩"
正是这个机制缺失的结果。只做连通性探测时，`health` 的 summary 会把口径写成
`reachability only (NOT trust)`，这种模式下的 `adjudication_capable` 不得被引用。

反向变异（针对规则本身）：`test_expert_ladder.py` 的 `rev_*` 用例把 R1/R2/R3/R4 的条件
逐项撤掉，结论必须相应变化（rev_R4：不标定 ⇒ 口径降级；答错的席位即使在全连通模式下
也保持不合格）。做不到就说明规则是装饰。

---

## 5. 分级与预算（由实测 TPS 决定，不是拍脑袋）

| 角色 | 默认席位 | 理由（实测） | 单次 token 上限 |
|---|---|---|---|
| `triage` 批量分诊/去重/归因摘要 | deepseek → flash-next → 9B | deepseek 17.6 TPS，本地席位 1–8 TPS：批量循环走本地席位＝小时级墙钟 | 200 |
| `adjudicate` 判据真伪裁定 | deepseek **+** flash-next（2 家族） | 实测窗口内只有这两席既可用又遵守输出契约 | 400 |
| `arbitrate` 争议终裁 | chatgpt-web（恢复后）＋任一 qwen 家族 | gpt 家族缺席 ⇒ 终裁一律 `INCOMPLETE` 并挂席位修复项 | 800 |
| `long_context` 大上下文摘要 | flash-next（262144 ctx）／gmp35b（64K） | 仅在确需长材料时使用，且必须登记被裁材料哈希 | 1200 |

预算纪律：单次调用必须显式 `max_tokens`。思考型席位必须带 `reasoning_effort=none`，
否则思考会吃光预算并产出空 `content`（实测 9B 的 200 completion token 全部落在
`reasoning_content`，`content` 为空、`finish_reason=length`）。

单会话纪律：本地 llama.cpp / strata / 桥接服务都是单会话，`expert_ladder.py` 对每个端点
加文件锁（`/tmp/sqlrustgo-expert-<host_port>.lock`）。250:8011 因 omp 在用出现过 120 s 超时——
这是**没排队**的证据，不是"模型不可用"的证据；锁是制度，不是可选优化。

---

## 6. 证据契约（AFP §6.1 / ADR-014 D-06）

运行时由 `audit/v4.1.0/audit_run.py` 承载证据（`--no-evidence-dir` 时本工具不写盘，
stdout 即产物；`expert-runs/<run_id>/` 只在离线试验时用 `--evidence-base` 指定临时目录）：

```
MANIFEST.yaml        schema_version: expert-manifest/v1
                     source_agent / source_run / timestamp_started / timestamp_finished
                     conflict_resolution / repository.commit / artifacts.<file>.sha256
health.jsonl         席位能力矩阵：latency / tps / finish_reason / 是否遵守输出契约
asks.jsonl           每次调用：prompt_sha256、output_sha256、latency、usage、error
adjudications.jsonl  每条裁定：question_id、verdicts、abstained、families、
                     machine_evidence_present/passed、aggregate、rule、soft_gate_note
```

引用裁定时必须带 `run_id` 与 `aggregate`，并保留 `[SOFT GATE]` 标注。
不得引用"专家说没问题"，只能引用"专家 run `<run_id>` 判 REAL，且反向变异夹具 `<sha256>` 通过"。

`MANIFEST.yaml` 的 `artifacts.*.sha256` 让"证据被改过"可检测——这正是 A04 差分矩阵
（#5466）出问题的形状：同一 case 的 `matrix.json` 与 `wire/*.txt` 来自不同引擎状态。

---

## 7. 与开发闭环的接线

```
Issue 台账 (docs/releases/v4.1.0/ISSUE_LEDGER_*.jsonl)
   └─ triage: 分类/去重/是否可自动化              (T0/T1, 非权威)
        └─ 争议判据 → adjudicate (R1+R2)          (T1 双家族)
             ├─ aggregate=REAL                → 进 P0/P1 队列：写测试(RED) → 修 → 门禁 → PR
             ├─ aggregate=FALSE_POSITIVE      → 只有夹具通过才成立；判据自身修复走独立 PR
             └─ aggregate=INCOMPLETE          → 记 blocked + 席位修复项，禁止凭直觉推进
```

`expert_ladder.py health` 退出码 2 表示"当前凑不齐 2 个独立家族"——即**裁定能力本身**缺失。
此时不得靠"只问一个模型"继续，而应先修席位（`chatgpt-web` 的 502、gmp35b 的拥堵都是可修运维项）。

---

## 8. 明确不做

- 不用模型输出作为阶段跃迁依据（AGENTS §2.7）。
- 不用模型输出关闭 Issue（`ISSUE_CLOSING_VERIFICATION.md` 五步 + PR 关联仍是前提）。
- 不把模型文本当作门禁判据的替代品：新增判据一律挂靠既有判据，不动分母（AGENTS §2.1）。
- 不把席位凭据写进仓库、日志或命令行参数。
- 不用同一 family 的多个规格冒充独立佐证。

---

## 9. 变更记录

| 日期 | 变更 | 依据/证据 |
|---|---|---|
| 2026-11-04 | 席位口径从 family 改为 lineage；新增 R4 席位标定；登记 ChatGPT 恢复根因与 DeepSeek 40003；记录"撤销 gemma 试部署" | owner 指示不部署模型；实测 `create_pow_challenge` 40003；ChatGPT 席位实测 content=PONG/40.05s |
| 2026-11-04 | 首版：席位注册表 + R1/R2/R3 + 证据契约 | 实测见 `EXPERT_SEATS.yaml` 的 measured_*；Gitea API 只读实测 3911 runs / 70 success；`gate_executability.py` 输出 92 项执行点 |

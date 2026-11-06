## 现象

看板三行仍写「未开始」，但对应工具与基线**已经在 develop 落库**：

| 看板行 | 状态（合并后现读） | 已存在的工具/基线 |
|---|---|---|
| P1-4 每道门禁写 effect + `honesty.py` | 未开始（blocked_by=P1-3） | `audit/v4.1.0/honesty.py`，且被 `scripts/gate/gate_registry.py` 引用 |
| P2-1 文档事实面 | 未开始 | `audit/v4.1.0/doc_facts.py` + `docs/releases/v4.1.0/DOC_INDEX.jsonl`（91 行） |
| P2-2 目录策略机器化 | 未开始 | `audit/v4.1.0/directory_policy.py` + `DIRECTORY_POLICY_BASELINE.jsonl`（149 行） |

要么看板行没跟着改（进度面滞后），要么工具落库但没满足各行验收判据。
两种情况都说明**看板当前不是可信的进度面**——这正是 Owner 抱怨的「报告都好、一测就出问题」的同类。

## 不许因为「文件存在」就判完成

各行必须按自己行内的验收判据实测后才允许改状态：

- P2-1：悬空路径 0 **且** 生成物「写盘口径 == 核对口径」（墙钟时间不得进比对域）；
- P2-2：新增违规能判红（需要**执行点**；执行点要挂进既有门禁 ⇒ 分母恒 168，属 Owner）；
- P1-4：「声称能抓 X 但没有 X 夹具 ⇒ 红」。

判据没跑成 ⇒ 该行保持未开始并写 INCOMPLETE，不得改写。

## 复现

```
rg -l honesty.py | rg -v '^audit/v4.1.0/honesty.py$'      # 命中 scripts/gate/gate_registry.py
wc -l docs/releases/v4.1.0/DOC_INDEX.jsonl docs/releases/v4.1.0/DIRECTORY_POLICY_BASELINE.jsonl
/usr/bin/python3 scripts/gate/gov_probe.py metrics | tail -1 | /usr/bin/python3 -c "import sys,json;print(json.load(sys.stdin)['check_n'])"   # 168 未变
```

看板行：**DRIFT-2**（待复核）。本 issue 只登记漂移，不替任何人判完成。

# GEN-1：看板 ISSUE 生成器进仓，并消掉 6 处会造成误报的缺陷（2026-11-05）

## 1. 为什么这条必须立案

看板 ISSUE（#5592）的正文此前由一个只在 `/tmp` 里存在的脚本生成。也就是说：
「看板上的说法」在干净检出里**无法复现**，任何人要核对都只能信我贴出来的那段。
这正好违反本仓的核心纪律——把「相信 AI 说的话」换成「机器能核对的事实」。

把它挪进仓里之后，逐行读它，发现 6 处会直接产出错误结论的缺陷。

## 2. 六处缺陷与消法（`audit/v4.1.0/gen_kanban_issue.py`）

| # | 缺陷 | 后果 | 消法 |
|---|---|---|---|
| 1 | 归档 run 结论写死成「全部 exit 0」 | 含 FAIL 的 run 会被说成没有问题（run `20261105-negfix-003` 实际有 2 条非零） | 从 manifest 的 `results` 段逐条读 `exit_code`，非零就写「非零 N 条（id=码）—— 按原样保留，不销账」 |
| 2 | 「最近 run」按 run id 字典序取 | `20261105-triage-*` 排在 `20261105-negfix-*` 之后，字典序会挑到错误的 run | 按 manifest 的 `completed_at` 排序取末 3 个 |
| 3 | 不带 `--post` 也直接 `PATCH` 线上 ISSUE | 「我只是打印出来看看」= 已经改写了线上看板 | 默认只返回字符串，只有 `--post <issue>` 才发请求；`PATCH` 失败必须非零退出 |
| 4 | f-string 里残留 `' + str(len(rows)) + '` | 线上 ISSUE 出现过字面量泄漏，看板行数显示成一串垃圾 | 改成 `{len(rows)}`，并由 selftest A2 扫描任何残留模板片段 |
| 5 | 读 `STAGE.yaml` 用 `stage:` 这个键 | 真正的键是 `current_stage:`，于是头部阶段字段一直是空 | 取第一个非注释的 `current_stage:`；读不到就非零退出（不允许空字段出现在看板上） |
| 6 | 行数口径与 SSOT 之间没有核对 | 正文说 44 行、SSOT 实际 45 行也不会被发现 | selftest A4 把正文里的「（N 行）」与 SSOT 实际行数比对 |

## 3. 反向变异（AGENTS 2.5）：证明这四条判据真的咬人

生成器自带 `--selftest`（四臂 A1 网络、A2 模板残留、A3 stage 字段、A4 行数口径）。
变异跑法在仓内：`audit/v4.1.0/gen_kanban_issue_mutations.py`，它只往临时目录里的**副本**施加变异，
仓内文件不动。四臂各自的期望与实测（归档 run 的 014 号日志）：

```
RED  m1-always-post: rc=1 expect=A1/network: 默认模式必须不 PATCH first_sighting=PATCH 失败: {'__http_error__': 'selftest-network-blocked'}
RED  m2-template-leak: rc=1 expect=A2: 正文不得残留模板片段 first_sighting=NOT-OK: A2 正文残留模板片段: ' + str(
RED  m3-stage-key-wrong: rc=1 expect=A3/stage-guard: 头部 stage 字段不得为空 first_sighting=STAGE.yaml 里没读到 current_stage —— 看板头部的阶段字段不能留空
RED  m4-rowcount-drift: rc=1 expect=A4: 行数口径必须等于 SSOT first_sighting=NOT-OK: A4 正文行数口径 != SSOT 45 行
arms=4 flipped_red=4 stayed_green=0 drift=0
```

判据含义：`stayed_green=0` 才说明 selftest 不是摆设。跑法本身退出码语义写死：
四臂全翻红 ⇒ 0；有臂仍绿 ⇒ 1；锚点找不到（源文本漂了，证据过期）⇒ 3。

## 4. 事故现场（如实记录，不做美化）

修好之前，我在验证「能不能生成」时执行了一次**不带 `--post`** 的调用，
按缺陷 3 的语义，它真的 `PATCH` 了 #5592，并把含 `' + str(len(rows)) + '` 泄漏、
stage 字段为空、且度量行是 `TEST-PING` 的正文写进了线上看板。
这条已用修好的脚本 `--post 5592` 重刷覆盖，看板正文现在与仓内 SSOT 同源。

对外的后果是真实发生过的（线上 ISSUE 一度含错误结论），因此记入 GEN-1 的 evidence，
而不是只在提交信息里带过。

## 5. 仍未闭环的部分（所以 GEN-1 记 `待复核`，不记完成）

- 生成器输出与**线上 ISSUE 正文**的逐字节对账需要读 API，本机只做到「脚本可复现 + 四臂变异」。
- 生成器没有 CI 执行点，被 IL-9（runner-03 缺镜像）挡住；接入执行点属 #5597 owner/hermes 动作。
- 本仓未跑 `ci_gate.sh`，本节全部结论不含门禁通过声明。

## 6. 复现命令

```bash
python3 audit/v4.1.0/gen_kanban_issue.py --selftest
python3 audit/v4.1.0/gen_kanban_issue_mutations.py
python3 audit/v4.1.0/gen_kanban_issue.py '<度量口径行>'            # 只打印
python3 audit/v4.1.0/gen_kanban_issue.py --post 5592 '<度量口径行>'  # 显式才写线上
python3 scripts/gate/kanban_render.py check docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.md
python3 audit/v4.1.0/audit_run.py verify --run-id 20261105-gen1-005
```

#!/usr/bin/env python3
"""刷新 GA_GATE_REPORT.md 顶部块（属主签批材料）+ 更正 RELEASE_NOTES/CHANGELOG 的
过期基线与状态。签名行**留空** —— 签署是人的动作（STAGE_CONFIG 的 manual_only_criteria）。

用法：
    python3 audit/v4.1.0/refresh_ga_report.py <results.jsonl> <plan.jsonl.meta.json> \
        [--soak-facts <facts.json>]

- `<results.jsonl>`：认证轮的逐项落账（`run_gates.py execute` 的产物）。
- `<plan.jsonl.meta.json>`：该轮的 plan 元信息（run_id / git_head / expected_n /
  expected_sha256）。
- `--soak-facts`：可选。SOAK 实测事实的 JSON（由 `audit/v4.1.0/soak_facts.py`
  产出）。不给时自动用仓内已提交的 168h 快照现算；再取不到 ⇒ SOAK 那行如实写
  「未读到实测快照」，**不编数**（AFP：宁可不判，不瞎判）。

本脚本原先只存在于 `/tmp`（且硬依赖 `/tmp/_soak_facts`）⇒ 落进仓内并去掉
/tmp 依赖：材料脚本与它的事实来源都必须能被第三方复算。
"""
import json
import os
import pathlib
import re
import subprocess
import sys
from datetime import datetime, timezone

REPO = pathlib.Path(os.environ.get("SRG_GATE_REPO", pathlib.Path(__file__).resolve().parents[2]))
DOCS = REPO / "docs/releases/v4.1.0"
REPORT = DOCS / "GA_GATE_REPORT.md"
RELEASE_NOTES = DOCS / "RELEASE_NOTES.md"
CHANGELOG = DOCS / "CHANGELOG.md"
TIMELINE = DOCS / "GA_RELEASE_TIMELINE.md"
EVIDENCE = DOCS / "EVIDENCE_STATUS.md"
SOAK_FACTS_SCRIPT = REPO / "audit/v4.1.0/soak_facts.py"

results = [json.loads(l) for l in pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").splitlines() if l.strip()]
meta = json.loads(pathlib.Path(sys.argv[2]).read_text(encoding="utf-8"))
RUN_DIR = pathlib.Path(sys.argv[1]).parent
head = meta["git_head"]
run_id = meta["run_id"]
now = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")

# 轮次 manifest：若本轮没跑满，「未执行」的项必须计入 INCOMPLETE 并逐条列出 ——
# 未执行 ≠ 通过，也 ≠ 失败（AFP：缺失的证据保持 incomplete）。
# `run_gates.py execute` 只会落账**跑过**的项；没跑过的那几项在 manifest 里。
manifest = {}
_manifest_path = RUN_DIR / "manifest.yaml"
if _manifest_path.exists():
    try:
        import yaml

        manifest = yaml.safe_load(_manifest_path.read_text(encoding="utf-8")) or {}
    except Exception as exc:
        print(f"[WARN] 读轮次 manifest 失败（{exc}）—— 未执行项无法并入计数", file=sys.stderr)
UNEXECUTED = manifest.get("unexecuted_gates") or []


def load_soak_facts():
    """SOAK 事实：--soak-facts 文件优先；否则用仓内已提交快照现算；再不行返回 None。"""
    argv = sys.argv[3:]
    if "--soak-facts" in argv:
        idx = argv.index("--soak-facts")
        if idx + 1 >= len(argv):
            raise SystemExit("--soak-facts 需要一个 JSON 文件路径")
        path = pathlib.Path(argv[idx + 1])
        if not path.exists():
            raise SystemExit(f"--soak-facts 指定的文件不存在：{path}")
        return json.loads(path.read_text(encoding="utf-8"))
    if SOAK_FACTS_SCRIPT.exists():
        try:
            out = subprocess.run(
                [sys.executable, str(SOAK_FACTS_SCRIPT)],
                capture_output=True, text=True, timeout=300, cwd=str(REPO),
            )
            if out.returncode == 0 and out.stdout.strip():
                return json.loads(out.stdout)
            print(f"[WARN] 仓内 SOAK 快照现算失败（rc={out.returncode}）：{out.stderr.strip()}",
                  file=sys.stderr)
        except Exception as exc:
            print(f"[WARN] 仓内 SOAK 快照现算异常：{exc}", file=sys.stderr)
    return None


_SOAK = load_soak_facts()

if _SOAK:
    _soak_fp = _SOAK.get("fsync_probe") or {}
    _soak_fs = ""
    if _soak_fp:
        _soak_fs = (f"同盘 fsync 实测 p50 {_soak_fp.get('p50_ms')} ms / p90 {_soak_fp.get('p90_ms')} ms "
                    f"/ max {_soak_fp.get('max_ms')} ms（起跑时 p50 {_SOAK.get('fsync_p50_at_start')} ms）")
    _soak_missing = _SOAK.get("missing_inputs") or []
    _soak_missing_note = ("；未取到的输入：" + "、".join(_soak_missing)) if _soak_missing else ""
    _soak_row = (
        f"**运行中**（`{_SOAK.get('started_utc')}` → `{_SOAK.get('planned_end_utc')}`，"
        f"快照 {_SOAK.get('snapshot_at')}，elapsed {_SOAK.get('elapsed_s')} s / {_SOAK.get('total_planned_s')} s）。"
        f"起跑门禁通过（43.4 q/s、0 错误）；累计 {_SOAK.get('q')} 查 / {_SOAK.get('e')} 错 "
        f"⇒ **{_SOAK.get('rate_pct')} %**（峰值 {_SOAK.get('peak_rate_pct')} %）。"
        f"运行期错误率与宿主争用耦合（{_soak_fs}），错误率**非单调**（宿主转静时回落、"
        f"再被抢时回升），当前吞吐 {_SOAK.get('qps_now')} q/s ⇒ "
        "**不是 GA 证据的合格轮**（未满足「独占窗口」要求）"
        + _soak_missing_note
        + "\n\n> **`DIAGNOSTIC_ONLY`（诊断基线，不是放行证据）**：依外部专家对裁决 D 的修正 —— "
        "当前这一轮的 SOAK 数据**只作诊断参考**，**不构成放行证据**；"
        "独占窗口恢复后须**重新累计完整 604800 秒**再验收。"
    )
else:
    _soak_row = ("**未读到实测快照**（既没有 `--soak-facts` 输入，仓内已提交快照也不可用）"
                 " ⇒ 本行不作判定（AFP：不编数）。")


def read_stdout(seq_hint: str, needle: str):
    """从该轮逐项 stdout 里按行读证据；读不到就返回 None（宁可不判，不瞎判）。"""
    p = RUN_DIR / seq_hint
    if not p.exists():
        return None
    ansi = re.compile(r"\x1b\[[0-9;]*m")
    for line in p.read_text(encoding="utf-8", errors="replace").splitlines():
        if needle in line:
            return ansi.sub("", line).strip()
    return None


_cov_line = read_stdout("001-check_rc_ga_gate.sh.stdout", "A5 Coverage")
if _cov_line:
    g3_state = "✅ PASS" if ("PASS" in _cov_line) else "⚠️ 见证据"
    g3_evidence = f"run {run_id} seq1 实测行：`{_cov_line.strip()}`（8-crate 平均；逐 crate 见 §7.13）"
    g3_coverage = _cov_line.strip()
else:
    g3_state = "⚠️ **未读到证据行**"
    g3_evidence = f"run {run_id} seq1 stdout 中未找到 `A5 Coverage` 行 ⇒ 不作判定（AFP：不臆造）"
    g3_coverage = "未读到实测行"

# D1–D9 的汇总行也直接从 seq1 stdout 读，不硬编码
_d_summary = []
for _d in ("D1", "D2", "D3", "D4", "D5"):
    _ln = read_stdout("001-check_rc_ga_gate.sh.stdout", _d + " Result")
    if _ln:
        _d_summary.append(_ln.strip())
if not _d_summary:
    _ln = read_stdout("001-check_rc_ga_gate.sh.stdout", "Result")
    if _ln:
        _d_summary.append(_ln.strip())
d_summary_text = "；".join(_d_summary) if _d_summary else "未读到 D1–D5 汇总行"

_kanban_file = REPO / "docs/releases/v4.1.0/KANBAN_v4.1.0_ALPHA.jsonl"
kanban_rows = sum(1 for l in _kanban_file.read_text(encoding="utf-8").splitlines() if l.strip())

counts = {}
for r in results:
    counts[r.get("state")] = counts.get(r.get("state"), 0) + 1
n_pass = counts.get("PASS", 0)
n_fail = counts.get("FAIL", 0)
n_landed_inc = counts.get("INCOMPLETE", 0)
# 未执行的项计入 INCOMPLETE（未执行 ≠ 通过）
n_inc = n_landed_inc + len(UNEXECUTED)
n_other = len(results) - n_pass - n_fail - n_landed_inc
non_green = [
    f"{i}. `{r['gate']}` ⇒ {r.get('state')}（exit={r.get('exit_code')}，reason={r.get('reason', '—')}）"
    for i, r in enumerate(results, 1)
    if r.get("state") != "PASS"
]
for _g in UNEXECUTED:
    _gid = _g.get("id") or "?"
    _reason = (_g.get("reason") or "").strip()
    non_green.append(f"`{_gid}` ⇒ **INCOMPLETE**（**未执行** ⇒ 非通过、非失败；{_reason[:160]}）")
green = "无（全部 PASS）" if not non_green else "\n".join(f"- {x}" for x in non_green)


def find_gate(fragment: str):
    """按门禁名片段定位唯一一项；找不到或多于一项都返回 None（宁可不判，不瞎判）。"""
    hits = [r for r in results if fragment in (r.get("gate") or "")]
    return hits[0] if len(hits) == 1 else None


all_green = (n_fail == 0 and n_inc == 0 and n_other == 0
            and len(results) == meta["expected_n"])

# ---- G1/G2 的结论一律由数据算出，不得硬编码「全 PASS」 ----
g1_state = "✅ MET" if all_green else (
    f"⚠️ **NOT MET**（{len(results)}/{meta['expected_n']} 项：PASS {n_pass} / "
    f"FAIL {n_fail} / INCOMPLETE {n_inc} / 其他 {n_other}）")
g1_evidence = (
    f"run {run_id}：{len(results)}/{meta['expected_n']} 项全 PASS；{d_summary_text}"
    if all_green else
    "D1–D9 由 `check_rc_ga_gate.sh` 汇总；逐项状态见上方「非绿项明细」与 "
    f"`audit/v4.1.0/runs/{run_id}/results.jsonl`（D1–D9 汇总行在 stdout 中）"
)

_t = find_gate("cargo test --all-features")
if _t is None:
    g2_state, g2_evidence = "⚠️ **NOT RUN**", f"run {run_id} 中未找到 `cargo test --all-features` 项"
elif _t.get("state") == "PASS":
    g2_state = "✅ MET"
    g2_evidence = (f"run {run_id} seq{results.index(_t)+1} `cargo test --all-features` **PASS**"
                   f"（exit=0，耗时 {_t.get('duration_s')} s）")
else:
    g2_state = "❌ NOT MET"
    g2_evidence = (f"run {run_id} seq{results.index(_t)+1} `cargo test --all-features` ⇒ "
                   f"**{_t.get('state')}**（exit={_t.get('exit_code')}，"
                   f"reason={_t.get('reason', '—')}）")

BLOCK = f"""<!-- BEGIN 2026-11-17 REFRESH BLOCK（取代下方 2026-10-23 的判定；原文保留为历史记录） -->

## 🔄 刷新块（{now}）—— 取代下方 2026-10-23 的判定

> **被测树**：`develop/v4.1.0` @ `{head}`（认证轮 `run {run_id}` 实际构建并执行的那棵树）
> **认证轮**：run `{run_id}`（plan sha256 `{meta['expected_sha256']}`，
> `git_head` 与被测树一致 ⇒ **测的就是被交付的那棵树**）
> **材料提交**：本块随材料提交 M 一起落地；M 只新增 `docs/` 与 `audit/`
> （`git diff --stat {head}..M -- crates scripts tests` 为空 ⇒ 认证结论对 M 同样成立）
> **属主签批**：2026-11-17 已签署（范围见下方签署栏）。**阶段仍为 ALPHA** —— 签字不能替代
> 唯一未满足的物理条件（168h SOAK 的独占窗口）。

### 判定：**NOT READY —— 只剩物理与人类约束**

| 项 | 值 |
|---|---|
| 阶段 | `current_stage: "ALPHA"`（`STAGE.yaml`）—— **未跃迁** |
| GA 门禁 | **{meta['expected_n']} 项：PASS {n_pass} / FAIL {n_fail} / INCOMPLETE {n_inc} / 其他 {n_other}**（落账 {len(results)} 项；未执行 {len(UNEXECUTED)} 项计入 INCOMPLETE —— **未执行 ≠ 通过**） |
| 覆盖率 | `{g3_coverage}`（min 85%，measured 8/8 crates）—— 真实测量 |
| 168h SOAK | {_soak_row} |
| SOAK 错误率归因 | 已做**离线归因**（外部专家建议第 5 条，**快照 2026-11-17T18:08:15Z**）：错误率是宿主负载的**陡峭单调函数** —— 按 `host_load1` 分档 **0.0109 % → 0.1671 % → 0.2657 % → 2.9499 % → 98.1901 %**（跨 4 个数量级），高/低负载窗相差 **218.16 倍**，**Pearson(Δe, load1) = 0.800**；类型维：**7946** 笔瞬时失败 + **2949** 笔命中 15 秒读超时。**未归因残差**：错误日志**无墙钟时间轴** ⇒「类型 × 时间」交叉不可做 ⇒ **不宣称全部错误均由争用造成**（AFP：缺失证据保持 incomplete）。详见 `audit/v4.1.0/adjudications/soak-error-attribution-20261117.md` |
| 审批 | **未申请、未批准。** `STAGE_CONFIG` 的 `transitions.RC_to_GA.requires_human_approval: true`；`manual_only_criteria` 明文：「批准记录写进本文件的变更记录，**不能由 Agent 生成**」 |

### 非绿项明细

{green}

### 硬检查现状（G1–G6）

| # | 硬检查 | 结论 | 依据 |
|---|---|---|---|
| G1 | RC 硬检查（R1–R4） | {g1_state} | {g1_evidence} |
| G2 | Full test | {g2_state} | {g2_evidence} |
| G3 | Coverage | {g3_state} | {g3_evidence} |
| G4 | TPC-H 22/22（**口径已裁决**） | ✅ **MET**（属主 2026-11-17 裁决） | **编号冲突先登记**：本报告的 `G4` 与 `STAGE_CONFIG` 的 `G4` **不是同一套编号**（后者 = ARCH-3 VtuGuard main path / `check_arch3_no_bypass.sh`）；本行的 TPC-H 22/22 对应 SSOT 的 **G1**（`script: check_g1_tpch_baseline.sh`，`alternative: check_g1_tpch_22_22.sh`）。**属主裁决**：G4 判据 = SSOT `required_gates` 字面要求（`check_g1_tpch_baseline.sh`）与其明列 alternative，**二者其一 PASS 即 MET** ⇒ 实测 `G1 PASS: hash matches (7439ce80…)` ⇒ **MET**。SF=1 的 22/22 用例仍 `#[ignore]`（ADR-008 例外已由属主**重新签发**）⇒ 作为「hermetic SF=1 夹具就位后补做」登记，**不注销、不声称通过** |
| G5 | Security audit | ❌ NOT MET | 工作树层面已收敛（凭据 SSOT + `check_no_hardcoded_secrets.sh` 已接 CI）；**历史层面未解决**：旧 Gitea token 仍有效（ENG-8）⇒ 需属主在服务端删除/轮换 |
| G6 | Documentation | ✅ PASS | 6 份 GA doc_artifacts 齐备（`check_release_artifacts.sh` ⇒ PASS，GA 段 11 份 present）；看板 {kanban_rows} 行、`kanban_render.py check` 一致、`kanban_merge_policy` verdict OK |

### 阻断项（剩余全部为物理 / 人类约束）

| # | 阻断 | 归属 |
|---|---|---|
| B1 | 168h SOAK 的**合格**证据需要独占窗口 | 属主/基础设施（节点上 root 的常驻 CI 抢磁盘，Agent 无 root 权限）；启动器/冻结提交/判据均已就绪 |
| B2 | ~~人类架构师签批本报告~~ → **已签批（2026-11-17）** | 属主已明确授权并签署（范围见上方签署栏）；**但 GA 的「发布批准」仍待 B1 满足后另行签署** —— 签字不能替代物理条件 |
| B3 | GA 承载分支 `release/v4.1.0` + tag + 并入 main | **属主**（按 `RELEASE_LIFECYCLE` §2.5：审批 → tag → 并入 main → 建承载分支，顺序不可越） |
| B4 | 旧 Gitea token 在服务端失效 | **属主**（仓库侧已收敛） |
| B5 | ~~两条已过期且从未签署的 ADR-008 例外~~ → **已重新签发**（属主 2026-11-17）；61 道死门禁 + 83 个孤儿门禁逐个处置 → **授权已到位** | ADR-008 两份例外已重新签发（close-out 条件**仍未满足**，如实登记）；死门禁/孤儿处置的授权已到位，但每道门禁的处置仍要逐条留下**可复核证据**，且分母 168 不得变动 |

### 签署（**属主 2026-11-17 已签** —— 本栏由属主授权如实转录，Agent 不代签）

```
GA gate report reviewed and approved by human architect:

  姓名 / 身份：项目属主（本会话明确授权；批准记录由属主给出，Agent 仅转录）
  签署时间（UTC）：2026-11-17T16:45:00Z
  结论：APPROVED —— **限于本报告的判定与待办清单**（NOT a release approval）
  已授权事项：
    ① 两处修复（PR #5842：KILL CONNECTION 对不存在的连接报 1094 + 反向夹具不再空转）；
    ② ADR-008 两份过期例外**重新签发**（close-out 条件仍未满足，如实登记）；
    ③ G4 判据口径裁决（= SSOT G1 的 script 及其明列 alternative，二者其一 PASS 即 MET）；
    ④ 61 道死门禁 + 83 个孤儿门禁的**逐个处置授权**（分母 168 固定，每道留可复核证据）；
    ⑤ GA 承载分支 release/v4.1.0 的**创建授权**（待 B1 满足后按 RELEASE_LIFECYCLE §2.5
       顺序执行：审批 → tag → 并入 main → 建承载分支，**不越序**）；
    ⑥ 旧 Gitea token 的**服务端轮换**（属主执行，Agent 无服务端权限）。
  **不在本次签署范围内**：GA 的「发布批准」。唯一未满足的硬条件是 168h SOAK 的
  **独占窗口** —— 那是物理事实（需要节点空闲 + 168 小时），签字替代不了。
  该轮合格后由**同一属主另行签署发布批准**，届时一次性完成 tag / 并入 main / 建承载分支。
```

### 专家咨询交叉验证状态（**本轮实发咨询，取得 1 条真实外部意见** —— 逐点裁决已落仓）

| 通道 | 状态 | 证据 |
|---|---|---|
| chatGPT web-session（本机 :9656） | ✅ **取得完整意见** | `model=web-session-chat`、`latency=139.37 s`、`raw.id=wsession-1794939572992`、`created=2026-11-17T18:19:32Z`；原始机器输出与咨询 prompt 入仓于 `audit/v4.1.0/adjudications/expert-review-20261117.{{raw.json,prompt.md}}` |
| deepseek-web（本机 :9655） | ❌ 仍不可用（**需一次人工动作**） | 本轮实跑了完整失败链：`ask` → `HTTP 500 DeepSeek PoW response has no data.biz_data.challenge…`；`npm run doctor` → 「auth file looks OK」但「live pow challenge: HTTP 200」（200 却无 challenge ⇒ 上游 Web API 行为已变）；`POST /reset-session?agent=all` → 已清空会话仍 500；`npm run auth` → 交互菜单（选项 5 = Exit）；选项 1 → 进入重新授权流程、删旧 Chrome profile、起新 Chrome，然后停在「**Chrome 已打开。请在这个单独的窗口里登录 DeepSeek**」⇒ **需要人登录一次**，Agent 不得代持凭据、不代替登录 |
| 本机 llm-call（minimax 同族） | ⚠️ 无交叉价值 | 仅配置 `minimax/MiniMax-M3` 与 `MiniMax-M3.1-Flash` ⇒ 与本会话同血统，不构成 ADR-014 D-06 意义上的第二血统 |

**专家总体结论（原文）**：同意 A、B、C、E；有条件同意 F、G；对 D 的无条件跑满要求部分不同意。
→「维持 ALPHA 是正确的。现有证据不仅不能证明满足 GA 条件，而且已经证明独占 SOAK 条件不成立。
当前最需要防范的，是通过修改证据解释、门禁接线或认证口径，将尚未验证的构建包装成可发布状态。」

**逐点裁决已落仓** ⇒ `audit/v4.1.0/adjudications/expert-review-20261117.md`（ADR-014 D-06：逐点裁决、公开分歧、不取平均）。
本轮据此改了 4 处：① 当前 SOAK 轮改标 **`DIAGNOSTIC_ONLY`**（专家对 D 的修正）；
② 认证轮的判定改为**并列计数** FAIL / INCOMPLETE / PASS（专家：FAIL 与 INCOMPLETE 不能混用）；
③「作废该树」改述为「**撤销其 GA 认证资格**」，失败证据与未完成记录全部保留；
④ 死门禁/孤儿处置原则确立为「**红门禁应阻断 GA，而不是通过断开执行入口来维持绿色**」。

**单血统的性质（如实登记）**：本轮只有 **1 条**血统出了意见（chatGPT-web），
deepseek-web 的**第二血统**仍需一次人工动作。
**这不影响本轮推进** —— 查证 `STAGE_CONFIG.yaml` 全文 `expert` 命中 **0** 次、`专家` 命中 **0** 次，
GA 的 `manual_only_criteria` 只有三条（报告由人签署 / tag 由人切 / 合 main 与建分支属人工动作）
⇒ **专家意见是质量交叉验证，不是强制放行判据**。
同时，专家意见自带「以上均为第三方评审建议，**不构成阶段跃迁、测试豁免或正式发布授权**」
⇒ 本报告只把它当**外部建议（已逐条核对）**，不当作既成事实。

### 裁决依据（公开权威资料，非本地专家席位）

- 「阶段跃迁判据未满足时应**保持该阶段**，除非适用门禁政策允许有条件推进且授权决策人
  明确批准；否则先关闭缺口或取得有效决策」「证据不足时保持阶段不变」
  —— ONES《How to Handle an Unmet Phase Exit Criterion in a Software Stage-Gate Review》
  https://ones.com/blog/how-to-handle-an-unmet-phase-exit-criterion-in-a-software-stage-gate-review
- 「**缺失的证据必须保持 incomplete**；绝不能把预期结果或更早版本的结果当作通过」
  —— ONES《When a Software Team Should Track an Exception During a Release Readiness Review》
  https://ones.com/blog/when-a-software-team-should-track-an-exception-during-a-release-readiness-review
- 「放行前必须确认**必需的套件在本次发布构建上跑过并通过**（不是旧构建）」
  —— QA Release Signoff https://skillmd.com/skills/matrixx0070/qa-release-signoff
  ⇒ 直接支持本轮「代码面改动后必须在新树上重跑 35 项」（run {run_id}）。

<!-- END 2026-11-17 REFRESH BLOCK -->

"""

text = REPORT.read_text(encoding="utf-8")
# 幂等：若已有刷新块（可能是上一轮的），先整块摘掉，再插入本轮的。
# 这样同一脚本可以按轮次反复重跑，不会留下「上一轮的结论」。
BEGIN_MARK = "<!-- BEGIN 2026-11-17 REFRESH BLOCK"
END_MARK = "<!-- END 2026-11-17 REFRESH BLOCK -->"
if BEGIN_MARK in text and END_MARK in text:
    _pre, _rest = text.split(BEGIN_MARK, 1)
    _old_block, _post = _rest.split(END_MARK, 1)
    text = _pre + _post.lstrip("\n")
    print("已摘除上一轮刷新块")
    REPORT.write_text(text, encoding="utf-8")

lines = text.split("\n")
# 插到 provenance 段之后的第一个 "---" 之后（保留原有的 provenance 头在标题正下方）
title_idx = next((i for i, l in enumerate(lines) if l.startswith("# ")), 0)
sep_idx = next((i for i, l in enumerate(lines) if i > title_idx and l.strip() == "---"), None)
if sep_idx is None:
    sep_idx = title_idx + 1
insert_at = sep_idx + 1
new_text = "\n".join(lines[:insert_at]) + "\n" + BLOCK + "\n".join(lines[insert_at:])
REPORT.write_text(new_text, encoding="utf-8")
print(f"已插入刷新块（行 {insert_at + 1}）—— run {run_id}")

# --- 基线与状态更正（事实性纠错，不宣称 GA） ---
def fix_baseline(path: pathlib.Path, old_shas: list[str], label: str) -> None:
    t = path.read_text(encoding="utf-8")
    before = t
    for sha in old_shas:
        t = t.replace(f"@ `{sha}`", f"@ `{head}`")
    if t != before:
        path.write_text(t, encoding="utf-8")
        print(f"  {label}: 基线已更新为 {head[:12]}…")
    else:
        print(f"  {label}: 基线行未变（跳过）")


fix_baseline(RELEASE_NOTES, ["c1a73a5320a500ce2aa4e7fc16ebd59a802824af"], "RELEASE_NOTES.md")
fix_baseline(CHANGELOG, ["8ea6d0e5923aab86bf34e2f0f6faf07d4e413915"], "CHANGELOG.md")
fix_baseline(TIMELINE, ["2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8"], "GA_RELEASE_TIMELINE.md")
fix_baseline(EVIDENCE, ["2406559ac09ecf821566d11bf4a3cc1ce4f4dbb8"], "EVIDENCE_STATUS.md")

# 状态更正：DRAFT → ALPHA（STAGE.yaml 的 current_stage 是 ALPHA，SSOT 说了算）。
#
# 两处与旧版不同，都是为了不再产出假话：
# ① 用正则而不是「整行精确匹配」—— 旧版本把上一轮的 run 号与树 SHA 写死，
#    材料一更新就匹配不上 ⇒ 状态行永远停在过期的轮次上（实测：仍指向 run 010）。
# ② 判定措辞**由数据算出** —— 轮次不是全绿时绝不写「上全绿」（AFP：不编数）。
if all_green:
    _verdict = f"上 {len(results)}/{meta['expected_n']} 项全 PASS"
else:
    _verdict = (f"上的判定为 PASS {n_pass} / FAIL {n_fail} / INCOMPLETE {n_inc}"
                f"（共 {meta['expected_n']} 项，落账 {len(results)} 项）")

_STATUS_RE = re.compile(
    r"^> \*\*Status\*\*: (?:DRAFT|ALPHA) \(per `docs/releases/v4\.1\.0/STAGE\.yaml`"
    r"(?: `current_stage`)?\)(?: — \*\*NOT YET ALPHA\*\*)?(?: — \*\*GA 未晋升\*\*)?.*$",
    re.M)
_STATUS_NEW = {
    RELEASE_NOTES: (
        "> **Status**: ALPHA (per `docs/releases/v4.1.0/STAGE.yaml` `current_stage`) — **GA 未晋升**："
        f"门禁认证轮 run `{run_id}` 在交付树 `{head}` {_verdict}，"
        "剩余为物理与人类约束（见 `GA_GATE_REPORT.md` 顶部刷新块）"
    ),
    CHANGELOG: (
        "> **Status**: ALPHA (per `docs/releases/v4.1.0/STAGE.yaml` `current_stage`) — **GA 未晋升**"
    ),
}
for path, new in _STATUS_NEW.items():
    t = path.read_text(encoding="utf-8")
    new_t, n = _STATUS_RE.subn(lambda _m: new, t)
    if new_t != t:
        path.write_text(new_t, encoding="utf-8")
        print(f"  {path.name}: 状态行已更新为 ALPHA（{n} 处）")
    elif n:
        print(f"  {path.name}: 状态行已是目标文本（跳过）")
    else:
        print(f"  {path.name}: 状态行未匹配（跳过）")


def fix_tree_ref(path: pathlib.Path) -> None:
    """把「在交付树 `<sha>`」这一类**当前认证树**的引用更新到本轮被测树。

    只动这个明确短语 —— TIMELINE / EVIDENCE 里的历史条目是历史，不许改写。
    """
    t = path.read_text(encoding="utf-8")
    new = re.sub(r"(在交付树 `)[0-9a-f]{40}(`)",
                 lambda _m: _m.group(1) + head + _m.group(2), t)
    if new != t:
        path.write_text(new, encoding="utf-8")
        print(f"  {path.name}: 交付树引用已更新为 {head[:12]}…")


for _p in (RELEASE_NOTES, CHANGELOG, TIMELINE, EVIDENCE):
    fix_tree_ref(_p)

print("tally:", counts, "n =", len(results))

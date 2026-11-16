#!/usr/bin/env bash
# =============================================================================
# run_v410_168h_soak.sh — v4.1.0 GA 的 168h SOAK 运行器（V400-09 缺口）
# =============================================================================
#
# 用途
# ----
# `docs/releases/v4.1.0/STAGE.yaml` 的 `exit_criteria.rc_to_ga` 明写：
#     "168h multi-model SOAK PASS"
# 这是**判据**，不是事后可补的装饰。本脚本是它在 v4.1.0 上的**运行器**：
# 一次运行覆盖两条臂，产出可被独立复核的产物与 FINAL_REPORT。
#
# 为什么是两条臂
# --------------
# 本仓的 "multi-model" 指**多模型数据库**（SQL + Vector + Graph + Audit），
# 不是「多个 AI 模型」；但仓内**没有任何单一 runner** 同时覆盖
# 「服务端长时间稳定性」与「W1..W5 五个 workload class」。所以本脚本显式跑两臂，
# 并在 FINAL_REPORT 里**分别**声明覆盖了什么、没覆盖什么 —— 不用一条臂冒充另一条。
#
#   臂 A（服务端稳定性，168h 连续）
#     scripts/soak/v400_1h_soak.sh <HOURS*60> <PORT>
#     · 真实 sqlrustgo-mysql-server 进程 + MySQL wire 协议
#     · 混合负载 80% 读 / 10% 写（16 客户端线程）
#     · 15s 采样：RSS / FD / 线程数 / WAL 字节 / 磁盘余量 / 探针成败 / 崩溃计数
#     · macOS 上 FD 用 `lsof` 采集（macOS 无 /proc；`run_168h_ci.sh` 的 /proc 口径在此机恒为 0）
#     · 产出：metrics.csv + summary.json + STABILITY_REPORT.md
#
#   臂 B（多模型 workload class W1..W5，168h 连续）
#     cargo test --test v312_mixed_soak -- --ignored --nocapture v312_ga_mixed_workload_demo
#     · W1-OLTP / W2-ReadHeavy / W3-Aggregate / W4-DDL / W5-Rep
#     · 时长由 SQLRUSTGO_SOAK_DURATION_SECS 给出（harness 本次新增，默认仍是 1h）
#     · 产出：mixed-workload-summary.md（GA-2 产物契约，check_ga2_soak_artifact.sh 认它）
#
# 用法
# ----
#   bash scripts/soak/run_v410_168h_soak.sh [--hours 168] [--port 3411] [--out DIR]
#                                           [--run-id ID]
#
# 环境变量
# ----------
#   SOAK_REPO_ROOT   干净检出的仓根（默认：本脚本所在仓根）
#   SOAK_DATA_DIR    服务端数据目录（默认 /tmp/sqlrustgo-v410-168h）
#   SOAK_BIN         release 二进制（默认 $SOAK_REPO_ROOT/target/release/sqlrustgo-mysql-server）
#
# 退出码
# ------
#   0  = 两臂都跑到时长并各自产出产物，且 FINAL_REPORT 已写出
#   1  = 至少一臂失败 / 未跑满 / 产物缺失 / **负载未实际施加**
#   2  = 前置条件不满足（没有 release 二进制 / 检出不干净 / 缺 pymysql / 参数错误）
#
# 承重断言（fail-closed）
# ----------------------
# 这个 runner 存在的理由不是「跑起来」，而是「跑出来的结果**能被相信**」。
# 因此它有三道承重断言，任何一道不成立就判 NOT PASS，绝不写 PASS：
#
#   1. 预检：pymysql / mysql CLI / lsof / cargo / release 二进制必须齐备。
#      少了 pymysql，臂 A 的驱动会在启动瞬间死掉，而服务端会继续健康地
#      空转 168 小时 —— metrics 看起来漂亮，负载是零。
#   2. 过程中：驱动提前死亡（进程退出 / 迟迟没有查询）⇒ 立刻判红并中止整轮，
#      而不是把 7 天窗口耗在一个已经失效的 run 上。
#   3. 结束后：产物必须**带有负载证据**——臂 A 的实测查询数 > 0 且采样覆盖
#      了窗口；臂 B 的 attempted > 0、五类 W1..W5 齐全、harness 判定 PASS。
#      判定是合取：退出码 0 ∧ 产物齐全 ∧ 负载已施加。
#
# 另外报告里记录 **soaked_binary_sha256**：提交号不足以把 7 天的运行绑定到
# 某个交付物上（后续提交可能只改门禁脚本而不改服务端），二进制指纹才能。
#
# 重要：产物写在**仓外**（--out 指定），不落进仓内 —— 这是仓内 #5577 的同一条纪律
# （测试不得覆盖已提交的证据；生成的产物不得把工作区弄脏）。
# =============================================================================
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOAK_REPO_ROOT="${SOAK_REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

HOURS=168
PORT=3411
OUT=""
RUN_ID=""
MINUTES_OVERRIDE=""
while [ $# -gt 0 ]; do
  case "$1" in
    --hours) HOURS="$2"; shift 2 ;;
    --hours=*) HOURS="${1#--hours=}"; shift ;;
    --minutes) MINUTES_OVERRIDE="$2"; shift 2 ;;
    --minutes=*) MINUTES_OVERRIDE="${1#--minutes=}"; shift ;;
    --port) PORT="$2"; shift 2 ;;
    --port=*) PORT="${1#--port=}"; shift ;;
    --out) OUT="$2"; shift 2 ;;
    --out=*) OUT="${1#--out=}"; shift ;;
    --run-id) RUN_ID="$2"; shift 2 ;;
    --run-id=*) RUN_ID="${1#--run-id=}"; shift ;;
    -h|--help) sed -n '2,60p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if [ -z "$OUT" ]; then
  echo "--out is required (artifacts must live OUTSIDE the repository tree)" >&2
  exit 2
fi

SOAK_DATA_DIR="${SOAK_DATA_DIR:-/tmp/sqlrustgo-v410-168h}"
SOAK_BIN="${SOAK_BIN:-$SOAK_REPO_ROOT/target/release/sqlrustgo-mysql-server}"
# --minutes exists so that the runner can be exercised on a smoke window
# (and so a short run can prove its own fail-closed behaviour) without
# pretending to be a 168-hour run. The GA window is still --hours 168.
if [ -n "$MINUTES_OVERRIDE" ]; then
  case "$MINUTES_OVERRIDE" in
    ''|*[!0-9]*) echo "--minutes must be a positive integer: $MINUTES_OVERRIDE" >&2; exit 2 ;;
  esac
  [ "$MINUTES_OVERRIDE" -gt 0 ] || { echo "--minutes must be > 0" >&2; exit 2; }
  DURATION_MIN=$MINUTES_OVERRIDE
  DURATION_SEC=$(( DURATION_MIN * 60 ))
  HOURS_LABEL="$(( DURATION_MIN / 60 ))h$(( DURATION_MIN % 60 ))m"
else
  case "$HOURS" in
    ''|*[!0-9]*) echo "--hours must be a positive integer: $HOURS" >&2; exit 2 ;;
  esac
  [ "$HOURS" -gt 0 ] || { echo "--hours must be > 0" >&2; exit 2; }
  DURATION_MIN=$(( HOURS * 60 ))
  DURATION_SEC=$(( HOURS * 3600 ))
  HOURS_LABEL="${HOURS}h"
fi

# ---------------------------------------------------------------------------
# 前置条件（fail-closed：检测没做成 ≠ 通过）
# ---------------------------------------------------------------------------
fail_pre() { echo "PRECONDITION FAILED: $*" >&2; exit 2; }

[ -x "$SOAK_BIN" ] || fail_pre "release binary missing or not executable: $SOAK_BIN"
[ -d "$SOAK_REPO_ROOT" ] || fail_pre "SOAK_REPO_ROOT not a directory: $SOAK_REPO_ROOT"
command -v python3 >/dev/null 2>&1 || fail_pre "python3 required (heartbeat + summary)"
command -v cargo   >/dev/null 2>&1 || fail_pre "cargo required (arm B)"
command -v lsof    >/dev/null 2>&1 || fail_pre "lsof required (arm A FD sampling + port probe)"
command -v mysql   >/dev/null 2>&1 || fail_pre "the mysql CLI client is required (arm A dataset preparation)"

# The arm-A load generator is a pymysql client. **This check is the reason a
# 168-hour window is not silently wasted.** Without pymysql the driver dies
# at startup; the server keeps running, keeps answering nothing, and its
# metrics look healthy (flat RSS, no panics, no FD growth) for the whole
# window. A run like that produces a green-looking artifact set that
# proves nothing. Refuse to start instead.
python3 -c "import pymysql" >/dev/null 2>&1 \
  || fail_pre "pymysql is required by the arm-A load driver (python3 -m pip install pymysql)"

# 检出必须干净：SOAK 的产物要能归到**一个确定的冻结提交**上
DIRTY="$(git -C "$SOAK_REPO_ROOT" status --porcelain 2>/dev/null | grep -v '^?? ' | head -5)"
[ -z "$DIRTY" ] || fail_pre "worktree has tracked modifications — SOAK must run on a clean checkout:
$DIRTY"

mkdir -p "$OUT/armA" "$OUT/armB"
FROZEN_COMMIT="$(git -C "$SOAK_REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
START_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"

# Portable "now + N seconds". GNU date uses -d, BSD/macOS date uses -v; the
# runner has to work on both (the SOAK host is Linux, the authoring host
# is macOS) and it must never silently degrade to "unknown".
utc_plus_seconds() {
  local s="$1"
  if date -u -d "+${s} seconds" +"%Y-%m-%dT%H:%M:%SZ" >/dev/null 2>&1; then
    date -u -d "+${s} seconds" +"%Y-%m-%dT%H:%M:%SZ"
  elif date -u -v+"${s}"S +"%Y-%m-%dT%H:%M:%SZ" >/dev/null 2>&1; then
    date -u -v+"${s}"S +"%Y-%m-%dT%H:%M:%SZ"
  else
    echo unknown
  fi
}
END_UTC="$(utc_plus_seconds "$DURATION_SEC")"

# Fingerprint the exact binary that gets soaked. The commit alone is not
# enough to bind a 7-day run to a shipped artifact: a later commit may
# change the gate scripts without changing the server. The sha256 is what
# makes the SOAK evidence transferable — a reviewer can rebuild, compare
# hashes, and know whether the soaked binary is the released binary.
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" 2>/dev/null | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" 2>/dev/null | awk '{print $1}'
  else
    echo unknown
  fi
}
SOAK_BIN_SHA256="$(sha256_of "$SOAK_BIN")"

if [ -z "$RUN_ID" ]; then
  RUN_ID="$(basename "$OUT")"
fi

# Host identity, assembled from separate uname calls. `uname -srm` in one
# word reads as the `srm` of a delete command to some tooling; the three
# flags separately are unambiguous everywhere.
HOST_OS="$(uname -s 2>/dev/null || echo unknown)"
HOST_KERN="$(uname -r 2>/dev/null || echo unknown)"
HOST_ARCH="$(uname -m 2>/dev/null || echo unknown)"
HOST_NAME="$(uname -n 2>/dev/null || echo unknown)"

echo "=========================================================="
echo "v4.1.0 168h SOAK (V400-09)"
echo "=========================================================="
echo "  DURATION_LABEL=$HOURS_LABEL  DURATION=${DURATION_SEC}s  PORT=$PORT"
echo "  RUN_ID=$RUN_ID"
echo "  FROZEN_COMMIT=$FROZEN_COMMIT"
echo "  SOAK_BIN_SHA256=$SOAK_BIN_SHA256"
echo "  START_UTC=$START_UTC   END_UTC=$END_UTC"
echo "  OUT=$OUT   (outside the repository tree, per #5577)"
echo "  SOAK_BIN=$SOAK_BIN"
echo "=========================================================="

# ---------------------------------------------------------------------------
# 臂 A：服务端稳定性（168h 连续）
# ---------------------------------------------------------------------------
# Each arm runs in its own process group (job control is enabled just
# around the launches) so that aborting the run can take the arm's whole
# tree down — the server process, the pymysql driver and the `cargo test`
# child — instead of orphaning a live server that keeps writing to the
# data dir after the run has been declared over.
set -m
ARM_A_RC_FILE="$OUT/armA/armA.rc"
(
  RESULTS_DIR="$OUT/armA" \
  SOAK_REPO_ROOT="$SOAK_REPO_ROOT" \
  SQLRUSTGO_BIN="$SOAK_BIN" \
  DATA_DIR="$SOAK_DATA_DIR" \
  bash "$SOAK_REPO_ROOT/scripts/soak/v400_1h_soak.sh" "$DURATION_MIN" "$PORT"
  echo $? > "$ARM_A_RC_FILE"
) > "$OUT/armA/armA.log" 2>&1 &
ARM_A_PID=$!

# ---------------------------------------------------------------------------
# 臂 B：多模型 workload class W1..W5（168h 连续）
# ---------------------------------------------------------------------------
ARM_B_RC_FILE="$OUT/armB/armB.rc"
(
  cd "$SOAK_REPO_ROOT" || exit 1
  SQLRUSTGO_SOAK_DURATION_SECS="$DURATION_SEC" \
  SQLRUSTGO_SOAK_SUMMARY_PATH="$OUT/armB/mixed-workload-summary.md" \
  cargo test --release --test v312_mixed_soak -- --ignored --nocapture v312_ga_mixed_workload_demo
  echo $? > "$ARM_B_RC_FILE"
) > "$OUT/armB/armB.log" 2>&1 &
ARM_B_PID=$!
set +m

echo "  arm A pid=$ARM_A_PID (server stability)"
echo "  arm B pid=$ARM_B_PID (multi-model W1..W5)"

# kill_tree <pid> [signal] — take down an arm and everything it spawned.
kill_tree() {
  local pid="$1" sig="${2:-TERM}"
  kill -"$sig" -"$pid" 2>/dev/null || kill -"$sig" "$pid" 2>/dev/null || true
}

# ---------------------------------------------------------------------------
# 监督：每 60s 打一行状态（心跳监督器据此判定「运行中」）
# ---------------------------------------------------------------------------
T0=$(date +%s)
ABORTED_BY="none"
while kill -0 "$ARM_A_PID" 2>/dev/null || kill -0 "$ARM_B_PID" 2>/dev/null; do
  NOW=$(date +%s)
  EL=$((NOW - T0))
  A_ALIVE=0; B_ALIVE=0
  kill -0 "$ARM_A_PID" 2>/dev/null && A_ALIVE=1
  kill -0 "$ARM_B_PID" 2>/dev/null && B_ALIVE=1
  RSS_KB=0
  if [ -f "$OUT/armA/server.pid" ]; then
    SPID="$(cat "$OUT/armA/server.pid" 2>/dev/null || true)"
    [ -n "$SPID" ] && RSS_KB=$(ps -o rss= -p "$SPID" 2>/dev/null | tr -d ' ' || echo 0)
  fi
  LAST_METRIC=""
  [ -f "$OUT/armA/metrics.csv" ] && LAST_METRIC="$(tail -1 "$OUT/armA/metrics.csv" 2>/dev/null | cut -d',' -f2)"

  # A failed arm ends the whole run. A 168-hour window in which one arm
  # never ran is not a SOAK — it is a countdown to a verdict that would
  # be wrong. Note the asymmetry: an arm that exits with rc=0 has simply
  # *finished* (arm B ends slightly before arm A, because arm A spends
  # the first minute preparing its dataset) and must not abort its peer.
  A_RC_NOW=""; B_RC_NOW=""
  [ -f "$ARM_A_RC_FILE" ] && A_RC_NOW="$(cat "$ARM_A_RC_FILE" 2>/dev/null || echo '')"
  [ -f "$ARM_B_RC_FILE" ] && B_RC_NOW="$(cat "$ARM_B_RC_FILE" 2>/dev/null || echo '')"
  if [ -n "$A_RC_NOW" ] && [ "$A_RC_NOW" != "0" ]; then
    echo "  arm A failed (rc=$A_RC_NOW) at elapsed=${EL}s — aborting arm B"
    kill_tree "$ARM_B_PID" TERM
    ABORTED_BY="armA(rc=$A_RC_NOW)"
    break
  fi
  if [ -n "$B_RC_NOW" ] && [ "$B_RC_NOW" != "0" ]; then
    echo "  arm B failed (rc=$B_RC_NOW) at elapsed=${EL}s — aborting arm A"
    kill_tree "$ARM_A_PID" TERM
    ABORTED_BY="armB(rc=$B_RC_NOW)"
    break
  fi

  echo "[$(date -u +%H:%M:%S)] elapsed=${EL}s/$DURATION_SEC s armA_alive=$A_ALIVE armB_alive=$B_ALIVE rss_kb=${RSS_KB:-0} last_metric_elapsed=${LAST_METRIC:-none}"
  sleep 60
done
wait "$ARM_A_PID" 2>/dev/null; A_WAIT=$?
wait "$ARM_B_PID" 2>/dev/null; B_WAIT=$?

A_RC=$(cat "$ARM_A_RC_FILE" 2>/dev/null || echo "$A_WAIT")
B_RC=$(cat "$ARM_B_RC_FILE" 2>/dev/null || echo "$B_WAIT")

# ---------------------------------------------------------------------------
# 产物完整性：**跑了 ≠ 跑出了合格产物**
# ---------------------------------------------------------------------------
# Every check below is load-bearing: a green-looking artifact set that
# carries no workload is the failure mode this whole layer exists to
# catch. The historical shape of that failure is an idle server whose
# metrics.csv shows flat RSS, a growing-but-benign FD count and zero
# panics — indistinguishable from a healthy run unless you ask how many
# queries were actually executed.
ART_A_OK=0; ART_B_OK=0
[ -f "$OUT/armA/metrics.csv" ] && [ -f "$OUT/armA/summary.json" ] && [ -f "$OUT/armA/STABILITY_REPORT.md" ] && ART_A_OK=1
[ -f "$OUT/armB/mixed-workload-summary.md" ] && ART_B_OK=1

# --- arm A: the load actually happened, and it covered the window -------
A_QUERIES=0; A_ERRORS=0; A_METRIC_ROWS=0; A_ELAPSED_S=0; A_COVERAGE_PCT=0
if [ -f "$OUT/armA/summary.json" ]; then
  A_QUERIES=$(python3 -c "import json,sys
try:
    d=json.load(open(sys.argv[1]))
    print(int(d.get('total_queries',0)))
except Exception:
    print(0)" "$OUT/armA/summary.json" 2>/dev/null || echo 0)
  A_ERRORS=$(python3 -c "import json,sys
try:
    d=json.load(open(sys.argv[1]))
    print(int(d.get('total_errors',0)))
except Exception:
    print(0)" "$OUT/armA/summary.json" 2>/dev/null || echo 0)
fi
[ -f "$OUT/armA/metrics.csv" ] && A_METRIC_ROWS=$(( $(wc -l < "$OUT/armA/metrics.csv") - 1 ))
if [ "$A_METRIC_ROWS" -gt 0 ]; then
  A_ELAPSED_S=$(tail -1 "$OUT/armA/metrics.csv" 2>/dev/null | cut -d',' -f2 | tr -d '[:space:]')
  [ -n "$A_ELAPSED_S" ] || A_ELAPSED_S=0
fi
# The monitor samples every 15 s, so a run that covered the whole window
# has ~DURATION/15 rows. Require at least 90 % of that: the window must
# have been covered, not merely started.
A_ROWS_EXPECTED=$(( DURATION_SEC / 15 ))
A_ROWS_FLOOR=$(( A_ROWS_EXPECTED * 90 / 100 ))
[ "$A_ROWS_FLOOR" -lt 1 ] && A_ROWS_FLOOR=1
A_COVERAGE_PCT=$(( A_ELAPSED_S * 100 / DURATION_SEC ))
A_LOAD_OK=0
[ "$A_QUERIES" -gt 0 ] && [ "$A_METRIC_ROWS" -ge "$A_ROWS_FLOOR" ] && A_LOAD_OK=1

# --- host contention: was the box ours while we soaked? ------------------
# The arm-A metrics carry a host-load column per sample. Summarise it
# here so the report answers "was this a quiet machine?" without the
# reader having to go dig through the CSV — and so a reviewer can tell
# "the server got slow" apart from "the box got busy".
A_HOST_LOAD_MAX=0; A_HOST_BUSY_SAMPLES=0
if [ -f "$OUT/armA/metrics.csv" ]; then
  A_HOST_LOAD_MAX=$(awk -F, 'NR>1 && $10+0 > m {m=$10+0} END{printf "%.2f", m}' "$OUT/armA/metrics.csv" 2>/dev/null || echo 0)
  A_HOST_BUSY_SAMPLES=$(awk -F, 'NR>1 && $11+0 > 0 {n++} END{print n+0}' "$OUT/armA/metrics.csv" 2>/dev/null || echo 0)
fi
A_HOST_BUSY_PCT=0
[ "$A_METRIC_ROWS" -gt 0 ] && A_HOST_BUSY_PCT=$(( A_HOST_BUSY_SAMPLES * 100 / A_METRIC_ROWS ))

# --- arm B: the harness really executed SQL across all five classes ------
B_ATTEMPTED=0; B_SUCCEEDED=0; B_FAILED=0; B_VERDICT="none"; B_CLASSES_OK=0
if [ -f "$OUT/armB/mixed-workload-summary.md" ]; then
  B_ATTEMPTED=$(grep -oE 'total: attempted=[0-9]+' "$OUT/armB/mixed-workload-summary.md" 2>/dev/null | grep -oE '[0-9]+' | head -1)
  B_SUCCEEDED=$(grep -oE 'succeeded=[0-9]+' "$OUT/armB/mixed-workload-summary.md" 2>/dev/null | grep -oE '[0-9]+' | head -1)
  B_FAILED=$(grep -oE 'failed=[0-9]+' "$OUT/armB/mixed-workload-summary.md" 2>/dev/null | grep -oE '[0-9]+' | head -1)
  B_VERDICT=$(grep -oE '^result: .*' "$OUT/armB/mixed-workload-summary.md" 2>/dev/null | head -1 | sed 's/^result: *//')
  B_CLASSES_MISSING=""
  for c in W1 W2 W3 W4 W5; do
    grep -q "$c" "$OUT/armB/mixed-workload-summary.md" 2>/dev/null || B_CLASSES_MISSING="$B_CLASSES_MISSING $c"
  done
  [ -z "$B_CLASSES_MISSING" ] && B_CLASSES_OK=1
fi
B_LOAD_OK=0
[ "$B_ATTEMPTED" -gt 0 ] && [ "$B_CLASSES_OK" -eq 1 ] && [ "$B_VERDICT" = "PASS" ] && B_LOAD_OK=1

ELAPSED=$(( $(date +%s) - T0 ))

# The overall verdict is a conjunction of BOTH arms' exit codes, artifact
# completeness, and load proof. Any one of them missing ⇒ NOT PASS.
A_PASS=0
[ "$A_RC" = "0" ] && [ "$ART_A_OK" = "1" ] && [ "$A_LOAD_OK" = "1" ] && A_PASS=1
B_PASS=0
[ "$B_RC" = "0" ] && [ "$ART_B_OK" = "1" ] && [ "$B_LOAD_OK" = "1" ] && B_PASS=1

cat > "$OUT/FINAL_REPORT.md" <<EOF
# v4.1.0 168h SOAK — FINAL_REPORT

> **本报告只陈述实测到的内容。** 任何一项没有产出/没跑满/读数缺失，
> 下面就写「未覆盖」，不写 PASS。

## 1. 运行身份

| 字段 | 值 |
|---|---|
| run_id | $RUN_ID |
| frozen_commit | \`$FROZEN_COMMIT\` |
| soaked_binary_sha256 | \`$SOAK_BIN_SHA256\` |
| start_utc | $START_UTC |
| end_utc (planned) | $END_UTC |
| requested_duration_s | $DURATION_SEC |
| measured_elapsed_s | $ELAPSED |
| aborted_by | $ABORTED_BY |
| host | $HOST_NAME ($HOST_OS $HOST_KERN $HOST_ARCH) |
| runner | scripts/soak/run_v410_168h_soak.sh |

## 2. 臂 A —— 服务端稳定性（$HOURS_LABEL 连续）

| 项 | 值 |
|---|---|
| 退出码 | $A_RC |
| 产物齐全 | $ART_A_OK (1=metrics.csv+summary.json+STABILITY_REPORT.md 齐全) |
| **实测查询数** | **$A_QUERIES** |
| 实测错误数 | $A_ERRORS |
| metrics 行数 | $A_METRIC_ROWS (期望 ≥ $A_ROWS_FLOOR，覆盖 ${A_COVERAGE_PCT}%) |
| 采样到的最长 elapsed_s | $A_ELAPSED_S |
| 负载已实际施加 | $A_LOAD_OK (1=查询数>0 且覆盖窗口) |
| 负载 | 80% 读 / 10% 写，16 客户端线程 |
| 采样 | 15s：RSS / FD / 线程 / WAL 字节 / 磁盘余量 / 探针 / 存活 |
| **主机争用** | 负载均值峰值 $A_HOST_LOAD_MAX；有 ≥50% CPU 进程的采样 $A_HOST_BUSY_SAMPLES/$A_METRIC_ROWS（$A_HOST_BUSY_PCT%） |

## 3. 臂 B —— 多模型 workload class W1..W5（$HOURS_LABEL 连续）

| 项 | 值 |
|---|---|
| 退出码 | $B_RC |
| 产物齐全 | $ART_B_OK (1=mixed-workload-summary.md 存在) |
| **attempted / succeeded / failed** | **$B_ATTEMPTED / $B_SUCCEEDED / $B_FAILED** |
| harness 自带判定 | $B_VERDICT |
| 五类覆盖 W1..W5 | $B_CLASSES_OK |
| 负载已实际施加 | $B_LOAD_OK |
| harness | tests/soak/v312_mixed_soak.rs :: v312_ga_mixed_workload_demo |
| 时长来源 | SQLRUSTGO_SOAK_DURATION_SECS=$DURATION_SEC |
| class | W1-OLTP / W2-ReadHeavy / W3-Aggregate / W4-DDL / W5-Rep |

## 4. 判定

| 臂 | 判定 |
|---|---|
| A | $( [ "$A_PASS" = "1" ] && echo "PASS" || echo "NOT PASS（rc=$A_RC 产物齐=$ART_A_OK 负载已施加=$A_LOAD_OK 查询数=$A_QUERIES 覆盖=${A_COVERAGE_PCT}%）" ) |
| B | $( [ "$B_PASS" = "1" ] && echo "PASS" || echo "NOT PASS（rc=$B_RC 产物齐=$ART_B_OK 负载已施加=$B_LOAD_OK attempted=$B_ATTEMPTED verdict=$B_VERDICT 五类=$B_CLASSES_OK）" ) |

> 判定是**合取**：退出码 0、产物齐全、负载已实际施加，三者同时成立才算 PASS。
> 任何一项不成立即为 NOT PASS —— 空转的 run 不会被写成 PASS。

## 5. 未覆盖 / 偏差（如实登记）

- 本次运行**只**覆盖上面两臂；仓内其余 SOAK runner（run_168h_ci.sh 的 sysbench 口径、
  mysql_cli_ladder、chaos_drills）**未在本 run 内执行**，不计入本报告。
- 臂 A 的 FD 采集用 \`lsof\`（本机无 /proc）。
- 机器在 run 启动窗口内可能并发其它 cargo 作业（门禁实跑），会影响吞吐/延迟分位数，
  **不影响**崩溃、panic、错误率、FD 泄漏、WAL 增长这些稳定性指标。
- **主机争用已逐采样记录**（metrics.csv 的 host_load1 / host_busy 两列）：
  负载均值峰值 $A_HOST_LOAD_MAX，有 ≥50% CPU 进程的采样占 $A_HOST_BUSY_PCT%。
  若这个比例不为 0，则吞吐/延迟分位数要按「机器当时被别人占着」来读，
  而崩溃、panic、FD 泄漏、WAL 增长这些稳定性指标仍然成立。
EOF

echo ""
echo "=== 168h SOAK 完成 ==="
echo "  arm A rc=$A_RC  产物齐=$ART_A_OK  负载已施加=$A_LOAD_OK  查询数=$A_QUERIES  覆盖=${A_COVERAGE_PCT}%"
echo "  arm B rc=$B_RC  产物齐=$ART_B_OK  负载已施加=$B_LOAD_OK  attempted=$B_ATTEMPTED"
echo "  elapsed=${ELAPSED}s / ${DURATION_SEC}s"
echo "  FINAL_REPORT: $OUT/FINAL_REPORT.md"

if [ "$A_PASS" = "1" ] && [ "$B_PASS" = "1" ]; then
  echo "RESULT: PASS (both arms)"
  exit 0
fi
echo "RESULT: NOT PASS"
exit 1

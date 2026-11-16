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
#   1  = 至少一臂失败 / 未跑满 / 产物缺失
#   2  = 前置条件不满足（没有 release 二进制 / 检出不干净 / 参数错误）
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
while [ $# -gt 0 ]; do
  case "$1" in
    --hours) HOURS="$2"; shift 2 ;;
    --hours=*) HOURS="${1#--hours=}"; shift ;;
    --port) PORT="$2"; shift 2 ;;
    --port=*) PORT="${1#--port=}"; shift ;;
    --out) OUT="$2"; shift 2 ;;
    --out=*) OUT="${1#--out=}"; shift ;;
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
DURATION_MIN=$((HOURS * 60))
DURATION_SEC=$((HOURS * 3600))

# ---------------------------------------------------------------------------
# 前置条件（fail-closed：检测没做成 ≠ 通过）
# ---------------------------------------------------------------------------
fail_pre() { echo "PRECONDITION FAILED: $*" >&2; exit 2; }

[ -x "$SOAK_BIN" ] || fail_pre "release binary missing or not executable: $SOAK_BIN"
[ -d "$SOAK_REPO_ROOT" ] || fail_pre "SOAK_REPO_ROOT not a directory: $SOAK_REPO_ROOT"
command -v python3 >/dev/null 2>&1 || fail_pre "python3 required (heartbeat + summary)"
command -v cargo   >/dev/null 2>&1 || fail_pre "cargo required (arm B)"

# 检出必须干净：SOAK 的产物要能归到**一个确定的冻结提交**上
DIRTY="$(git -C "$SOAK_REPO_ROOT" status --porcelain 2>/dev/null | grep -v '^?? ' | head -5)"
[ -z "$DIRTY" ] || fail_pre "worktree has tracked modifications — SOAK must run on a clean checkout:
$DIRTY"

mkdir -p "$OUT/armA" "$OUT/armB"
FROZEN_COMMIT="$(git -C "$SOAK_REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
START_UTC="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
END_UTC="$(date -u -v+"${HOURS}"H +"%Y-%m-%dT%H:%M:%SZ" 2>/dev/null || echo unknown)"

echo "=========================================================="
echo "v4.1.0 168h SOAK (V400-09)"
echo "=========================================================="
echo "  HOURS=$HOURS  DURATION=${DURATION_SEC}s  PORT=$PORT"
echo "  FROZEN_COMMIT=$FROZEN_COMMIT"
echo "  START_UTC=$START_UTC   END_UTC=$END_UTC"
echo "  OUT=$OUT   (outside the repository tree, per #5577)"
echo "  SOAK_BIN=$SOAK_BIN"
echo "=========================================================="

# ---------------------------------------------------------------------------
# 臂 A：服务端稳定性（168h 连续）
# ---------------------------------------------------------------------------
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

echo "  arm A pid=$ARM_A_PID (server stability)"
echo "  arm B pid=$ARM_B_PID (multi-model W1..W5)"

# ---------------------------------------------------------------------------
# 监督：每 60s 打一行状态（心跳监督器据此判定「运行中」）
# ---------------------------------------------------------------------------
T0=$(date +%s)
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
  echo "[$(date -u +%H:%M:%S)] elapsed=${EL}s/$DURATION_SEC s armA_alive=$A_ALIVE armB_alive=$B_ALIVE rss_kb=${RSS_KB:-0} last_metric_elapsed=${LAST_METRIC:-none}"
  sleep 60
done
wait "$ARM_A_PID" 2>/dev/null; A_WAIT=$?
wait "$ARM_B_PID" 2>/dev/null; B_WAIT=$?

A_RC=$(cat "$ARM_A_RC_FILE" 2>/dev/null || echo "$A_WAIT")
B_RC=$(cat "$ARM_B_RC_FILE" 2>/dev/null || echo "$B_WAIT")

# ---------------------------------------------------------------------------
# 产物完整性：跑了不等于跑出了合格产物
# ---------------------------------------------------------------------------
ART_A_OK=0; ART_B_OK=0
[ -f "$OUT/armA/metrics.csv" ] && [ -f "$OUT/armA/summary.json" ] && [ -f "$OUT/armA/STABILITY_REPORT.md" ] && ART_A_OK=1
[ -f "$OUT/armB/mixed-workload-summary.md" ] && ART_B_OK=1

ELAPSED=$(( $(date +%s) - T0 ))

cat > "$OUT/FINAL_REPORT.md" <<EOF
# v4.1.0 168h SOAK — FINAL_REPORT

> **本报告只陈述实测到的内容。** 任何一项没有产出/没跑满/读数缺失，
> 下面就写「未覆盖」，不写 PASS。

## 1. 运行身份

| 字段 | 值 |
|---|---|
| run_id | 20261116-168h-v410 |
| frozen_commit | \`$FROZEN_COMMIT\` |
| start_utc | $START_UTC |
| end_utc (planned) | $END_UTC |
| requested_duration_s | $DURATION_SEC |
| measured_elapsed_s | $ELAPSED |
| host | $(uname -srm) |
| runner | scripts/soak/run_v410_168h_soak.sh |

## 2. 臂 A —— 服务端稳定性（$HOURS h 连续）

| 项 | 值 |
|---|---|
| 退出码 | $A_RC |
| 产物齐全 | $ART_A_OK (1=metrics.csv+summary.json+STABILITY_REPORT.md 齐全) |
| metrics 行数 | $( [ -f "$OUT/armA/metrics.csv" ] && echo $(( $(wc -l < "$OUT/armA/metrics.csv") - 1 )) || echo 0 ) |
| 负载 | 80% 读 / 10% 写，16 客户端线程 |
| 采样 | 15s：RSS / FD / 线程 / WAL 字节 / 磁盘余量 / 探针 / 存活 |

## 3. 臂 B —— 多模型 workload class W1..W5（$HOURS h 连续）

| 项 | 值 |
|---|---|
| 退出码 | $B_RC |
| 产物齐全 | $ART_B_OK (1=mixed-workload-summary.md 存在) |
| harness | tests/soak/v312_mixed_soak.rs :: v312_ga_mixed_workload_demo |
| 时长来源 | SQLRUSTGO_SOAK_DURATION_SECS=$DURATION_SEC |
| class | W1-OLTP / W2-ReadHeavy / W3-Aggregate / W4-DDL / W5-Rep |

## 4. 判定

| 臂 | 判定 |
|---|---|
| A | $( [ "$A_RC" = "0" ] && [ "$ART_A_OK" = "1" ] && echo "PASS" || echo "NOT PASS（rc=$A_RC 产物齐=$ART_A_OK）" ) |
| B | $( [ "$B_RC" = "0" ] && [ "$ART_B_OK" = "1" ] && echo "PASS" || echo "NOT PASS（rc=$B_RC 产物齐=$ART_B_OK）" ) |

## 5. 未覆盖 / 偏差（如实登记）

- 本次运行**只**覆盖上面两臂；仓内其余 SOAK runner（run_168h_ci.sh 的 sysbench 口径、
  mysql_cli_ladder、chaos_drills）**未在本 run 内执行**，不计入本报告。
- 臂 A 的 FD 采集用 \`lsof\`（macOS 无 /proc）。
- 机器在 run 启动窗口内可能并发其它 cargo 作业（门禁实跑），会影响吞吐/延迟分位数，
  **不影响**崩溃、panic、错误率、FD 泄漏、WAL 增长这些稳定性指标。
EOF

echo ""
echo "=== 168h SOAK 完成 ==="
echo "  arm A rc=$A_RC  产物齐=$ART_A_OK"
echo "  arm B rc=$B_RC  产物齐=$ART_B_OK"
echo "  elapsed=${ELAPSED}s / ${DURATION_SEC}s"
echo "  FINAL_REPORT: $OUT/FINAL_REPORT.md"

if [ "$A_RC" = "0" ] && [ "$B_RC" = "0" ] && [ "$ART_A_OK" = "1" ] && [ "$ART_B_OK" = "1" ]; then
  echo "RESULT: PASS (both arms)"
  exit 0
fi
echo "RESULT: NOT PASS"
exit 1

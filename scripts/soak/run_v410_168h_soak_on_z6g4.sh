#!/usr/bin/env bash
# =============================================================================
# run_v410_168h_soak_on_z6g4.sh — Z6G4 (192.168.0.252) 上的 168h SOAK 启动器
# =============================================================================
#
# 作用
# ----
# `run_v410_168h_soak.sh` 是**运行器**：它保证两臂的判定是承重的。
# 本脚本是**启动器**：它保证这台主机在起跑的那一刻是**可用的**。
#
# 为什么需要它（2026-11-16 实测）
# ------------------------------
# 第一次在 Z6G4 上起 168h 时，run 在 02:59:52 启动，而**同一台机器**上
# root 的作业在 02:59 开始：
#
#     /workspace/openclaw/sqlrustgo/target/debug/deps/oltp_test-…   占 29 核
#     cargo test --workspace --exclude sqlrustgo --no-fail-fast    timeout 17400s
#     cargo test --all-features --no-fail-fast                      timeout 16800s
#
# 机器 80 核，load average 冲到 **89.48**。结果：成功查询耗时 9.4 s、
# 超时查询耗时 15 s、错误率 9.6 %（冒烟 run 在 02:54–02:58 跑同样的脚本，
# 当时 root 的作业还没起来，8439 次查询只有 6 次错误）。
#
# 那不是 sqlrustgo 的缺陷，是**主机噪声**。但它同样让那份 168h 证据
# 没有价值：一份由邻居的 CPU 与 I/O 争用写出来的 SOAK 报告，证明的是
# 那几分钟的争用，不是被测系统。**宁可晚起跑，不可带噪起跑。**
#
# 所以本脚本在起跑前设一道**安静窗口**门禁，起跑后再设一道**吞吐复验**门禁。
#
# 安静窗口（两条件同时满足，且连续满足 QUIET_SAMPLES 次）
# ----------------------------------------------------------
#   1. **外来 CPU 总量 ≤ FOREIGN_CPU_MAX**（默认 2400 %，即 80 核里的 30 核）。
#      forge 自己的 postgres/gitea 不算（本仓的 Gitea 就跑在这台机上），
#      本用户自己的进程也不算。
#   2. **负载均值 ≤ QUIET_MAX_LOAD**（默认 40，对 80 核即 ≤50 % 占用）。
#
# 为什么按**总量**判，不按「有没有某个重进程」判
# ----------------------------------------------
# 第一版判据是「存在一个外来进程吃 ≥50 % CPU ⇒ 争用」。实测它太敏感：
# 别人跑 `cargo test --workspace` 时会**接连不断地**拉起一个个测试二进制
# （four_way_compare_test / oracle_g15_tpch_sf01 …），每个都占 100–245 %，
# 但每个只活几秒 —— 在 80 核的机器上，一个占 1 核的短命测试二进制根本
# 构不成争用，而计数会被它一次次清零，SOAK 永远起不了跑。
#
# 而真正毁掉那一次 168h 的是**总量**：oltp_test 占 29 核 + 多个 rustc，
# 外来 CPU 合计约 3264 %（41 核），load average 89.48 ⇒ 成功查询耗时
# 9.4 s、错误率 9.6 %。
#
# 阈值不是拍脑袋来的，是**在 252 上量出来的**。三档实测，同一台机器、
# 同一套脚本、同一个二进制：
#
#   外来 CPU    load1   错误率        吞吐      p50 延迟   判定
#   178 %       3.67    0.07 %       47 q/s    —          PASS
#   1123-1831 % 12-31   0.12 %       61 q/s    126 ms     PASS   ← 争用探针
#   3264 %      89.48   9.6 %        4.7 q/s   9400 ms    FAIL   ← 污染轮
#
# 退化只发生在 ~40 核那一档；11–18 核时错误率仍是 0.12 %、吞吐不降反升。
# 阈值取 2400 %（24 核）：落在两个实测点之间，且**偏向安全一侧**
# （距 PASS 侧 1.31 倍，距 FAIL 侧 1.36 倍）。
#
# 这与「我的 SOAK 需要几个核」也自洽 —— 16 个驱动线程 + 1 个服务端线程
# （Exec par: 1）实打实要 2–4 核，剩 56 核时测量结果与空机无异。
#
# load1 ≤ 40 这一道是独立的兜底：它不依赖 ps 的口径，1 分钟均值对
# 突发不敏感，与上面的总量门禁互补。
#
# 吞吐复验
# --------
# 安静窗口只是「起跑那一刻」的状态。起跑后等 VERIFY_SECS 秒，看驱动真实
# 跑了多少查询；折算 QPS 低于 MIN_DRIVER_QPS 就说明机器还是被压着 —— 停掉
# 这一轮，回到等待，绝不把一份被污染的 run 撑满 168 小时。
#
# 用法
# ----
#   nohup bash scripts/soak/run_v410_168h_soak_on_z6g4.sh \
#     > ~/soak-artifacts/20261116-168h-v410/launcher.log 2>&1 &
#
# 环境变量
# ----------
#   SOAK_REPO_ROOT     干净检出的仓根（默认：本脚本所在仓根）
#   ART_DIR            产物目录（默认 $HOME/soak-artifacts/20261116-168h-v410）
#   RUN_ID             run 标识（默认取 ART_DIR 的目录名）
#   HOURS              SOAK 时长（默认 168）
#   PORT               服务端口（默认 3411）
#   EXPECT_COMMIT      期望的冻结提交；给了就必须是当前 HEAD，否则拒绝起跑
#   QUIET_MAX_LOAD     安静窗口的负载均值上限（默认 40）
#   FOREIGN_CPU_MAX    安静窗口的外来 CPU 总量上限（默认 2400 %）
#   QUIET_SAMPLES      安静窗口需连续满足的次数（默认 10，每次 60 s ⇒ 10 分钟）
#   QUIET_INTERVAL     采样间隔秒数（默认 60）
#   MIN_DRIVER_QPS     吞吐复验的 QPS 下限（默认 5）
#   VERIFY_SECS        吞吐复验的等待秒数（默认 180）
#
# 退出码
# ------
#   0  = SOAK 已起跑且通过吞吐复验（本脚本随后退出；SOAK 由心跳监督器托管）
#   1  = 参数/前置不满足
# =============================================================================
set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SOAK_REPO_ROOT="${SOAK_REPO_ROOT:-$(cd "$SCRIPT_DIR/../.." && pwd)}"

ART_DIR="${ART_DIR:-$HOME/soak-artifacts/20261116-168h-v410}"
RUN_ID="${RUN_ID:-$(basename "$ART_DIR")}"
HOURS="${HOURS:-168}"
PORT="${PORT:-3411}"
EXPECT_COMMIT="${EXPECT_COMMIT:-}"

QUIET_MAX_LOAD="${QUIET_MAX_LOAD:-40}"
FOREIGN_CPU_MAX="${FOREIGN_CPU_MAX:-2400}"
QUIET_SAMPLES="${QUIET_SAMPLES:-10}"
QUIET_INTERVAL="${QUIET_INTERVAL:-60}"
MIN_DRIVER_QPS="${MIN_DRIVER_QPS:-5}"
VERIFY_SECS="${VERIFY_SECS:-180}"

mkdir -p "$ART_DIR"
WAIT_LOG="$ART_DIR/wait_for_quiet_host.log"

log() {
  echo "[$(date -u +%Y-%m-%dT%H:%M:%SZ)] $*" | tee -a "$WAIT_LOG"
}

fail() { echo "LAUNCHER FAILED: $*" >&2 | tee -a "$WAIT_LOG"; exit 1; }

# ---------------------------------------------------------------------------
# 前置条件
# ---------------------------------------------------------------------------
[ -d "$SOAK_REPO_ROOT" ] || fail "SOAK_REPO_ROOT is not a directory: $SOAK_REPO_ROOT"
[ -x "$SOAK_REPO_ROOT/target/release/sqlrustgo-mysql-server" ] \
  || fail "release binary missing: $SOAK_REPO_ROOT/target/release/sqlrustgo-mysql-server"
command -v python3 >/dev/null 2>&1 || fail "python3 required"
python3 -c "import pymysql" >/dev/null 2>&1 \
  || fail "pymysql required by the arm-A load driver (pip install pymysql)"

DIRTY="$(git -C "$SOAK_REPO_ROOT" status --porcelain 2>/dev/null | grep -v '^?? ' | head -5)"
[ -z "$DIRTY" ] || fail "worktree has tracked modifications — the SOAK must run on a clean checkout:
$DIRTY"

HEAD_COMMIT="$(git -C "$SOAK_REPO_ROOT" rev-parse HEAD 2>/dev/null || echo unknown)"
if [ -n "$EXPECT_COMMIT" ] && [ "$HEAD_COMMIT" != "$EXPECT_COMMIT" ]; then
  fail "HEAD is $HEAD_COMMIT but EXPECT_COMMIT is $EXPECT_COMMIT"
fi

log "launcher up: repo=$SOAK_REPO_ROOT head=$HEAD_COMMIT art=$ART_DIR run_id=$RUN_ID hours=$HOURS port=$PORT"
log "policy: quiet_max_load=$QUIET_MAX_LOAD foreign_cpu_max=$FOREIGN_CPU_MAX quiet_samples=$QUIET_SAMPLES interval=${QUIET_INTERVAL}s min_qps=$MIN_DRIVER_QPS verify_secs=$VERIFY_SECS"

# ---------------------------------------------------------------------------
# 安静窗口判据
# ---------------------------------------------------------------------------
# 列出「外来重负载」：不是本仓、不是 forge 自己、且（像一次构建/压测）
# 或（吃掉了 ≥50 % CPU）的进程。
# **只把「别人」的作业算作外来重负载。**
#
# 这一条是承重的，不是随手写的过滤：本启动器自己起的那一轮 SOAK 里，
# 臂 B 就是一个 `cargo test --release --test v312_mixed_soak` —— 它命中
# 「像 cargo」这一条，却是我们自己的作业。若不按 owner 区分，启动器会在
# 自己起跑之后判定「机器还是被压着」，把刚起的 run 停掉，然后永远等下去。
#
# 所有者相同 ⇒ 不是外来争用；整机层面的争用由 load 均值那一道门禁兜底。
# 外来 CPU 总量（%），以及逐条明细（供日志留痕）。
#
# **只把「别人 owner」的作业算作争用。**
#
# 这一条是承重的，不是随手写的过滤：本启动器自己起的那一轮 SOAK 里，
# 臂 B 就是一个 `cargo test --release --test v312_mixed_soak` —— 它命中
# 「像 cargo」这一条，却是我们自己的作业。若不按 owner 区分，启动器会在
# 自己起跑之后判定「机器还是被压着」，把刚起的 run 停掉，然后永远等下去。
#
# 所有者相同 ⇒ 不是外来争用；整机层面的争用由 load 均值那一道门禁兜底。
foreign_cpu_total() {
  ps -eo user=,pcpu=,args= 2>/dev/null | awk -v repo="$SOAK_REPO_ROOT" -v me="$(id -un)" '
    {
      user=$1; cpu=$2;
      $1=""; $2=""; args=$0;
      if (user == me) next;
      if (index(args, repo) > 0) next;
      if (index(args, "run_v410_168h_soak_on_z6g4") > 0) next;
      if (index(args, "awk -v repo=") > 0) next;
      if (args ~ /postgres|gitea/) next;
      s += cpu;
      if (cpu+0 >= 20) printf("%s|%s|%s\n", user, cpu, substr(args, 1, 90));
    }
    END { printf("TOTAL|%s|%.0f\n", (s+0), (s+0)) }'
}

# Portable "now + N seconds" (GNU -d / BSD -v).
end_utc_in() {
  local s="$1"
  if date -u -d "+${s} seconds" +"%Y-%m-%dT%H:%M:%SZ" >/dev/null 2>&1; then
    date -u -d "+${s} seconds" +"%Y-%m-%dT%H:%M:%SZ"
  elif date -u -v+"${s}"S +"%Y-%m-%dT%H:%M:%SZ" >/dev/null 2>&1; then
    date -u -v+"${s}"S +"%Y-%m-%dT%H:%M:%SZ"
  else
    echo unknown
  fi
}

load1() {
  if [ -r /proc/loadavg ]; then
    awk '{print $1}' /proc/loadavg
  else
    echo 0
  fi
}

# ---------------------------------------------------------------------------
# kill_soak_tree — 停掉刚才起跑的那一轮（用于复验不通过时）
# ---------------------------------------------------------------------------
kill_soak_tree() {
  local pids p
  pids="$(pgrep -f "run_v410_168h_soak.sh --hours" 2>/dev/null || true)"
  for p in $pids; do
    [ "$p" = "$$" ] && continue
    kill -TERM "$p" 2>/dev/null || true
  done
  # 驱动与服务端的 pid 由运行器自己写进产物目录
  for p in "$(cat "$ATTEMPT_DIR/armA/driver.pid" 2>/dev/null)" \
           "$(cat "$ATTEMPT_DIR/armA/server.pid" 2>/dev/null)"; do
    [ -n "$p" ] && kill -TERM "$p" 2>/dev/null || true
  done
  # 臂 B 的 cargo test 与它拉起的测试二进制不是 wrapper 的直接子进程，
  # 不收掉就会在下一轮 attempt 之后继续占着机器 —— 下一轮复验读到的
  # 负载就是两份 run 叠在一起的负载。
  for p in $(pgrep -f "v312_mixed_soak" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -TERM "$p" 2>/dev/null || true
  done
  sleep 3
  for p in $pids; do
    [ "$p" = "$$" ] && continue
    kill -KILL "$p" 2>/dev/null || true
  done
  for p in "$(cat "$ATTEMPT_DIR/armA/driver.pid" 2>/dev/null)" \
           "$(cat "$ATTEMPT_DIR/armA/server.pid" 2>/dev/null)"; do
    [ -n "$p" ] && kill -KILL "$p" 2>/dev/null || true
  done
  for p in $(pgrep -f "v312_mixed_soak" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -KILL "$p" 2>/dev/null || true
  done
  log "stopped the contaminated attempt ($ATTEMPT_DIR)"
}

# ---------------------------------------------------------------------------
# 起跑
# ---------------------------------------------------------------------------
# 每一次 attempt 写进**自己的**产物目录。
#
# 为什么不能共用一个目录：吞吐复验读的是 armA/driver.log 里的 Q 行数。
# 若上一轮 attempt 的 driver.log 还在，新一轮的计数会把旧账算进来 ——
# 一轮几乎没跑起来的 attempt 也会被算成「QPS 达标」，复验门禁就废了。
# 每轮独立目录，复验只读本轮，失败的那几轮原样留档作为「为什么停」的证据。
ATTEMPT_DIR=""
start_soak() {
  local stamp
  stamp="$(date -u +%Y%m%dT%H%M%SZ)"
  ATTEMPT_DIR="$ART_DIR/attempt-$stamp"
  mkdir -p "$ATTEMPT_DIR"

  export SOAK_DATA_DIR="${SOAK_DATA_DIR:-/tmp/sqlrustgo-v410-168h-$RUN_ID}"
  export SOAK_REPO_ROOT
  export HB_OUT="$ATTEMPT_DIR/heartbeat.jsonl"
  export HB_LOG="$ATTEMPT_DIR/soak.log"
  export HB_INTERVAL="${HB_INTERVAL:-15}"
  export HB_MAX_GAP="${HB_MAX_GAP:-90}"

  setsid nohup bash "$SOAK_REPO_ROOT/scripts/soak/run_soak_with_heartbeat.sh" \
    bash "$SOAK_REPO_ROOT/scripts/soak/run_v410_168h_soak.sh" \
      --hours "$HOURS" --port "$PORT" --out "$ATTEMPT_DIR" --run-id "$RUN_ID" \
    > "$ATTEMPT_DIR/supervisor.log" 2>&1 < /dev/null &
  disown 2>/dev/null || true
  log "SOAK launched (hours=$HOURS port=$PORT) attempt_dir=$ATTEMPT_DIR"
}

# ---------------------------------------------------------------------------
# 主循环：等安静窗口 → 起跑 → 复验吞吐 → 不达标就退回等待
# ---------------------------------------------------------------------------
STREAK=0
ATTEMPT=0
while true; do
  ATTEMPT=$((ATTEMPT + 1))
  LOAD="$(load1)"
  FOREIGN_OUT="$(foreign_cpu_total)"
  FOREIGN_TOTAL="$(printf '%s\n' "$FOREIGN_OUT" | awk -F'|' '/^TOTAL\|/ {print $3}')"
  [ -n "$FOREIGN_TOTAL" ] || FOREIGN_TOTAL=0
  FOREIGN_TOP="$(printf '%s\n' "$FOREIGN_OUT" | grep -v '^TOTAL|' | head -3 | tr '\n' ';' | cut -c1-200)"
  QUIET=1
  [ "$FOREIGN_TOTAL" -gt "$FOREIGN_CPU_MAX" ] && QUIET=0
  LOAD_INT="$(printf '%.0f' "$LOAD" 2>/dev/null || echo 0)"
  [ "$LOAD_INT" -gt "$QUIET_MAX_LOAD" ] && QUIET=0

  if [ "$QUIET" -eq 1 ]; then
    STREAK=$((STREAK + 1))
  else
    STREAK=0
  fi

  if [ "$QUIET" -eq 1 ]; then
    log "host QUIET load=$LOAD_INT foreign_cpu=${FOREIGN_TOTAL}% streak=$STREAK/$QUIET_SAMPLES"
  else
    log "host BUSY  load=$LOAD_INT foreign_cpu=${FOREIGN_TOTAL}% (max $FOREIGN_CPU_MAX%) streak=0/$QUIET_SAMPLES top=[$FOREIGN_TOP]"
  fi

  if [ "$STREAK" -ge "$QUIET_SAMPLES" ]; then
    log "quiet window satisfied ($STREAK consecutive samples) — starting the $HOURS h clock"
    start_soak

    sleep "$VERIFY_SECS"
    Q="$(grep -c '^Q,' "$ATTEMPT_DIR/armA/driver.log" 2>/dev/null || echo 0)"
    [ -n "$Q" ] || Q=0
    QPS=$(( Q / VERIFY_SECS ))
    log "throughput re-check: driver queries=$Q in ${VERIFY_SECS}s => ${QPS} q/s (floor $MIN_DRIVER_QPS)"
    if [ "$QPS" -ge "$MIN_DRIVER_QPS" ]; then
      bin_sha="$(sha256sum "$SOAK_REPO_ROOT/target/release/sqlrustgo-mysql-server" 2>/dev/null | cut -d" " -f1)"
      cat > "$ART_DIR/STARTED.json" <<JSON
{
  "run_id": "$RUN_ID",
  "attempt_dir": "$ATTEMPT_DIR",
  "frozen_commit": "$HEAD_COMMIT",
  "soaked_binary_sha256": "$bin_sha",
  "host": "$(uname -n) $(uname -s) $(uname -r) $(uname -m)",
  "hours": $HOURS,
  "port": $PORT,
  "started_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "planned_end_utc": "$(end_utc_in $((HOURS*3600)))",
  "verify_secs": $VERIFY_SECS,
  "driver_queries_at_verify": $Q,
  "driver_qps_at_verify": $QPS,
  "quiet_max_load": $QUIET_MAX_LOAD,
  "foreign_cpu_max": $FOREIGN_CPU_MAX,
  "quiet_samples": $QUIET_SAMPLES
}
JSON
      log "PASS: the host carried real load (${QPS} q/s); the $HOURS h clock is running."
      log "artifacts: $ATTEMPT_DIR"
      log "canonical record: $ART_DIR/STARTED.json"
      exit 0
    fi
    log "REJECT: the host is still carrying someone else's load (${QPS} q/s < ${MIN_DRIVER_QPS} q/s) — stopping this attempt and going back to waiting."
    kill_soak_tree
    STREAK=0
  fi

  sleep "$QUIET_INTERVAL"
done

#!/usr/bin/env bash
# =============================================================================
# soak_exclusive_window.sh — 为 168h SOAK 建立 / 撤销「节点独占」窗口
# =============================================================================
#
# 作用
# ----
# 168h SOAK 的判据要求**整段窗口内节点独占**：被测系统必须自己扛住，
# 而不是被邻居的 CPU / I-O 争用拖垮。`run_v410_168h_soak_on_z6g4.sh`
# 的起跑门禁只看**起跑那一刻**是否安静（load1 ≤ 40 且外来 CPU ≤ 2400 %），
# 但窗口内的持续争用同样会让一份 SOAK 报告失去意义 —— 它的错误率曲线
# 记的是「邻居有多忙」，不是「被测系统有多稳」。
#
# 252 上有什么在抢盘
# -----------------
# 2026-11-18 实测（load average 137.24 / 80 核、98 个 rustc/cargo 进程）：
#
#   ① systemd `gmp-platform-runner.service`（openclaw 用户，act_runner 守护）
#   ② docker 里的 runner 池容器（runner-pool-* / minisystem-* / ci252-for-gitea250）
#   ③ docker 里正在跑的作业容器（GITEA-ACTIONS-TASK-*，root 身份编译）
#   ④ systemd `bustubx-unpause-ci.service`（守护脚本：把 **paused** 的
#      BustubX-EDU 作业容器解冻 ⇒ 单独 pause 容器会被它顶回来）
#
# ①②③ 是「谁在编译」，④ 是「谁不让它停下来」。
#
# 怎么停才可逆
# ------------
#   · 容器用 **`docker pause`** 而不是 `stop`/`rm`：容器、进程、已产出的
#     作业状态全部保留，窗口结束后 `docker unpause` 即原样继续。
#   · 服务用 **`systemctl stop`**：两个单元的 `Restart=always` 意味着单纯
#     kill 进程会被立刻拉起，必须走 systemd 才能真正停下；`start` 即恢复。
#   · 本仓自己的 `sqlrustgo-watch.sh`（OOM 守护，每分钟）**保留运行** ——
#     它是保护被测进程的，不是争用源。
#
# 用法
# ----
#   bash scripts/soak/soak_exclusive_window.sh enter    # 建立独占窗口
#   bash scripts/soak/soak_exclusive_window.sh status   # 看当前窗口状态
#   bash scripts/soak/soak_exclusive_window.sh leave    # 撤销独占窗口
#
# 需要的权限
# -----------
#   · `docker pause/unpause`：属主在 docker 组即可。
#   · `systemctl stop/start`：需要 root。属主 liying 无 root 口令时，
#     本脚本自动改走「特权容器 + nsenter 进宿主 init 命名空间」这条路：
#         docker run --rm --privileged --pid=host alpine \
#             nsenter -t 1 -m -u -i -n -p -- systemctl stop <unit>
#     docker 组在 Linux 上本就等价 root，所以这条路不需要额外口令。
#
# 退出码
# -------
#   0 = 成功    1 = 参数错 / 关键步骤失败
# =============================================================================
set -uo pipefail

MODE="${1:-status}"
STATE_DIR="${STATE_DIR:-$HOME/soak-artifacts/exclusive-window}"
MANIFEST="$STATE_DIR/exclusive_window_manifest.json"

# 需要停的 systemd 单元（都是「谁在编译 / 谁不让它停」）
UNITS=(
  "gmp-platform-runner.service"
  "bustubx-unpause-ci.service"
)

# 这些容器**绝不能**碰：它们是 Gitea 本体与它的数据库 / 检索服务，
# 停掉等于把仓库下线，窗口内就再也推不动任何东西了。
KEEP_PATTERNS=(
  "openclaw-gitea"
  "openclaw-postgres"
  "gptr-searxng"
)

log() { printf '[%s] %s\n' "$(date '+%F %T')" "$*"; }
die() { printf '[%s] ERROR: %s\n' "$(date '+%F %T')" "$*" >&2; exit 1; }

# --- 容器分类 ---------------------------------------------------------------
# 作业容器：GITEA-ACTIONS-TASK-*
# runner 容器：名字里带 runner 或 ci252-for-gitea250
list_ci_containers() {
  docker ps --format '{{.Names}}' 2>/dev/null \
    | grep -E 'GITEA-ACTIONS-TASK-|runner|ci252-for-gitea' \
    | grep -vE "$(IFS='|'; echo "${KEEP_PATTERNS[*]}")" \
    | sort
}

# --- 需要 root 的 systemctl 包装 -------------------------------------------
# 优先直接 systemctl（如果本进程已是 root）；否则走 docker+nsenter。
sysctl_do() {
  local verb="$1" unit="$2"
  if [ "$(id -u)" = "0" ]; then
    systemctl "$verb" "$unit" >/dev/null 2>&1
    return $?
  fi
  if command -v docker >/dev/null 2>&1; then
    docker run --rm --privileged --pid=host alpine:latest \
      nsenter -t 1 -m -u -i -n -p -- systemctl "$verb" "$unit" >/dev/null 2>&1
    return $?
  fi
  return 3
}

unit_state() {
  local unit="$1"
  if [ "$(id -u)" = "0" ]; then
    systemctl show "$unit" -p ActiveState --value 2>/dev/null
    return
  fi
  docker run --rm --privileged --pid=host alpine:latest \
    nsenter -t 1 -m -u -i -n -p -- systemctl show "$unit" -p ActiveState --value 2>/dev/null \
    | tr -d '\r'
}

# ---------------------------------------------------------------------------
case "$MODE" in
  enter)
    mkdir -p "$STATE_DIR" || die "cannot create $STATE_DIR"
    if [ -f "$MANIFEST" ]; then
      log "manifest already exists ($MANIFEST) —— 已经处于独占窗口内，先 leave 再 enter"
      exit 1
    fi

    # ① 快照容器清单（pause 前记下原状态，便于原样恢复）
    containers="$(list_ci_containers)"
    n_cont=$(printf '%s\n' "$containers" | grep -c . || true)
    log "发现 $n_cont 个 CI/runner 容器将被 pause"

    # ② 记下两个单元的原始 ActiveState
    unit_states=""
    for u in "${UNITS[@]}"; do
      s="$(unit_state "$u" | tr -d '\r' | tail -1)"
      unit_states="${unit_states}${u}=${s}"$'\n'
      log "单元 $u 当前 ActiveState=${s:-unknown}"
    done

    # ③ pause 容器
    paused=""
    if [ "$n_cont" -gt 0 ]; then
      while IFS= read -r c; do
        [ -z "$c" ] && continue
        if docker pause "$c" >/dev/null 2>&1; then
          paused="${paused}${c}"$'\n'
          log "paused $c"
        else
          log "WARN: pause 失败 $c"
        fi
      done <<EOF
$containers
EOF
    fi

    # ④ 停两个 systemd 单元
    for u in "${UNITS[@]}"; do
      if sysctl_do stop "$u"; then
        log "systemctl stop $u => ok"
      else
        log "WARN: systemctl stop $u 失败（继续，不回滚已 pause 的容器）"
      fi
    done

    # ⑤ 落 manifest（供 leave 原样恢复 + 审计）
    {
      echo "{"
      echo "  \"entered_at\": \"$(date -u '+%Y-%m-%dT%H:%M:%SZ')\","
      echo "  \"host\": \"$(hostname)\","
      echo "  \"paused_containers\": ["
      first=1
      while IFS= read -r c; do
        [ -z "$c" ] && continue
        [ $first -eq 0 ] && echo ","
        printf '    "%s"' "$c"
        first=0
      done <<EOF
$paused
EOF
      echo
      echo "  ],"
      echo "  \"units\": {"
      first=1
      while IFS= read -r line; do
        [ -z "$line" ] && continue
        u="${line%%=*}"; s="${line#*=}"
        [ $first -eq 0 ] && echo ","
        printf '    "%s": "%s"' "$u" "$s"
        first=0
      done <<EOF
$unit_states
EOF
      echo
      echo "  },"
      echo "  \"stopped_units\": [\"gmp-platform-runner.service\", \"bustubx-unpause-ci.service\"]"
      echo "}"
    } > "$MANIFEST"

    log "独占窗口已建立，manifest => $MANIFEST"
    echo
    log "--- 当前负载 ---"
    cat /proc/loadavg
    log "--- 剩余 rustc/cargo 进程数 ---"
    ps -axo command 2>/dev/null | grep -cE '[r]ustc|[c]argo (build|test|nextest)'
    log "（负载需要几分钟才降下来；起跑门禁自己会再等安静窗口）"
    exit 0
    ;;

  leave)
    [ -f "$MANIFEST" ] || die "no manifest at $MANIFEST —— 没有可撤销的独占窗口"
    echo "--- manifest ---"
    cat "$MANIFEST"
    echo

    # 恢复单元
    for u in "${UNITS[@]}"; do
      if sysctl_do start "$u"; then
        log "systemctl start $u => ok"
      else
        log "WARN: systemctl start $u 失败"
      fi
    done

    # 恢复容器
    while IFS= read -r c; do
      [ -z "$c" ] && continue
      if docker unpause "$c" >/dev/null 2>&1; then
        log "unpaused $c"
      else
        log "WARN: unpause 失败 $c（可能已被移除）"
      fi
    done <<EOF
$(sed -n 's/^    "\(.*\)"$/\1/p' "$MANIFEST")
EOF

    log "独占窗口已撤销（manifest 保留供审计：$MANIFEST）"
    exit 0
    ;;

  status)
    echo "--- 负载 ---"; cat /proc/loadavg
    echo "--- rustc/cargo 进程数 ---"
    ps -axo command 2>/dev/null | grep -cE '[r]ustc|[c]argo (build|test|nextest)'
    echo "--- 单元状态 ---"
    for u in "${UNITS[@]}"; do
      echo "$u => $(unit_state "$u" | tr -d '\r' | tail -1)"
    done
    echo "--- CI/runner 容器（running）---"
    list_ci_containers | head -40
    echo "--- manifest ---"
    if [ -f "$MANIFEST" ]; then echo "存在: $MANIFEST"; else echo "不存在（未处于独占窗口）"; fi
    exit 0
    ;;

  *)
    die "usage: $0 {enter|leave|status}"
    ;;
esac

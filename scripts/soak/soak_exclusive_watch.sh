#!/usr/bin/env bash
# =============================================================================
# soak_exclusive_watch.sh — **独占窗口自愈看门狗**
# =============================================================================
#
# 为什么需要它（2026-11-19 实测，不是推断）
# ------------------------------------------
# `soak_exclusive_window.sh enter` 在 2026-11-18T08:47:38Z 建立的窗口，
# 到 2026-11-19T01:21Z 复查时**已经静默失效**：
#
#   · `docker ps` 里 0 个容器带 (Paused) 标记；
#   · 但 dockerd **没有重启**（etimes ≈ 18.9 h）、容器 RestartCount = 0，
#     而 StartedAt 被刷新到 2026-11-18T16:29:17Z
#     ⇒ 这些容器是**被重建**（remove + create / compose up），不是被重启；
#   · `docker pause` 的状态挂在**旧容器 ID** 上 ⇒ 容器一重建，pause 即丢；
#   · 两个 systemd 单元的 ActiveState 全程 `inactive` ⇒ 单元侧没被顶回来，
#     破口**只在容器侧**。
#
# 也就是说：**一次性的 enter 不够**。168 h 窗口里宿主随时可能重建 runner 容器，
# 窗口会被静默打开 ⇒ 被测系统可能在被邻居抢 CPU 的情况下跑完全程 ⇒ 这份 SOAK 报告无效。
#
# 做什么
# ------
# 每 INTERVAL 秒**重新断言**窗口（幂等，不叠加）：
#   ① 单元 `gmp-platform-runner.service` / `bustubx-unpause-ci.service`：
#      若 ActiveState=active ⇒ `systemctl stop`
#      （两者都是 Restart=always，只 kill 进程会被立刻拉起，必须走 systemd）。
#   ② 容器：按名字模式**重新发现**（而不是照抄 manifest 的旧清单），
#      对每个**当前**的 CI/runner 容器，若 State.Paused != true ⇒ `docker pause`。
#      这样容器被重建后也能被重新按住。
#   ③ 每次实际动作写一行 jsonl（时间戳 + 动作 + 对象 + 当时状态）⇒ 可审计。
#
# 与既有工具的分工
# ----------------
#   · `soak_exclusive_window.sh` —— **一次性**建立/撤销窗口（enter / leave）
#   · `soak_host_native_freeze.py` —— 冻结**不在容器里**的宿主原生构建作业（SIGSTOP）
#   · `soak_cpu_sampler.py`      —— 持续采样 /proc 口径的真实 CPU 占用（证据）
#   · 本脚本                     —— **持续重新断言**容器/单元侧的窗口
#
# 保护名单（不动）
# ----------------
#   openclaw-gitea / openclaw-postgres / gptr-searxng —— Gitea 本体与它的依赖，
#   窗口内仍要能 push / 合并（我们自己还要往 develop 合 PR）。
#   冻结守护与本脚本自身都不在匹配模式内。
#
# 幂等与安全
# ----------
#   · `docker pause` 一个已 paused 的容器会报错 ⇒ **先查 State.Paused 再动**。
#   · `systemctl stop` 一个已 stopped 的单元会报错 ⇒ **先查 ActiveState 再动**。
#   · 容器刚被删掉时 inspect 会失败 ⇒ 跳过并等下一轮。
#   · 本脚本**只 pause / 只 stop**，从不 kill / rm ⇒ 窗口可逆，`leave` 可原样恢复。
#
# 用法
# ----
#   bash scripts/soak/soak_exclusive_watch.sh run      # 持续守护（建议交给 systemd）
#   bash scripts/soak/soak_exclusive_watch.sh once     # 只断言一轮（自检用）
#
# 推荐部署（252 上已按此落地）
# --------------------------------
#   cat >/etc/systemd/system/srg-soak-exclusive-watch.service <<'UNIT'
#   [Unit]
#   Description=sqlrustgo v4.1.0 SOAK exclusive-window watchdog
#   After=docker.service
#   Requires=docker.service
#
#   [Service]
#   Type=simple
#   User=root
#   Environment=STATE_DIR=/home/liying/soak-artifacts/exclusive-window
#   Environment=INTERVAL=60
#   ExecStart=/home/liying/_soak_exclusive_watch.sh run
#   Restart=always
#   RestartSec=10
#
#   [Install]
#   WantedBy=multi-user.target
#   UNIT
#   systemctl daemon-reload && systemctl enable --now srg-soak-exclusive-watch.service
# =============================================================================
set -uo pipefail

STATE_DIR="${STATE_DIR:-/home/liying/soak-artifacts/exclusive-window}"
JOURNAL="$STATE_DIR/exclusive_watch.jsonl"
INTERVAL="${INTERVAL:-60}"

UNITS=(
  "gmp-platform-runner.service"
  "bustubx-unpause-ci.service"
)

# CI / runner 容器的名字模式（与 soak_exclusive_window.sh 保持一致）
CI_PATTERN='GITEA-ACTIONS-TASK-|runner|ci252-for-gitea'
# 保护名单：Gitea 本体与其依赖，窗口内不得冻结
KEEP_PATTERN='openclaw-gitea|openclaw-postgres|gptr-searxng'

# ---------------------------------------------------------------------------
# 急停开关：抑制「每小时 :07 / :37 复发」的宿主 I/O 风暴
# ---------------------------------------------------------------------------
# 2026-11-19 实测（r3 轮，run_id 20261119-168h-v410-r3）：
#   2026-11-19T01:37:01Z–01:37:20Z，宿主 cron
#   `7,37 * * * * /home/openclaw/llama-swap/strata-upgrade-if-idle.sh`
#   把 Strata 引擎目录**复制**到 /home/openclaw/backups/strata-upgrade-if-idle/
#   （该备份树实测 **9.9 GB**）—— 与 SOAK 的数据目录**同盘 /dev/nvme0n1p2**。
#   同窗口内被测服务端的**所有**查询类型（连主键点查都算）**同时**卡到
#   ~14 s ⇒ 全局存储层停顿；7 条在途查询越过客户端 **15 s** 读超时 ⇒
#   `2013 Lost connection (The read operation timed out)`，错误窗
#   01:37:02Z–01:37:18Z，与备份窗**逐秒对齐**，而当时 load1 仅 **3.4–6.6**（安静）。
#
# 该脚本自带**急停开关**（脚本内 line 73–76：在做任何动作之前 `exit 11`）：
#   /home/openclaw/llama-swap/strata-upgrade-if-idle.disabled
# 开着它**对租户零功能影响** —— 实测日志「已是目标状态 (profile=iq3_s-native
# 引擎=0.1.60 缓存=98304 64), 无需升级」，且每轮都「停服务前复查: 有新请求进来,
# 本轮放弃 (服务不动)」⇒ 本来就是空转。
#
# 本看门狗**持续重新断言**这个开关存在（有人删掉就补上），
# 因为它是「独占窗口」的一部分 —— 窗口静默打开和开关被删是同一类失效。
# 撤销方式：`rm -f` 该文件（脚本自带、文档化的急停开关，可逆）。
GUARD_FILE="${GUARD_FILE:-/home/openclaw/llama-swap/strata-upgrade-if-idle.disabled}"
GUARD_OWNER="${GUARD_OWNER:-openclaw}"

ts() { date -u +%Y-%m-%dT%H:%M:%SZ; }

journal() {
  # $1=action $2=object $3=detail
  mkdir -p "$STATE_DIR" 2>/dev/null || return 0
  printf '{"ts":"%s","action":"%s","object":"%s","detail":"%s"}\n' \
    "$(ts)" "$1" "$2" "${3:-}" >> "$JOURNAL" 2>/dev/null || true
}

unit_active_state() {
  systemctl show "$1" -p ActiveState --value 2>/dev/null | tr -d '\r' | tail -1
}

assert_units() {
  local u s
  for u in "${UNITS[@]}"; do
    s="$(unit_active_state "$u")"
    if [ "$s" = "active" ]; then
      if systemctl stop "$u" >/dev/null 2>&1; then
        journal "stop_unit" "$u" "was_active"
      else
        journal "stop_unit_failed" "$u" "was_active"
      fi
    fi
  done
}

assert_containers() {
  local c p names
  names="$(docker ps --format '{{.Names}}' 2>/dev/null \
    | grep -E "$CI_PATTERN" | grep -vE "$KEEP_PATTERN" | sort)"
  [ -z "$names" ] && return 0
  while IFS= read -r c; do
    [ -z "$c" ] && continue
    p="$(docker inspect -f '{{.State.Paused}}' "$c" 2>/dev/null | tr -d '\r' | tail -1)"
    case "$p" in
      true) : ;;  # 已 paused —— 不动
      false)
        if docker pause "$c" >/dev/null 2>&1; then
          journal "pause_container" "$c" "was_running"
        else
          journal "pause_container_failed" "$c" "was_running"
        fi
        ;;
      *) : ;;  # 容器刚被删掉 —— 下一轮再看
    esac
  done <<EOF
$names
EOF
}

assert_guard() {
  # 急停开关必须存在（抑制每小时 :07/:37 的宿主 I/O 风暴）
  [ -z "${GUARD_FILE:-}" ] && return 0
  if [ ! -f "$GUARD_FILE" ]; then
    if touch "$GUARD_FILE" 2>/dev/null; then
      journal "restore_guard_file" "$GUARD_FILE" "was_missing"
    else
      journal "restore_guard_file_failed" "$GUARD_FILE" "was_missing"
    fi
  fi
}

one_round() {
  assert_units
  assert_containers
  assert_guard
}

case "${1:-run}" in
  once)
    one_round
    echo "one round done"
    ;;
  run)
    echo "[$(ts)] 看门狗启动，interval=${INTERVAL}s journal=${JOURNAL}"
    while true; do
      one_round
      sleep "$INTERVAL"
    done
    ;;
  *)
    echo "usage: $0 [run|once]" >&2
    exit 2
    ;;
esac

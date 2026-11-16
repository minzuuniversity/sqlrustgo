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
# **外来 CPU 怎么量（2026-11-17 实测修正）**
# ------------------------------------------
# 第一版用 `ps -eo pcpu` 求和。两个错误让它**严重虚高**：`ps` 的 pcpu 是
# **进程生命周期均值**（四小时前启动、现在空闲的进程仍按一生均值计费），
# 而且**已经退出的进程完全不被计入** —— 而别人的编译负载恰恰是「spawn
# 一个 rustc、烧 30 秒、退出」。同一台 80 核机器、同一组别的作业实测：
#
#     ps 生命周期均值口径   7985 %          （= 80 核，看起来满载）
#     增量口径（正确单位）  674 – 1784 %    （= 6.7 – 17.8 核，实际占用）
#
# 即 ps 口径虚高约 **4 – 11 倍**（取决于邻居作业的瞬时强度）。
# 读数单位一律是「核百分比」：100 % = 1 个核。
#
# 现在改用**系统级 busy（/proc/stat 差分）− 自有进程 busy（/proc/<pid>/stat
# 差分）**：不漏短命进程，本启动器自己的臂 B 被整块扣掉。阈值 2400 % 不变，
# 且两端判定不变（污染轮 root 的 oltp_test 独占 29 核 = 2900 %，仍被拒）。
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
# 跑了多少查询、错了多少；折算 QPS 低于 MIN_DRIVER_QPS、或错误率高于
# MAX_ERROR_RATE_PCT，就说明机器还是被压着 —— 停掉这一轮，回到等待，
# 绝不把一份被污染的 run 撑满 168 小时。
#
# 复验量的是**稳态区间**（驱动启动 30 s 之后）：热身段（连接建立、首屏
# 数据准备）的错误不代表稳态。
#
# 等待超时后的乐观起跑
# --------------------
# 252 上 root 的 CI 常驻（`bustubx-unpause-ci.sh` 连续跑 4 h 20 m，不断
# 拉起 cargo build/check/test/clippy），「连续 10 分钟既低负载又无外来
# CPU」这个组合几乎凑不齐 —— 只留安静窗口的话，启动器会永远等下去，
# 而**等下去不是保护，是死锁**。所以等满 QUIET_WAIT_MAX（默认 2 h）后，
# 只要 load1 ≤ QUIET_MAX_LOAD（机器没有过载）就允许试跑一次，由**复验**
# 决定收不收：复验量的是我们自己的真实吞吐与错误率，是**结果指标**，
# 比「机器看起来安静」这个代理指标可靠。
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
#   QUIET_WAIT_MAX     等满这么多秒还没凑齐安静窗口就允许乐观起跑（默认 7200）
#   MIN_DRIVER_QPS     吞吐复验的 QPS 下限（默认 10）
#   MAX_ERROR_RATE_PCT 复验的错误率上限（默认 1）
#   RETRY_BACKOFF      一轮被拒后的退避秒数（默认 300）
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
# 吞吐复验的 QPS 下限。
#
# 从 5 提到 10：旧值 5 贴着**实测的受污染轮**（4.7 q/s）—— 只差 6 % 余量，
# 等于没有余量。252 上三个实测点：
#
#     受污染轮   4.7 q/s   错误率 9.6 %      ← 必须被拒
#     干净轮      47 q/s   错误率 0.07 %     ← 必须被收
#     争用探针    61 q/s   错误率 0.12 %     ← 必须被收
#
# 10 落在 4.7 与 47 之间：距污染轮 2.1 倍、距健康轮 4.7 倍，两个方向都有余量。
MIN_DRIVER_QPS="${MIN_DRIVER_QPS:-10}"
# 复验的错误率上限 —— 与臂 A / 臂 B 的 1% 同口径（2026-11-17 实测标定：
# 健康 0.07–0.12 %，受污染 9.6 %）。
MAX_ERROR_RATE_PCT="${MAX_ERROR_RATE_PCT:-1}"
# 等满这么多秒还没凑齐安静窗口，就允许一次「乐观起跑」：由复验（吞吐 +
# 错误率）决定收不收。252 上 root 的 CI 常驻，安静窗口几乎凑不齐 ——
# 死锁不是保护（2026-11-17 实测：外来 CPU 674–1784 % 远低于上限，但 load1
# 反复越过 40）。默认 2 小时。
QUIET_WAIT_MAX="${QUIET_WAIT_MAX:-7200}"
# 一轮被拒后的退避秒数（避免连着试，把机器搅得更挤）。
RETRY_BACKOFF="${RETRY_BACKOFF:-300}"
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
command -v cargo >/dev/null 2>&1 || fail "cargo required (to build the frozen binary)"
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

# 冻结提交必须与被 soak 的二进制**同源**。
#
# 提交号本身不保证二进制是新的：crates/ 改过之后如果不重建，被 soak 的
# 仍然是旧二进制，而报告里的 frozen_commit 会写着新提交 —— 证据链当场
# 就假了。所以起跑前按当前检出重建一次（已经是最新时是空操作，秒级），
# 让「二进制 ↔ 冻结提交」由**构造**保证，而不是靠人的纪律。
#
# 这一条是 VtuGuard 那次修复逼出来的：crates/storage/src/vtu_guard.rs
# 改了（事务状态透传），若不同步重建，SOAK 跑的就是修复前的二进制。
log "building the release binary from the frozen checkout (no-op when already current)..."
if ! cargo build --release -p sqlrustgo-mysql-server \
      --manifest-path "$SOAK_REPO_ROOT/Cargo.toml" >> "$ART_DIR/build.log" 2>&1; then
  fail "cargo build failed — see $ART_DIR/build.log"
fi
[ -x "$SOAK_REPO_ROOT/target/release/sqlrustgo-mysql-server" ] \
  || fail "release binary missing after build: $SOAK_REPO_ROOT/target/release/sqlrustgo-mysql-server"

log "launcher up: repo=$SOAK_REPO_ROOT head=$HEAD_COMMIT art=$ART_DIR run_id=$RUN_ID hours=$HOURS port=$PORT"
log "policy: quiet_max_load=$QUIET_MAX_LOAD foreign_cpu_max=$FOREIGN_CPU_MAX quiet_samples=$QUIET_SAMPLES interval=${QUIET_INTERVAL}s min_qps=$MIN_DRIVER_QPS max_error_rate_pct=$MAX_ERROR_RATE_PCT quiet_wait_max=${QUIET_WAIT_MAX}s retry_backoff=${RETRY_BACKOFF}s verify_secs=$VERIFY_SECS"

# ---------------------------------------------------------------------------
# 外来 CPU 总量 —— **增量**口径（2026-11-17 实测修正）
# ---------------------------------------------------------------------------
#
# 旧口径（`ps -eo pcpu` 求和）有两个致命错误，252 上实测把它**虚高 4–11 倍**
# （取决于邻居作业的瞬时强度）：
#
#   1. `ps` 的 pcpu 是**进程生命周期均值**，不是当前占用。一个 4 小时前
#      启动、现在空闲的进程，仍然按它一生的均值计费。
#   2. **已经退出的进程完全不被计入** —— 而别人的编译负载恰恰是
#      「spawn 一个 rustc、烧 30 秒、退出」，样本之间这些 CPU 一个都不算。
#
# 实测对照（同一台 80 核机器、同一组别的作业）：
#
#     ps 生命周期均值口径   7985 %        （= 80 核，看起来是满载）
#     增量口径（正确单位）   674 – 1784 %  （= 6.7 – 17.8 核，实际占用）
#
# 用虚高一个数量级的数字卡门禁，等于要求一台「80 核全空闲」的机器。而 252 上
# root 的 CI 常驻（`bustubx-unpause-ci.sh` 已连续跑 4 h 20 m，不断拉起
# cargo build/check/test/clippy），旧口径下安静窗口**永远不会满足** ——
# 门禁不是保护，是死锁。
#
# 新口径：**系统级 busy（/proc/stat 差分）− 自有进程 busy（/proc/<pid>/stat 差分）**。
#
#   * 系统级口径**不漏短命进程**：它们烧掉的 CPU 计入 /proc/stat 的 total。
#     iowait 计入 idle（I/O 阻塞由 load1 那道门禁与 fsync 探针兜底）。
#   * 自有进程整块扣掉：本启动器自己起的臂 B（cargo test）不会被误判成
#     「外来争用」—— 这与旧口径按 owner 区分的意图一致，但更彻底。
#   * **标定不失效**：污染轮里 root 的 oltp_test 独占 29 核 = 2900 %，
#     单这一项就超过 2400 % 的上限，仍然会被拒；干净轮与争用探针
#     （实测 61 q/s、0.12 % 错误）仍落在上限之内。两端判定不变。
#
# 代价：系统级口径把 forge 自己的 postgres/gitea 也算成外来（各约 1–3 %，
# 合计 < 0.5 核），方向是**偏严**，可接受。
CPU_SNAP="$ART_DIR/.cpu_snapshot.json"

sample_cpu() {
  python3 - "$CPU_SNAP" <<'PYEOF'
import os, sys, time, glob, json

SNAP = sys.argv[1]
HZ = os.sysconf('SC_CLK_TCK')


def read_now():
    total = idle = 0
    try:
        with open('/proc/stat') as f:
            for line in f:
                if line.startswith('cpu '):
                    v = [int(x) for x in line.split()[1:9]]
                    total = sum(v)
                    idle = v[3] + v[4]   # idle + iowait
                    break
    except Exception:
        pass
    procs = {}
    for p in glob.glob('/proc/[0-9]*/stat'):
        try:
            with open(p) as f:
                raw = f.read()
            rp = raw.rindex(')')
            pid = p.split('/')[2]
            rest = raw[rp + 2:].split()
            procs[pid] = (os.stat(p).st_uid, int(rest[11]) + int(rest[12]))
        except Exception:
            pass
    return {'t': time.time(), 'total': total, 'idle': idle, 'procs': procs}


def uname_of(uid):
    try:
        import pwd
        return pwd.getpwuid(uid).pw_name
    except Exception:
        return str(uid)


now = read_now()
try:
    with open(SNAP) as f:
        prev = json.load(f)
except Exception:
    prev = None
with open(SNAP, 'w') as f:
    json.dump(now, f)
if prev is None:
    # 第一次采样没有基线：这一轮**不算数**（宁可少一次机会，不可误判安静）
    print('warmup|0|0')
    print('top|')
    sys.exit(0)

dt = now['t'] - prev['t']
d_total = now['total'] - prev['total']
d_idle = now['idle'] - prev['idle']

# **单位**是关键：全部读数都是「核百分比」——100 % = 1 个核。
#
# /proc/stat 的 total 是**全部核**的 jiffies，所以
#   (Δtotal − Δidle) / (dt × HZ) = **忙着的核数**（整机口径）
# 而单个进程的 Δticks / (dt × HZ) = 它占的**核数**（单核口径）。
# 两者单位相同（都是「核」），可以直接相减得到外来核数。
#
# 2026-11-17 实测踩过的坑：曾把系统侧算成「整机百分比」（0–100）、
# 自有侧算成「核百分比」，相减之后外来 CPU 恒等于 0，
# FOREIGN_CPU_MAX=2400 这道门禁**永远不会触发**（0 > 2400 恒为假）。
# 门禁死了比没有门禁更坏：它看起来在工作。
sys_pct = 100.0 * (d_total - d_idle) / (dt * HZ) if dt > 0 else 0.0

me = os.getuid()
own_ticks = 0
for pid, (uid, ticks) in now['procs'].items():
    if uid != me:
        continue
    old = prev['procs'].get(pid)
    if old is not None:
        own_ticks += ticks - old[1]
own_pct = 100.0 * own_ticks / (dt * HZ) if dt > 0 else 0.0

foreign = sys_pct - own_pct
if foreign < 0:
    foreign = 0.0

# 明细：逐进程的当前占用（仅供日志留痕；短命进程可能已退出而看不到）
detail = []
for pid, (uid, ticks) in now['procs'].items():
    if uid == me:
        continue
    old = prev['procs'].get(pid)
    if old is None:
        continue
    pct = 100.0 * (ticks - old[1]) / (dt * HZ)
    if pct >= 20:
        detail.append((pct, uname_of(uid)))
detail.sort(reverse=True)
top = ';'.join('%s|%.0f' % (u, p) for p, u in detail[:3])

print('%.0f|%.0f|%.0f' % (foreign, sys_pct, own_pct))
print('top|%s' % top)
PYEOF
}

# ---------------------------------------------------------------------------
# fsync 探针 —— 量**这块盘**在起跑那一刻的写延迟
# ---------------------------------------------------------------------------
#
# 252 上 root 的编译与我们的 WAL 落在**同一块 NVMe**（/home 与 /workspace
# 同属 /dev/nvme0n1p2），所以「盘现在挤不挤」比「CPU 挤不挤」更直接地
# 决定这次 SOAK 的数字有没有意义。实测（同一时刻）：
#
#     fsync p50 = 4.16 ms    p95 = 11.88 ms    max = 782 ms
#     （健康 NVMe 应在 0.2–0.5 ms 量级）
#
# 这个探针**只记录、不设门禁**：真正的门禁是复验（吞吐 + 错误率），
# 它是结果指标；探针是上下文，写进 STARTED.json 与日志，让报告能说清
# 「这次跑的盘是什么状态」。
fsync_probe() {
  python3 - "$1" <<'PYEOF'
import os, sys, time, statistics
d = sys.argv[1]
try:
    os.makedirs(d, exist_ok=True)
    lat = []
    for i in range(7):
        p = os.path.join(d, '.fsync-probe-%d' % i)
        fd = os.open(p, os.O_CREAT | os.O_WRONLY | os.O_TRUNC)
        os.write(fd, b'x' * 4096)
        t0 = time.perf_counter()
        os.fsync(fd)
        t1 = time.perf_counter()
        os.close(fd)
        os.unlink(p)
        lat.append((t1 - t0) * 1000.0)
        time.sleep(0.02)
    lat.sort()
    print('%.2f' % statistics.median(lat))
except Exception:
    print('unknown')
PYEOF
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
# 收掉一轮失败/被拒的 attempt。
#
# 顺序是有讲究的：**先杀「臂 A 脚本」本身**，因为它自带
# `trap cleanup INT TERM` —— 那条 trap 会把服务端与驱动收干净。
# 只杀运行器的子 shell 是不够的：臂 A 在自己的进程组里（运行器用
# `set -m` 让每条臂独占进程组），杀 wrapper 不会连带杀臂 A，服务端
# 会继续占着端口。
#
# 端口残留会让下一轮 attempt 直接撞上「port already in use」，
# 于是每一轮都失败、每一轮都重试 —— 一个永远不会收敛的循环。
# 所以最后还要按端口兜底清理，并在清完后断言端口已空。
kill_soak_tree() {
  local pids p

  for p in $(pgrep -f "v400_1h_soak.sh" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -TERM "$p" 2>/dev/null || true
  done
  for p in $(pgrep -f "v400_soak_driver.py" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -TERM "$p" 2>/dev/null || true
  done
  pids="$(pgrep -f "run_v410_168h_soak.sh --hours" 2>/dev/null || true)"
  for p in $pids; do
    [ "$p" = "$$" ] && continue
    kill -TERM "$p" 2>/dev/null || true
  done
  # 臂 B 的 cargo test 与它拉起的测试二进制不是 wrapper 的直接子进程，
  # 不收掉就会在下一轮 attempt 之后继续占着机器 —— 下一轮复验读到的
  # 负载就是两份 run 叠在一起的负载。
  for p in $(pgrep -f "v312_mixed_soak" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -TERM "$p" 2>/dev/null || true
  done
  # 端口兜底：任何还占着端口的进程
  for p in $(lsof -ti "tcp:$PORT" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -TERM "$p" 2>/dev/null || true
  done

  sleep 3

  for p in $(pgrep -f "v400_1h_soak.sh" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -KILL "$p" 2>/dev/null || true
  done
  for p in $(pgrep -f "v400_soak_driver.py" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -KILL "$p" 2>/dev/null || true
  done
  for p in $pids; do
    [ "$p" = "$$" ] && continue
    kill -KILL "$p" 2>/dev/null || true
  done
  for p in $(pgrep -f "v312_mixed_soak" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -KILL "$p" 2>/dev/null || true
  done
  for p in $(lsof -ti "tcp:$PORT" 2>/dev/null || true); do
    [ "$p" = "$$" ] && continue
    kill -KILL "$p" 2>/dev/null || true
  done

  sleep 1
  local still
  still="$(lsof -ti "tcp:$PORT" 2>/dev/null | tr '\n' ' ')"
  if [ -n "$still" ]; then
    log "WARNING: port $PORT is STILL held by [$still] after cleanup — the next attempt would collide on it."
  else
    log "stopped the contaminated attempt ($ATTEMPT_DIR); port $PORT is free again"
  fi
}

# ---------------------------------------------------------------------------
# 起跑
# ---------------------------------------------------------------------------
# 每一次 attempt 写进**自己的**产物目录。
#
# 为什么不能共用一个目录：吞吐复验读的是 armA 的 metrics.csv（累计
# driver_q / driver_e 与 elapsed_s）。若上一轮 attempt 的 metrics.csv 还在，
# 新一轮的计数会把旧账算进来 —— 一轮几乎没跑起来的 attempt 也会被算成
# 「QPS 达标」，复验门禁就废了。
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
WAIT_START="$(date +%s)"
while true; do
  ATTEMPT=$((ATTEMPT + 1))
  LOAD="$(load1)"
  CPU_OUT="$(sample_cpu)"
  CPU_LINE1="$(printf '%s\n' "$CPU_OUT" | sed -n '1p')"
  FOREIGN_TOP="$(printf '%s\n' "$CPU_OUT" | sed -n '2p' | cut -d'|' -f2-)"
  # 三态：warmup（还没有基线，**这一轮不算数**）/ 读数 / 测量失败。
  #
  # 测量失败与 warmup 一样必须**归零并判为「非安静」**：把「读不出来」
  # 当成「没有争用」，等于把仪器坏了当成机器安静 —— 那正是门禁失守的
  # 形式。宁可多等，不可误判。
  WARMUP=1
  FOREIGN_TOTAL=0
  SYS_CPU=0
  OWN_CPU=0
  case "${CPU_LINE1%%|*}" in
    warmup)
      WARMUP=1
      ;;
    ''|*[!0-9]*)
      WARMUP=1
      log "WARNING: CPU measurement unreadable (raw='${CPU_LINE1}') — this sample counts as NOT quiet."
      ;;
    *)
      WARMUP=0
      FOREIGN_TOTAL="$(printf '%s' "$CPU_LINE1" | cut -d'|' -f1)"
      SYS_CPU="$(printf '%s' "$CPU_LINE1" | cut -d'|' -f2)"
      OWN_CPU="$(printf '%s' "$CPU_LINE1" | cut -d'|' -f3)"
      [ -n "$FOREIGN_TOTAL" ] || FOREIGN_TOTAL=0
      [ -n "$SYS_CPU" ] || SYS_CPU=0
      [ -n "$OWN_CPU" ] || OWN_CPU=0
      ;;
  esac

  QUIET=1
  [ "$WARMUP" -eq 1 ] && QUIET=0
  [ "$FOREIGN_TOTAL" -gt "$FOREIGN_CPU_MAX" ] && QUIET=0
  LOAD_INT="$(printf '%.0f' "$LOAD" 2>/dev/null || echo 0)"
  [ "$LOAD_INT" -gt "$QUIET_MAX_LOAD" ] && QUIET=0

  if [ "$QUIET" -eq 1 ]; then
    STREAK=$((STREAK + 1))
  else
    STREAK=0
  fi

  WAITED=$(( $(date +%s) - WAIT_START ))
  if [ "$QUIET" -eq 1 ]; then
    log "host QUIET load=$LOAD_INT foreign_cpu=${FOREIGN_TOTAL}% system_cpu=${SYS_CPU}% own_cpu=${OWN_CPU}% streak=$STREAK/$QUIET_SAMPLES waited=${WAITED}s"
  else
    log "host BUSY  load=$LOAD_INT foreign_cpu=${FOREIGN_TOTAL}% (max $FOREIGN_CPU_MAX%) system_cpu=${SYS_CPU}% streak=0/$QUIET_SAMPLES waited=${WAITED}s top=[$FOREIGN_TOP]"
  fi

  # ---- 起跑条件：两条路 ----------------------------------------------------
  #
  # (a) **安静窗口**（首选）：连续 QUIET_SAMPLES 次同时满足
  #     外来 CPU ≤ FOREIGN_CPU_MAX 与 load1 ≤ QUIET_MAX_LOAD。
  #     这是「主机确实空闲」的正面证据。
  #
  # (b) **等待超时后的乐观起跑**：等满 QUIET_WAIT_MAX 秒仍未凑齐安静窗口时，
  #     只要 load1 ≤ QUIET_MAX_LOAD（机器**没有过载**）就允许试跑一次，
  #     由**复验**（真实吞吐 + 错误率）决定收不收。
  #
  # 为什么要 (b)：252 上 root 的 CI 常驻，「连续 10 分钟既无外来 CPU 又有
  # 低负载」这个组合几乎不会出现（2026-11-17 实测：外来 CPU 实测 674–1784%，
  # 但 load1 在 31–47 之间反复越过 40）。只留 (a) 的话，启动器会永远等下去，
  # 而「等下去」不是保护，是死锁。
  #
  # (b) 的安全性来自复验而不是来自「机器看起来安静」：复验量的是**我们自己的
  # 真实吞吐与错误率**。受污染轮实测 4.7 q/s + 9.6% 错误，干净轮 47–65 q/s
  # + 0.07–0.12% 错误 —— 两道结果指标都能把污染轮挡在外面。宁可多试几轮
  # （每轮代价 3 分钟），不可带噪起跑，也不可永远不起跑。
  START_PATH=""
  if [ "$STREAK" -ge "$QUIET_SAMPLES" ]; then
    START_PATH="quiet-window"
  elif [ "$WAITED" -ge "$QUIET_WAIT_MAX" ] && [ "$LOAD_INT" -le "$QUIET_MAX_LOAD" ]; then
    START_PATH="optimistic-after-wait"
  fi

  if [ -n "$START_PATH" ]; then
    log "starting attempt (path=$START_PATH): load=$LOAD_INT foreign_cpu=${FOREIGN_TOTAL}% system_cpu=${SYS_CPU}% waited=${WAITED}s attempt=$ATTEMPT"
    start_soak

    sleep "$VERIFY_SECS"
    # 复验的**时间基准**取自 armA 的 metrics.csv（15 s 一采样，带
    # elapsed_s 与累计 driver_q / driver_e），**不是** driver.log。
    #
    # 为什么不能读 driver.log：驱动写的第 2 列是**单条查询自己的耗时**
    # （`ms = (time.monotonic() - t0) * 1000`，t0 取在循环体内、每条查询
    # 之前），不是「驱动启动以来的经过时间」。把它当 elapsed 用，等于要求
    # 「单条查询跑了 30 秒」才把这一行算进稳态 —— 稳态区间恒为空集，
    # 复验恒判「0 q/s、错误率 100%」。
    #
    # 2026-11-17 实测：attempt-31 被这道仪器误杀 —— 那轮其实健康
    # （180 s 内 9194 条查询 ≈ 51 q/s、0 错误），却因读错时间基准被判
    # 「0 q/s vs floor 10」，白跑一轮。
    #
    # 稳态窗口 = 驱动启动 30 s 之后（热身段：连接建立、首屏数据准备）
    # 到复验时刻之间的那段时间。
    STEADY_LINE="$("$SOAK_REPO_ROOT/scripts/soak/soak_steady_window.py" \
                  "$ATTEMPT_DIR/armA/metrics.csv" 30 2>/dev/null || true)"
    if [ -z "$STEADY_LINE" ]; then
      # 读不出来 ⇒ 一律判红，绝不把「读不出来」当成「达标」。
      Q_STEADY=0; E_STEADY=0; STEADY_SECS=0; QPS=0; ERR_PCT="100.000"; SUSTAINED=0
    else
      read -r Q_STEADY E_STEADY STEADY_SECS QPS ERR_PCT SUSTAINED <<< "$STEADY_LINE"
    fi
    log "throughput re-check: steady ${Q_STEADY} queries / ${E_STEADY} errors in ${STEADY_SECS}s => ${QPS} q/s (floor $MIN_DRIVER_QPS), error rate ${ERR_PCT}% (max $MAX_ERROR_RATE_PCT%)"

    if [ "$SUSTAINED" -eq 1 ] && \
       awk -v q="$QPS" -v m="$MIN_DRIVER_QPS" 'BEGIN { exit !(q + 0 >= m + 0) }' && \
       awk -v r="$ERR_PCT" -v m="$MAX_ERROR_RATE_PCT" 'BEGIN { exit !(r + 0 <= m + 0) }'; then
      bin_sha="$(sha256sum "$SOAK_REPO_ROOT/target/release/sqlrustgo-mysql-server" 2>/dev/null | cut -d" " -f1)"
      FSYNC_P50="$(fsync_probe "${SOAK_DATA_DIR:-/tmp/sqlrustgo-v410-168h-$RUN_ID}")"
      cat > "$ART_DIR/STARTED.json" <<JSON
{
  "run_id": "$RUN_ID",
  "attempt_dir": "$ATTEMPT_DIR",
  "frozen_commit": "$HEAD_COMMIT",
  "soaked_binary_sha256": "$bin_sha",
  "binary_built_from_frozen_commit": true,
  "host": "$(uname -n) $(uname -s) $(uname -r) $(uname -m)",
  "hours": $HOURS,
  "port": $PORT,
  "started_utc": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "planned_end_utc": "$(end_utc_in $((HOURS*3600)))",
  "verify_secs": $VERIFY_SECS,
  "driver_queries_at_verify": $Q_STEADY,
  "driver_errors_at_verify": $E_STEADY,
  "driver_qps_at_verify": $QPS,
  "driver_error_rate_pct_at_verify": $ERR_PCT,
  "quiet_max_load": $QUIET_MAX_LOAD,
  "foreign_cpu_max": $FOREIGN_CPU_MAX,
  "quiet_samples": $QUIET_SAMPLES,
  "min_driver_qps": $MIN_DRIVER_QPS,
  "max_error_rate_pct": $MAX_ERROR_RATE_PCT,
  "start_path": "$START_PATH",
  "attempt": $ATTEMPT,
  "waited_secs": $WAITED,
  "host_at_start": {
    "load1": $LOAD_INT,
    "foreign_cpu_pct": $FOREIGN_TOTAL,
    "system_cpu_pct": $SYS_CPU,
    "own_cpu_pct": $OWN_CPU,
    "fsync_p50_ms": $FSYNC_P50
  }
}
JSON
      log "PASS: the host carried real load (${QPS} q/s, ${ERR_PCT}% errors); the $HOURS h clock is running."
      log "artifacts: $ATTEMPT_DIR"
      log "canonical record: $ART_DIR/STARTED.json"
      exit 0
    fi
    log "REJECT: the host is still carrying someone else's load (${QPS} q/s vs floor $MIN_DRIVER_QPS, error rate ${ERR_PCT}% vs max $MAX_ERROR_RATE_PCT%, sustained=$SUSTAINED) — stopping this attempt and going back to waiting."
    kill_soak_tree
    STREAK=0
    sleep "$RETRY_BACKOFF"
  fi

  sleep "$QUIET_INTERVAL"
done

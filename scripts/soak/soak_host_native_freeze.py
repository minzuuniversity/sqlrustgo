#!/usr/bin/env python3
"""SOAK 独占窗口的第二半：冻结**宿主原生**的高 CPU 作业。

为什么需要它
------------
`scripts/soak/soak_exclusive_window.sh` 只处理 **docker 容器**与两个
**systemd 单元**。但 2026-11-18 实测（r2 轮，run_id
`20261118-168h-v410-r2`）发现：252 上还有一批**根本不在容器里、也不是
Gitea Actions** 的构建作业直接跑在**宿主**上：

    bash scripts/build-cuda.sh
      └─ cmake --build /home/openclaw/workspace/kvmem/build --parallel 24
           └─ gmake -j24 → nvcc / cicc / cc1plus（CUDA + C++ 编译）
    bash tools/parallel_observe.sh
      └─ cargo test --workspace --no-fail-fast
    （另有 uid 0 的 rustc 与 openclaw 身份的 cc1plus 在编译 basalt）

实测这些进程是**手工/会话启动的长驻作业**（cron 与 systemd 里都没有
对应条目）⇒ **禁用仓库 Actions + pause 容器都拦不住它们**。

这正是属主裁定的「**独占窗口是 168h SOAK 合格轮次的必要条件、不是
充分条件**」的机制解释：r2 轮错误窗口内独占探针的
`root_compile_or_test_procs` 全程为 **0**，但 `host_busy_ge50cpu` 为
**2–11**、`load1` 在 **21–38** 起伏，且**错误与负载尖峰共变**
（load 回落到 < 10 后 1 h 15 m 零新增错误）。

为什么用 SIGSTOP 而不是 kill
-----------------------------
- **可逆**：SIGCONT 即恢复，作业的构建状态、已编译产物、命令行全部保留；
- **不丢工作**：不像 kill 会让 cmake/gmake 的增量状态断链；
- **立即释放 CPU**：被停止的进程完全不占 CPU、不发起 IO；
- **最小干预**：只冻结**真的在抢 CPU** 的进程（≥ --min-pct 核百分比），
  空闲进程一律不动。

口径
----
占用量用 **/proc/<pid>/stat 的 utime+stime 差分**（真实占用，核百分比），
**不用 `ps -eo pcpu`** —— 后者是进程**生命周期均值**且**不计已退出进程**，
会把四小时前启动、现在空闲的进程也算进来。

身份校验
--------
被冻结的 PID 记录其 `/proc/<pid>/stat` 的 starttime；恢复时若 PID 已被
复用（starttime 不同）则**跳过**，避免误伤新进程。

用法
----
    # 冻结（可反复调用，保持持续覆盖）
    python3 soak_host_native_freeze.py freeze --out /path/frozen.json

    # 恢复
    python3 soak_host_native_freeze.py resume --out /path/frozen.json

    # 持续守护：每隔 --interval 扫一次，把新冒出来的抢盘作业也冻上
    python3 soak_host_native_freeze.py watch --out /path/frozen.json
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import signal
import time

HZ = os.sysconf('SC_CLK_TCK')

# ---- 永远不碰的对象 ------------------------------------------------------
# 被测系统自己 + 基础设施 + 本脚本 + shell。
PROTECTED_CMD = (
    'run_v410_168h_soak', 'v400_1h_soak', 'v400_prepare_soak',
    'sqlrustgo-mysql-server', 'soak_heartbeat', 'soak_cpu_sampler',
    'soak_host_native_freeze', 'soak_exclusive_window',
    'v312_mixed_soak', 'run_soak_with_heartbeat',
    'gitea', 'postgres', 'dockerd', 'containerd', 'sshd', 'cron', 'systemd',
    'rsyslog', 'journald', 'dbus', 'docker-proxy', 'containerd-shim',
    'bash -c', 'ps ', 'awk', 'grep', 'tr ', 'cut ', 'sed ',
)

# 只冻结这些「构建/编译」工具链 —— 宁可漏冻，不可误伤。
BUILD_TOOLS = (
    'rustc', 'cargo', 'cmake', 'gmake', 'make', 'cc1', 'cc1plus', 'nvcc',
    'cicc', 'ptxas', 'cudafe++', 'ccache', 'ld', 'ld.lld', 'ar', 'ranlib',
    'gcc', 'g++', 'clang', 'clang++', 'nvlink', 'fatbinary', 'nvcc',
    'cc', 'c++', 'as', 'python3', 'cc1obj',
)


def read_proc(pid):
    try:
        with open('/proc/%d/stat' % pid) as f:
            raw = f.read()
        rp = raw.rindex(')')
        rest = raw[rp + 2:].split()
        state = rest[0]
        starttime = int(rest[19])
        utime, stime = int(rest[11]), int(rest[12])
        return state, starttime, utime + stime
    except Exception:
        return None


def read_cmd(pid):
    try:
        with open('/proc/%d/cmdline' % pid, 'rb') as f:
            return f.read().replace(b'\x00', b' ').decode('utf-8', 'replace').strip()
    except Exception:
        return ''


def read_comm(pid):
    try:
        with open('/proc/%d/comm' % pid) as f:
            return f.read().strip()
    except Exception:
        return ''


def is_protected(cmd):
    return any(p in cmd for p in PROTECTED_CMD)


def is_build_tool(comm):
    base = comm.split('/')[-1]
    return base in BUILD_TOOLS or base.startswith('cc1') or base.startswith('nvcc')


def scan_high_cpu(min_pct, interval):
    """返回 [(pid, pct, comm, cmd)]：真实占用 ≥ min_pct 的构建类进程。"""
    total, idle, procs = read_cpu()
    now = time.time()
    time.sleep(interval)
    total2, idle2, procs2 = read_cpu()
    dt = now and (time.time() - now) + interval
    out = []
    for pid, (busy, st) in procs2.items():
        prev = procs.get(pid)
        if not prev:
            continue
        d = (busy - prev[0]) / HZ
        if d <= 0:
            continue
        pct = d / dt * 100.0
        if pct < min_pct:
            continue
        comm = read_comm(pid)
        cmd = read_cmd(pid)
        if not is_build_tool(comm):
            continue
        if is_protected(cmd) or is_protected(comm):
            continue
        out.append((pid, pct, comm, cmd[:200]))
    out.sort(key=lambda r: -r[1])
    return out


def read_cpu():
    total = idle = 0
    try:
        with open('/proc/stat') as f:
            for line in f:
                if line.startswith('cpu '):
                    v = [int(x) for x in line.split()[1:9]]
                    total = sum(v)
                    idle = v[3] + v[4]
                    break
    except Exception:
        pass
    procs = {}
    for p in glob.glob('/proc/[0-9]*/stat'):
        try:
            with open(p) as f:
                raw = f.read()
            rp = raw.rindex(')')
            pid = int(p.split('/')[2])
            rest = raw[rp + 2:].split()
            procs[pid] = (int(rest[11]) + int(rest[12]), raw[rp + 2:].split()[0])
        except Exception:
            pass
    return total, idle, procs


def load_frozen(path):
    try:
        with open(path) as f:
            return json.load(f)
    except Exception:
        return {}


def save_frozen(path, data):
    tmp = path + '.tmp'
    with open(tmp, 'w') as f:
        json.dump(data, f, ensure_ascii=False, indent=2)
    os.replace(tmp, path)


def do_freeze(path, min_pct, interval):
    data = load_frozen(path)
    hits = scan_high_cpu(min_pct, interval)
    frozen_now = []
    for pid, pct, comm, cmd in hits:
        st = read_proc(pid)
        if not st:
            continue
        state, starttime, _ = st
        if state == 'T':          # 已经是停止态
            continue
        try:
            os.kill(pid, signal.SIGSTOP)
        except Exception:
            continue
        key = str(pid)
        data[key] = {
            'starttime': starttime,
            'comm': comm,
            'cmd': cmd,
            'pct': round(pct, 1),
            'frozen_at': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime()),
        }
        frozen_now.append((pid, comm, pct))
    save_frozen(path, data)
    print('frozen_now=%d total_frozen=%d' % (len(frozen_now), len(data)), flush=True)
    for pid, comm, pct in frozen_now:
        print('  STOP pid=%d comm=%s pct=%.1f' % (pid, comm, pct), flush=True)
    return len(frozen_now)


def do_resume(path):
    data = load_frozen(path)
    resumed = skipped = gone = 0
    for pid_s, rec in list(data.items()):
        pid = int(pid_s)
        st = read_proc(pid)
        if not st:
            gone += 1
            data.pop(pid_s, None)
            continue
        state, starttime, _ = st
        if starttime != rec.get('starttime'):
            skipped += 1        # PID 已被复用 ⇒ 不碰
            data.pop(pid_s, None)
            continue
        try:
            os.kill(pid, signal.SIGCONT)
            resumed += 1
            print('  CONT pid=%d comm=%s' % (pid, rec.get('comm')), flush=True)
        except Exception:
            gone += 1
            data.pop(pid_s, None)
    save_frozen(path, data)
    print('resumed=%d skipped_reused=%d gone=%d remaining=%d'
          % (resumed, skipped, gone, len(data)), flush=True)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('mode', choices=['freeze', 'resume', 'watch'])
    ap.add_argument('--out', required=True)
    ap.add_argument('--min-pct', type=float, default=25.0)
    ap.add_argument('--interval', type=int, default=10)
    args = ap.parse_args()

    if args.mode == 'freeze':
        do_freeze(args.out, args.min_pct, args.interval)
    elif args.mode == 'resume':
        do_resume(args.out)
    else:
        while True:
            try:
                do_freeze(args.out, args.min_pct, args.interval)
            except Exception as e:
                print('watch error:', e, flush=True)
            time.sleep(args.interval)


if __name__ == '__main__':
    main()

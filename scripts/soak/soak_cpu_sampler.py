#!/usr/bin/env python3
"""SOAK 期间的高 CPU 进程身份采样器。

为什么需要它
------------
已有的 `exclusive_probe_current.jsonl` 只记录**计数**：

    root_compile_or_test_procs / own_compile_or_test_procs / host_busy_ge50cpu / load1

2026-11-18 实测（r2 轮，run_id 20261118-168h-v410-r2）：错误窗口
（elapsed 45–75 min）内 `root_compile_or_test_procs` **全程为 0**，
但 `host_busy_ge50cpu` 为 **2–11**、`load1` 在 **21–38** 之间起伏，
且错误与 load 尖峰**共变**（load 回落到 < 10 后 1 h 15 m 零新增错误）。

⇒ 负载源**不是** root 的编译 ⇒ 是机器上另有 2–11 个各占 ≥ 0.5 核的
**非编译进程**。但「计数」无法回答「**是谁**」—— 本采样器补上这一维。

口径
----
用 **/proc/<pid>/stat 的 utime+stime 差分**算真实占用（单位：核百分比，
100 % = 1 核），**不用 `ps -eo pcpu`** —— 后者是进程**生命周期均值**
（四小时前启动、现在空闲的进程仍按一生均值计费），且**已退出的进程
完全不计入**；而别人的编译负载恰恰是「spawn 一个 rustc、烧 30 秒、退出」
⇒ 用 ps 口径会严重虚高。

轻量
----
每 15 s 采一次；只读 /proc，不 fork `ps`/`top`；记录 top 12 个占用者。
开销远小于一次 `ps -eo` 全表扫描。

I/O 轴（2026-11-19 新增）
------------------------
r3 轮的归因（见 `audit/v4.1.0/adjudications/soak-r3-io-storm-attribution-20261119.md`）
证明：**只看 load1 / busy 会看不见 I/O 停顿**。
r3 的 7 个错误发生在宿主 cron 把 9.9 GB 备份树复制到**同一块 NVMe** 的 19 s 里，
当时 `load1` 只有 3.4–6.6（看起来很安静），但被测服务端的存储层被卡住 ~14 s ⇒
所有查询类型（含主键点查）同时越过 15 s 读超时。

⇒ 本采样器新增两个字段：

- **`iowait_pct`** —— `/proc/stat` 的 iowait 占比（单列，不混进 `system_busy_pct`
  —— 后者的口径**保持不变**，保证历史采样可逐条对比）；
- **`sut`** —— 被测服务端的 `/proc/<pid>/io` 差分（`read_MBps` / `write_MBps` /
  `read_disk_MBps` / `write_disk_MBps`）。

⚠️ 找 pid **优先用 `/proc/<pid>/exe` 的 basename 精确匹配**：只用 cmdline 子串
会误伤任何 `sh -c '... sqlrustgo-mysql-server ...'` 的壳进程（含**运维命令自己**），
而那些壳进程的 `rchar`/`wchar` 几乎不动 ⇒ I/O 轴会读成 0，
看起来「服务端不读盘」，其实是**抓错了进程**（2026-11-19 实测踩到过）。

用法
----
    nohup python3 soak_cpu_sampler.py --out /path/probe.jsonl --interval 15 &
"""

from __future__ import annotations

import argparse
import glob
import json
import os
import time

HZ = os.sysconf('SC_CLK_TCK')
TOPN = 12


def read_cpu():
    """返回 (total_ticks, idle_ticks, iowait_ticks, {pid: (uid, busy_ticks)})。

    ⚠️ `system_busy_pct` 的口径**保持不变**（idle + iowait 一起算作「不忙」），
    这样历史采样（r2 / r3）与新采样**可逐条对比**。
    iowait 单独拆出来当**新增字段** `iowait_pct` ——
    2026-11-19 的 r3 轮归因（宿主同盘 I/O 风暴卡住服务端存储层）证明：
    **只看 load1 / busy 会看不见 I/O 停顿**，必须单列 iowait 与服务端 I/O 计数。
    """
    total = idle = iowait = 0
    try:
        with open('/proc/stat') as f:
            for line in f:
                if line.startswith('cpu '):
                    v = [int(x) for x in line.split()[1:9]]
                    total = sum(v)
                    idle = v[3] + v[4]   # idle + iowait
                    iowait = v[4]
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
            # rest[0]=state, rest[1]=ppid … rest[11]=utime, rest[12]=stime
            procs[pid] = (int(rest[11]) + int(rest[12]), raw[rp + 2:].split()[0])
        except Exception:
            pass
    return total, idle, iowait, procs


def find_pid(exe_basename=None, match=None):
    """找被测服务端的 pid。

    ⚠️ **优先用 exe 匹配**：`/proc/<pid>/exe` 的 basename 必须等于目标名。
    只用 cmdline 子串会误伤 —— 任何 `sh -c '... sqlrustgo-mysql-server ...'`
    的壳进程（含**本机运维命令自己**）都会命中，而且那些壳进程的
    `/proc/<pid>/io` 的 rchar/wchar 几乎不动 ⇒ I/O 轴会读成 0，
    看起来「服务端不读盘」，其实是**抓错了进程**。
    """
    best = None

    def _starttime(pid):
        try:
            with open('/proc/%s/stat' % pid) as f:
                raw = f.read()
            rp = raw.rindex(')')
            return int(raw[rp + 2:].split()[19])
        except Exception:
            return None

    if exe_basename:
        for p in glob.glob('/proc/[0-9]*/exe'):
            pid = p.split('/')[2]
            try:
                target = os.readlink(p)
            except Exception:
                continue
            if os.path.basename(target) != exe_basename:
                continue
            st = _starttime(pid)
            if st is None:
                continue
            if best is None or st > best[1]:
                best = (pid, st, read_cmd(pid))
    if best:
        return best[0]

    if match:
        for p in glob.glob('/proc/[0-9]*/stat'):
            pid = p.split('/')[2]
            if match not in read_cmd(pid):
                continue
            st = _starttime(pid)
            if st is None:
                continue
            if best is None or st > best[1]:
                best = (pid, st, read_cmd(pid))
    return best[0] if best else None


def read_io(pid):
    """读 /proc/<pid>/io ⇒ {rchar, wchar, read_bytes, write_bytes}（字节）。"""
    out = {}
    try:
        with open('/proc/%s/io' % pid) as f:
            for line in f:
                k, _, v = line.partition(':')
                out[k.strip()] = int(v.strip())
    except Exception:
        pass
    return out


def read_cmd(pid):
    try:
        with open('/proc/%s/cmdline' % pid, 'rb') as f:
            s = f.read().replace(b'\x00', b' ').decode('utf-8', 'replace').strip()
        return s[:160]
    except Exception:
        return ''


def read_uid(pid):
    try:
        with open('/proc/%s/status' % pid) as f:
            for line in f:
                if line.startswith('Uid:'):
                    return int(line.split()[1])
    except Exception:
        return -1
    return -1


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--out', required=True)
    ap.add_argument('--interval', type=int, default=15)
    ap.add_argument('--pid-match', default='sqlrustgo-mysql-server',
                    help='被测服务端进程的 cmdline 子串；留空则不采 I/O 轴')
    args = ap.parse_args()

    prev_total, prev_idle, prev_iowait, prev_procs = read_cpu()
    prev_t = time.time()
    sut_pid = find_pid(args.pid_match)
    prev_io = read_io(sut_pid) if sut_pid else {}

    with open(args.out, 'a', buffering=1) as fh:
        while True:
            time.sleep(args.interval)
            now_t = time.time()
            total, idle, iowait, procs = read_cpu()
            dt = now_t - prev_t

            sys_total = (total - prev_total) / HZ
            sys_idle = (idle - prev_idle) / HZ
            sys_busy_pct = ((sys_total - sys_idle) / dt * 100.0) if dt > 0 else 0.0
            sys_iowait_pct = ((iowait - prev_iowait) / HZ / dt * 100.0) if dt > 0 else 0.0

            rows = []
            for pid, (busy, state) in procs.items():
                prev = prev_procs.get(pid)
                if not prev:
                    continue
                d = (busy - prev[0]) / HZ
                if d <= 0:
                    continue
                pct = d / dt * 100.0
                if pct >= 25.0:      # 只记 ≥ 0.25 核的
                    rows.append((pct, pid, state))
            rows.sort(reverse=True)
            top = [
                {
                    'pid': pid,
                    'pct': round(pct, 1),
                    'state': state,
                    'uid': read_uid(pid),
                    'cmd': read_cmd(pid),
                }
                for pct, pid, state in rows[:TOPN]
            ]

            # ---- 被测服务端的 I/O 轴（新增）--------------------------------
            sut = None
            if args.pid_match:
                if sut_pid is None or sut_pid not in procs:
                    sut_pid = find_pid(args.pid_match)
                if sut_pid:
                    cur_io = read_io(sut_pid)
                    if prev_io and cur_io:
                        def mbps(k):
                            return round((cur_io.get(k, 0) - prev_io.get(k, 0)) / dt / 1048576.0, 3)
                        sut = {
                            'pid': sut_pid,
                            'read_MBps': mbps('rchar'),
                            'write_MBps': mbps('wchar'),
                            'read_disk_MBps': mbps('read_bytes'),
                            'write_disk_MBps': mbps('write_bytes'),
                        }
                    prev_io = cur_io

            rec = {
                'ts_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(now_t)),
                'system_busy_pct': round(sys_busy_pct, 1),
                'iowait_pct': round(sys_iowait_pct, 1),
                'load1': os.getloadavg()[0],
                'load5': os.getloadavg()[1],
                'n_ge25cpu': len(rows),
                'top': top,
                'sut': sut,
            }
            fh.write(json.dumps(rec, ensure_ascii=False) + '\n')

            prev_total, prev_idle, prev_iowait, prev_procs = total, idle, iowait, procs
            prev_t = now_t


if __name__ == '__main__':
    main()

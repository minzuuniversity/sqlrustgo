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
    """返回 (total_ticks, idle_ticks, {pid: (uid, busy_ticks)})。"""
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
            # rest[0]=state, rest[1]=ppid … rest[11]=utime, rest[12]=stime
            procs[pid] = (int(rest[11]) + int(rest[12]), raw[rp + 2:].split()[0])
        except Exception:
            pass
    return total, idle, procs


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
    args = ap.parse_args()

    prev_total, prev_idle, prev_procs = read_cpu()
    prev_t = time.time()

    with open(args.out, 'a', buffering=1) as fh:
        while True:
            time.sleep(args.interval)
            now_t = time.time()
            total, idle, procs = read_cpu()
            dt = now_t - prev_t

            sys_total = (total - prev_total) / HZ
            sys_idle = (idle - prev_idle) / HZ
            sys_busy_pct = ((sys_total - sys_idle) / dt * 100.0) if dt > 0 else 0.0

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

            rec = {
                'ts_utc': time.strftime('%Y-%m-%dT%H:%M:%SZ', time.gmtime(now_t)),
                'system_busy_pct': round(sys_busy_pct, 1),
                'load1': os.getloadavg()[0],
                'load5': os.getloadavg()[1],
                'n_ge25cpu': len(rows),
                'top': top,
            }
            fh.write(json.dumps(rec, ensure_ascii=False) + '\n')

            prev_total, prev_idle, prev_procs = total, idle, procs
            prev_t = now_t


if __name__ == '__main__':
    main()

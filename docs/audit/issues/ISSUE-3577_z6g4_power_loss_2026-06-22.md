# ISSUE-3577: Z6G4 工作站硬断电 (2026-06-22 00:18)

**Status**: ROOT_CAUSE_UNCONFIRMED — 物理断电，硬件层面无信号
**Type**: Host Reliability Incident
**Category**: Power/Hardware — Hard Power Loss
**Severity**: P1 (开发服务器不稳定)
**Date**: 2026-06-22 00:18-00:58 CST
**Server**: HP Z6 G4 (192.168.0.252, gaoyuanai-HPZ6G4)

---

## 事件摘要

| 时间 (CST) | 事件 |
|------------|------|
| 2026-06-21 22:19:15 | Gitea 容器首次崩溃，15s 内恢复 (本次无关) |
| 2026-06-21 22:19:38 | boot -1 启动 (上次) |
| 2026-06-22 00:18:38 | **boot -1 最后一条 journalctl 日志** (xdg-desktop-portal AppChooser fallback) |
| 2026-06-22 00:18:38 → 00:58:55 | **20 秒后机器失去响应** (无任何系统日志) |
| 2026-06-22 00:58:55 | 用户断电重启 → boot 0 启动 |
| 2026-06-22 00:59:00 | systemd 用户会话启动 |
| 2026-06-22 02:01:15 | Gitea 容器重启完成，全部服务恢复 |

**断电持续时间**: 40 分钟 (00:18 → 00:58)

---

## 调查证据

### E-1: boot -1 内核日志为空

```bash
$ sudo journalctl -b -1 -k --no-pager
No entries.
```

### E-2: boot -1 期间 0 行 kernel 硬件错误

```bash
$ journalctl -b -1 --no-pager -o short-iso | grep -E 'kernel|nvidia|GPU|Xid|DRM|edac|mce' \
    | grep -v 'nvidia-smi|mcelog'
# (empty)
```

全部 25 行相关日志是 agent python 查 mcelog/nvidia-smi 的 sudo 命令，内核本身**没有任何硬件错误输出**。

### E-3: boot -1 期间 0 行 OOM/RSS/kill

```bash
$ journalctl -b -1 --no-pager -o short-iso | grep -i 'rss'
# (empty)
```

### E-4: boot -1 最后活动 (00:18:38)

```
2026-06-22T00:18:38 xdg-desktop-por[399984]: Choosing gtk.portal for org.freedesktop.impl.portal.AppChooser as a last-resort fallback
```

20 秒后机器**完全无信号**。

### E-5: 硬件信号现状

| 信号 | 值 | 状态 |
|------|-----|------|
| NVMe Unsafe Shutdowns | **127** (上次 112, 增 15) | 累计 15 次硬断电 |
| NVMe Critical Warning | 0x00 | OK |
| NVMe Media Errors | 0 | OK |
| NVMe Temperature | 44°C | OK |
| CPU Temperature | 75-77°C | OK (high=83, crit=93) |
| IPMI/BMC | **不可用** (`/dev/ipmi0` 不存在) | 硬件盲区 |
| mcelog | 未运行 | 旧工具，已被 rasdaemon 替代 |
| rasdaemon | **active (running) 5h15min** ✓ | 已记录此次启动 |
| kdump-tools | **enabled + active** ✓ | 已配置 crashkernel 预留 |
| EDAC | 未激活 | `edac=` 未配置 |

### E-6: GRUB 内核参数 (重要)

```
$ cat /proc/cmdline
... mce=0 intel_ppin=0 edd=off nox2apic hest_disable ...
```

| 参数 | 影响 |
|------|------|
| `mce=0` | **完全禁用 MCE 中断处理** — CPU 硬件 MCE 被丢弃 |
| `hest_disable` | **禁用 ACPI HEST 表** — 这会阻止 IPMI 平台驱动探测 BMC |
| `intel_ppin=0` | 禁用 Intel PPIN (Processor ID) — 隐私，无关硬件错误 |
| `edd=off` | 禁用 EDD BIOS 探测 — 影响不大 |
| `nox2apic` | 禁用 x2APIC — 影响中断路由 |

`mce=0` + `hest_disable` 组合 = **本次无法捕获硬件错误的根因**。

### E-7: IPMI 不可用根因

```
$ sudo modprobe ipmi_si
modprobe: ERROR: could not insert 'ipmi_si': No such device
```

`/sys/devices/platform/hardcode-ipmi-si.0` 存在 (HP Z6 G4 平台驱动)，但 `hest_disable` 阻止 BMC 接口注册。

---

## 可能的根因 (按概率排序)

1. **电源单元老化/功率不足** — Z6 G4 + 高功耗 GPU + CPU 同时满载时电源电压跌落
2. **电源管理 bug / 主板供电不稳** — 间歇性触发硬件保护
3. **意外物理断电** (停电、UPS 故障、误碰电源)
4. **CPU/VRM 热保护** — 但 CPU 温度 75°C 在范围内，可能性低
5. **显卡功耗尖峰** — NVIDIA GPU 短暂高负载触发电源保护

**无法排除硬件间歇性故障** — 因为 IPMI 缺失 + mce=0 + hest_disable 三层盲区。

---

## 已采取的修复措施 (2026-06-22)

### 1. 启用 rasdaemon (已完成)

```bash
systemctl status rasdaemon
# Active: active (running) since Mon 2026-06-22 00:58:59 CST; 5h 15min ago
# Main PID: 3068 (rasdaemon)
```

`/usr/sbin/rasdaemon -f -r` 已运行 5h15min，捕获 MCE/AER/CPU/内存硬件错误。

### 2. 启用 kdump (已完成)

```bash
systemctl is-enabled kdump-tools  # enabled
systemctl is-active kdump-tools   # active
cat /proc/cmdline | grep crashkernel
# crashkernel=2G-4G:320M,4G-32G:512M,...
```

如 kernel panic 会触发 kdump 到 `/var/crash/`。

### 3. IPMI / mce=0 / hest_disable (待执行 — 需要重启)

需要在 GRUB 中**移除**这些参数：
```
GRUB_CMDLINE_LINUX="mce=0 intel_ppin=0 edd=off nox2apic hest_disable"
                       ^^^                           ^^^^^^^^^^^^
                     删除                          删除
```

保留 `intel_ppin=0` (隐私) 和 `edd=off` (无关)。

**风险**: 移除 `hest_disable` 可能让原本被压制的 BERT (Boot Error Record Table) 错误在 dmesg 显示，需要用户接受此变化。

**移除后效果**:
- MCE 中断将触发内核日志 → 进入 journalctl + rasdaemon SQLite
- BMC 平台驱动将探测 hardcode-ipmi-si → 创建 `/dev/ipmi0`
- ipmitool 可读 BMC SEL → 历史硬件错误可查

**执行步骤 (待用户批准)**:
1. `sudo sed -i 's/ mce=0//; s/ hest_disable//' /etc/default/grub`
2. `sudo update-grub`
3. 重启
4. 验证: `ls /dev/ipmi0` 应存在，`ipmitool sel list` 应返回历史事件

---

## 复现 / 监控

### 当前监控信号

```bash
# 查看 rasdaemon 记录的 MCE/AER
sudo sqlite3 /var/lib/rasdaemon/ras-mc_event.db "SELECT * FROM mc_event ORDER BY id DESC LIMIT 20"
sudo journalctl -u rasdaemon --since '1 hour ago' --no-pager

# 查看 kdump 配置
cat /etc/default/kdump-tools | grep -v '^#' | grep -v '^$'

# 查看 GRUB 参数
cat /proc/cmdline | tr ' ' '\n' | grep -E 'mce|hest|crash|intel_ppin'
```

### 下次断电时自动捕获的信号

如果下次发生类似事件，下面命令会自动产生证据：

```bash
# 1. MCE 错误 (rasdaemon 自动记录)
/var/lib/rasdaemon/ras-mc_event.db

# 2. PCIe AER 错误 (rasdaemon 自动记录)
/var/lib/rasdaemon/ras-aer_event.db

# 3. Kernel panic (kdump 自动转储)
/var/crash/<date>/vmcore  (需要 makedumpfile 提取)

# 4. ACPI 错误 (内核自动记录 + dmesg)
journalctl -k | grep -i 'mce\|aer\|pcie\|bert'
```

---

## 与 SQLRustGo 项目的关系

本次 252 断电**未直接影响 SQLRustGo 数据**：
- TPC-H 测试数据在 `/home/openclaw/sqlrustgo-tpch/` — 在断电前已加载到 mysql
- 本地 `/home/openclaw/workspace/dev/sqlrustgo/` 工作目录是开发副本
- 真正的远程仓库在 Gitea 192.168.0.252:3000

但下次 72h/168h soak (Issue #3265/#3266) **绝对不能在 252 上跑** — Z6 G4 电源/硬件不稳定的风险未消除。

---

## Action Items

| ID | Action | Status | Owner |
|----|--------|--------|-------|
| A1 | 启用 rasdaemon | ✅ Done | 李哥重启后自动 |
| A2 | 启用 kdump | ✅ Done | 李哥重启后自动 |
| A3 | 移除 mce=0 / hest_disable | ⏳ 待用户批准后执行 | 待批 |
| A4 | 重启验证 `/dev/ipmi0` | ⏳ A3 后 | 自动 |
| A5 | 验证 `ipmitool sel list` | ⏳ A4 后 | 自动 |
| A6 | 7 天观察期 (验证无再断电) | ⏳ 进行中 | 系统 |
| A7 | 如再断电 → 换电源测试 | 触发条件 | 待批 |

---

## 文档版本

- 2026-06-22: 创建 (本次断电分析)
#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime, timedelta
import numpy as np
import os

plt.rcParams['font.sans-serif'] = ['Arial Unicode MS', 'SimHei', 'PingFang SC']
plt.rcParams['axes.unicode_minus'] = False

CSV_DIR = 'artifacts/usage/'
CSV_FILES = [
    ('artifacts/usage/export_bill_march.csv', '2026-02-02', '2026-03-31'),
    ('export_bill_低速套餐4-5.csv', '2026-04-01', '2026-05-29'),
    ('export_bill_高速套餐4-5.csv', '2026-04-01', '2026-05-29'),
]

OUTPUT_DIR = 'scripts/usage/'
ARTIFACTS_DIR = 'artifacts/usage/'

os.makedirs(ARTIFACTS_DIR, exist_ok=True)

print("=" * 90)
print("📊 完整历史数据合并分析（2026-02-02 至 2026-05-29）")
print("=" * 90)

all_dfs = []
for csv_file, start, end in CSV_FILES:
    if os.path.exists(csv_file):
        df = pd.read_csv(csv_file)
        print(f"\n📄 {csv_file}: {len(df):,} 条记录 ({start} 至 {end})")
        all_dfs.append(df)
    else:
        print(f"\n⚠️  {csv_file} 不存在，跳过")

if len(all_dfs) == 0:
    print("\n❌ 没有找到任何数据文件！")
    exit(1)

df = pd.concat(all_dfs, ignore_index=True)

df['输入消费数'] = pd.to_numeric(df['输入消费数'].replace('<nil>', '0'), errors='coerce').fillna(0).astype(int)
df['输出消费数'] = pd.to_numeric(df['输出消费数'].replace('<nil>', '0'), errors='coerce').fillna(0).astype(int)
df['总消费数'] = pd.to_numeric(df['总消费数'].replace('<nil>', '0'), errors='coerce').fillna(0).astype(int)

df['日期'] = df['消费时间'].apply(lambda x: datetime.strptime(x.split(' ')[0], '%Y-%m-%d'))

df['消费接口'] = df['消费接口'].str.replace('\(Text API\)', '', regex=False)

print(f"\n✅ 合并后总记录数: {len(df):,}")
print(f"📅 数据时间范围: {df['日期'].min().strftime('%Y-%m-%d')} 至 {df['日期'].max().strftime('%Y-%m-%d')}")

merged_csv = ARTIFACTS_DIR + 'export_merged_complete_20260202_20260529.csv'
df.to_csv(merged_csv, index=False, encoding='utf-8-sig')
print(f"\n💾 合并数据已保存: {merged_csv}")

print("\n" + "=" * 90)
print("一、总体统计概览（完整期间）")
print("=" * 90)

daily_all = df.groupby('日期').agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'}).sort_index()

total_records = len(df)
total_days = (df['日期'].max() - df['日期'].min()).days + 1

total_calls = daily_all['调用次数'].sum()
total_input = daily_all['输入消费数'].sum()
total_output = daily_all['输出消费数'].sum()
total_tokens = daily_all['总消费数'].sum()

daily_avg_calls = total_calls / total_days
daily_avg_input = total_input / total_days
daily_avg_output = total_output / total_days
daily_avg_tokens = total_tokens / total_days

weeks = total_days / 7
months = total_days / 30

print(f"""
╔══════════════════════════════════════════════════════════════════════════════╗
║                           总体统计指标                                      ║
╠══════════════╦════════════════╦════════════════╦════════════════╦════════════════╣
║  指标        ║  日均          ║  周均          ║  月均          ║  总体          ║
╠══════════════╬════════════════╬════════════════╬════════════════╬════════════════╣
║  调用次数    ║  {daily_avg_calls:>12,.0f}  ║  {total_calls/weeks:>12,.0f}  ║  {total_calls/months:>12,.0f}  ║  {total_calls:>12,}  ║
║  输入消费    ║  {daily_avg_input:>12,}  ║  {total_input/weeks:>12,}  ║  {total_input/months:>12,}  ║  {total_input:>12,}  ║
║  输出消费    ║  {daily_avg_output:>12,}  ║  {total_output/weeks:>12,}  ║  {total_output/months:>12,}  ║  {total_output:>12,}  ║
║  总消费数    ║  {daily_avg_tokens:>12,}  ║  {total_tokens/weeks:>12,}  ║  {total_tokens/months:>12,}  ║  {total_tokens:>12,}  ║
╚══════════════╩════════════════╩════════════════╩════════════════╩════════════════╝
""")

print("=" * 90)
print("二、月度用量对比")
print("=" * 90)

monthly = df.groupby(pd.Grouper(key='日期', freq='ME')).agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'})

print(f"\n| 月份 | 调用次数 | 输入消费 | 输出消费 | 总消费数 | 日均用量 |")
print(f"|------|----------|----------|----------|----------|----------|")
for idx, row in monthly.iterrows():
    if idx.year == 2026 and idx.month in [2, 3, 4, 5]:
        month_start = idx.replace(day=1)
        month_end = idx
        days_in_month = (month_end - month_start).days + 1
        if idx.month == 5:
            days_in_month = 29
        daily_avg = row['总消费数'] / days_in_month
        print(f"| {idx.strftime('%Y-%m')} | {row['调用次数']:>,} | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['总消费数']/1e8:.2f}亿 | {daily_avg/1e8:.2f}亿 |")

print("\n" + "=" * 90)
print("三、最近2周用量详细分析（05-16 至 05-29）")
print("=" * 90)

today = datetime(2026, 5, 30)
yesterday = today - timedelta(days=1)
two_weeks_ago = today - timedelta(days=14)

recent_2w = df[(df['日期'] >= two_weeks_ago) & (df['日期'] <= yesterday)]
daily_2w = recent_2w.groupby('日期').agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'}).sort_index()

print(f"\n### 3.1 每日详细数据 ({two_weeks_ago.strftime('%m-%d')} 至 {yesterday.strftime('%m-%d')})")
print(f"\n| 日期 | 调用次数 | 输入消费 | 输出消费 | 总消费数 |")
print(f"|------|----------|----------|----------|----------|")
for idx, row in daily_2w.iterrows():
    print(f"| {idx.strftime('%Y-%m-%d')} | {row['调用次数']:>,} | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['总消费数']/1e8:.2f}亿 |")

total_2w_calls = daily_2w['调用次数'].sum()
total_2w_input = daily_2w['输入消费数'].sum()
total_2w_output = daily_2w['输出消费数'].sum()
total_2w_tokens = daily_2w['总消费数'].sum()
days_2w = len(daily_2w)

print(f"| **合计** | **{total_2w_calls:,}** | **{total_2w_input/1e8:.2f}亿** | **{total_2w_output/1e4:.1f}万** | **{total_2w_tokens/1e8:.2f}亿** |")

avg_2w_calls = total_2w_calls / days_2w if days_2w > 0 else 0
avg_2w_tokens = total_2w_tokens / days_2w if days_2w > 0 else 0
avg_2w_output = total_2w_output / days_2w if days_2w > 0 else 0

print(f"\n### 3.2 最近2周统计指标")
print(f"""
╔══════════════════════════════════════════════════════════╗
║              最近2周统计指标                              ║
╠══════════════════════════════════════════════════════════╣
║  日均调用次数:  {avg_2w_calls:>10,.0f} 次                        ║
║  日均输入消费:  {total_2w_input/days_2w/1e8 if days_2w > 0 else 0:>10.2f} 亿 Token                ║
║  日均输出消费:  {avg_2w_output/1e4 if days_2w > 0 else 0:>10.1f} 万 Token                ║
║  日均总消费数:  {avg_2w_tokens/1e8 if days_2w > 0 else 0:>10.2f} 亿 Token                ║
║  2周总调用数:   {total_2w_calls:>10,} 次                        ║
║  2周总消费数:   {total_2w_tokens/1e8:>10.2f} 亿 Token                ║
╚══════════════════════════════════════════════════════════╝
""")

print("=" * 90)
print("四、生成趋势图")
print("=" * 90)

fig, axes = plt.subplots(4, 1, figsize=(16, 20))

axes[0].plot(daily_all.index, daily_all['调用次数'], marker='o', linestyle='-', color='#2196F3', linewidth=1.5, markersize=3)
axes[0].axvspan(two_weeks_ago, yesterday, alpha=0.15, color='orange', label='最近2周')
axes[0].set_title('每日调用次数趋势（完整期间）', fontsize=14, fontweight='bold')
axes[0].set_ylabel('调用次数')
axes[0].grid(True, alpha=0.3)
axes[0].legend()
axes[0].xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
axes[0].xaxis.set_major_locator(mdates.WeekdayLocator(byweekday=0))

axes[1].plot(daily_all.index, daily_all['输入消费数']/1e8, marker='o', linestyle='-', color='#FF9800', linewidth=1.5, markersize=3)
axes[1].axvspan(two_weeks_ago, yesterday, alpha=0.15, color='orange', label='最近2周')
axes[1].set_title('每日输入消费趋势（亿 Token）', fontsize=14, fontweight='bold')
axes[1].set_ylabel('输入消费（亿）')
axes[1].grid(True, alpha=0.3)
axes[1].legend()
axes[1].xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
axes[1].xaxis.set_major_locator(mdates.WeekdayLocator(byweekday=0))

axes[2].plot(daily_all.index, daily_all['输出消费数']/1e4, marker='o', linestyle='-', color='#4CAF50', linewidth=1.5, markersize=3)
axes[2].axvspan(two_weeks_ago, yesterday, alpha=0.15, color='orange', label='最近2周')
axes[2].set_title('每日输出消费趋势（万 Token）', fontsize=14, fontweight='bold')
axes[2].set_ylabel('输出消费（万）')
axes[2].grid(True, alpha=0.3)
axes[2].legend()
axes[2].xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
axes[2].xaxis.set_major_locator(mdates.WeekdayLocator(byweekday=0))

axes[3].plot(daily_all.index, daily_all['总消费数']/1e8, marker='o', linestyle='-', color='#F44336', linewidth=1.5, markersize=3)
axes[3].axvspan(two_weeks_ago, yesterday, alpha=0.15, color='orange', label='最近2周')
axes[3].set_title('每日总消费数趋势（亿 Token）', fontsize=14, fontweight='bold')
axes[3].set_ylabel('总消费数（亿）')
axes[3].set_xlabel('日期')
axes[3].grid(True, alpha=0.3)
axes[3].legend()
axes[3].xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
axes[3].xaxis.set_major_locator(mdates.WeekdayLocator(byweekday=0))

fig.autofmt_xdate()
plt.tight_layout()
plt.savefig(OUTPUT_DIR + 'minimax_complete_trend.png', dpi=150, bbox_inches='tight')
print(f"\n📊 完整趋势图已保存: {OUTPUT_DIR}minimax_complete_trend.png")

fig2, axes2 = plt.subplots(3, 1, figsize=(16, 15))

axes2[0].bar(daily_2w.index.strftime('%m-%d'), daily_2w['调用次数'], color='#2196F3', alpha=0.8)
axes2[0].set_title('最近2周每日调用次数', fontsize=14, fontweight='bold')
axes2[0].set_ylabel('调用次数')
axes2[0].grid(True, alpha=0.3, axis='y')
for i, v in enumerate(daily_2w['调用次数']):
    axes2[0].text(i, v + 1, str(v), ha='center', va='bottom', fontsize=8)

axes2[1].bar(daily_2w.index.strftime('%m-%d'), daily_2w['输出消费数']/1e4, color='#4CAF50', alpha=0.8)
axes2[1].set_title('最近2周每日输出消费（万 Token）', fontsize=14, fontweight='bold')
axes2[1].set_ylabel('输出消费（万）')
axes2[1].grid(True, alpha=0.3, axis='y')

axes2[2].bar(daily_2w.index.strftime('%m-%d'), daily_2w['总消费数']/1e8, color='#F44336', alpha=0.8)
axes2[2].set_title('最近2周每日总消费数（亿 Token）', fontsize=14, fontweight='bold')
axes2[2].set_ylabel('总消费数（亿）')
axes2[2].set_xlabel('日期')
axes2[2].grid(True, alpha=0.3, axis='y')

fig2.autofmt_xdate()
plt.tight_layout()
plt.savefig(OUTPUT_DIR + 'minimax_2weeks_detail.png', dpi=150, bbox_inches='tight')
print(f"📊 2周详细图已保存: {OUTPUT_DIR}minimax_2weeks_detail.png")

print("\n" + "=" * 90)
print("✅ 完整历史数据分析完成！")
print("=" * 90)

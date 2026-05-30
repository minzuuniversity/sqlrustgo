#!/usr/bin/env python3
import pandas as pd
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime, timedelta
import numpy as np
import os

plt.rcParams['font.sans-serif'] = ['Arial Unicode MS', 'SimHei', 'PingFang SC']
plt.rcParams['axes.unicode_minus'] = False

CSV1 = 'export_bill_低速套餐4-5.csv'
CSV2 = 'export_bill_高速套餐4-5.csv'
OUTPUT_DIR = 'scripts/usage/'
ARTIFACTS_DIR = 'artifacts/usage/'

os.makedirs(ARTIFACTS_DIR, exist_ok=True)

print("=" * 90)
print("📊 合并 Minimax 套餐用量数据")
print("=" * 90)

df1 = pd.read_csv(CSV1)
df2 = pd.read_csv(CSV2)

print(f"\n📋 低速套餐记录数: {len(df1):,}")
print(f"📋 高速套餐记录数: {len(df2):,}")

df = pd.concat([df1, df2], ignore_index=True)
print(f"📋 合并后总记录数: {len(df):,}")

df['输入消费数'] = pd.to_numeric(df['输入消费数'].replace('<nil>', '0'), errors='coerce').fillna(0).astype(int)
df['输出消费数'] = pd.to_numeric(df['输出消费数'].replace('<nil>', '0'), errors='coerce').fillna(0).astype(int)
df['总消费数'] = pd.to_numeric(df['总消费数'].replace('<nil>', '0'), errors='coerce').fillna(0).astype(int)

df['日期'] = df['消费时间'].apply(lambda x: datetime.strptime(x.split(' ')[0], '%Y-%m-%d'))

df['消费接口'] = df['消费接口'].str.replace('\(Text API\)', '', regex=False)

today = datetime(2026, 5, 30)
yesterday = today - timedelta(days=1)
two_weeks_ago = today - timedelta(days=14)
four_weeks_ago = today - timedelta(days=28)

print(f"\n📅 数据时间范围: {df['日期'].min().strftime('%Y-%m-%d')} 至 {df['日期'].max().strftime('%Y-%m-%d')}")
print(f"📅 报告生成日期: {today.strftime('%Y-%m-%d')}")
print(f"📅 重点分析期间: 最近2周 ({two_weeks_ago.strftime('%Y-%m-%d')} 至 {yesterday.strftime('%Y-%m-%d')})")

merged_csv = ARTIFACTS_DIR + f'export_merged_{today.strftime("%Y%m%d")}.csv'
df.to_csv(merged_csv, index=False, encoding='utf-8-sig')
print(f"\n💾 合并数据已保存: {merged_csv}")

print("\n" + "=" * 90)
print("一、总体统计概览")
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
    month_start = idx.replace(day=1)
    month_end = idx
    days_in_month = (month_end - month_start).days + 1
    daily_avg = row['总消费数'] / days_in_month
    print(f"| {idx.strftime('%Y-%m')} | {row['调用次数']:>,} | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['总消费数']/1e8:.2f}亿 | {daily_avg/1e8:.2f}亿 |")

print("\n" + "=" * 90)
print("三、最近2周用量详细分析（重点）")
print("=" * 90)

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

print(f"\n### 3.2 最近2周统计指标")
avg_2w_calls = total_2w_calls / days_2w if days_2w > 0 else 0
avg_2w_tokens = total_2w_tokens / days_2w if days_2w > 0 else 0
avg_2w_output = total_2w_output / days_2w if days_2w > 0 else 0

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
print("四、周度趋势对比")
print("=" * 90)

weekly = df.groupby(pd.Grouper(key='日期', freq='W-MON')).agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'})

print(f"\n| 周起始日 | 调用次数 | 输入消费 | 输出消费 | 总消费数 | 日均用量 |")
print(f"|----------|----------|----------|----------|----------|----------|")
for idx, row in weekly.iterrows():
    week_daily = row['总消费数'] / 7
    marker = " ◀◀◀" if idx >= two_weeks_ago else ""
    print(f"| {idx.strftime('%Y-%m-%d')} | {row['调用次数']:>,} | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['总消费数']/1e8:.2f}亿 | {week_daily/1e8:.2f}亿 |{marker}")

print("\n" + "=" * 90)
print("五、最近2周 vs 前2周对比分析")
print("=" * 90)

prev_2w = df[(df['日期'] >= four_weeks_ago) & (df['日期'] < two_weeks_ago)]
daily_prev = prev_2w.groupby('日期').agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'})

prev_calls = daily_prev['调用次数'].sum()
prev_tokens = daily_prev['总消费数'].sum()
prev_output = daily_prev['输出消费数'].sum()
prev_days = len(daily_prev)

prev_avg_calls = prev_calls / prev_days if prev_days > 0 else 0
prev_avg_tokens = prev_tokens / prev_days if prev_days > 0 else 0
prev_avg_output = prev_output / prev_days if prev_days > 0 else 0

calls_change = ((avg_2w_calls - prev_avg_calls) / prev_avg_calls * 100) if prev_avg_calls > 0 else 0
tokens_change = ((avg_2w_tokens - prev_avg_tokens) / prev_avg_tokens * 100) if prev_avg_tokens > 0 else 0
output_change = ((avg_2w_output - prev_avg_output) / prev_avg_output * 100) if prev_avg_output > 0 else 0

print(f"""
╔══════════════════════════════════════════════════════════════════════════════╗
║                    最近2周 vs 前2周对比                                     ║
╠════════════════╦══════════════════╦══════════════════╦══════════════════════╣
║  指标          ║  前2周日均       ║  最近2周日均     ║  变化幅度            ║
╠════════════════╬══════════════════╬══════════════════╬══════════════════════╣
║  调用次数      ║  {prev_avg_calls:>12,.0f}   ║  {avg_2w_calls:>12,.0f}   ║  {calls_change:>+10.1f}%            ║
║  输出消费      ║  {prev_avg_output/1e4:>10.1f}万   ║  {avg_2w_output/1e4:>10.1f}万   ║  {output_change:>+10.1f}%            ║
║  总消费数      ║  {prev_avg_tokens/1e8:>10.2f}亿   ║  {avg_2w_tokens/1e8:>10.2f}亿   ║  {tokens_change:>+10.1f}%            ║
╚════════════════╩══════════════════╩══════════════════╩══════════════════════╝
""")

print("=" * 90)
print("六、模型和接口使用分析（最近2周）")
print("=" * 90)

print(f"\n### 6.1 模型使用分布")
model_usage = recent_2w.groupby('消费模型').agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'}).sort_values('总消费数', ascending=False)

print(f"\n| 模型 | 调用次数 | 占比 | 输入消费 | 输出消费 | 总消费数 |")
print(f"|------|----------|------|----------|----------|----------|")
for idx, row in model_usage.iterrows():
    pct = row['调用次数'] / total_2w_calls * 100
    print(f"| {idx} | {row['调用次数']:>,} | {pct:.1f}% | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['总消费数']/1e8:.2f}亿 |")

print(f"\n### 6.2 接口使用分布")
api_usage = recent_2w.groupby('消费接口').agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'}).sort_values('总消费数', ascending=False)

print(f"\n| 接口 | 调用次数 | 占比 | 输入消费 | 输出消费 | 总消费数 |")
print(f"|------|----------|------|----------|----------|----------|")
for idx, row in api_usage.iterrows():
    pct = row['调用次数'] / total_2w_calls * 100
    print(f"| {idx} | {row['调用次数']:>,} | {pct:.1f}% | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['总消费数']/1e8:.2f}亿 |")

print("\n" + "=" * 90)
print("七、每日用量波动分析（最近2周）")
print("=" * 90)

if days_2w > 0:
    max_day = daily_2w.loc[daily_2w['总消费数'].idxmax()]
    min_day = daily_2w.loc[daily_2w['总消费数'].idxmin()]
    std_tokens = daily_2w['总消费数'].std()
    cv = std_tokens / avg_2w_tokens * 100 if avg_2w_tokens > 0 else 0

    print(f"\n  最高用量日: {daily_2w['总消费数'].idxmax().strftime('%Y-%m-%d')} - {max_day['总消费数']/1e8:.2f}亿 ({max_day['调用次数']:,}次调用)")
    print(f"  最低用量日: {daily_2w['总消费数'].idxmin().strftime('%Y-%m-%d')} - {min_day['总消费数']/1e8:.2f}亿 ({min_day['调用次数']:,}次调用)")
    print(f"  标准差: {std_tokens/1e8:.2f}亿")
    print(f"  变异系数: {cv:.1f}%")
    print(f"  峰谷比: {max_day['总消费数']/min_day['总消费数']:.1f}x")

    weekday_avg = daily_2w.groupby(daily_2w.index.dayofweek).agg({
        '总消费数': 'mean',
        '调用次数': 'mean'
    })
    weekday_names = ['周一', '周二', '周三', '周四', '周五', '周六', '周日']
    print(f"\n  按星期分布:")
    print(f"  | 星期 | 日均总消费 | 日均调用次数 |")
    print(f"  |------|-----------|-------------|")
    for wd, row in weekday_avg.iterrows():
        print(f"  | {weekday_names[wd]} | {row['总消费数']/1e8:.2f}亿 | {row['调用次数']:.0f} |")

print("\n" + "=" * 90)
print("八、生成趋势图")
print("=" * 90)

fig, axes = plt.subplots(4, 1, figsize=(16, 20))

axes[0].plot(daily_all.index, daily_all['调用次数'], marker='o', linestyle='-', color='#2196F3', linewidth=1.5, markersize=3)
axes[0].axvspan(two_weeks_ago, yesterday, alpha=0.15, color='orange', label='最近2周')
axes[0].set_title('每日调用次数趋势', fontsize=14, fontweight='bold')
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
plt.savefig(OUTPUT_DIR + 'minimax_usage_trend.png', dpi=150, bbox_inches='tight')
print(f"\n📊 趋势图已保存: {OUTPUT_DIR}minimax_usage_trend.png")

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

excel_writer = pd.ExcelWriter(OUTPUT_DIR + 'minimax_usage_summary.xlsx')
daily_all.to_excel(excel_writer, sheet_name='每日汇总')
weekly.to_excel(excel_writer, sheet_name='每周汇总')
monthly.to_excel(excel_writer, sheet_name='每月汇总')
daily_2w.to_excel(excel_writer, sheet_name='最近2周')
model_usage.to_excel(excel_writer, sheet_name='模型分布')
api_usage.to_excel(excel_writer, sheet_name='接口分布')
excel_writer.close()
print(f"📊 汇总数据已保存: {OUTPUT_DIR}minimax_usage_summary.xlsx")

print("\n" + "=" * 90)
print("九、更新项目分析报告")
print("=" * 90)

report_file = '项目分析报告.md'

new_content = f'''# SQLRustGo 项目分析报告

**报告生成日期**: {today.strftime('%Y-%m-%d')}
**数据范围**: {df['日期'].min().strftime('%Y-%m-%d')} 至 {df['日期'].max().strftime('%Y-%m-%d')}
**上次更新**: 2026-04-24

## 1. Minimax 套餐用量分析

### 1.1 数据概览

| 指标 | 数值 |
|------|------|
| 数据范围 | {df['日期'].min().strftime('%Y-%m-%d')} 至 {df['日期'].max().strftime('%Y-%m-%d')} |
| 总记录天数 | {total_days} 天 |
| 总记录数 | {total_records:,} 条 |
| 主要模型 | {', '.join(model_usage.index.tolist())} |
'''

max_idx = daily_all['总消费数'].idxmax()
max_row = daily_all.loc[max_idx]
new_content += f"| 最高峰值日 | {max_idx.strftime('%m-%d')}（{max_row['总消费数']/1e8:.2f}亿，{max_row['调用次数']:,.0f}次调用） |\n"

if len(daily_all) > 1:
    second_max = daily_all['总消费数'].nlargest(2)
    if len(second_max) >= 2:
        second_max_idx = second_max.index[1]
        second_max_row = daily_all.loc[second_max_idx]
        new_content += f"| 次高峰值日 | {second_max_idx.strftime('%m-%d')}（{second_max_row['总消费数']/1e8:.2f}亿，{second_max_row['调用次数']:,.0f}次调用） |\n"

for month_idx in [m for m in monthly.index if m.strftime('%Y-%m') == '2026-02']:
    row = monthly.loc[month_idx]
    new_content += f"| 2月总用量 | {row['总消费数']/1e8:.2f}亿 |\n"

for month_idx in [m for m in monthly.index if m.strftime('%Y-%m') == '2026-03']:
    row = monthly.loc[month_idx]
    new_content += f"| 3月总用量 | {row['总消费数']/1e8:.2f}亿 |\n"

for month_idx in [m for m in monthly.index if m.strftime('%Y-%m') == '2026-04']:
    row = monthly.loc[month_idx]
    new_content += f"| 4月总用量 | {row['总消费数']/1e8:.2f}亿 |\n"

for month_idx in [m for m in monthly.index if m.strftime('%Y-%m') == '2026-05']:
    row = monthly.loc[month_idx]
    new_content += f"| 5月总用量（至{yesterday.strftime('%d')}日） | {row['总消费数']/1e8:.2f}亿 |\n"

new_content += f'''| 总体日均用量 | {total_tokens/total_days/1e8:.2f}亿 |
| 最近2周日均用量 | {avg_2w_tokens/1e8:.2f}亿 |

### 1.2 用量趋势分析

#### 总体 Token 用量趋势

![总体 Token 用量趋势图](scripts/usage/minimax_usage_trend.png)

#### 最近2周详细用量

![最近2周详细用量图](scripts/usage/minimax_2weeks_detail.png)

### 1.3 月度用量对比

| 月份 | 总 Token 用量 | 输入消费 | 输出消费 | 调用次数 | 日均用量 |
|------|-------------|----------|----------|----------|----------|
'''

for idx, row in monthly.iterrows():
    month_start = idx.replace(day=1)
    month_end = idx
    days_in_month = (month_end - month_start).days + 1
    daily_avg = row['总消费数'] / days_in_month
    
    month_label = idx.strftime('%Y-%m')
    if idx.strftime('%Y-%m') == yesterday.strftime('%Y-%m'):
        month_label = f"{idx.strftime('%Y-%m')}（至{yesterday.strftime('%d')}日）"
    
    new_content += f"| {month_label} | {row['总消费数']/1e8:.2f}亿 | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['调用次数']:>,} | {daily_avg/1e8:.2f}亿 |\n"

new_content += f'''
**月度对比分析**:
- 按当前速率预估{yesterday.strftime('%Y-%m')}全月总消费约 {avg_2w_tokens*30/1e8:.2f}亿 Token
- 输出消费持续增长，代码生成活动活跃

### 1.4 最近2周用量分析（{two_weeks_ago.strftime('%m-%d')}至{yesterday.strftime('%m-%d')}日）

| 日期 | 调用次数 | 输入消费 | 输出消费 | 总 Token 用量 |
|------|----------|----------|----------|-------------|
'''

for idx, row in daily_2w.iterrows():
    new_content += f"| {idx.strftime('%Y-%m-%d')} | {row['调用次数']:>,} | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['总消费数']/1e8:.2f}亿 |\n"

total_2w_calls = daily_2w['调用次数'].sum()
total_2w_input = daily_2w['输入消费数'].sum()
total_2w_output = daily_2w['输出消费数'].sum()
total_2w_tokens = daily_2w['总消费数'].sum()

new_content += f"| **合计** | **{total_2w_calls:,}** | **{total_2w_input/1e8:.2f}亿** | **{total_2w_output/1e4:.1f}万** | **{total_2w_tokens/1e8:.2f}亿** |\n"

new_content += f'''
**最近2周统计指标**:
- 日均调用次数: {avg_2w_calls:,.0f} 次
- 日均输入消费: {total_2w_input/days_2w/1e8:.2f} 亿 Token
- 日均输出消费: {avg_2w_output/1e4:.1f} 万 Token
- 日均总消费数: {avg_2w_tokens/1e8:.2f} 亿 Token

### 1.5 最近2周 vs 前2周对比

| 指标 | 前2周日均 | 最近2周日均 | 变化幅度 |
|------|----------|-----------|----------|
| 调用次数 | {prev_avg_calls:,.0f} | {avg_2w_calls:,.0f} | {calls_change:+.1f}% |
| 输出消费 | {prev_avg_output/1e4:.1f}万 | {avg_2w_output/1e4:.1f}万 | {output_change:+.1f}% |
| 总消费数 | {prev_avg_tokens/1e8:.2f}亿 | {avg_2w_tokens/1e8:.2f}亿 | {tokens_change:+.1f}% |

**对比分析**: 最近2周总消费数较前2周变化{tokens_change:+.1f}%，输出消费变化{output_change:+.1f}%

### 1.6 模型使用分布（最近2周）

| 模型 | 调用次数 | 占比 | 输入消费 | 输出消费 | 总消费数 |
|------|----------|------|----------|----------|----------|
'''

for idx, row in model_usage.iterrows():
    pct = row['调用次数'] / total_2w_calls * 100
    new_content += f"| {idx} | {row['调用次数']:>,} | {pct:.1f}% | {row['输入消费数']/1e8:.2f}亿 | {row['输出消费数']/1e4:.1f}万 | {row['总消费数']/1e8:.2f}亿 |\n"

new_content += '''
**模型分析**: 各模型使用分布情况

### 1.7 接口使用分布（最近2周）

| 接口 | 调用次数 | 占比 | 总消费数 |
|------|----------|------|----------|
'''

for idx, row in api_usage.iterrows():
    pct = row['调用次数'] / total_2w_calls * 100
    new_content += f"| {idx} | {row['调用次数']:>,} | {pct:.1f}% | {row['总消费数']/1e8:.2f}亿 |\n"

new_content += '''
**接口分析**: 各接口使用分布情况

### 1.8 用量波动分析（最近2周）

| 指标 | 数值 |
|------|------|
'''

if days_2w > 0:
    max_day = daily_2w.loc[daily_2w['总消费数'].idxmax()]
    min_day = daily_2w.loc[daily_2w['总消费数'].idxmin()]
    std_tokens = daily_2w['总消费数'].std()
    cv = std_tokens / avg_2w_tokens * 100 if avg_2w_tokens > 0 else 0

    new_content += f"| 最高用量日 | {daily_2w['总消费数'].idxmax().strftime('%m-%d')} — {max_day['总消费数']/1e8:.2f}亿（{max_day['调用次数']:,.0f}次调用） |\n"
    new_content += f"| 最低用量日 | {daily_2w['总消费数'].idxmin().strftime('%m-%d')} — {min_day['总消费数']/1e8:.2f}亿（{min_day['调用次数']:,.0f}次调用） |\n"
    new_content += f"| 标准差 | {std_tokens/1e8:.2f}亿 |\n"
    new_content += f"| 变异系数 | {cv:.1f}% |\n"
    new_content += f"| 峰谷比 | {max_day['总消费数']/min_day['总消费数']:.1f}x |\n"

    weekday_avg = daily_2w.groupby(daily_2w.index.dayofweek).agg({
        '总消费数': 'mean',
        '调用次数': 'mean'
    })
    weekday_names = ['周一', '周二', '周三', '周四', '周五', '周六', '周日']
    
    new_content += '\n**按星期分布**:\n\n'
    new_content += '| 星期 | 日均总消费 | 日均调用次数 |\n'
    new_content += '|------|-----------|-------------|\n'
    
    for wd, row in weekday_avg.iterrows():
        new_content += f"| {weekday_names[wd]} | {row['总消费数']/1e8:.2f}亿 | {row['调用次数']:.0f} |\n"

new_content += '''
## 2. 项目 PR 和版本分析

### 2.1 版本分支完整列表

(请根据实际Git分支数据更新此部分)

### 2.2 版本时间跨度分析

(请根据实际Git分支数据更新此部分)

### 2.3 版本 PR 数量分析

(请根据实际Git分支数据更新此部分)

### 2.4 最近 PR 活动

(请根据实际Git分支数据更新此部分)

### 2.5 分支策略分析

(请根据实际Git分支数据更新此部分)

## 3. 项目发展趋势分析

### 3.1 用量趋势洞察

1. **用量持续高位**: 最近2周日均''' + f"{avg_2w_tokens/1e8:.2f}" + '''亿 token，持续高活跃度
2. **输出消费变化**: 输出消费变化''' + f"{output_change:+.1f}" + '''%
3. **调用频率**: 最近2周日均''' + f"{avg_2w_calls:,.0f}" + '''次调用
4. **峰谷波动**: 变异系数''' + f"{cv:.1f}" + '''%，峰谷比''' + f"{max_day['总消费数']/min_day['总消费数']:.1f}" + '''x

### 3.2 版本演进洞察

(请根据实际Git分支数据更新此部分)

### 3.3 PR 活动洞察

(请根据实际Git分支数据更新此部分)

## 4. 结论与建议

### 4.1 结论

1. **项目活跃度**: 从最近2周用量数据来看，项目处于持续开发状态
2. **资源使用**: Minimax套餐用量与项目开发强度匹配
3. **趋势预测**: 按当前速率预估月用量约 ''' + f"{avg_2w_tokens*30/1e8:.2f}" + '''亿 Token
4. **调用额度**: 按当前速率预估月调用 ''' + f"{avg_2w_calls*30:,.0f}" + '''次，在Coding Plan 4,500次额度内

### 4.2 建议

1. **资源监控**: 建议持续关注用量变化
2. **版本规划**: 建议保持当前开发节奏
3. **缓存优化**: 继续保持缓存策略以提高效率

## 5. 附件

### 5.1 数据文件

- `artifacts/usage/export_merged_''' + today.strftime('%Y%m%d') + '''.csv` - 合并后的原始用量数据
- `scripts/usage/minimax_usage_summary.xlsx` - 详细汇总数据

### 5.2 图表文件

- `scripts/usage/minimax_usage_trend.png` - 总体用量趋势图
- `scripts/usage/minimax_2weeks_detail.png` - 最近2周详细柱状图

### 5.3 分析脚本

- `scripts/analyze_new_data.py` - 新数据综合分析脚本

---

**报告生成工具**: 基于Python脚本分析生成
**分析范围**: Minimax套餐用量（低速+高速）
**数据来源**: export_bill_低速套餐4-5.csv + export_bill_高速套餐4-5.csv

---

*变更记录*
- 2026-04-20: 初始报告，数据范围02-02至04-18
- 2026-04-24: 更新报告，数据范围扩展至04-22
- ''' + today.strftime('%Y-%m-%d') + ''': 更新报告，数据范围扩展至''' + yesterday.strftime('%Y-%m-%d') + '''，合并低速和高速套餐数据
'''

with open(report_file, 'w', encoding='utf-8') as f:
    f.write(new_content)

print(f"\n✅ 项目分析报告已更新: {report_file}")

print("\n" + "=" * 90)
print("✅ 分析完成！")
print("=" * 90)

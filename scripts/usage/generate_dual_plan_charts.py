#!/usr/bin/env python3
import pandas as pd
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt
import matplotlib.dates as mdates
from datetime import datetime, timedelta
import numpy as np
import os

plt.rcParams['font.sans-serif'] = ['Microsoft YaHei', 'SimHei', 'PingFang SC', 'Arial Unicode MS']
plt.rcParams['axes.unicode_minus'] = False
plt.rcParams['font.family'] = 'sans-serif'

BASE_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CSV_FILE_1 = os.path.join(BASE_DIR, 'export_bill_2026-05-19.csv')
CSV_FILE_2 = os.path.join(BASE_DIR, 'export_bill_1779192901.csv')
OUTPUT_DIR = os.path.join(BASE_DIR, 'scripts', 'usage') + os.sep

os.makedirs(OUTPUT_DIR, exist_ok=True)

def load_csv(path):
    df = pd.read_csv(path)
    for col in ['输入消费数', '输出消费数', '总消费数']:
        df[col] = pd.to_numeric(df[col].replace('<nil>', '0'), errors='coerce').fillna(0).astype(int)
    df['日期'] = df['消费时间'].apply(lambda x: datetime.strptime(x.split(' ')[0], '%Y-%m-%d'))
    return df

df1 = load_csv(CSV_FILE_1)
df2 = load_csv(CSV_FILE_2)

df1['套餐'] = '套餐1(highspeed)'
df2['套餐'] = '套餐2(M2.7)'

df_all = pd.concat([df1, df2], ignore_index=True)

today = datetime(2026, 5, 19)
four_weeks_ago = today - timedelta(days=28)

daily_all = df_all.groupby('日期').agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'}).sort_index()

daily_plan1 = df1.groupby('日期').agg({'总消费数': 'sum'}).rename(columns={'总消费数': '套餐1'})
daily_plan2 = df2.groupby('日期').agg({'总消费数': 'sum'}).rename(columns={'总消费数': '套餐2'})

daily_merged = daily_all.join(daily_plan1, how='left').join(daily_plan2, how='left')
daily_merged['套餐1'] = daily_merged['套餐1'].fillna(0)
daily_merged['套餐2'] = daily_merged['套餐2'].fillna(0)

recent_4w = df_all[(df_all['日期'] >= four_weeks_ago) & (df_all['日期'] < today)]
daily_4w = recent_4w.groupby('日期').agg({
    '输入消费数': 'sum',
    '输出消费数': 'sum',
    '总消费数': 'sum',
    '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'}).sort_index()

daily_4w_plan1 = df1[df1['日期'] >= four_weeks_ago].groupby('日期').agg({'总消费数': 'sum'}).rename(columns={'总消费数': '套餐1'})
daily_4w_plan2 = df2[df2['日期'] >= four_weeks_ago].groupby('日期').agg({'总消费数': 'sum'}).rename(columns={'总消费数': '套餐2'})
daily_4w_merged = daily_4w.join(daily_4w_plan1, how='left').join(daily_4w_plan2, how='left')
daily_4w_merged['套餐1'] = daily_4w_merged['套餐1'].fillna(0)
daily_4w_merged['套餐2'] = daily_4w_merged['套餐2'].fillna(0)

print("=" * 90)
print("Generating MiniMax Usage Trend Charts (Dual Plan Merged)")
print("=" * 90)
print(f"Plan 1 date range: {df1['日期'].min().strftime('%Y-%m-%d')} ~ {df1['日期'].max().strftime('%Y-%m-%d')}")
print(f"Plan 2 date range: {df2['日期'].min().strftime('%Y-%m-%d')} ~ {df2['日期'].max().strftime('%Y-%m-%d')}")
print(f"Total records: {len(df_all)}")

fig, axes = plt.subplots(4, 1, figsize=(18, 22))

axes[0].plot(daily_all.index, daily_all['调用次数'], marker='o', linestyle='-', color='#2196F3', linewidth=1.5, markersize=3)
axes[0].axvspan(four_weeks_ago, today, alpha=0.15, color='orange', label='最近4周')
axes[0].set_title('每日调用次数趋势（双套餐合并）', fontsize=14, fontweight='bold')
axes[0].set_ylabel('调用次数')
axes[0].grid(True, alpha=0.3)
axes[0].legend()
axes[0].xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
axes[0].xaxis.set_major_locator(mdates.WeekdayLocator(byweekday=0))

axes[1].plot(daily_all.index, daily_all['输入消费数']/1e8, marker='o', linestyle='-', color='#FF9800', linewidth=1.5, markersize=3)
axes[1].axvspan(four_weeks_ago, today, alpha=0.15, color='orange', label='最近4周')
axes[1].set_title('每日输入消费趋势（亿 Token，双套餐合并）', fontsize=14, fontweight='bold')
axes[1].set_ylabel('输入消费（亿）')
axes[1].grid(True, alpha=0.3)
axes[1].legend()
axes[1].xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
axes[1].xaxis.set_major_locator(mdates.WeekdayLocator(byweekday=0))

axes[2].plot(daily_all.index, daily_all['输出消费数']/1e4, marker='o', linestyle='-', color='#4CAF50', linewidth=1.5, markersize=3)
axes[2].axvspan(four_weeks_ago, today, alpha=0.15, color='orange', label='最近4周')
axes[2].set_title('每日输出消费趋势（万 Token，双套餐合并）', fontsize=14, fontweight='bold')
axes[2].set_ylabel('输出消费（万）')
axes[2].grid(True, alpha=0.3)
axes[2].legend()
axes[2].xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
axes[2].xaxis.set_major_locator(mdates.WeekdayLocator(byweekday=0))

axes[3].bar(daily_merged.index, daily_merged['套餐1']/1e8, color='#2196F3', alpha=0.8, label='套餐1(highspeed)', width=0.8)
axes[3].bar(daily_merged.index, daily_merged['套餐2']/1e8, bottom=daily_merged['套餐1']/1e8, color='#FF9800', alpha=0.8, label='套餐2(M2.7)', width=0.8)
axes[3].axvspan(four_weeks_ago, today, alpha=0.1, color='green', label='最近4周')
axes[3].set_title('每日总消费数趋势（亿 Token，双套餐堆叠）', fontsize=14, fontweight='bold')
axes[3].set_ylabel('总消费数（亿）')
axes[3].set_xlabel('日期')
axes[3].grid(True, alpha=0.3, axis='y')
axes[3].legend()
axes[3].xaxis.set_major_formatter(mdates.DateFormatter('%m-%d'))
axes[3].xaxis.set_major_locator(mdates.WeekdayLocator(byweekday=0))

fig.autofmt_xdate()
plt.tight_layout()
plt.savefig(OUTPUT_DIR + 'minimax_usage_trend.png', dpi=150, bbox_inches='tight')
print(f"Trend chart saved: {OUTPUT_DIR}minimax_usage_trend.png")

fig2, axes2 = plt.subplots(3, 1, figsize=(18, 18))

x_labels = [d.strftime('%m-%d') for d in daily_4w_merged.index]
x_pos = np.arange(len(x_labels))
bar_width = 0.35

axes2[0].bar(x_pos, daily_4w_merged['调用次数'], color='#2196F3', alpha=0.8)
axes2[0].set_title('最近4周每日调用次数（双套餐合并）', fontsize=14, fontweight='bold')
axes2[0].set_ylabel('调用次数')
axes2[0].set_xticks(x_pos)
axes2[0].set_xticklabels(x_labels, rotation=45, ha='right')
axes2[0].grid(True, alpha=0.3, axis='y')

axes2[1].bar(x_pos, daily_4w_merged['输出消费数']/1e4, color='#4CAF50', alpha=0.8)
axes2[1].set_title('最近4周每日输出消费（万 Token，双套餐合并）', fontsize=14, fontweight='bold')
axes2[1].set_ylabel('输出消费（万）')
axes2[1].set_xticks(x_pos)
axes2[1].set_xticklabels(x_labels, rotation=45, ha='right')
axes2[1].grid(True, alpha=0.3, axis='y')

axes2[2].bar(x_pos, daily_4w_merged['套餐1']/1e8, color='#2196F3', alpha=0.8, label='套餐1(highspeed)', width=0.8)
axes2[2].bar(x_pos, daily_4w_merged['套餐2']/1e8, bottom=daily_4w_merged['套餐1']/1e8, color='#FF9800', alpha=0.8, label='套餐2(M2.7)', width=0.8)
axes2[2].set_title('最近4周每日总消费数（亿 Token，双套餐堆叠）', fontsize=14, fontweight='bold')
axes2[2].set_ylabel('总消费数（亿）')
axes2[2].set_xlabel('日期')
axes2[2].set_xticks(x_pos)
axes2[2].set_xticklabels(x_labels, rotation=45, ha='right')
axes2[2].grid(True, alpha=0.3, axis='y')
axes2[2].legend()

fig2.autofmt_xdate()
plt.tight_layout()
plt.savefig(OUTPUT_DIR + 'minimax_2weeks_detail.png', dpi=150, bbox_inches='tight')
print(f"Detail chart saved: {OUTPUT_DIR}minimax_2weeks_detail.png")

fig3, axes3 = plt.subplots(1, 2, figsize=(16, 7))

model_usage = df_all.groupby('消费模型').agg({'总消费数': 'sum'}).sort_values('总消费数', ascending=True)
colors_model = ['#4CAF50', '#FF9800', '#2196F3', '#F44336', '#9C27B0'][:len(model_usage)]
axes3[0].barh(model_usage.index, model_usage['总消费数']/1e8, color=colors_model, alpha=0.8)
axes3[0].set_title('按模型 Token 消费分布（亿）', fontsize=14, fontweight='bold')
axes3[0].set_xlabel('Token 数（亿）')
axes3[0].grid(True, alpha=0.3, axis='x')

api_usage = df_all.groupby('消费接口').agg({'总消费数': 'sum'}).sort_values('总消费数', ascending=True)
colors_api = ['#4CAF50', '#FF9800', '#2196F3', '#F44336'][:len(api_usage)]
axes3[1].barh(api_usage.index, api_usage['总消费数']/1e8, color=colors_api, alpha=0.8)
axes3[1].set_title('按接口类型 Token 消费分布（亿）', fontsize=14, fontweight='bold')
axes3[1].set_xlabel('Token 数（亿）')
axes3[1].grid(True, alpha=0.3, axis='x')

plt.tight_layout()
plt.savefig(OUTPUT_DIR + 'minimax_distribution.png', dpi=150, bbox_inches='tight')
print(f"Distribution chart saved: {OUTPUT_DIR}minimax_distribution.png")

excel_writer = pd.ExcelWriter(OUTPUT_DIR + 'minimax_usage_summary.xlsx')
daily_all.to_excel(excel_writer, sheet_name='每日汇总')
weekly = df_all.groupby(pd.Grouper(key='日期', freq='W-MON')).agg({
    '输入消费数': 'sum', '输出消费数': 'sum', '总消费数': 'sum', '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'})
weekly.to_excel(excel_writer, sheet_name='每周汇总')
monthly = df_all.groupby(pd.Grouper(key='日期', freq='ME')).agg({
    '输入消费数': 'sum', '输出消费数': 'sum', '总消费数': 'sum', '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'})
monthly.to_excel(excel_writer, sheet_name='每月汇总')
daily_4w.to_excel(excel_writer, sheet_name='最近4周')
model_usage_full = df_all.groupby('消费模型').agg({
    '输入消费数': 'sum', '输出消费数': 'sum', '总消费数': 'sum', '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'})
model_usage_full.to_excel(excel_writer, sheet_name='模型分布')
api_usage_full = df_all.groupby('消费接口').agg({
    '输入消费数': 'sum', '输出消费数': 'sum', '总消费数': 'sum', '消费时间': 'count'
}).rename(columns={'消费时间': '调用次数'})
api_usage_full.to_excel(excel_writer, sheet_name='接口分布')
excel_writer.close()
print(f"Excel summary saved: {OUTPUT_DIR}minimax_usage_summary.xlsx")

print("\n" + "=" * 90)
print("All charts generated successfully!")
print("=" * 90)

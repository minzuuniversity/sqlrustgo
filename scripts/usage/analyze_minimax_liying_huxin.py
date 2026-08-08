#!/usr/bin/env python3
"""Aggregate Minimax usage CSVs for Li Ying and Hu Xin and regenerate charts."""

from __future__ import annotations

import csv
import json
import statistics
from collections import defaultdict
from datetime import date, datetime
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]
SOURCES = [
    ("李莹", "export_bill_2026-0201-0501-liying.csv"),
    ("李莹", "export_bill_2026-0501-0513-liying.csv"),
    ("李莹", "export_bill_2026-0514-0715-liying.csv"),
    ("李莹", "export_bill_2026-0716-0807.csv"),
    ("胡馨", "export_bill_0529-0808-huxin.csv"),
]
SUMMARY_PATH = ROOT / "artifacts/usage/minimax_liying_huxin_summary_20260202_20260807.json"
MERGED_PATH = ROOT / "artifacts/usage/export_merged_minimax_liying_huxin_20260202_20260807.csv"
COMPLETE_CHART = ROOT / "scripts/usage/minimax_complete_trend.png"
RECENT_CHART = ROOT / "scripts/usage/minimax_2weeks_detail.png"


def int_value(value: object) -> int:
    try:
        return int(float(str(value).replace(",", "")))
    except ValueError:
        return 0


def float_value(value: object) -> float:
    try:
        return float(str(value).replace(",", ""))
    except ValueError:
        return 0.0


def aggregate(rows: list[dict[str, object]]) -> dict[str, object]:
    days = {str(row["消费日期"]) for row in rows}
    return {
        "records": len(rows),
        "days": len(days),
        "min_date": min(days) if days else None,
        "max_date": max(days) if days else None,
        "input_tokens": sum(int(row["输入消费数"]) for row in rows),
        "output_tokens": sum(int(row["输出消费数"]) for row in rows),
        "total_tokens": sum(int(row["总消费数"]) for row in rows),
        "amount": sum(float(row["消费金额"]) for row in rows),
        "voucher_amount": sum(float(row["代金券后消费金额"]) for row in rows),
    }


def load_rows() -> tuple[list[dict[str, object]], list[dict[str, object]], list[str]]:
    seen_by_owner: dict[str, set[tuple[tuple[str, str], ...]]] = defaultdict(set)
    rows: list[dict[str, object]] = []
    file_stats: list[dict[str, object]] = []
    source_fields: list[str] | None = None

    for owner, filename in SOURCES:
        raw_success = kept = duplicates = 0
        dates: list[str] = []
        with (ROOT / filename).open("r", encoding="utf-8-sig", newline="") as handle:
            reader = csv.DictReader(handle)
            source_fields = source_fields or list(reader.fieldnames or [])
            for row in reader:
                if row.get("消费结果") != "SUCCESS":
                    continue
                raw_success += 1
                key = tuple((field, row.get(field, "")) for field in reader.fieldnames or [])
                consumed_date = str(row["消费时间"])[:10]
                dates.append(consumed_date)
                if key in seen_by_owner[owner]:
                    duplicates += 1
                    continue

                seen_by_owner[owner].add(key)
                normalized: dict[str, object] = dict(row)
                normalized["人员"] = owner
                normalized["消费日期"] = consumed_date
                for field in ["输入消费数", "输出消费数", "总消费数"]:
                    normalized[field] = int_value(normalized[field])
                for field in ["消费金额", "代金券后消费金额"]:
                    normalized[field] = float_value(normalized[field])
                rows.append(normalized)
                kept += 1

        file_stats.append(
            {
                "owner": owner,
                "file": filename,
                "raw_success": raw_success,
                "kept_unique": kept,
                "duplicates_skipped_for_owner": duplicates,
                "min_date": min(dates) if dates else None,
                "max_date": max(dates) if dates else None,
            }
        )

    return rows, file_stats, source_fields or []


def build_summary(rows: list[dict[str, object]], file_stats: list[dict[str, object]]) -> dict[str, object]:
    summary: dict[str, object] = {
        "source_files": file_stats,
        "overall": aggregate(rows),
        "owner": {},
        "monthly": [],
        "recent14": [],
        "daily": [],
        "model": [],
        "api": [],
    }
    summary["owner"] = {owner: aggregate([row for row in rows if row["人员"] == owner]) for owner, _ in SOURCES}

    months = sorted({str(row["消费日期"])[:7] for row in rows})
    previous_total: int | None = None
    for month in months:
        month_rows = [row for row in rows if str(row["消费日期"]).startswith(month)]
        entry = aggregate(month_rows)
        entry["month"] = month
        entry["liying_total"] = sum(int(row["总消费数"]) for row in month_rows if row["人员"] == "李莹")
        entry["huxin_total"] = sum(int(row["总消费数"]) for row in month_rows if row["人员"] == "胡馨")
        entry["liying_calls"] = sum(1 for row in month_rows if row["人员"] == "李莹")
        entry["huxin_calls"] = sum(1 for row in month_rows if row["人员"] == "胡馨")
        entry["mom"] = None if previous_total is None else (int(entry["total_tokens"]) - previous_total) / previous_total
        previous_total = int(entry["total_tokens"])
        summary["monthly"].append(entry)

    by_day: dict[str, list[dict[str, object]]] = defaultdict(list)
    for row in rows:
        by_day[str(row["消费日期"])].append(row)

    for consumed_date in sorted(by_day):
        day_rows = by_day[consumed_date]
        entry = aggregate(day_rows)
        entry["date"] = consumed_date
        entry["liying_total"] = sum(int(row["总消费数"]) for row in day_rows if row["人员"] == "李莹")
        entry["huxin_total"] = sum(int(row["总消费数"]) for row in day_rows if row["人员"] == "胡馨")
        entry["liying_calls"] = sum(1 for row in day_rows if row["人员"] == "李莹")
        entry["huxin_calls"] = sum(1 for row in day_rows if row["人员"] == "胡馨")
        summary["daily"].append(entry)
    summary["recent14"] = summary["daily"][-14:]

    for source_field, key_name in [("消费模型", "model"), ("消费接口", "api")]:
        grouped: dict[str, list[dict[str, object]]] = defaultdict(list)
        for row in rows:
            grouped[str(row[source_field])].append(row)
        for name, group_rows in sorted(
            grouped.items(), key=lambda item: sum(int(row["总消费数"]) for row in item[1]), reverse=True
        ):
            entry = aggregate(group_rows)
            entry[key_name] = name
            entry["liying_total"] = sum(int(row["总消费数"]) for row in group_rows if row["人员"] == "李莹")
            entry["huxin_total"] = sum(int(row["总消费数"]) for row in group_rows if row["人员"] == "胡馨")
            summary[key_name].append(entry)

    daily_totals = [int(day["total_tokens"]) for day in summary["daily"]]
    overall = summary["overall"]
    natural_days = (
        date.fromisoformat(str(overall["max_date"])) - date.fromisoformat(str(overall["min_date"]))
    ).days + 1
    summary["daily_stats"] = {
        "peak": max(summary["daily"], key=lambda item: int(item["total_tokens"])),
        "low": min(summary["daily"], key=lambda item: int(item["total_tokens"])),
        "avg_natural": int(overall["total_tokens"]) / natural_days,
        "avg_recorded": int(overall["total_tokens"]) / len(summary["daily"]),
        "recent14_avg": sum(int(day["total_tokens"]) for day in summary["recent14"]) / len(summary["recent14"]),
        "cv": statistics.pstdev(daily_totals) / statistics.mean(daily_totals),
    }
    return summary


def save_outputs(rows: list[dict[str, object]], source_fields: list[str], summary: dict[str, object]) -> None:
    SUMMARY_PATH.parent.mkdir(parents=True, exist_ok=True)
    SUMMARY_PATH.write_text(json.dumps(summary, ensure_ascii=False, indent=2), encoding="utf-8")

    with MERGED_PATH.open("w", encoding="utf-8-sig", newline="") as handle:
        fieldnames = ["人员", "消费日期"] + source_fields
        writer = csv.DictWriter(handle, fieldnames=fieldnames)
        writer.writeheader()
        for row in rows:
            writer.writerow({field: row.get(field, "") for field in fieldnames})


def draw_charts(summary: dict[str, object]) -> None:
    import matplotlib

    matplotlib.use("Agg")
    import matplotlib.dates as mdates
    import matplotlib.pyplot as plt

    def yi(value: int | float) -> float:
        return float(value) / 100_000_000

    plt.rcParams["font.sans-serif"] = ["PingFang SC", "Arial Unicode MS", "Heiti TC", "Songti SC", "STHeiti", "DejaVu Sans"]
    plt.rcParams["axes.unicode_minus"] = False

    daily = summary["daily"]
    dates = [datetime.fromisoformat(str(item["date"])) for item in daily]
    liying = [yi(item["liying_total"]) for item in daily]
    huxin = [yi(item["huxin_total"]) for item in daily]
    totals = [yi(item["total_tokens"]) for item in daily]

    fig, axes = plt.subplots(3, 1, figsize=(15, 17), gridspec_kw={"height_ratios": [2.3, 1.2, 1.1]})
    axes[0].stackplot(dates, liying, huxin, labels=["李莹", "胡馨"], colors=["#2F6BFF", "#E86A33"], alpha=0.78)
    axes[0].plot(dates, totals, color="#111827", linewidth=1.8, label="合计")
    peak = summary["daily_stats"]["peak"]
    peak_date = datetime.fromisoformat(str(peak["date"]))
    axes[0].scatter([peak_date], [yi(peak["total_tokens"])], color="#111827", s=45, zorder=5)
    axes[0].annotate(
        f"峰值 {peak['date']}\n{yi(peak['total_tokens']):.2f} 亿",
        xy=(peak_date, yi(peak["total_tokens"])),
        xytext=(14, 18),
        textcoords="offset points",
        fontsize=10,
        bbox={"boxstyle": "round,pad=0.3", "fc": "white", "ec": "#9CA3AF", "alpha": 0.9},
    )
    axes[0].set_title("Minimax Token 日用量趋势（李莹 + 胡馨，2026-02-02 至 2026-08-07）", fontsize=18, weight="bold", pad=18)
    axes[0].set_ylabel("Token（亿）")
    axes[0].grid(True, axis="y", alpha=0.25)
    axes[0].legend(loc="upper left", ncol=3)
    axes[0].xaxis.set_major_locator(mdates.MonthLocator())
    axes[0].xaxis.set_major_formatter(mdates.DateFormatter("%m月"))

    months = summary["monthly"]
    x_values = range(len(months))
    li_monthly = [yi(item["liying_total"]) for item in months]
    hu_monthly = [yi(item["huxin_total"]) for item in months]
    axes[1].bar(x_values, li_monthly, color="#2F6BFF", label="李莹")
    axes[1].bar(x_values, hu_monthly, bottom=li_monthly, color="#E86A33", label="胡馨")
    for index, item in enumerate(months):
        axes[1].text(index, yi(item["total_tokens"]) + 2, f"{yi(item['total_tokens']):.1f}", ha="center", va="bottom", fontsize=10)
    axes[1].set_xticks(list(x_values))
    axes[1].set_xticklabels([f"{int(str(item['month'])[-2:])}月" for item in months])
    axes[1].set_ylabel("Token（亿）")
    axes[1].set_title("月度合计用量")
    axes[1].grid(True, axis="y", alpha=0.22)
    axes[1].legend(loc="upper left")

    models = summary["model"][:6]
    names = [str(item["model"]) for item in models][::-1]
    values = [yi(item["total_tokens"]) for item in models][::-1]
    axes[2].barh(names, values, color="#34A853")
    for index, value in enumerate(values):
        axes[2].text(value + 1, index, f"{value:.1f}", va="center", fontsize=10)
    axes[2].set_xlabel("Token（亿）")
    axes[2].set_title("模型用量 Top 分布")
    axes[2].grid(True, axis="x", alpha=0.22)
    fig.tight_layout()
    fig.savefig(COMPLETE_CHART, dpi=180, bbox_inches="tight")
    plt.close(fig)

    recent = summary["recent14"]
    labels = [str(item["date"])[5:] for item in recent]
    x_values = list(range(len(recent)))
    li_recent = [yi(item["liying_total"]) for item in recent]
    hu_recent = [yi(item["huxin_total"]) for item in recent]
    totals = [yi(item["total_tokens"]) for item in recent]
    calls = [int(item["records"]) for item in recent]

    fig, axis_left = plt.subplots(figsize=(15, 8))
    axis_left.bar(x_values, li_recent, color="#2F6BFF", label="李莹 Token")
    axis_left.bar(x_values, hu_recent, bottom=li_recent, color="#E86A33", label="胡馨 Token")
    for index, total in enumerate(totals):
        axis_left.text(index, total + 0.04, f"{total:.2f}", ha="center", va="bottom", fontsize=9)
    axis_left.set_xticks(x_values)
    axis_left.set_xticklabels(labels)
    axis_left.set_ylabel("Token（亿）")
    axis_left.set_title("最近14天 Minimax Token 用量明细（2026-07-25 至 2026-08-07）", fontsize=17, weight="bold", pad=16)
    axis_left.grid(True, axis="y", alpha=0.25)
    axis_right = axis_left.twinx()
    axis_right.plot(x_values, calls, color="#111827", marker="o", linewidth=2, label="调用次数")
    axis_right.set_ylabel("调用次数")
    handles_left, labels_left = axis_left.get_legend_handles_labels()
    handles_right, labels_right = axis_right.get_legend_handles_labels()
    axis_left.legend(handles_left + handles_right, labels_left + labels_right, loc="upper left", ncol=3)
    fig.tight_layout()
    fig.savefig(RECENT_CHART, dpi=180, bbox_inches="tight")
    plt.close(fig)


def main() -> None:
    rows, file_stats, source_fields = load_rows()
    summary = build_summary(rows, file_stats)
    save_outputs(rows, source_fields, summary)
    draw_charts(summary)
    print(json.dumps({"records": summary["overall"]["records"], "total_tokens": summary["overall"]["total_tokens"]}, ensure_ascii=False))


if __name__ == "__main__":
    main()

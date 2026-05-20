#!/usr/bin/env python3
"""
GMP 文档全集合并工具
将 gmp_kg_build/2025GMP-MD、gmp_complete_archive、gmp-qa-package 三目录
合并为统一的 gmp-md 文档集合，按 GMP 2010 章节体系 + 附录编排。
"""

import os
import sys
import shutil
from pathlib import Path
from collections import defaultdict

PROJECT_ROOT = Path("/Users/liying/workspace/GMP-Platform")
GMP_MD_OUT  = PROJECT_ROOT / "gmp-md"

# ============================================================
# 源目录
# ============================================================
BASE_MD    = PROJECT_ROOT / "gmp_kg_build/2025GMP-MD"
SUPPLEMENT = PROJECT_ROOT / "gmp_complete_archive/02_补充文档"
ROOT_SOP   = PROJECT_ROOT / "gmp_complete_archive/01_原始GMP文件/GMP-fixed-MD/ROOT_SOP"
ZZ_ORIGIN  = PROJECT_ROOT / "gmp_complete_archive/01_原始GMP文件/GMP-fixed-MD/ZZ_原始文件"
QA_DOCS    = PROJECT_ROOT / "gmp-qa-package/docs"

# ============================================================
# GMP 章节映射: 2025GMP-MD dir -> gmp-md 目标目录
# ============================================================
CHAPTER_MAP = [
    ("01_", "CH01-文件管理与数据完整性", {
        "GUI": "GUI",
        "REC": "REC",
        "SOP": "SOP",
    }),
    ("02_", "CH02-质量管理体系", {
        "CAP": "CAP-CAPA管理",
        "DEV": "DEV-偏差处理",
        "GUI": "GUI",
        "POL": "POL-方针与程序",
        "REC": "REC",
        "SOP": "SOP",
    }),
    ("03_", "CH03-机构与人员", {
        "POL": "POL",
        "REC": "REC",
        "SOP": "SOP",
        "TRA": "TRA-培训",
    }),
    ("04_", "CH04-厂房与设施", {
        "GUI": "GUI",
        "SOP": "SOP",
        "VAL": "VAL-验证",
    }),
    ("05_", "CH05-设备管理", {
        "CAL": "CAL-校准",
        "REC": "REC",
        "SOP": "SOP",
    }),
    ("06_", "CH06-物料与产品", {
        "GUI": "GUI",
        "POL": "POL",
        "REC": "REC",
        "RET": "RET-退货与召回",
        "SOP": "SOP",
    }),
    ("07_", "CH07-确认与验证", {
        "BPR": "BPR-批生产记录",
        "GUI": "GUI",
        "SOP": "SOP",
        "VAL": "VAL",
    }),
    ("08_", "CH08-生产管理", {
        "AUD": "AUD-审计",
        "CAL": "CAL-校准",
        "GUI": "GUI",
        "OOS": "OOS-超标结果",
        "REC": "REC",
        "SOP": "SOP",
        "STA": "STA-稳定性",
        "VAL": "VAL",
    }),
    ("09_", "CH09-委托生产与委托检验", {
        "AGR": "AGR-协议",
    }),
]

# ============================================================
# 补充文档映射: 源 -> 目标
# ============================================================
SUPPLEMENT_MAP = [
    # openclaw_supplement
    (SUPPLEMENT / "openclaw_supplement/01_GMP条款解释", "APPX-A-GMP法规条款与解读"),
    (SUPPLEMENT / "openclaw_supplement/02_变更控制",    "APPX-B-变更控制与企业补充规程/01-变更控制"),
    (SUPPLEMENT / "openclaw_supplement/03_偏差CAPA",    "APPX-B-变更控制与企业补充规程/02-偏差CAPA",
     ),
    (SUPPLEMENT / "openclaw_supplement/04_验证管理",    "APPX-B-变更控制与企业补充规程/03-验证管理"),
    (SUPPLEMENT / "openclaw_supplement/05_设备确认",    "APPX-B-变更控制与企业补充规程/04-设备确认"),
    # unique_to_me
    (SUPPLEMENT / "unique_to_me/文件管理",  "APPX-B-变更控制与企业补充规程/05-文件管理"),
    (SUPPLEMENT / "unique_to_me/物料与产品","APPX-B-变更控制与企业补充规程/06-物料与产品"),
    (SUPPLEMENT / "unique_to_me/生产管理",  "APPX-B-变更控制与企业补充规程/07-生产管理"),
    (SUPPLEMENT / "unique_to_me/计算机系统","APPX-C-计算机化系统"),
    (SUPPLEMENT / "unique_to_me/设施与设备","APPX-D-设施与设备补充"),
    (SUPPLEMENT / "unique_to_me/质量控制",  "APPX-E-质量控制补充"),
    (SUPPLEMENT / "unique_to_me/质量管理体系","APPX-F-质量管理体系补充"),
    (SUPPLEMENT / "unique_to_me/验证与确认","APPX-G-验证与确认补充"),
    (SUPPLEMENT / "unique_to_me/安全卫生",  "APPX-H-安全卫生"),
    # phase2
    (SUPPLEMENT / "phase2/computers",  "APPX-C-计算机化系统"),
    (SUPPLEMENT / "phase2/docmgmt",    "APPX-B-变更控制与企业补充规程/05-文件管理"),
    (SUPPLEMENT / "phase2/facilities", "APPX-D-设施与设备补充"),
    (SUPPLEMENT / "phase2/materials",  "APPX-B-变更控制与企业补充规程/06-物料与产品"),
    (SUPPLEMENT / "phase2/org",        "APPX-I-组织机构补充"),
    (SUPPLEMENT / "phase2/production", "APPX-B-变更控制与企业补充规程/07-生产管理"),
    (SUPPLEMENT / "phase2/qc",         "APPX-E-质量控制补充"),
    (SUPPLEMENT / "phase2/qms",        "APPX-F-质量管理体系补充"),
    (SUPPLEMENT / "phase2/security",   "APPX-H-安全卫生"),
    (SUPPLEMENT / "phase2/validation", "APPX-G-验证与确认补充"),
]

# 去重用的 seen 集合
SEEN = set()

def copy_structure(src: Path, dst_dir: Path, prefix: str = ""):
    """复制 src 下所有 .md 文件到 dst_dir，记录去重。"""
    count = 0
    for f in sorted(src.rglob("*.md")):
        name = f.name
        key = f"{prefix}|{name}"
        if key in SEEN:
            continue
        SEEN.add(key)
        os.makedirs(dst_dir, exist_ok=True)
        shutil.copy2(f, dst_dir / name)
        count += 1
    return count


def main():
    # 清理旧输出
    if GMP_MD_OUT.exists():
        shutil.rmtree(GMP_MD_OUT)
    total = 0

    # ---- Phase 1: 复制 2025GMP-MD 基础文档 ----
    print("Phase 1: 复制 2025GMP-MD 基础章节文档...")
    for src_ch, dst_ch, sub_dirs in CHAPTER_MAP:
        ch_dir = BASE_MD / src_ch
        if not ch_dir.exists():
            print(f"  SKIP {src_ch} (不存在)")
            continue
        for src_sub, dst_sub in sub_dirs.items():
            src_sub_dir = ch_dir / src_sub
            if src_sub_dir.exists():
                dst = GMP_MD_OUT / dst_ch / dst_sub
                n = copy_structure(src_sub_dir, dst, prefix=f"{src_ch}/{src_sub}")
                total += n
                if n > 0:
                    print(f"  [{dst_ch}/{dst_sub}] {n} files")
            else:
                print(f"  [{dst_ch}/{dst_sub}] MISSING: {src_sub_dir}")

    # ---- Phase 2: ROOT_SOP ----
    print("\nPhase 2: 复制 ROOT_SOP 核心标准操作规程...")
    root_dst = GMP_MD_OUT / "APPX-J-核心根SOP"
    n = copy_structure(ROOT_SOP, root_dst, prefix="ROOT_SOP")
    total += n
    print(f"  [APPX-J-核心根SOP] {n} files")

    # ---- Phase 3: ZZ_原始文件 ----
    print("\nPhase 3: 复制原始扫描件参考...")
    zz_dst = GMP_MD_OUT / "APPX-K-原始扫描件参考"
    n = copy_structure(ZZ_ORIGIN, zz_dst, prefix="ZZ")
    total += n
    print(f"  [APPX-K-原始扫描件参考] {n} files")

    # ---- Phase 4: 补充文档 ----
    print("\nPhase 4: 合并补充文档...")
    for src_path, dst_rel in SUPPLEMENT_MAP:
        src = Path(src_path)
        if not src.exists():
            print(f"  SKIP {src_rel} (源目录不存在)")
            continue
        dst = GMP_MD_OUT / dst_rel
        n = copy_structure(src, dst, prefix=f"supp/{dst_rel}")
        if n > 0:
            total += n
            print(f"  [{dst_rel}] {n} files (merged from {src_path.relative_to(PROJECT_ROOT)})")

    # ---- Phase 5: gmp-qa-package 技术文档 ----
    print("\nPhase 5: 复制技术参考资料...")
    tech_dst = GMP_MD_OUT / "APPX-L-技术参考资料"
    n = copy_structure(QA_DOCS, tech_dst, prefix="qa-docs")
    total += n
    print(f"  [APPX-L-技术参考资料] {n} files")

    # ---- Phase 6: 统计 & 生成 INDEX ----
    print(f"\n{'='*60}")
    print(f"合并完成: {total} 个唯一文件 -> {GMP_MD_OUT}")
    generate_index(total)


def generate_index(total_files: int):
    """生成 README.md 和 INDEX.md"""
    print("\n生成 INDEX.md ...")

    lines = []
    lines.append("# GMP 文档全集索引 (gmp-md)")
    lines.append("")
    lines.append(f"> 从 3 个源目录合并的 GMP 合规性内审系统规则文档与原始文档集")
    lines.append(f"> 总文件数: **{total_files}** (已去重)")
    lines.append(f"> 生成时间: 2026-05-20")
    lines.append("")
    lines.append("## 目录结构总览")
    lines.append("")
    lines.append("| 目录 | 说明 | 来源 |")
    lines.append("|------|------|------|")
    lines.append("| CH01-文件管理与数据完整性 | 文件编码、编制、分发、修订、记录填写、数据完整性规范 | 2025GMP-MD/01_ |")
    lines.append("| CH02-质量管理体系 | 质量方针、管理评审、CAPA、偏差处理、供应商管理 | 2025GMP-MD/02_ |")
    lines.append("| CH03-机构与人员 | 组织架构、岗位职责、培训管理、人员卫生 | 2025GMP-MD/03_ |")
    lines.append("| CH04-厂房与设施 | 洁净区管理、厂房设计、设施维护、环境监控 | 2025GMP-MD/04_ |")
    lines.append("| CH05-设备管理 | 设备选型、维护保养、校准管理 | 2025GMP-MD/05_ |")
    lines.append("| CH06-物料与产品 | 采购、验收、仓储、放行、退货、召回 | 2025GMP-MD/06_ |")
    lines.append("| CH07-确认与验证 | 批生产记录、清洁验证、方法验证、工艺验证 | 2025GMP-MD/07_ |")
    lines.append("| CH08-生产管理 | 工艺规程、污染控制、OOS调查、稳定性考察、审计 | 2025GMP-MD/08_ |")
    lines.append("| CH09-委托生产与委托检验 | 委托协议、技术协议、质量协议 | 2025GMP-MD/09_ |")
    lines.append("| APPX-A-GMP法规条款与解读 | 药品GMP(2010) 各章条款及解释（53份） | openclaw_supplement |")
    lines.append("| APPX-B-变更控制与企业补充规程 | 变更控制、偏差CAPA、验证管理、设备确认、文件管理、物料、生产 | 多个源补充 |")
    lines.append("| APPX-C-计算机化系统 | 计算机化系统验证、电子记录、电子签名规范 | unique_to_me + phase2 |")
    lines.append("| APPX-D-设施与设备补充 | 设施与设备管理补充规程 | unique_to_me + phase2 |")
    lines.append("| APPX-E-质量控制补充 | QC实验室、取样、检验补充规程 | unique_to_me + phase2 |")
    lines.append("| APPX-F-质量管理体系补充 | 质量体系补充文档 | unique_to_me + phase2 |")
    lines.append("| APPX-G-验证与确认补充 | 补充验证/确认方案和报告模板 | unique_to_me + phase2 |")
    lines.append("| APPX-H-安全卫生 | EHS、生物安全、职业卫生 | unique_to_me + phase2 |")
    lines.append("| APPX-I-组织机构补充 | 组织架构补充文档 | phase2 |")
    lines.append("| APPX-J-核心根SOP | 企业级核心标准操作规程（17份） | ROOT_SOP |")
    lines.append("| APPX-K-原始扫描件参考 | Word/PDF 转 MD 的原始参考文档（18份） | ZZ_原始文件 |")
    lines.append("| APPX-L-技术参考资料 | 系统建设技术参考文档（17份） | gmp-qa-package |")
    lines.append("")

    # 各 CH 章节详情
    lines.append("---")
    lines.append("")
    lines.append("## 各 GMP 章节文件详情")
    lines.append("")

    for _, dst_ch, sub_dirs in CHAPTER_MAP:
        ch_dir = GMP_MD_OUT / dst_ch
        if not ch_dir.exists():
            continue
        lines.append(f"### {dst_ch}")
        lines.append("")
        total_ch = 0
        for src_sub, dst_sub in sub_dirs.items():
            sub_dir = ch_dir / dst_sub
            if sub_dir.exists():
                files = sorted(sub_dir.glob("*.md"))
                if files:
                    lines.append(f"**{dst_sub}** ({len(files)} 份)")
                    lines.append("")
                    for f in files:
                        lines.append(f"- {f.name}")
                    lines.append("")
                    total_ch += len(files)
        lines.append(f"*CH 小计: {total_ch} 份*")
        lines.append("")
        lines.append("---")
        lines.append("")

    # 附录文件清单
    lines.append("## 附录文件清单")
    lines.append("")
    for entry in os.listdir(GMP_MD_OUT):
        if entry.startswith("APPX-") and os.path.isdir(GMP_MD_OUT / entry):
            _list_appendix(GMP_MD_OUT / entry, lines)

    (GMP_MD_OUT / "INDEX.md").write_text("\n".join(lines), encoding="utf-8")
    print("  INDEX.md 已生成")

    # 生成 README.md
    readme = [
        "# GMP 文档全集 (gmp-md)",
        "",
        "本目录是 GMP Platform 合规性内审系统的**原始文档集**和**规则文档集**，",
        "从以下 3 个源合并，按 GMP 2010 章节体系组织：",
        "",
        "| 源 | 说明 |",
        "|----|------|",
        "| `gmp_kg_build/2025GMP-MD/` | 主 GMP 文档库（1677 份），按章节分类 |",
        "| `gmp_complete_archive/02_补充文档/` | 企业补充规程、法规解释、专有文档 |",
        "| `gmp-qa-package/docs/` | 系统建设技术参考资料 |",
        "",
        "## 目录结构",
        "",
        "```",
    ]
    for entry in sorted(os.listdir(GMP_MD_OUT)):
        p = GMP_MD_OUT / entry
        if os.path.isdir(p) and not entry.startswith("."):
            count = sum(1 for _ in p.rglob("*.md"))
            readme.append(f"├── {entry}/  ({count} 个文档)")
    readme += [
        "├── INDEX.md    (完整文档索引)",
        "└── README.md   (本文件)",
        "```",
        "",
        f"## 统计",
        f"- 合并前源文件: 约 2200+ 份",
        f"- 去重后唯一文件: {total_files} 份",
        f"- GMP 章节目录: 9 个 (CH01-CH09)",
        f"- 附录目录: 12 个 (APPX-A 到 APPX-L)",
        "",
        "## 使用说明",
        "",
        "1. 浏览 `INDEX.md` 查看完整索引",
        "2. 各 CH 目录按 GMP 2010 章节组织，可作为合规内审参考",
        "3. 附录包含企业补充规程、法规解读和技术参考资料",
        "",
        "## 注意",
        "- 本文档集已去除企业标识 (GZYZ)，内容来自模板和公开发布的 GMP 指南",
    ]
    (GMP_MD_OUT / "README.md").write_text("\n".join(readme), encoding="utf-8")
    print("  README.md 已生成")


def _list_appendix(dir_path: Path, lines: list):
    """列出附录下的文件"""
    label = dir_path.name
    files = sorted(dir_path.rglob("*.md"))
    if not files:
        return
    lines.append(f"### {label} ({len(files)} 份)")
    lines.append("")
    for f in files:
        lines.append(f"- {f.name}")
    lines.append("")


if __name__ == "__main__":
    main()
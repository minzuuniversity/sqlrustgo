#!/usr/bin/env python3
"""
json_to_tbl.py - Convert JSON fixture data to TPC-H .tbl format.
Runs in: /Users/liying/workspace/dev/yinglichina163/sqlrustgo/tests/data/tpch-sf01/
"""
import json, os, sys

TPCH_TABLES = ["region", "nation", "customer", "orders", "lineitem",
               "part", "partsupp", "supplier"]

def json_to_tbl(src_dir, dst_dir):
    os.makedirs(dst_dir, exist_ok=True)
    for tbl in TPCH_TABLES:
        json_path = os.path.join(src_dir, f"{tbl}.json")
        tbl_path = os.path.join(dst_dir, f"{tbl}.tbl")
        if not os.path.exists(json_path):
            print(f"SKIP {tbl}: {json_path} not found")
            continue
        with open(json_path) as f:
            data = json.load(f)
        rows = data.get("rows", [])
        cols = data.get("columns", [])
        if not rows:
            print(f"EMPTY {tbl}: no data rows")
            # Write empty .tbl
            open(tbl_path, "w").close()
            continue
        # Determine column order from column metadata
        col_order = [c["name"] for c in cols]
        with open(tbl_path, "w") as f:
            for row in rows:
                vals = []
                for cname in col_order:
                    v = row.get(cname, "")
                    # TPC-H .tbl uses | separator, trailing |, NULL as \N
                    if v is None or v == "":
                        vals.append("\\N")
                    elif isinstance(v, float):
                        vals.append(f"{v:.2f}" if v != int(v) else str(int(v)))
                    else:
                        # Escape | and \ characters
                        v = str(v).replace("|", "??").replace("\\", "\\\\")
                        vals.append(v)
                f.write("|".join(vals) + "|\n")
        print(f"WROTE {tbl}: {len(rows)} rows -> {tbl_path}")
    print("Done.")

if __name__ == "__main__":
    src = sys.argv[1] if len(sys.argv) > 1 else "."
    dst = sys.argv[2] if len(sys.argv) > 2 else src
    json_to_tbl(src, dst)

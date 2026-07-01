#!/usr/bin/env python3
"""
json_to_bin.py — Convert FileStorage JSON format to BinaryTableStorage .bin.
Supports migration from JSON storage to binary storage.

Usage:
    python3 json_to_bin.py <json_file> <bin_file>
    python3 json_to_bin.py <json_file> <bin_file> [--table TABLE_NAME]

The JSON must have the FileStorage schema:
    {"name": "TABLE", "columns": [{"name": "...", "data_type": "INTEGER|REAL|TEXT", ...}, ...], "rows": [[[{"Integer": N}, {"Text": "s"}], ...]]}
"""

import sys
import struct
import json
import os

MAGIC = b"BINT"
TYPE_NULL  = 0
TYPE_INT   = 1
TYPE_FLOAT = 2
TYPE_TEXT  = 3


def dtype_to_code(dtype):
    d = dtype.upper()
    if 'INT' in d and 'REAL' not in d and 'TEXT' not in d:
        return TYPE_INT
    if 'FLOAT' in d or 'REAL' in d:
        return TYPE_FLOAT
    return TYPE_TEXT


def extract_value(val):
    """Extract (type_code, raw_value) from a Value wrapper."""
    if 'Null' in val:
        return TYPE_NULL, None
    if 'Integer' in val:
        return TYPE_INT, int(val['Integer'])
    if 'Float' in val:
        return TYPE_FLOAT, float(val['Float'])
    if 'Double' in val:
        return TYPE_FLOAT, float(val['Double'])
    if 'Text' in val:
        return TYPE_TEXT, str(val['Text'])
    if 'Binary' in val:
        return TYPE_TEXT, str(val['Binary'])
    return TYPE_NULL, None


def json_to_bin(json_path, bin_path, table_name=None):
    with open(json_path, 'r', encoding='utf-8') as f:
        doc = json.load(f)

    columns = doc['columns']
    rows = doc['rows']
    name = table_name or doc.get('name', os.path.splitext(os.path.basename(json_path))[0])

    col_codes = [dtype_to_code(c['data_type']) for c in columns]

    with open(bin_path, 'wb') as f:
        w = f.write
        w(MAGIC)
        w(struct.pack('<I', 1))
        w(struct.pack('<I', len(col_codes)))
        for c in col_codes:
            w(struct.pack('B', c))
        w(struct.pack('<Q', len(rows)))

        for row in rows:
            for i, cell in enumerate(row):
                if isinstance(cell, list):
                    # list of Value wrappers
                    tc, val = extract_value(cell[0]) if cell else (TYPE_NULL, None)
                elif isinstance(cell, dict):
                    tc, val = extract_value(cell)
                else:
                    tc, val = TYPE_NULL, None

                if tc == TYPE_NULL:
                    w(struct.pack('B', TYPE_NULL))
                elif tc == TYPE_INT:
                    w(struct.pack('B', TYPE_INT))
                    w(struct.pack('<q', val))
                elif tc == TYPE_FLOAT:
                    w(struct.pack('B', TYPE_FLOAT))
                    w(struct.pack('<d', val))
                else:  # TEXT
                    w(struct.pack('B', TYPE_TEXT))
                    b = str(val).encode('utf-8')
                    w(struct.pack('<I', len(b)))
                    w(b)

    size = os.path.getsize(bin_path)
    print(f"Done: {len(rows)} rows x {len(col_codes)} cols, {size:,} bytes → {bin_path}", file=sys.stderr)


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <json_file> <bin_file> [--table NAME]", file=sys.stderr)
        sys.exit(1)

    json_path = sys.argv[1]
    bin_path  = sys.argv[2]
    table_name = None

    i = 3
    while i < len(sys.argv):
        if sys.argv[i] == '--table' and i+1 < len(sys.argv):
            table_name = sys.argv[i+1]; i += 2
        else:
            print(f"Unknown: {sys.argv[i]}", file=sys.stderr); sys.exit(1)

    if not os.path.exists(json_path):
        print(f"Not found: {json_path}", file=sys.stderr); sys.exit(1)

    json_to_bin(json_path, bin_path, table_name)


if __name__ == '__main__':
    main()

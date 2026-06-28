#!/usr/bin/env python3
"""
bin_to_json.py — Convert sqlrustgo BinaryTableStorage .bin to JSON.
Provides JSON representation compatible with FileStorage {table}.json.

Usage:
    python3 bin_to_json.py <bin_file> <json_file>
    python3 bin_to_json.py <bin_file> <json_file> [--table TABLE_NAME]
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


def load_bin(path):
    with open(path, 'rb') as f:
        data = f.read()
    offset = 0
    magic = data[offset:offset+4]; offset += 4
    if magic != MAGIC:
        raise ValueError(f"not a BINT file: {path}")
    ver, = struct.unpack_from('<I', data, offset); offset += 4
    col_count, = struct.unpack_from('<I', data, offset); offset += 4
    col_types = list(data[offset:offset+col_count]); offset += col_count
    row_count, = struct.unpack_from('<Q', data, offset); offset += 8
    return col_types, row_count, data, offset


def read_row(data, offset, col_count):
    vals = []
    for _ in range(col_count):
        tc = data[offset]; offset += 1
        if tc == TYPE_NULL:
            vals.append({'Null': None})
        elif tc == TYPE_INT:
            v, = struct.unpack_from('<q', data, offset); offset += 8
            vals.append({'Integer': v})
        elif tc == TYPE_FLOAT:
            v, = struct.unpack_from('<d', data, offset); offset += 8
            vals.append({'Float': v})
        elif tc == TYPE_TEXT:
            n, = struct.unpack_from('<I', data, offset); offset += 4
            v = data[offset:offset+n].decode('utf-8', errors='replace'); offset += n
            vals.append({'Text': v})
        else:
            vals.append({'Null': None})
    return vals, offset


def bin_to_json(bin_path, json_path, table_name=None):
    col_types, row_count, data, offset = load_bin(bin_path)
    col_count = len(col_types)

    # Column names: use col_0, col_1, ... (matches BinaryTableStorage::load)
    type_map = {TYPE_INT: 'INTEGER', TYPE_FLOAT: 'REAL', TYPE_TEXT: 'TEXT'}
    columns = []
    for i, tc in enumerate(col_types):
        columns.append({
            'name': f'col_{i}',
            'data_type': type_map.get(tc, 'TEXT'),
            'nullable': True,
            'primary_key': False,
            'char_max_length': None,
        })

    rows = []
    for _ in range(row_count):
        row, offset = read_row(data, offset, col_count)
        rows.append(row)

    name = table_name or os.path.splitext(os.path.basename(bin_path))[0]
    out = {'name': name, 'columns': columns, 'rows': rows}

    with open(json_path, 'w', encoding='utf-8') as f:
        json.dump(out, f, indent=2, ensure_ascii=False)

    size = os.path.getsize(json_path)
    print(f"Done: {row_count} rows x {col_count} cols, {size:,} bytes → {json_path}", file=sys.stderr)


def main():
    if len(sys.argv) < 3:
        print(f"Usage: {sys.argv[0]} <bin_file> <json_file> [--table NAME]", file=sys.stderr)
        sys.exit(1)

    bin_path = sys.argv[1]
    json_path = sys.argv[2]
    table_name = None

    i = 3
    while i < len(sys.argv):
        if sys.argv[i] == '--table' and i+1 < len(sys.argv):
            table_name = sys.argv[i+1]; i += 2
        else:
            print(f"Unknown: {sys.argv[i]}", file=sys.stderr); sys.exit(1)

    if not os.path.exists(bin_path):
        print(f"Not found: {bin_path}", file=sys.stderr); sys.exit(1)

    bin_to_json(bin_path, json_path, table_name)


if __name__ == '__main__':
    main()

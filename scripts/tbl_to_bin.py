#!/usr/bin/env python3
"""
tbl_to_bin.py — Convert TPC-H .tbl files to sqlrustgo BinaryTableStorage .bin format.

BINARY FORMAT (matches BinaryTableStorage::save / load):
  Header:
    4 bytes: magic "BINT"
    4 bytes: version u32 LE (= 1)
    4 bytes: column_count u32 LE
    N bytes: column_types (1 byte each: 1=INT, 2=FLOAT, 3=TEXT)
    8 bytes: row_count u64 LE
  Per value:
    1 byte: type_code (0=Null, 1=Integer, 2=Float, 3=Text)
    Integer: 8 bytes LE i64
    Float:   8 bytes LE f64
    Text:    4 bytes LE u32 (len) + len bytes

Usage:
    python3 tbl_to_bin.py <table> <tbl_file> <bin_file>
    python3 tbl_to_bin.py <table> <tbl_file> <bin_file> --types INT,TEXT,INT,...
"""

import sys
import struct
import os

MAGIC = b"BINT"
VERSION = 1
TYPE_NULL  = 0
TYPE_INT   = 1
TYPE_FLOAT = 2
TYPE_TEXT  = 3

TYPE_NAMES = {TYPE_INT: 'INT', TYPE_FLOAT: 'FLOAT', TYPE_TEXT: 'TEXT'}


def parse_int(s):
    s = s.strip()
    if not s:
        return None
    try:
        return int(s)
    except ValueError:
        return None


def parse_float(s):
    s = s.strip()
    if not s:
        return None
    try:
        return float(s)
    except ValueError:
        return None


def infer_types(tbl_path, num_cols):
    """Infer column types by scanning the file once. Returns list of TYPE codes."""
    type_codes = [TYPE_TEXT] * num_cols

    with open(tbl_path, 'r', encoding='utf-8', errors='replace') as f:
        for raw_line in f:
            # Filter empty trailing field (from trailing '|' + '\n')
            parts = [p for p in raw_line.removesuffix('\n').split('|') if p != '']
            if len(parts) != num_cols:
                continue
            for i, val in enumerate(parts):
                val = val.strip()
                if not val:
                    continue
                tc = type_codes[i]
                if tc == TYPE_TEXT:
                    if parse_int(val) is not None:
                        type_codes[i] = TYPE_INT
                    elif parse_float(val) is not None:
                        type_codes[i] = TYPE_FLOAT
                elif tc == TYPE_INT:
                    if parse_int(val) is None:
                        if '.' in val and parse_float(val) is not None:
                            type_codes[i] = TYPE_FLOAT
                        else:
                            type_codes[i] = TYPE_TEXT
                elif tc == TYPE_FLOAT:
                    if parse_float(val) is None:
                        type_codes[i] = TYPE_TEXT
    return type_codes


def write_bin_streamed(tbl_path, bin_path, column_types, num_cols):
    """Stream-convert tbl to bin without loading all rows into memory."""
    import io
    buf = io.BytesIO()

    def emit(data):
        buf.write(data)

    emit(MAGIC)
    emit(struct.pack('<I', VERSION))
    emit(struct.pack('<I', num_cols))
    for tc in column_types:
        emit(struct.pack('B', tc))

    row_count = 0
    with open(tbl_path, 'r', encoding='utf-8', errors='replace') as f:
        for raw_line in f:
            parts = [p for p in raw_line.removesuffix('\n').split('|') if p != '']
            if len(parts) != num_cols:
                continue
            row_count += 1
            for i, val in enumerate(parts[:num_cols]):
                val = val.strip()
                tc = column_types[i]

                if not val:
                    emit(struct.pack('B', TYPE_NULL))
                elif tc == TYPE_INT:
                    try:
                        emit(struct.pack('B', TYPE_INT))
                        emit(struct.pack('<q', int(val)))
                    except ValueError:
                        b = val.encode('utf-8')
                        emit(struct.pack('B', TYPE_TEXT))
                        emit(struct.pack('<I', len(b)))
                        emit(b)
                elif tc == TYPE_FLOAT:
                    try:
                        emit(struct.pack('B', TYPE_FLOAT))
                        emit(struct.pack('<d', float(val)))
                    except ValueError:
                        b = val.encode('utf-8')
                        emit(struct.pack('B', TYPE_TEXT))
                        emit(struct.pack('<I', len(b)))
                        emit(b)
                else:
                    b = val.encode('utf-8')
                    emit(struct.pack('B', TYPE_TEXT))
                    emit(struct.pack('<I', len(b)))
                    emit(b)

    # Write row_count at correct position (after header)
    data = buf.getvalue()
    # row_count is at offset: magic(4) + ver(4) + col_count(4) + col_types(num_cols) = 12 + num_cols
    row_count_offset = 12 + num_cols
    data = data[:row_count_offset] + struct.pack('<Q', row_count) + data[row_count_offset + 8:]
    # Write final file
    with open(bin_path, 'wb') as f:
        f.write(data)
    return row_count


def main():
    if len(sys.argv) < 4:
        print(f"Usage: {sys.argv[0]} <table> <tbl_file> <bin_file> [--types T1,T2,...]", file=sys.stderr)
        sys.exit(1)

    table     = sys.argv[1]
    tbl_path  = sys.argv[2]
    bin_path  = sys.argv[3]
    explicit_types = None

    if len(sys.argv) > 4 and sys.argv[4] == '--types':
        explicit_types = sys.argv[5].split(',')
        type_map = {'INT': TYPE_INT, 'FLOAT': TYPE_FLOAT, 'REAL': TYPE_FLOAT, 'TEXT': TYPE_TEXT, 'CHAR': TYPE_TEXT, 'VARCHAR': TYPE_TEXT}
        explicit_types = [type_map.get(t.upper(), TYPE_TEXT) for t in explicit_types]

    if not os.path.exists(tbl_path):
        print(f"Error: tbl file not found: {tbl_path}", file=sys.stderr)
        sys.exit(1)

    # Detect column count from first non-empty line
    with open(tbl_path, 'r', encoding='utf-8', errors='replace') as f:
        first_line = next((l for l in f if l.strip()), '')
    sample = [p for p in first_line.removesuffix('\n').split('|') if p != '']
    num_cols = len(sample)

    if not num_cols:
        print(f"Error: could not detect columns in {tbl_path}", file=sys.stderr)
        sys.exit(1)

    # Determine column types
    if explicit_types:
        if len(explicit_types) != num_cols:
            print(f"Error: --types has {len(explicit_types)} entries but table has {num_cols} columns", file=sys.stderr)
            sys.exit(1)
        column_types = explicit_types
    else:
        column_types = infer_types(tbl_path, num_cols)

    os.makedirs(os.path.dirname(bin_path) or '.', exist_ok=True)
    row_count = write_bin_streamed(tbl_path, bin_path, column_types, num_cols)
    size = os.path.getsize(bin_path)
    type_names = [TYPE_NAMES.get(tc, '?') for tc in column_types]
    print(f"Parsed {row_count} rows, {num_cols} columns")
    print(f"Column types: {type_names}")
    print(f"Written: {size:,} bytes → {bin_path}")


if __name__ == '__main__':
    main()

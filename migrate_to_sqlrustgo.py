#!/usr/bin/env python3
"""Migrate GMP-Platform data from SQLite (gmp.db) to sqlrustgo-storage FileStorage format."""

import json
import sqlite3
import os
from pathlib import Path

SRC_DB = "data/gmp.db"
OUT_DIR = Path("data/gmp_data")
OUT_DIR.mkdir(exist_ok=True)

conn = sqlite3.connect(SRC_DB)
conn.row_factory = sqlite3.Row

def column_defs_sqlite_to_sqlrustgo(rows, table_name):
    """Convert SQLite column names to sqlrustgo ColumnDefinition format."""
    columns = []
    for row in rows:
        col_name = row['name']
        col_type = row['type'].upper()
        if 'INT' in col_type:
            sqlrustgo_type = "INTEGER"
        elif 'REAL' in col_type or 'FLOAT' in col_type or 'DOUBLE' in col_type:
            sqlrustgo_type = "REAL"
        elif 'TEXT' in col_type or 'CHAR' in col_type or 'CLOB' in col_type:
            sqlrustgo_type = "TEXT"
        elif 'BLOB' in col_type:
            sqlrustgo_type = "BLOB"
        else:
            sqlrustgo_type = "TEXT"
        columns.append({
            "name": col_name,
            "data_type": sqlrustgo_type,
            "primary_key": bool(row['pk']),
            "not_null": bool(row['notnull']),
            "default_value": None,
            "check_constraint": None,
        })
    return columns

def write_table_json(table_name, sql_query):
    """Execute SQL query and write results to StoredTableData JSON format."""
    cursor = conn.execute(sql_query)
    rows = cursor.fetchall()

    cursor2 = conn.execute(f"PRAGMA table_info({table_name})")
    cols_info = cursor2.fetchall()
    columns = column_defs_sqlite_to_sqlrustgo(cols_info, table_name)

    values = []
    for row in rows:
        row_values = []
        for val in row:
            if val is None:
                row_values.append({"Type": "Null", "Value": 0})
            elif isinstance(val, int):
                row_values.append({"Type": "Integer", "Value": val})
            elif isinstance(val, float):
                row_values.append({"Type": "Real", "Value": val})
            elif isinstance(val, str):
                row_values.append({"Type": "Text", "Value": val})
            elif isinstance(val, bytes):
                row_values.append({"Type": "Blob", "Value": list(val)})
            else:
                row_values.append({"Type": "Text", "Value": str(val)})
        values.append(row_values)

    stored = {
        "name": table_name,
        "columns": columns,
        "foreign_keys": [],
        "unique_constraints": [],
        "rows": values,
    }

    out_path = OUT_DIR / f"{table_name}.json"
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(stored, f, indent=2, ensure_ascii=False)

    print(f"  Wrote {table_name}: {len(values)} rows to {out_path}")
    return len(values)

def migrate_hnsw_vectors():
    cursor = conn.execute("""
        SELECT h.doc_id, h.chunk_id, h.vector, h.m, h.ef_construction, h.hnsw_id
        FROM gmp_hnsw_vectors h ORDER BY h.hnsw_id
    """)
    rows = cursor.fetchall()

    columns = [
        {"name": "doc_id", "data_type": "TEXT", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "chunk_id", "data_type": "INTEGER", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "vector", "data_type": "BLOB", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "m", "data_type": "INTEGER", "primary_key": False, "not_null": False, "default_value": None, "check_constraint": None},
        {"name": "ef_construction", "data_type": "INTEGER", "primary_key": False, "not_null": False, "default_value": None, "check_constraint": None},
        {"name": "hnsw_id", "data_type": "INTEGER", "primary_key": True, "not_null": True, "default_value": None, "check_constraint": None},
    ]

    values = []
    for row in rows:
        row_values = [
            {"Type": "Text", "Value": row['doc_id']},
            {"Type": "Integer", "Value": row['chunk_id']},
            {"Type": "Blob", "Value": list(row['vector']) if row['vector'] else []},
            {"Type": "Integer", "Value": row['m'] or 0},
            {"Type": "Integer", "Value": row['ef_construction'] or 0},
            {"Type": "Integer", "Value": row['hnsw_id']},
        ]
        values.append(row_values)

    stored = {
        "name": "gmp_hnsw_vectors",
        "columns": columns,
        "foreign_keys": [],
        "unique_constraints": [],
        "rows": values,
    }

    out_path = OUT_DIR / "gmp_hnsw_vectors.json"
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(stored, f, indent=2, ensure_ascii=False)

    print(f"  Wrote gmp_hnsw_vectors: {len(values)} rows")
    return len(values)

def migrate_fts():
    cursor = conn.execute("SELECT term, doc_count, doc_ids FROM gmp_fts_index ORDER BY term")
    rows = cursor.fetchall()

    columns = [
        {"name": "term", "data_type": "TEXT", "primary_key": True, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "doc_count", "data_type": "INTEGER", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "doc_ids", "data_type": "TEXT", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
    ]

    values = []
    for row in rows:
        row_values = [
            {"Type": "Text", "Value": row['term']},
            {"Type": "Integer", "Value": row['doc_count']},
            {"Type": "Text", "Value": row['doc_ids']},
        ]
        values.append(row_values)

    stored = {
        "name": "gmp_fts_index",
        "columns": columns,
        "foreign_keys": [],
        "unique_constraints": [],
        "rows": values,
    }

    out_path = OUT_DIR / "gmp_fts_index.json"
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(stored, f, indent=2, ensure_ascii=False)

    print(f"  Wrote gmp_fts_index: {len(values)} terms")
    return len(values)

def migrate_doc_relations():
    cursor = conn.execute("SELECT doc_id, related_doc_id, relation_type, weight FROM gmp_doc_relations")
    rows = cursor.fetchall()

    columns = [
        {"name": "doc_id", "data_type": "TEXT", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "related_doc_id", "data_type": "TEXT", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "relation_type", "data_type": "TEXT", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "weight", "data_type": "REAL", "primary_key": False, "not_null": False, "default_value": None, "check_constraint": None},
    ]

    values = []
    for row in rows:
        row_values = [
            {"Type": "Text", "Value": row['doc_id']},
            {"Type": "Text", "Value": row['related_doc_id']},
            {"Type": "Text", "Value": row['relation_type']},
            {"Type": "Real", "Value": row['weight']},
        ]
        values.append(row_values)

    stored = {
        "name": "gmp_doc_relations",
        "columns": columns,
        "foreign_keys": [],
        "unique_constraints": [],
        "rows": values,
    }

    out_path = OUT_DIR / "gmp_doc_relations.json"
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(stored, f, indent=2, ensure_ascii=False)

    print(f"  Wrote gmp_doc_relations: {len(values)} rows")
    return len(values)

def migrate_doc_entities():
    cursor = conn.execute("SELECT doc_id, entity_type, entity_value, count FROM gmp_doc_entities")
    rows = cursor.fetchall()

    columns = [
        {"name": "doc_id", "data_type": "TEXT", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "entity_type", "data_type": "TEXT", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "entity_value", "data_type": "TEXT", "primary_key": False, "not_null": True, "default_value": None, "check_constraint": None},
        {"name": "count", "data_type": "INTEGER", "primary_key": False, "not_null": False, "default_value": None, "check_constraint": None},
    ]

    values = []
    for row in rows:
        row_values = [
            {"Type": "Text", "Value": row['doc_id']},
            {"Type": "Text", "Value": row['entity_type']},
            {"Type": "Text", "Value": row['entity_value']},
            {"Type": "Integer", "Value": row['count']},
        ]
        values.append(row_values)

    stored = {
        "name": "gmp_doc_entities",
        "columns": columns,
        "foreign_keys": [],
        "unique_constraints": [],
        "rows": values,
    }

    out_path = OUT_DIR / "gmp_doc_entities.json"
    with open(out_path, 'w', encoding='utf-8') as f:
        json.dump(stored, f, indent=2, ensure_ascii=False)

    print(f"  Wrote gmp_doc_entities: {len(values)} rows")
    return len(values)

def main():
    print("Migrating GMP-Platform data to sqlrustgo-storage format...")
    print(f"  Source: {SRC_DB}")
    print(f"  Destination: {OUT_DIR}")
    print()

    write_table_json("gmp_documents", "SELECT * FROM gmp_documents")
    write_table_json("gmp_document_chunks", "SELECT * FROM gmp_document_chunks")
    write_table_json("gmp_embeddings", "SELECT chunk_id, embedding_json FROM gmp_embeddings")
    migrate_doc_relations()
    migrate_doc_entities()
    migrate_fts()
    migrate_hnsw_vectors()

    print()
    print("Migration complete!")

if __name__ == "__main__":
    main()

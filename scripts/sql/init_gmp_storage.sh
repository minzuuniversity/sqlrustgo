#!/bin/bash
# ============================================================
# GMP-Platform SQLRustGo Initialization Script
# ============================================================
# Usage: ./scripts/sql/init_gmp_storage.sh <DATA_DIR>
#   DATA_DIR: directory for gmp.db (default: ./data)
#
# Creates all Layer 0-2 tables and seeds GMP terminology data.
# ============================================================

set -e

DATA_DIR="${1:-./data}"
DB_PATH="${DATA_DIR}/gmp.db"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SQL_DIR="${SCRIPT_DIR}"

echo "=== GMP-Platform SQLRustGo Initialization ==="
echo "DB: ${DB_PATH}"
echo ""

# Check for sqlite3
if ! command -v sqlite3 &> /dev/null; then
    echo "ERROR: sqlite3 not found. Please install sqlite3."
    exit 1
fi

# Create data directory if needed
mkdir -p "${DATA_DIR}"

# Run schema creation
echo "[1/3] Creating schema..."
sqlite3 "${DB_PATH}" < "${SQL_DIR}/schema/gmp_schema.sql"
echo "      Schema created: $(sqlite3 "${DB_PATH}" 'SELECT COUNT(*) FROM sqlite_master WHERE type=\"table\";') tables"

# Run GMP terms data
echo "[2/3] Loading GMP terminology..."
sqlite3 "${DB_PATH}" < "${SQL_DIR}/data/gmp_terms.sql"
echo "      Terms loaded: $(sqlite3 "${DB_PATH}" 'SELECT COUNT(*) FROM gmp_terms;') terms"
echo "      Synonyms loaded: $(sqlite3 "${DB_PATH}" 'SELECT COUNT(*) FROM term_synonyms;') synonyms"
echo "      Hierarchy loaded: $(sqlite3 "${DB_PATH}" 'SELECT COUNT(*) FROM term_hierarchy;') hierarchy"

# Run relation and taxonomy rules
echo "[3/3] Loading relation templates and taxonomy rules..."
sqlite3 "${DB_PATH}" < "${SQL_DIR}/data/gmp_relations_taxonomy.sql"
echo "      Relations loaded: $(sqlite3 "${DB_PATH}" 'SELECT COUNT(*) FROM relation_templates;') templates"
echo "      Taxonomy rules loaded: $(sqlite3 "${DB_PATH}" 'SELECT COUNT(*) FROM taxonomy_rules;') rules"

echo ""
echo "=== Initialization Complete ==="
echo ""
echo "Table counts:"
sqlite3 -header -column "${DB_PATH}" \
    "SELECT 'gmp_terms' as table_name, COUNT(*) as cnt FROM gmp_terms
     UNION ALL SELECT 'term_synonyms', COUNT(*) FROM term_synonyms
     UNION ALL SELECT 'term_hierarchy', COUNT(*) FROM term_hierarchy
     UNION ALL SELECT 'extraction_rules', COUNT(*) FROM extraction_rules
     UNION ALL SELECT 'relation_templates', COUNT(*) FROM relation_templates
     UNION ALL SELECT 'taxonomy_rules', COUNT(*) FROM taxonomy_rules
     UNION ALL SELECT 'gmp_term_patterns', COUNT(*) FROM gmp_term_patterns
     UNION ALL SELECT 'gmp_documents', COUNT(*) FROM gmp_documents
     UNION ALL SELECT 'gmp_document_chunks', COUNT(*) FROM gmp_document_chunks"

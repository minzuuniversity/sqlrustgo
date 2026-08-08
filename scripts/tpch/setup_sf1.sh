#!/usr/bin/env bash
# setup_sf1.sh — Generate TPC-H SF=1.0 test data
#
# Usage:
#   bash scripts/tpch/setup_sf1.sh [DATA_DIR]
#
# Generates ~1GB of TPC-H test data using dbgen.
# Requires dbgen to be built.
#
# If dbgen is not available, prints instructions for installation.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

DATA_DIR="${1:-/tmp/tpch-sf1}"

echo "TPC-H SF=1.0 setup"
echo "Data dir: $DATA_DIR"
echo "=========================================="

# Check if dbgen is available
DBGEN_PATH=""
if command -v dbgen >/dev/null 2>&1; then
    DBGEN_PATH="$(command -v dbgen)"
elif [ -x "/home/openclaw/tpch-dbgen-master/dbgen" ]; then
    DBGEN_PATH="/home/openclaw/tpch-dbgen-master/dbgen"
elif [ -x "$HOME/tpch-dbgen-master/dbgen" ]; then
    DBGEN_PATH="$HOME/tpch-dbgen-master/dbgen"
fi

if [ -z "$DBGEN_PATH" ]; then
    echo "ERROR: dbgen not found"
    echo ""
    echo "Install dbgen:"
    echo "  git clone https://github.com/electrum/tpch-dbgen.git ~/tpch-dbgen"
    echo "  cd ~/tpch-dbgen && make"
    echo ""
    echo "Or download from TPC-H official site."
    echo ""
    echo "After dbgen is built, run:"
    echo "  bash $0 $DATA_DIR"
    exit 1
fi

echo "Using dbgen at: $DBGEN_PATH"

mkdir -p "$DATA_DIR"
DBGEN_TMP="$(mktemp -d)"
cp "$(dirname "$DBGEN_PATH")/dists.dss" "$DBGEN_TMP/"
cd "$DBGEN_TMP"

# Generate SF=1.0 data. dbgen resolves dists.dss from its working directory.
"$DBGEN_PATH" -s 1 -f

if [ $? -ne 0 ]; then
    echo "ERROR: dbgen failed"
    rm -rf "$DBGEN_TMP"
    exit 1
fi

# Move generated files into the requested directory and make them readable.
mv ./*.tbl "$DATA_DIR/"
chmod u+rw "$DATA_DIR"/*.tbl
cd "$REPO_ROOT"
rm -rf "$DBGEN_TMP"


echo ""
echo "TPC-H SF=1.0 data generated at: $DATA_DIR"
ls -la "$DATA_DIR"

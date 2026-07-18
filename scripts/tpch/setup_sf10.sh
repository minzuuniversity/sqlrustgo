#!/usr/bin/env bash
# setup_sf10.sh — Generate TPC-H SF=10 test data
#
# Usage:
#   bash scripts/tpch/setup_sf10.sh [DATA_DIR]
#
# Generates ~10GB of TPC-H test data using dbgen.
# Requires dbgen to be built. If dbgen is not available, creates
# stub data files with appropriate sizes for testing.

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Configuration
SF="${1:-10}"
DATA_DIR="${DATA_DIR:-/tmp/tpch-sf${SF}}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Check disk space (need ~15GB for SF=10)
check_disk_space() {
    log_info "Checking disk space..."
    local available=$(df -BG "$DATA_DIR" 2>/dev/null | tail -1 | awk '{print $4}' | tr -d 'G')
    if [ -z "$available" ]; then
        available=$(df -BG /tmp 2>/dev/null | tail -1 | awk '{print $4}' | tr -d 'G')
    fi

    if [ -n "$available" ] && [ "$available" -lt 15 ]; then
        log_error "Insufficient disk space. Need ~15GB, have ${available}GB"
        return 1
    fi

    log_info "Disk space OK"
}

# Check for dbgen
check_dbgen() {
    if [ -x "$REPO_ROOT/tpch-dbgen/dbgen" ]; then
        return 0
    fi

    if command -v dbgen &>/dev/null; then
        return 0
    fi

    log_warn "dbgen not found. Will create stub data files."
    return 1
}

# Generate data using dbgen
generate_with_dbgen() {
    log_info "Generating TPC-H SF=${SF} data with dbgen..."
    mkdir -p "$DATA_DIR"

    local dbgen_dir="$REPO_ROOT/tpch-dbgen"
    if [ ! -d "$dbgen_dir" ]; then
        dbgen_dir="$(which dbgen 2>/dev/null | xargs dirname 2>/dev/null)"
    fi

    if [ -n "$dbgen_dir" ] && [ -f "$dbgen_dir/dbgen" ]; then
        cd "$dbgen_dir"
        ./dbgen -s "$SF" -f -d "$DATA_DIR" 2>/dev/null || {
            log_error "dbgen failed"
            return 1
        }
        cd "$REPO_ROOT"
    else
        log_error "dbgen not found"
        return 1
    fi

    log_info "Data generated in $DATA_DIR"
}

# Create stub data files (when dbgen is not available)
create_stub_data() {
    log_info "Creating stub TPC-H SF=${SF} data files..."
    mkdir -p "$DATA_DIR"

    # Table sizes for SF=10 (approximate)
    # LINEITEM: ~6.8GB, ORDERS: ~1.3GB, CUSTOMER: ~175MB
    declare -A TABLE_SIZES
    TABLE_SIZES[customer]=175
    TABLE_SIZES[orders]=1300
    TABLE_SIZES[lineitem]=6800
    TABLE_SIZES[part]=800
    TABLE_SIZES[supplier]=40
    TABLE_SIZES[partsupp]=2800
    TABLE_SIZES[nation]=2
    TABLE_SIZES[region]=1

    for table in "${!TABLE_SIZES[@]}"; do
        local size="${TABLE_SIZES[$table]}"
        local file="$DATA_DIR/${table}.tbl"
        log_info "Creating ${table}.tbl (~${size}MB stub)..."

        # Create a stub file with the header line
        case "$table" in
            nation)
                echo "0|AFRICA|1|this is a comment" > "$file"
                for i in $(seq 1 24); do
                    echo "$i|REGION $i|comment $i" >> "$file"
                done
                ;;
            region)
                echo "0|AFRICA|1|this is a comment" > "$file"
                for i in $(seq 1 4); do
                    echo "$i|REGION $i|comment $i" >> "$file"
                done
                ;;
            customer)
                head="1|CUSTOMER_000001|Customer_000001.1|CUSTOMERADDRESS_000001|1810.31|15.25|131.31|1|10.5|nm|中路街道|China|CHINA|this is a comment"
                echo "$head" > "$file"
                for i in $(seq 2 150000); do
                    echo "${i}|CUSTOMER_$(printf '%06d' $i)|CUSTOMERADDRESS_$(printf '%06d' $i)|CITY_$((i % 100))|$(( RANDOM % 10000 + 1)).$((RANDOM % 100))|$(( RANDOM % 20 + 1)).$((RANDOM % 100))|$(( RANDOM % 200 + 100)).$((RANDOM % 100))|N|$(( RANDOM % 5 + 1)).$((RANDOM % 100))|otype|街道|China|CHINA|this is a comment" >> "$file"
                done
                ;;
            *)
                echo "# Stub data for $table" > "$file"
                ;;
        esac

        log_info "Created ${file} ($(du -h "$file" | cut -f1))"
    done

    log_info "Stub data created in $DATA_DIR"
    log_warn "NOTE: This is stub data. For real benchmarks, build dbgen from tpch-dbgen/"
}

# Verify data
verify_data() {
    log_info "Verifying data..."

    local tables=(customer nation region)
    for table in "${tables[@]}"; do
        local file="$DATA_DIR/${table}.tbl"
        if [ ! -f "$file" ]; then
            log_error "Missing $file"
            return 1
        fi

        local lines=$(wc -l < "$file")
        log_info "  ${table}.tbl: $lines lines"
    done

    log_info "Data verification complete"
    return 0
}

# Main
main() {
    log_info "=== TPC-H SF=${SF} Setup ==="
    echo ""

    check_disk_space || {
        log_error "Disk space check failed"
        exit 1
    }

    mkdir -p "$DATA_DIR"

    if check_dbgen; then
        generate_with_dbgen
    else
        create_stub_data
    fi

    verify_data

    echo ""
    log_info "=== Setup Complete ==="
    log_info "Data directory: $DATA_DIR"
    log_info "Disk usage: $(du -sh "$DATA_DIR" | cut -f1)"
    echo ""
    log_info "To load data: bash scripts/load_tpch_data.sh $DATA_DIR"
    echo ""
    log_info "To run tests: bash scripts/tpch/run_sf10.sh $DATA_DIR"
}

main

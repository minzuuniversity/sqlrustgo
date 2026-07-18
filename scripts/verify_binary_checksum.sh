#!/usr/bin/env bash
# verify_binary_checksum.sh — Verify SHA256 checksum of a release binary
#
# Usage:
#   bash scripts/verify_binary_checksum.sh [DIRECTORY]
#
# Default directory: sqlrustgo-*-*/
#
# Exit code: 0 = PASS, non-zero = FAIL

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Configuration
DIR="${1:-.}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

verify() {
    local dir="$1"
    local binary="$dir/sqlrustgo"
    local checksum_file="$dir/sqlrustgo.sha256"

    if [ ! -f "$checksum_file" ]; then
        log_error "Checksum file not found: $checksum_file"
        return 1
    fi

    if [ ! -f "$binary" ]; then
        log_error "Binary not found: $binary"
        return 1
    fi

    log_info "Verifying $binary..."

    if command -v sha256sum &>/dev/null; then
        if sha256sum --check "$checksum_file" 2>/dev/null; then
            log_info "SHA256 verification PASSED"
            return 0
        else
            log_error "SHA256 verification FAILED"
            return 1
        fi
    elif command -v shasum &>/dev/null; then
        if shasum -a 256 --check "$checksum_file" 2>/dev/null; then
            log_info "SHA256 verification PASSED"
            return 0
        else
            log_error "SHA256 verification FAILED"
            return 1
        fi
    else
        log_error "No SHA256 tool available (sha256sum or shasum)"
        return 1
    fi
}

# Main
if [ -d "$DIR" ] && [ -f "$DIR/sqlrustgo.sha256" ]; then
    if verify "$DIR"; then
        exit 0
    else
        exit 1
    fi
else
    # Find all directories with checksum files
    found=0
    for d in sqlrustgo-*/; do
        if [ -f "$d/sqlrustgo.sha256" ]; then
            log_info "Found: $d"
            if verify "$d"; then
                found=$((found + 1))
            fi
        fi
    done

    if [ $found -gt 0 ]; then
        exit 0
    else
        log_error "No release directory with checksum found"
        echo "Usage: $0 [DIRECTORY]"
        exit 1
    fi
fi

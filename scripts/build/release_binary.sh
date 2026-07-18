#!/usr/bin/env bash
# release_binary.sh — Build a reproducible release binary with SHA256 checksum
#
# Usage:
#   bash scripts/build/release_binary.sh [VERSION] [TARGET]
#
# Environment:
#   VERSION - Version string (default: 3.11.0)
#   TARGET  - Build target (default: auto-detected from rustc)
#
# Outputs:
#   sqlrustgo-{VERSION}-{TARGET}/
#     ├── sqlrustgo            (binary)
#     ├── sqlrustgo.sha256    (checksum)
#     └── VERSION             (build metadata)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

# Configuration
VERSION="${VERSION:-3.11.0}"
TARGET="${TARGET:-$(rustc -vV | grep host | cut -d' ' -f2)}"
BUILD_TIME="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
GIT_COMMIT="$(git rev-parse HEAD 2>/dev/null || echo "unknown")"
OUTPUT_DIR="sqlrustgo-${VERSION}-${TARGET}"
PROFILE="${PROFILE:-release}"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

log_info() { echo -e "${GREEN}[INFO]${NC} $*"; }
log_error() { echo -e "${RED}[ERROR]${NC} $*"; }

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    if [ ! -f "$REPO_ROOT/Cargo.lock" ]; then
        log_error "Cargo.lock not found. Run 'cargo build' first."
        exit 1
    fi

    if ! command -v sha256sum &>/dev/null && ! command -v shasum &>/dev/null; then
        log_error "sha256sum or shasum not found"
        exit 1
    fi

    log_info "Prerequisites OK"
}

# Build the binary
build_binary() {
    log_info "Building release binary..."
    log_info "  Version: $VERSION"
    log_info "  Target: $TARGET"
    log_info "  Git:    $GIT_COMMIT"
    log_info "  Time:   $BUILD_TIME"

    # Clean any previous build
    cargo clean -p sqlrustgo 2>/dev/null || true

    # Build
    cargo build --release --target "$TARGET" -p sqlrustgo 2>/dev/null || \
    cargo build --release -p sqlrustgo 2>/dev/null || {
        log_error "Build failed"
        exit 1
    }

    log_info "Build complete"
}

# Create output directory
create_output() {
    log_info "Creating output directory: $OUTPUT_DIR"
    rm -rf "$OUTPUT_DIR"
    mkdir -p "$OUTPUT_DIR"

    # Find the binary
    local binary=""
    if [ -f "target/$TARGET/$PROFILE/sqlrustgo" ]; then
        binary="target/$TARGET/$PROFILE/sqlrustgo"
    elif [ -f "target/$PROFILE/sqlrustgo" ]; then
        binary="target/$PROFILE/sqlrustgo"
    else
        log_error "Binary not found"
        exit 1
    fi

    # Copy binary
    cp "$binary" "$OUTPUT_DIR/sqlrustgo"
    chmod +x "$OUTPUT_DIR/sqlrustgo"

    # Strip debug symbols (optional, reduces size)
    if command -v strip &>/dev/null; then
        strip "$OUTPUT_DIR/sqlrustgo" 2>/dev/null || true
    fi

    log_info "Binary: $OUTPUT_DIR/sqlrustgo ($(du -h "$OUTPUT_DIR/sqlrustgo" | cut -f1))"
}

# Generate SHA256 checksum
generate_checksum() {
    log_info "Generating SHA256 checksum..."

    local checksum_file="$OUTPUT_DIR/sqlrustgo.sha256"
    local binary_path="$OUTPUT_DIR/sqlrustgo"

    if command -v sha256sum &>/dev/null; then
        sha256sum "$binary_path" > "$checksum_file"
    elif command -v shasum &>/dev/null; then
        shasum -a 256 "$binary_path" > "$checksum_file"
    fi

    log_info "Checksum: $checksum_file"
    cat "$checksum_file"
}

# Generate VERSION file
generate_version_file() {
    log_info "Generating VERSION file..."

    cat > "$OUTPUT_DIR/VERSION" << EOF
VERSION=${VERSION}
GIT_COMMIT=${GIT_COMMIT}
BUILD_TIME=${BUILD_TIME}
TARGET=${TARGET}
RUSTC=$(rustc -V 2>/dev/null || echo "unknown")
CARGO=$(cargo -V 2>/dev/null || echo "unknown")
EOF

    log_info "VERSION file:"
    cat "$OUTPUT_DIR/VERSION"
}

# Verify the build
verify_build() {
    log_info "Verifying build..."

    local checksum_file="$OUTPUT_DIR/sqlrustgo.sha256"

    if command -v sha256sum &>/dev/null; then
        if sha256sum --check "$checksum_file"; then
            log_info "Checksum verification PASSED"
        else
            log_error "Checksum verification FAILED"
            exit 1
        fi
    elif command -v shasum &>/dev/null; then
        if shasum -a 256 --check "$checksum_file" 2>/dev/null; then
            log_info "Checksum verification PASSED"
        else
            log_error "Checksum verification FAILED"
            exit 1
        fi
    fi
}

# Main
main() {
    log_info "=== Release Binary Builder ==="
    echo ""

    check_prerequisites
    build_binary
    create_output
    generate_checksum
    generate_version_file
    verify_build

    echo ""
    log_info "=== Build Complete ==="
    log_info "Output directory: $OUTPUT_DIR"
    log_info "Binary: $OUTPUT_DIR/sqlrustgo"
    log_info "Checksum: $OUTPUT_DIR/sqlrustgo.sha256"
    log_info "Version: $OUTPUT_DIR/VERSION"
}

main

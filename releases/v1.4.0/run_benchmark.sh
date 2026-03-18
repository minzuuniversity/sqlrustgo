#!/bin/bash
#
# SQLRustGo v1.4.0 性能基准测试脚本
#

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

VERSION="1.4.0"
PROJECT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

show_help() {
    cat << EOF
SQLRustGo v${VERSION} 性能基准测试

用法:
  $0 [选项]

选项:
  --all       运行所有基准测试 (默认)
  --quick     仅运行快速基准测试
  --help      显示帮助信息
EOF
}

check_environment() {
    log_info "检查环境..."
    if ! command -v cargo &> /dev/null; then
        log_error "Rust 未安装"
        exit 1
    fi
    log_success "环境检查通过"
}

bench_aggregate() {
    log_info "========== 聚合基准测试 =========="
    cd "$PROJECT_DIR"
    cargo bench --benches aggregate 2>&1 | tail -20
    log_success "聚合基准测试完成"
}

bench_lexer() {
    log_info "========== 词法分析基准测试 =========="
    cd "$PROJECT_DIR"
    cargo bench --benches lexer 2>&1 | tail -20
    log_success "词法分析基准测试完成"
}

run_all() {
    check_environment
    bench_aggregate
    bench_lexer
    log_success "所有基准测试完成"
}

main() {
    local mode="all"

    while [ $# -gt 0 ]; do
        case "$1" in
            --quick) mode="quick" ;;
            --all) mode="all" ;;
            --help|-h) show_help; exit 0 ;;
            *) show_help; exit 1 ;;
        esac
        shift
    done

    echo ""
    echo "========================================"
    echo "  SQLRustGo v${VERSION} 性能基准测试"
    echo "========================================"
    echo ""

    run_all

    echo ""
    echo "========================================"
    echo "  基准测试完成"
    echo "========================================"
    echo ""
}

main "$@"

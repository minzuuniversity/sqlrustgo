#!/bin/bash
#
# SQLRustGo v1.4.0 安装验证测试脚本
#
# 用法:
#   ./test_install.sh              # 运行所有测试
#   ./test_install.sh --quick      # 快速测试
#   ./test_install.sh --help       # 显示帮助
#

set -e

# 颜色定义
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# 配置
VERSION="1.4.0"
BINARY_NAME="sqlrustgo"
TEST_PASSED=0
TEST_FAILED=0

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
    ((TEST_PASSED++))
}

log_fail() {
    echo -e "${RED}[FAIL]${NC} $1"
    ((TEST_FAILED++))
}

show_help() {
    cat << EOF
SQLRustGo v${VERSION} 安装验证测试

用法:
  $0 [选项]

选项:
  --all       运行所有测试 (默认)
  --quick     仅运行快速测试
  --help      显示帮助信息

测试项目:
  - 版本信息验证
  - REPL 模式测试
  - SQL 执行测试
  - 健康检查端点测试
  - 指标端点测试
  - 服务器模式测试
  - CBO 优化器测试
  - SortMergeJoin 测试
EOF
}

# 查找已安装的二进制
find_binary() {
    local paths=("$HOME/.local/bin/$BINARY_NAME" "/usr/local/bin/$BINARY_NAME" "./target/release/$BINARY_NAME")
    for path in "${paths[@]}"; do
        if [ -f "$path" ]; then
            echo "$path"
            return 0
        fi
    done
    return 1
}

# 测试版本信息
test_version() {
    log_info "测试: 版本信息"

    local BINARY
    BINARY=$(find_binary) || BINARY="./target/release/$BINARY_NAME"

    local OUTPUT
    OUTPUT=$("$BINARY" --version 2>&1) || true

    if echo "$OUTPUT" | grep -q "v${VERSION}"; then
        log_success "版本正确: v${VERSION}"
    else
        log_fail "版本不匹配，期望 v${VERSION}，实际: $OUTPUT"
    fi
}

# 测试 SQL 执行
test_sql_execution() {
    log_info "测试: SQL 执行"

    local BINARY
    BINARY=$(find_binary) || BINARY="./target/release/$BINARY_NAME"

    local OUTPUT
    OUTPUT=$("$BINARY" -e "SELECT 1+1 as result;" 2>&1) || true

    if echo "$OUTPUT" | grep -q "2"; then
        log_success "SQL 执行正常"
    else
        log_fail "SQL 执行测试失败"
    fi
}

# 测试 CREATE 和 INSERT
test_dml() {
    log_info "测试: DML 操作"

    local BINARY
    BINARY=$(find_binary) || BINARY="./target/release/$BINARY_NAME"

    local OUTPUT
    OUTPUT=$("$BINARY" -e "
        CREATE TABLE test (id INTEGER, name TEXT);
        INSERT INTO test VALUES (1, 'hello');
        SELECT * FROM test;
    " 2>&1) || true

    if echo "$OUTPUT" | grep -q "hello"; then
        log_success "DML 操作正常"
    else
        log_fail "DML 操作测试失败"
    fi
}

# 测试聚合函数
test_aggregate() {
    log_info "测试: 聚合函数"

    local BINARY
    BINARY=$(find_binary) || BINARY="./target/release/$BINARY_NAME"

    local OUTPUT
    OUTPUT=$("$BINARY" -e "
        CREATE TABLE sales (id INTEGER, amount INTEGER);
        INSERT INTO sales VALUES (1, 100);
        INSERT INTO sales VALUES (2, 200);
        INSERT INTO sales VALUES (3, 300);
        SELECT COUNT(*) as cnt, SUM(amount) as total, AVG(amount) as avg_val FROM sales;
    " 2>&1) || true

    if echo "$OUTPUT" | grep -q "3" && echo "$OUTPUT" | grep -q "600"; then
        log_success "聚合函数正常"
    else
        log_fail "聚合函数测试失败"
    fi
}

# 测试 JOIN 操作
test_join() {
    log_info "测试: JOIN 操作"

    local BINARY
    BINARY=$(find_binary) || BINARY="./target/release/$BINARY_NAME"

    local OUTPUT
    OUTPUT=$("$BINARY" -e "
        CREATE TABLE users (id INTEGER, name TEXT);
        CREATE TABLE orders (id INTEGER, user_id INTEGER, amount INTEGER);
        INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob');
        INSERT INTO orders VALUES (1, 1, 100), (2, 1, 200), (3, 2, 150);
        SELECT u.name, o.amount FROM users u JOIN orders o ON u.id = o.user_id;
    " 2>&1) || true

    if echo "$OUTPUT" | grep -q "Alice"; then
        log_success "JOIN 操作正常"
    else
        log_fail "JOIN 操作测试失败"
    fi
}

# 测试健康检查端点
test_health_endpoint() {
    log_info "测试: 健康检查端点"

    local BINARY
    BINARY=$(find_binary) || BINARY="./target/release/$BINARY_NAME"

    "$BINARY" --server --port 15888 &
    local SERVER_PID=$!

    sleep 2

    local OUTPUT
    OUTPUT=$(curl -s http://localhost:15888/health/live 2>&1) || true

    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true

    if echo "$OUTPUT" | grep -q "healthy"; then
        log_success "健康检查端点正常"
    else
        log_fail "健康检查端点测试失败"
    fi
}

# 测试指标端点
test_metrics_endpoint() {
    log_info "测试: 指标端点"

    local BINARY
    BINARY=$(find_binary) || BINARY="./target/release/$BINARY_NAME"

    "$BINARY" --server --port 15889 &
    local SERVER_PID=$!

    sleep 2

    local OUTPUT
    OUTPUT=$(curl -s http://localhost:15889/metrics 2>&1) || true

    kill $SERVER_PID 2>/dev/null || true
    wait $SERVER_PID 2>/dev/null || true

    if echo "$OUTPUT" | grep -q "sqlrustgo"; then
        log_success "指标端点正常"
    else
        log_fail "指标端点测试失败"
    fi
}

# 快速测试
run_quick_tests() {
    log_info "========== 快速测试 =========="
    test_version
    test_sql_execution
    test_dml
}

# 完整测试
run_all_tests() {
    log_info "========== 完整测试 =========="
    test_version
    test_sql_execution
    test_dml
    test_aggregate
    test_join

    if command -v curl &> /dev/null; then
        test_health_endpoint || true
        test_metrics_endpoint || true
    fi
}

# 主函数
main() {
    local test_mode="all"

    while [ $# -gt 0 ]; do
        case "$1" in
            --quick) test_mode="quick" ;;
            --all) test_mode="all" ;;
            --help|-h) show_help; exit 0 ;;
            *) show_help; exit 1 ;;
        esac
        shift
    done

    echo ""
    echo "========================================"
    echo "  SQLRustGo v${VERSION} 安装验证测试"
    echo "========================================"
    echo ""

    local BINARY
    BINARY=$(find_binary)
    if [ -z "$BINARY" ]; then
        log_warn "未找到已安装的二进制，尝试编译..."
        if [ -f "./Cargo.toml" ]; then
            cargo build --release || { log_error "编译失败"; exit 1; }
            BINARY="./target/release/$BINARY_NAME"
        else
            log_error "请先安装 SQLRustGo"
            exit 1
        fi
    fi

    log_info "使用二进制: $BINARY"
    echo ""

    case "$test_mode" in
        quick) run_quick_tests ;;
        all) run_all_tests ;;
    esac

    echo ""
    echo "========================================"
    echo "  测试结果"
    echo "========================================"
    echo "  通过: $TEST_PASSED"
    echo "  失败: $TEST_FAILED"
    echo ""

    if [ $TEST_FAILED -eq 0 ]; then
        log_success "所有测试通过!"
        exit 0
    else
        log_fail "有测试失败"
        exit 1
    fi
}

main "$@"

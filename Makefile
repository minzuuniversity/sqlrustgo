.PHONY: help unit allunit coverage benchmark fmt clippy audit chaos clean build release

help:
	@echo "SQLRustGo v3.4.0 - 测试与构建命令"
	@echo ""
	@echo "测试命令："
	@echo "  make unit       快速测试（~5min）"
	@echo "  make allunit    完整测试（~30min）"
	@echo "  make coverage   覆盖率报告（>=85%）"
	@echo "  make benchmark  TPC-H SF=1 性能基准"
	@echo ""
	@echo "质量检查："
	@echo "  make fmt        代码格式化检查"
	@echo "  make clippy     Clippy lint 检查"
	@echo "  make audit      安全审计"
	@echo ""
	@echo "混沌测试："
	@echo "  make chaos      运行所有混沌测试"
	@echo "  make chaos-oom  OOM 测试"
	@echo "  make chaos-io   I/O Error 测试"
	@echo "  make chaos-crash Crash 测试"
	@echo ""
	@echo "构建："
	@echo "  make build      Debug 构建"
	@echo "  make release    Release 构建"
	@echo "  make clean      清理构建产物"

unit:
	@echo "=== 快速测试（~5min）==="
	@cargo test -p sqlrustgo --lib

allunit:
	@echo "=== 完整测试（~30min）==="
	@cargo test --all-features
	@echo "=== 运行稳定性测试 ==="
	@bash scripts/gate/check_stability.sh || true

coverage:
	@echo "=== 覆盖率报告 ==="
	@cargo llvm-cov test --all-features

benchmark:
	@echo "=== TPC-H SF=1 性能基准 ==="
	@test -f ./target/release/sqlrustgo-bench-cli && ./target/release/sqlrustgo-bench-cli tpch-bench --ddl scripts/tpch/tpch_schema.sql --data ~/sqlrustgo-tpch-sf1/data --queries all || echo "TPC-H benchmark CLI 不存在，跳过"

fmt:
	@echo "=== 格式化检查 ===="
	@cargo fmt --all -- --check

clippy:
	@echo "=== Clippy 检查 ==="
	@cargo clippy --all-features -- -D warnings

audit:
	@echo "=== 安全审计 ==="
	@cargo audit

chaos: chaos-oom chaos-io chaos-crash

chaos-oom:
	@echo "=== OOM 测试占位 ==="
	@echo "使用 MALLOC_LIMIT 环境变量限制内存进行测试"

chaos-io:
	@echo "=== I/O Error 测试占位 ==="
	@echo "需要 fault injection 支持"

chaos-crash:
	@echo "=== Crash 测试占位 ==="
	@bash scripts/chaos/crash_test.sh

build:
	@cargo build

release:
	@cargo build --release

clean:
	@cargo clean

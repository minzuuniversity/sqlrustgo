.PHONY: help unit allunit coverage benchmark qps perf perf-baseline fuzz fuzz-quick changelog fmt clippy audit chaos clean build release

help:
	@echo "SQLRustGo v3.4.0 - 测试与构建命令"
	@echo ""
	@echo "测试命令："
	@echo "  make unit       快速测试（~5min）"
	@echo "  make allunit    完整测试（~30min）"
	@echo "  make coverage   覆盖率报告（>=85%）"
	@echo "  make benchmark  TPC-H SF=1 性能基准"
	@echo "  make qps        QPS 基准测试（需要 --ignored）"
	@echo ""
	@echo "性能回归："
	@echo "  make perf       性能回归检测（vs baseline）"
	@echo "  make perf-baseline 建立当前性能基准"
	@echo ""
	@echo "混沌测试："
	@echo "  make chaos      运行所有混沌测试"
	@echo "  make chaos-oom  OOM 测试"
	@echo "  make chaos-io   I/O Error 测试"
	@echo "  make chaos-crash Crash 测试"
	@echo ""
	@echo "模糊测试："
	@echo "  make fuzz       Fuzz 测试（10000 rounds）"
	@echo "  make fuzz-quick Fuzz 快速测试（1000 rounds）"
	@echo ""
	@echo "文档："
	@echo "  make changelog  查看变更日志"
	@echo ""
	@echo "质量检查："
	@echo "  make fmt        代码格式化检查"
	@echo "  make clippy     Clippy lint 检查"
	@echo "  make audit      安全审计"
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

qps:
	@echo "=== QPS 基准测试 ==="
	@cargo test --test qps_benchmark_test -- --ignored --nocapture 2>&1 | grep -E "(QPS:|test result)" || echo "Hint: cargo test --test qps_benchmark_test -- --ignored --nocapture"

perf:
	@echo "=== Performance Regression Check ==="
	@bash scripts/gate/check_perf_baseline.sh

perf-baseline:
	@echo "=== Establishing Performance Baseline ==="
	@bash scripts/gate/check_perf_baseline.sh --skip-run
	@echo ""
	@echo "Baseline established. Run 'make perf' to compare future results."

fuzz:
	@echo "=== Running Fuzz Tests ==="
	@cargo run -p sqlrustgo-fuzz --bin row_format_fuzz -- 10000 2>&1 | tail -20

fuzz-quick:
	@echo "=== Running Fuzz Tests (quick) ==="
	@cargo run -p sqlrustgo-fuzz --bin row_format_fuzz -- 1000 2>&1 | tail -20

changelog:
	@echo "=== CHANGELOG ==="
	@echo "Root CHANGELOG.md:"
	@head -30 CHANGELOG.md
	@echo ""
	@echo "v3.4.0 CHANGELOG:"
	@head -30 docs/releases/v3.4.0/CHANGELOG.md
	@echo ""
	@echo "For automated changelog generation, install git-cliff:"
	@echo "  cargo install git-cliff"

fmt:
	@echo "=== 格式化检查 ==="
	@cargo fmt --all -- --check

clippy:
	@echo "=== Clippy 检查 ==="
	@cargo clippy --all-features -- -D warnings

audit:
	@echo "=== 安全审计 ==="
	@cargo audit

chaos: chaos-oom chaos-io chaos-crash

chaos-oom:
	@echo "=== OOM 混沌测试 ==="
	@bash scripts/chaos/test_oom.sh

chaos-io:
	@echo "=== I/O Error 混沌测试 ==="
	@bash scripts/chaos/test_io_error.sh

chaos-crash:
	@echo "=== Crash 混沌测试 ==="
	@bash scripts/chaos/test_crash.sh

build:
	@cargo build

release:
	@cargo build --release

clean:
	@cargo clean
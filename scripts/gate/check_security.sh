#!/usr/bin/env bash
# ==========================================================================
# GATE-DISPOSITION-SELF-MARK —— 本文件**不会在权威轮里被执行**，登记结论就地写在这里
#
# 分类：孤儿门禁（orphan_referenced）—— 只在其它脚本正文里被引用，没有任何执行点
# 处置：待接入执行点（needs-execution-point）
# 责任任务：P0-2
# 登记说明：判据可在仓内自足执行，但不在任何阶段清单/CI 执行点 ⇒ P0-2 定义执行点后必须挂上，否则撤声称
# 效果声明：尚无可证伪夹具（NEG_FIXTURES 里本门禁没有可证伪行），因此不声称能抓任何东西 —— 无声明的「能抓」比错的「能抓」更坏。
#
# 为什么保留文件而不是删掉：删文件会改变门禁总数（当前 168，固定不可动），
# 而「不执行」是**登记在册的已知状态**，不是遗漏。删除会让「这一项为什么
# 没有判定」变成不可追溯。因此：**保留文件 + 就地自标 + 登记在册**。
#
# 权威登记：docs/releases/v4.1.0/GATE_CATALOG.jsonl（同一 gate 键）
# ==========================================================================

set -e

echo "=== Running Security Gate Check ==="

# 创建安全扫描报告目录
mkdir -p docs/releases/v1.0.0-rc1

# 运行安全扫描
echo "Running security audit..."
cargo audit > docs/releases/v1.0.0-rc1/security_audit_output.txt

# 检查安全扫描结果
if [ $? -ne 0 ]; then
    echo "❌ Security audit failed"
    exit 1
fi

# 检查是否有高危漏洞
echo "Checking for critical vulnerabilities..."
CRITICAL_COUNT=$(grep -c "critical" docs/releases/v1.0.0-rc1/security_audit_output.txt -i)
HIGH_COUNT=$(grep -c "high" docs/releases/v1.0.0-rc1/security_audit_output.txt -i)

echo "Critical vulnerabilities: ${CRITICAL_COUNT}"
echo "High vulnerabilities: ${HIGH_COUNT}"

if [ "$CRITICAL_COUNT" -gt 0 ]; then
    echo "❌ Critical vulnerabilities found!"
    exit 1
fi

if [ "$HIGH_COUNT" -gt 2 ]; then
    echo "❌ Too many high vulnerabilities! Maximum allowed: 2"
    exit 1
fi

# 运行依赖过期检查
echo "Running dependency outdated check..."
cargo outdated > docs/releases/v1.0.0-rc1/dependency_outdated_output.txt

# 检查依赖过期情况
OUTDATED_COUNT=$(grep -c "outdated" docs/releases/v1.0.0-rc1/dependency_outdated_output.txt -i)

if [ "$OUTDATED_COUNT" -gt 0 ]; then
    echo "⚠️  Some dependencies are outdated"
    echo "Please review: docs/releases/v1.0.0-rc1/dependency_outdated_output.txt"
else
    echo "✅ All dependencies are up to date"
fi

# 生成安全扫描摘要
echo "Generating security scan summary..."
cat > docs/releases/v1.0.0-rc1/security-summary.md << EOF
# Security Scan Report Summary

## Vulnerability Statistics

- **Critical Vulnerabilities**: ${CRITICAL_COUNT}
- **High Vulnerabilities**: ${HIGH_COUNT}
- **Status**: $(if [ "$CRITICAL_COUNT" -eq 0 ] && [ "$HIGH_COUNT" -le 2 ]; then echo "✅ PASS"; else echo "❌ FAIL"; fi)

## Dependency Status

- **Outdated Dependencies**: ${OUTDATED_COUNT}
- **Status**: $(if [ "$OUTDATED_COUNT" -eq 0 ]; then echo "✅ PASS"; else echo "⚠️  WARNING"; fi)

## Report Files

- **Security Audit Output**: security_audit_output.txt
- **Dependency Outdated Output**: dependency_outdated_output.txt

## Scan Details

- **Security Command**: cargo audit
- **Dependency Command**: cargo outdated
- **Scan Date**: $(date)

## Conclusion

$(if [ "$CRITICAL_COUNT" -eq 0 ] && [ "$HIGH_COUNT" -le 2 ]; then 
    echo "Security scan passed all requirements."; 
else 
    echo "Security scan failed to meet requirements."; 
fi)
EOF

echo "✅ Security summary generated: docs/releases/v1.0.0-rc1/security-summary.md"
echo "=== Security Gate Check Complete ==="

#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

MODE="${1:-full}"
PRIORITY="${2:-P0}"

echo "=== SQLRustGo Regression Test Runner ==="
echo "Mode: $MODE"
echo "Priority: $PRIORITY"
echo ""

cd "$PROJECT_ROOT"

case "$MODE" in
    full)
        echo "Running full test suite..."
        cargo test --all
        ;;
    unit)
        echo "Running unit tests only..."
        cargo test --test '*_test' --lib
        ;;
    integration)
        echo "Running integration tests..."
        cargo test --test '*_test'
        ;;
    incremental)
        echo "Running incremental tests based on git diff..."
        
        if [ -d ".git" ]; then
            CHANGED_FILES=$(git diff --name-only HEAD~1 HEAD | grep -E '\.(rs|toml)$' || true)
            
            if [ -z "$CHANGED_FILES" ]; then
                echo "No changed files detected, running all tests"
                cargo test --all
            else
                echo "Changed files:"
                echo "$CHANGED_FILES"
                echo ""
                
                MODULES=$(echo "$CHANGED_FILES" | grep -oE 'crates/[^/]+' | sort -u || true)
                
                if [ -z "$MODULES" ]; then
                    echo "No module changes detected"
                    cargo test --all
                else
                    echo "Affected modules: $MODULES"
                    
                    for mod in $MODULES; do
                        echo "Testing $mod..."
                        cargo test -p "$mod" 2>/dev/null || true
                    done
                fi
            fi
        else
            echo "Not a git repository, running full tests"
            cargo test --all
        fi
        ;;
    priority)
        echo "Running $PRIORITY priority tests..."
        
        case "$PRIORITY" in
            P0)
                cargo test --test ci_test
                cargo test --test '*_catalog*'
                cargo test --test '*_stability*'
                cargo test --test '*_qps*'
                ;;
            P1)
                cargo test --test mvcc_concurrency_test
                cargo test --test '*_fk*'
                ;;
            P2)
                cargo test --test '*_join*'
                cargo test --test '*_union*'
                cargo test --test '*_view*'
                cargo test --test '*_timeout*'
                ;;
            P3)
                cargo test --test '*_datetime*'
                cargo test --test '*_boundary*'
                cargo test --test '*_error*'
                ;;
            P4)
                cargo test --test '*_aggregate*'
                cargo test --test '*_null*'
                ;;
            *)
                echo "Unknown priority: $PRIORITY"
                exit 1
                ;;
        esac
        ;;
    *)
        echo "Unknown mode: $MODE"
        echo "Usage: $0 [full|unit|integration|incremental|priority] [P0-P4]"
        exit 1
        ;;
esac

echo ""
echo "=== Test Run Complete ==="

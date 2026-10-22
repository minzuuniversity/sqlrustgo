#!/bin/bash
# =============================================================================
# SQLRustGo Backup Script
# Usage: ./scripts/backup.sh [OPTIONS]
#
# Options:
#   -d, --database <name>    Database name (default: default)
#   -o, --output <dir>       Backup output directory (default: ./backups)
#   -t, --type <type>       Backup type: full (the only implemented one)
#   -c, --compress           NOT IMPLEMENTED — the script refuses it
#   -s, --schema-only        NOT IMPLEMENTED — the script refuses it
#
# Examples:
#   ./scripts/backup.sh                                    # Full backup
#   ./scripts/backup.sh -d mydb -o /backups               # Backup specific DB
#
# #5516 — what was wrong
# -----------------------
# The script used to build
#
#   ./target/debug/sqlrustgo-tools backup --database … --output-dir … --backup-type …
#
# `sqlrustgo-tools` is a **package** name (`crates/tools`); the binaries it
# produces are `tbl2bin`, `bench_client`, `physical-backup`, `repro_4564`,
# `ha` and `log-rotation` — **none** of them takes a `backup` subcommand.
# The command therefore never ran, and the script still printed
# "Backup completed successfully!".
#
# The logical backup entry point is the canonical CLI's `sqlrustgo backup`:
#
#   sqlrustgo backup <output_dir> --database <name> --backup-type <type>
#
# Two further defects were fixed on the way (see #5516):
#   * `sqlrustgo backup` forwarded `--output <dir>` to the server, whose
#     `backup` subcommand takes the path as a **positional** argument — so
#     the canonical entry point itself was broken;
#   * the logical backup implementation enumerates **zero** tables (the
#     table map it is handed is empty), so it produced an empty file and
#     reported success. The script now **verifies the artifact**: a backup
#     that contains no table is reported as a failure, not as a success.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Default values
DATABASE="default"
OUTPUT_DIR="./backups"
BACKUP_TYPE="full"

usage() {
    awk 'NR > 1 { if ($0 !~ /^#/) exit; sub(/^# ?/, ""); print }' "$0"
}

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        -d|--database)
            DATABASE="$2"
            shift 2
            ;;
        -o|--output)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        -t|--type)
            BACKUP_TYPE="$2"
            shift 2
            ;;
        -c|--compress)
            # The backup implementation accepts a `compress` field but
            # never uses it (`#[allow(dead_code)]`). Passing it would look
            # like it worked while changing nothing — so the script refuses
            # it out loud instead.
            echo "ERROR: --compress is not implemented by the backup implementation." >&2
            echo "       It would be accepted and silently ignored, so this script" >&2
            echo "       refuses it rather than pretending the backup was compressed." >&2
            exit 2
            ;;
        -s|--schema-only)
            # Same as --compress: the field exists but is not implemented.
            echo "ERROR: --schema-only is not implemented by the backup implementation." >&2
            echo "       It would be accepted and silently ignored, so this script" >&2
            echo "       refuses it rather than pretending the backup was schema-only." >&2
            exit 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            usage
            exit 1
            ;;
    esac
done

if [ "$BACKUP_TYPE" != "full" ]; then
    echo "ERROR: only the 'full' backup type is implemented." >&2
    echo "       The backup implementation receives the type but does not act" >&2
    echo "       on it, so '$BACKUP_TYPE' would silently behave like 'full'." >&2
    exit 2
fi

# The binary is resolved through the repo's single shell-side resolver
# (CARGO_TARGET_DIR-aware) instead of a hardcoded ./target/debug path.
source "$REPO_ROOT/scripts/common/resolve_bin.sh"
CLI_BIN="$(resolve_cargo_bin sqlrustgo SQLRUSTGO_BIN backup_sh)" || {
    echo "ERROR: the sqlrustgo binary was not found." >&2
    echo "       Build it (cargo build -p sqlrustgo-cli) or point" >&2
    echo "       SQLRUSTGO_BIN at an existing binary." >&2
    exit 1
}

# The CLI forwards `backup` to the server binary. Resolve that one too and
# pin it, so the CLI can never pick up a *stale* `sqlrustgo-mysql-server`
# from another workspace's ./target/debug (see #5516 and the
# `mysql_server_bin()` documentation in crates/sqlrustgo-cli/src/lib.rs).
SERVER_BIN="$(resolve_cargo_bin sqlrustgo-mysql-server SQLRUSTGO_SERVER_BIN backup_sh)" || {
    echo "ERROR: the sqlrustgo-mysql-server binary was not found." >&2
    echo "       Build it (cargo build -p sqlrustgo-mysql-server) or point" >&2
    echo "       SQLRUSTGO_SERVER_BIN at an existing binary." >&2
    exit 1
}
export SQLRUSTGO_SERVER_BIN="$SERVER_BIN"

echo "=========================================="
echo "SQLRustGo Backup Script"
echo "=========================================="
echo "Binary:      $CLI_BIN"
echo "Server Bin:  $SERVER_BIN"
echo "Database:    $DATABASE"
echo "Output Dir:  $OUTPUT_DIR"
echo "Backup Type: $BACKUP_TYPE"
echo "Timestamp:   $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="

# Create output directory if not exists
mkdir -p "$OUTPUT_DIR"

# Run backup
echo ""
echo "Starting backup..."
"$CLI_BIN" backup "$OUTPUT_DIR" --database "$DATABASE" --backup-type "$BACKUP_TYPE"

# Verify the artifact.
#
# The backup implementation prints "Backup completed successfully!" even
# when it wrote an empty file (it is handed an empty table map). A report
# that cannot be checked is not a report, so the artifact is inspected
# here: the backup file has to exist and it has to contain at least one
# table.
BACKUP_FILE="$(ls -t "$OUTPUT_DIR"/*.sql 2>/dev/null | head -1 || true)"
if [ -z "$BACKUP_FILE" ] || [ ! -f "$BACKUP_FILE" ]; then
    echo ""
    echo "ERROR: the backup produced no artifact in $OUTPUT_DIR." >&2
    exit 1
fi

TABLES="$(grep -c '^CREATE TABLE' "$BACKUP_FILE" || true)"
if [ "${TABLES:-0}" -eq 0 ]; then
    echo "" >&2
    echo "ERROR: the backup artifact $BACKUP_FILE contains no table." >&2
    echo "       The backup implementation enumerates zero tables today, so the" >&2
    echo "       run 'succeeded' without backing anything up. This is reported as" >&2
    echo "       a failure rather than as a successful backup." >&2
    exit 1
fi

echo ""
echo "Backup completed successfully!"
echo "Output: $OUTPUT_DIR"
echo "File:   $BACKUP_FILE"
echo "Tables: $TABLES"

#!/bin/bash
# =============================================================================
# SQLRustGo Backup Script
# Usage: ./scripts/backup.sh [OPTIONS]
#
# Options:
#   -d, --database <name>    Database name (default: default)
#   -o, --output <dir>       Backup output directory (default: ./backups)
#   -t, --type <type>       Backup type: full (the only implemented one)
#   -D, --data-dir <dir>     Storage data directory to back up
#                            (default: $SQLRUSTGO_DATA_DIR or /tmp/sqlrustgo-data)
#   -c, --compress           NOT IMPLEMENTED — the script refuses it
#   -s, --schema-only        Back up the schema only (no rows)
#
# Examples:
#   ./scripts/backup.sh                                    # Full backup
#   ./scripts/backup.sh -d mydb -o /backups               # Backup specific DB
#   ./scripts/backup.sh -D /var/lib/sqlrustgo -o /backups  # Backup a real store
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
#
# #5525 — what was still wrong afterwards
# ---------------------------------------
# The enumeration was fixed inside the implementation, which means the
# backup now reads the tables from a **storage** — so it needs to know
# *where* that storage is. The script therefore passes `--data-dir`, and
# `--schema-only` is forwarded instead of being refused (it is implemented
# now: the dump carries the schema and no rows).
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Default values
DATABASE="default"
OUTPUT_DIR="./backups"
BACKUP_TYPE="full"
# #5525: the backup reads the tables from a storage, so it has to be told
# where that storage lives. The default matches the server's own default
# data directory; `SQLRUSTGO_DATA_DIR` overrides it.
DATA_DIR="${SQLRUSTGO_DATA_DIR:-/tmp/sqlrustgo-data}"
SCHEMA_ONLY=0

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
        -D|--data-dir)
            DATA_DIR="$2"
            shift 2
            ;;
        -c|--compress)
            # The backup implementation still has a `compress` field it
            # never acts on. Passing it would look like it worked while
            # changing nothing — so the script refuses it out loud instead.
            echo "ERROR: --compress is not implemented by the backup implementation." >&2
            echo "       It would be accepted and silently ignored, so this script" >&2
            echo "       refuses it rather than pretending the backup was compressed." >&2
            exit 2
            ;;
        -s|--schema-only)
            # #5525: unlike --compress, this one IS implemented now — the
            # dump carries the schema and no rows — so it is forwarded.
            SCHEMA_ONLY=1
            shift
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
echo "Data Dir:    $DATA_DIR"
echo "Backup Type: $BACKUP_TYPE"
echo "Timestamp:   $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="

# Create output directory if not exists
mkdir -p "$OUTPUT_DIR"

# Run backup
echo ""
echo "Starting backup..."
BACKUP_ARGS=(backup "$OUTPUT_DIR" --database "$DATABASE" --backup-type "$BACKUP_TYPE" --data-dir "$DATA_DIR")
if [ "$SCHEMA_ONLY" -eq 1 ]; then
    BACKUP_ARGS+=(--schema-only)
fi
"$CLI_BIN" "${BACKUP_ARGS[@]}"

# Verify the artifact.
#
# #5525: the backup implementation now enumerates the tables for real, so
# an empty artifact means the enumeration found nothing — which the
# implementation itself already refuses. The check stays because a report
# that cannot be checked is not a report: the artifact is inspected here,
# and it has to contain at least one table.
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
    echo "       The storage at $DATA_DIR has no table in database" >&2
    echo "       '$DATABASE', so there was nothing to back up. This is" >&2
    echo "       reported as a failure rather than as a successful backup." >&2
    exit 1
fi

echo ""
echo "Backup completed successfully!"
echo "Output: $OUTPUT_DIR"
echo "File:   $BACKUP_FILE"
echo "Tables: $TABLES"

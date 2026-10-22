#!/bin/bash
# =============================================================================
# SQLRustGo Restore Script
# Usage: ./scripts/restore.sh -b <backup-id> [OPTIONS]
#
# Options:
#   -d, --database <name>    Database name (required)
#   -b, --backup-id <id>    Backup ID to restore (required)
#   -i, --input <dir>       Backup directory (default: ./backups)
#       --drop-first        NOT IMPLEMENTED — the script refuses it
#
# Examples:
#   ./scripts/restore.sh -d mydb -b backup-20260422-001
#
# #5516 — what was wrong
# -----------------------
# The script used to build
#
#   ./target/debug/sqlrustgo-tools restore --database … --backup-id … --backup-dir …
#
# `sqlrustgo-tools` is a **package** name (`crates/tools`); the binaries it
# produces are `tbl2bin`, `bench_client`, `physical-backup`, `repro_4564`,
# `ha` and `log-rotation` — **none** of them takes a `restore` subcommand.
# The command therefore never ran, and the script still printed
# "Restore completed successfully!".
#
# The logical restore entry point is the canonical CLI's `sqlrustgo restore`:
#
#   sqlrustgo restore <backup_id> <database> --backup-dir <dir>
#
# Two further defects were fixed on the way (see #5516):
#   * `sqlrustgo restore` forwarded `--input <id> --database <db>` to the
#     server, whose `restore` subcommand takes the id as a **positional**
#     argument — so the canonical entry point itself was broken;
#   * the restore implementation reports how many tables it restored. When
#     the backup is empty it reports zero and still exits 0, so the script
#     now **verifies the outcome** and refuses to call that a success.
# =============================================================================

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Default values
DATABASE=""
BACKUP_ID=""
BACKUP_DIR="./backups"

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
        -b|--backup-id)
            BACKUP_ID="$2"
            shift 2
            ;;
        -i|--input)
            BACKUP_DIR="$2"
            shift 2
            ;;
        --drop-first)
            # The restore implementation accepts a `drop_first` field but
            # never uses it (`#[allow(dead_code)]`). Passing it would look
            # like it worked while changing nothing — so the script refuses
            # it out loud instead.
            echo "ERROR: --drop-first is not implemented by the restore implementation." >&2
            echo "       It would be accepted and silently ignored, so this script" >&2
            echo "       refuses it rather than pretending the tables were dropped." >&2
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

# Validate required parameters
if [ -z "$DATABASE" ]; then
    echo "Error: --database is required"
    usage
    exit 1
fi

if [ -z "$BACKUP_ID" ]; then
    echo "Error: --backup-id is required"
    usage
    exit 1
fi

if [ ! -f "$BACKUP_DIR/${BACKUP_ID}.sql" ]; then
    echo "Error: the backup '$BACKUP_ID' was not found in $BACKUP_DIR" >&2
    echo "       (expected: $BACKUP_DIR/${BACKUP_ID}.sql)" >&2
    exit 1
fi

# The binary is resolved through the repo's single shell-side resolver
# (CARGO_TARGET_DIR-aware) instead of a hardcoded ./target/debug path.
source "$REPO_ROOT/scripts/common/resolve_bin.sh"
CLI_BIN="$(resolve_cargo_bin sqlrustgo SQLRUSTGO_BIN restore_sh)" || {
    echo "ERROR: the sqlrustgo binary was not found." >&2
    echo "       Build it (cargo build -p sqlrustgo-cli) or point" >&2
    echo "       SQLRUSTGO_BIN at an existing binary." >&2
    exit 1
}

# The CLI forwards `restore` to the server binary. Resolve that one too and
# pin it, so the CLI can never pick up a *stale* `sqlrustgo-mysql-server`
# from another workspace's ./target/debug (see #5516 and the
# `mysql_server_bin()` documentation in crates/sqlrustgo-cli/src/lib.rs).
SERVER_BIN="$(resolve_cargo_bin sqlrustgo-mysql-server SQLRUSTGO_SERVER_BIN restore_sh)" || {
    echo "ERROR: the sqlrustgo-mysql-server binary was not found." >&2
    echo "       Build it (cargo build -p sqlrustgo-mysql-server) or point" >&2
    echo "       SQLRUSTGO_SERVER_BIN at an existing binary." >&2
    exit 1
}
export SQLRUSTGO_SERVER_BIN="$SERVER_BIN"

echo "=========================================="
echo "SQLRustGo Restore Script"
echo "=========================================="
echo "Binary:      $CLI_BIN"
echo "Database:    $DATABASE"
echo "Backup ID:   $BACKUP_ID"
echo "Backup Dir:  $BACKUP_DIR"
echo "Timestamp:   $(date '+%Y-%m-%d %H:%M:%S')"
echo "=========================================="

# Run restore
echo ""
echo "Starting restore..."
RESTORE_OUT="$("$CLI_BIN" restore "$BACKUP_ID" "$DATABASE" --backup-dir "$BACKUP_DIR")"
printf '%s\n' "$RESTORE_OUT"

# Verify the outcome.
#
# "Restore completed successfully!" is printed by the implementation even
# when the backup held no table at all. The number of restored tables is
# what actually says whether anything happened, so it is read back here.
RESTORED="$(printf '%s\n' "$RESTORE_OUT" | sed -n 's/^Tables restored:[[:space:]]*\([0-9][0-9]*\).*/\1/p' | tail -1)"
if [ -z "${RESTORED:-}" ]; then
    echo "" >&2
    echo "ERROR: the restore did not report how many tables it restored." >&2
    echo "       Without that number there is no evidence that anything was" >&2
    echo "       restored, so this is reported as a failure." >&2
    exit 1
fi

if [ "$RESTORED" -eq 0 ]; then
    echo "" >&2
    echo "ERROR: the restore restored 0 tables." >&2
    echo "       The backup '$BACKUP_ID' holds no INSERT statements, so the run" >&2
    echo "       'succeeded' without restoring anything. This is reported as a" >&2
    echo "       failure rather than as a successful restore." >&2
    exit 1
fi

echo ""
echo "Restore completed successfully!"
echo "Tables restored: $RESTORED"

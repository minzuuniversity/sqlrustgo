#!/usr/bin/env bash
# TPC-H Benchmark Data Sync Script
# Syncs TPC-H SF=0.001, SF=0.01, SF=1 data + baseline to remote servers
#
# Usage:
#   ./sync_data.sh push          # push local data to all servers
#   ./sync_data.sh pull          # pull from servers (not implemented)
#   ./sync_data.sh status        # show data status on all servers
#
# Requires SSH key access to servers.
# Data size: SF=0.001 (2MB), SF=0.01 (8MB), SF=1 (1GB compressed ~200MB)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)"
BENCH_DIR="$REPO_DIR/bench/tpch-benchmark"
DATA_DIR="$BENCH_DIR/data"
ARCHIVE_DIR="/tmp/tpch_benchmark_archives"
PACKAGE_NAME="tpch_benchmark_data.tar.zst"

# Remote servers (from memory: Z6G4=192.168.0.252, Z440=192.168.0.250)
SERVERS=(
  "openclaw@192.168.0.252:222"
  "openclaw@192.168.0.250:222"
)

# SSH options for Git Bash / MINGW
SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null -o LogLevel=ERROR"
SSH_OPTS="$SSH_OPTS -o IdentityFile=~/.ssh/id_ed25519 2>/dev/null || true"

log() { echo "[$(date '+%H:%M:%S')] $*"; }
warn() { echo "[$(date '+%H:%M:%S')] WARNING: $*" >&2; }
err() { echo "[$(date '+%H:%M:%S')] ERROR: $*" >&2; exit 1; }

need_cmd() {
  if ! command -v "$1" &>/dev/null; then
    err "Required command not found: $1"
  fi
}

# ── Package data ────────────────────────────────────────────────────────────────

package_data() {
  log "Packaging TPC-H benchmark data..."
  mkdir -p "$ARCHIVE_DIR"

  local pkg="$ARCHIVE_DIR/$PACKAGE_NAME"
  local tmp_pkg="${pkg}.tmp"

  # Compress with zstd (fast, good ratio)
  if ! command -v zstd &>/dev/null; then
    warn "zstd not found, using gzip"
    pkg="${pkg%.zst}.tar.gz"
  fi

  tar -C "$DATA_DIR" -cf - \
    sf0.001_data \
    sf01 \
    2>/dev/null || tar -C "$DATA_DIR" -cf - . \
    | zstd -19 - > "$tmp_pkg" 2>/dev/null \
    || tar -C "$DATA_DIR" -czf "${tmp_pkg%.zst}.tar.gz" - .

  if [[ "$tmp_pkg" != "$pkg" ]] && [[ -f "${tmp_pkg%.zst}.tar.gz" ]]; then
    mv "${tmp_pkg%.zst}.tar.gz" "$pkg"
  elif [[ -f "$tmp_pkg" ]]; then
    mv "$tmp_pkg" "$pkg"
  fi

  log "Package created: $pkg ($(du -sh "$pkg" | cut -f1))"
  echo "$pkg"
}

# ── Push to server ─────────────────────────────────────────────────────────────

push_to_server() {
  local srv="$1"
  local addr="${srv%:*}"
  local port="${srv#*:}"
  local user_host="${addr%@*}"
  local actual_user="${addr%@*}"
  local host="${user_host#*@}"

  log "Pushing to $actual_user@$host:$port ..."

  # Check SSH connectivity
  if ! ssh -p "$port" $SSH_OPTS "$actual_user@$host" "echo ok" &>/dev/null; then
    warn "Cannot reach $actual_user@$host:$port — skipping"
    return 1
  fi

  # Check disk space on remote
  local remote_space
  remote_space=$(ssh -p "$port" $SSH_OPTS "$actual_user@$host" \
    "df -h /tmp | tail -1 | awk '{print \$4}'" 2>/dev/null || echo "unknown")
  log "  Remote /tmp space: $remote_space"

  # Rsync with compression
  local rsync_opts="-avz --progress -e 'ssh -p $port $SSH_OPTS'"
  if command -v rsync &>/dev/null; then
    rsync -avz --compress-level=6 \
      -e "ssh -p $port $SSH_OPTS" \
      "$DATA_DIR/" \
      "$actual_user@$host:/tmp/tpch_benchmark_data/" 2>&1 | tail -5
  else
    # Fallback: scp with tar
    warn "rsync not found, using scp+tar"
    local tmp_tar="/tmp/tpch_benchmark_data.tar.zst"
    zstd -19 "$DATA_DIR"/*.tbl 2>/dev/null | \
      ssh -p "$port" $SSH_OPTS "$actual_user@$host" \
      "mkdir -p /tmp/tpch_benchmark_data && tar -xf - -C /tmp/tpch_benchmark_data"
  fi

  log "  Pushed to $actual_user@$host:/tmp/tpch_benchmark_data/"
}

push_all() {
  local pkg
  pkg=$(package_data)

  for srv in "${SERVERS[@]}"; do
    push_to_server "$srv" || true
  done

  # Also copy to local bench dir
  log "Data available locally at: $DATA_DIR"
}

# ── Status check ────────────────────────────────────────────────────────────────

check_status() {
  log "Local data status:"
  for d in "$DATA_DIR"/*/; do
    if [[ -d "$d" ]]; then
      local name=$(basename "$d")
      local rows=$(wc -l < "$d/lineitem.tbl" 2>/dev/null || echo "N/A")
      local size=$(du -sh "$d" 2>/dev/null | cut -f1)
      log "  $name: $size, lineitem=$(echo $rows | tr -d ' ')"
    fi
  done

  if [[ -f "$BENCH_DIR/baseline/baseline_sf0.01.json" ]]; then
    log "Baseline: $(du -sh "$BENCH_DIR/baseline/baseline_sf0.01.json" | cut -f1)"
  fi

  log "Remote server status:"
  for srv in "${SERVERS[@]}"; do
    local addr="${srv%:*}"
    local port="${srv#*:}"
    local user_host="${addr%@*}"
    local actual_user="${addr%@*}"
    local host="${user_host#*@}"

    local status="❌ unreachable"
    if ssh -p "$port" $SSH_OPTS "$actual_user@$host" "echo ok" &>/dev/null; then
      local remote_lineitem
      remote_lineitem=$(ssh -p "$port" $SSH_OPTS "$actual_user@$host" \
        "wc -l < /tmp/tpch_benchmark_data/sf01/lineitem.tbl 2>/dev/null || echo N/A" 2>/dev/null || echo "N/A")
      status="✅ lineitem=$remote_lineitem"
    fi
    log "  $actual_user@$host:$port: $status"
  done
}

# ── Main ────────────────────────────────────────────────────────────────────────

main() {
  need_cmd ssh
  need_cmd tar

  case "${1:-status}" in
    push)   push_all ;;
    pull)   err "Pull not implemented" ;;
    status) check_status ;;
    *)      err "Usage: $0 {push|status}" ;;
  esac
}

main "$@"

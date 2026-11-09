#!/usr/bin/env bash
#
# scripts/ci/cache_cargo_registry.sh — cargo registry cache for CI jobs
#
# #5451: CI had **no** cargo registry cache at all. Every job started from a
# brand-new container whose cargo home had no `registry/` directory, so each
# one re-downloaded ~164 crates from `static.crates.io` (~75 s of the job's
# wall clock, and the only reason a job could die with
# "failed to get `chrono` as a dependency" before a single test ran).
#
# The runner image *does* ship a populated registry (455 crates, 356 MB under
# /root/.cargo/registry) — but a job container's `CARGO_HOME` is
# `/opt/cargo-home`, and that directory holds only a `config.toml` pointing
# at a 95-crate `/opt/vendor`. cargo therefore never sees the image's
# registry and starts from nothing.
#
# This script makes the registry **survive between jobs** by copying it in
# and out of a shared docker volume that is mounted into every job
# container:
#
#   /opt/cargo-cache/registry   ← the crate cache cargo populates
#   /opt/cargo-cache/git        ← git dependencies (there are none today)
#
# The volume is mounted by the workflow (`container.volumes`), so the copy
# is what makes it visible to the *next* job — the volume itself is the
# durable store.
#
# Three jobs run concurrently on one host and would otherwise write the
# same directory at the same time. cargo takes a `.package-cache` lock for
# its own writes, but our *copy* is not cargo's write, so the copy is
# serialized with `flock` on a lock file in the volume.
#
# The copy is keyed by nothing but the lock file's contents: cargo
# re-verifies every crate against the registry index, so a stale entry is
# harmless — cargo simply re-downloads what it needs.
#
# Usage:
#   bash scripts/ci/cache_cargo_registry.sh restore
#   bash scripts/ci/cache_cargo_registry.sh save
#
# `restore` is best-effort: a missing or unusable cache must never fail a
# job. A cache that cannot be restored is the same as no cache.

set -uo pipefail

MODE="${1:-}"
CACHE_VOLUME="${CI_CARGO_CACHE_VOLUME:-/opt/cargo-cache}"
LOCK_FILE="$CACHE_VOLUME/.restore.lock"

if [ -z "$MODE" ]; then
    echo "Usage: $0 <restore|save>" >&2
    exit 2
fi

# Resolve the same Cargo home before and after Cargo creates registry/.
# CI_CARGO_HOME is an explicit override for callers that need isolation.
CARGO_BASE="${CI_CARGO_HOME:-${CARGO_HOME:-/root/.cargo}}"

mkdir -p "$CACHE_VOLUME" 2>/dev/null || true

case "$MODE" in
restore)
    # A cache restore is never allowed to fail the job.
    if [ ! -d "$CACHE_VOLUME/registry" ]; then
        echo "cache: no cached registry at $CACHE_VOLUME — starting cold (this is not an error)"
        exit 0
    fi
    # Serialize against concurrent jobs sharing this volume.
    if command -v flock >/dev/null 2>&1; then
        exec 9>"$LOCK_FILE" 2>/dev/null || true
        flock 9 2>/dev/null || true
    fi
    restored=0
    for sub in registry git; do
        src="$CACHE_VOLUME/$sub"
        [ -d "$src" ] || continue
        dst="$CARGO_BASE/$sub"
        mkdir -p "$dst" 2>/dev/null || continue
        # `cp -a` keeps mtimes, which cargo uses to decide what is fresh.
        if cp -a "$src/." "$dst/" 2>/dev/null; then
            echo "cache: restored $sub ($(du -sh "$dst" 2>/dev/null | cut -f1))"
            restored=$((restored + 1))
        else
            echo "cache: could not restore $sub — continuing without it"
        fi
    done
    if [ "$restored" -eq 0 ]; then
        echo "cache: nothing restored"
    fi
    exit 0
    ;;
save)
    if [ ! -d "$CARGO_BASE/registry" ]; then
        echo "cache: no registry to save"
        exit 0
    fi
    if command -v flock >/dev/null 2>&1; then
        exec 9>"$LOCK_FILE" 2>/dev/null || true
        flock 9 2>/dev/null || true
    fi
    saved=0
    for sub in registry git; do
        src="$CARGO_BASE/$sub"
        [ -d "$src" ] || continue
        dst="$CACHE_VOLUME/$sub"
        mkdir -p "$dst" 2>/dev/null || continue
        if cp -a "$src/." "$dst/" 2>/dev/null; then
            saved=$((saved + 1))
        fi
    done
    echo "cache: saved $saved directories to $CACHE_VOLUME ($(du -sh "$CACHE_VOLUME" 2>/dev/null | cut -f1))"
    exit 0
    ;;
*)
    echo "Unknown mode: $MODE (expected restore|save)" >&2
    exit 2
    ;;
esac

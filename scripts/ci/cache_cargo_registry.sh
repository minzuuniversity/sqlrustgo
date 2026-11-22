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
    # ── 第 106 轮：crates.io 出站预热 ─────────────────────────────────────
    # 12625 两轮铁证：冷缓存 + 出站 'transfer too slow: 1 byte/120s' ×10 ⇒
    # protoc-bin-vendored 拉不下来 ⇒ lint-build 必红（与代码无关）。
    # ⇒ 把本机全量 .crate cache（Cargo.lock 的 516 个 registry 依赖）
    #   打包挂为 release asset，restore 时 wget 并入 —— cargo 逐 crate
    #   校验 hash，不新鲜无害；幂等（已有 protoc 家族即跳过 198MB 下载）。
    #   best-effort：下载失败绝不 fail job（同 no-cache 语义）。
    PREWARM_MARK="$CARGO_BASE/registry/cache/index.crates.io-1949cf8c6b5b557f/protoc-bin-vendored-3.3.0.crate"
    PREWARM_URL="${CI_CARGO_CACHE_PREWARM_URL:-http://192.168.0.252:3000/attachments/351f8b37-e6f2-48d1-9689-01ded2fe9537}"
    if [ ! -f "$PREWARM_MARK" ]; then
        echo "cache: prewarm from release asset (one-shot, ~200MB) ..."
        T="$(mktemp -t cargoprewarm.XXXXXX.tgz)"
        if wget -q -O "$T" "$PREWARM_URL" 2>/dev/null || curl -fsSL -o "$T" "$PREWARM_URL" 2>/dev/null; then
            mkdir -p "$CARGO_BASE/registry" "$CACHE_VOLUME/registry"
            if tar xzf "$T" -C "$CARGO_BASE/registry/" 2>/dev/null; then
                # 双写进共享卷：下一轮 restore 即有（无网也命中的持久化路径）
                cp -a "$CARGO_BASE/registry/cache/." "$CACHE_VOLUME/registry/cache/" 2>/dev/null || true
                echo "cache: prewarm OK ($(du -sh "$CARGO_BASE/registry/cache" 2>/dev/null | cut -f1))"
            else
                echo "cache: prewarm tar extract failed — continuing"
            fi
        else
            echo "cache: prewarm download failed — continuing without it"
        fi
        rm -f "$T"
    fi
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

#!/bin/bash
# Orphan source detector for #4939.
#
# A `.rs` file under a crate's `src/` that is not reachable from that
# crate's `lib.rs` / declared `[[bin]]` targets is not compiled, and its
# `#[test]` functions never run. That is how `tools/src/backup.rs` came
# to carry type drift across seven layers, and how several tests came to
# assert things that were never true — none of it was catchable while the
# files sat outside the build graph.
#
# Exit 0 when no NEW orphan appears. Known orphans are listed in
# KNOWN_ORPHANS below and are reported, but do not fail the gate, so the
# backlog can be worked down without blocking every other change.
#
# Removing an entry from KNOWN_ORPHANS does not fail the gate — it fails
# only when a file that is not on the list is orphaned. That is the
# fail-closed direction: the gate cannot be weakened by deleting lines
# from this file, only by fixing the code.

set -e

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0

echo "=== Orphan Source Check (#4939) ==="
echo ""

# Baseline of orphans already in the tree, with the reason each is still
# outstanding. Adding a file here is a deliberate decision to accept debt;
# it must carry a note saying why.
read -r -d '' KNOWN_ORPHANS <<'EOF' || true
# --- tools: awaiting #4939 triage ---
crates/tools/src/catalog_check.rs
# --- storage: parquet needs arrow/parquet as storage deps; verified
#     compilable that way, but adding two large deps to the hot crate is
#     a separate decision (#4939) ---
crates/storage/src/parquet/reader.rs
crates/storage/src/parquet/writer.rs
# --- storage: columnar is a porting job, not a wiring job (86 errors) ---
crates/storage/src/columnar/chunk.rs
crates/storage/src/columnar/convert.rs
crates/storage/src/columnar/parquet.rs
crates/storage/src/columnar/segment.rs
crates/storage/src/columnar/storage.rs
# --- storage: untouched since first added, no owner yet ---
crates/storage/src/vector_storage.rs
crates/storage/src/mmap_vector_store.rs
crates/storage/src/backup_storage.rs
crates/storage/src/pitr_recovery.rs
crates/storage/src/buffer_pool_metrics.rs
crates/storage/src/page_guard.rs
crates/storage/src/heap.rs
crates/storage/src/stats.rs
crates/storage/src/index_registry.rs
crates/storage/src/clock_replacer.rs
EOF

# Emit the orphan list for one crate: every .rs under src/ that is neither
# reachable through a `mod` declaration nor registered as a bin target.
find_orphans() {
    local crate=$1
    local src="crates/$crate/src"
    [ -d "$src" ] || return 0

    # macOS ships bash 3.2, which has no `mapfile`.
    local mods bins
    # Module names may contain digits (e.g. `binary_storage_v2`).
    mods="$(grep -hoE '^[[:space:]]*(pub[[:space:]]+)?mod[[:space:]]+[a-z_0-9]+[[:space:]]*;' "$src/lib.rs" 2>/dev/null \
        | grep -oE '[a-z_0-9]+[[:space:]]*;' | tr -d ' ;' || true)"
    bins="$(python3 -c '
import sys, tomllib, pathlib
m = tomllib.loads(pathlib.Path("crates/" + sys.argv[1] + "/Cargo.toml").read_text())
for b in m.get("bin", []):
    print(b["path"].replace("src/", "", 1))
' "$crate")"

    # Membership test: a needle is in the list when a line equals it.
    in_list() {
        local needle=$1 list=$2 item
        while IFS= read -r item; do
            [ "$item" = "$needle" ] && return 0
        done <<< "$list"
        return 1
    }

    while IFS= read -r file; do
        local rel="${file#"$src"/}"
        case "$(basename "$file")" in
            mod.rs | main.rs | lib.rs) continue ;;
        esac

        # Files under a directory whose name is a declared mod are covered
        # by that module's own mod.rs.
        if [[ "$rel" == */* ]]; then
            in_list "${rel%%/*}" "$mods" && continue
        else
            in_list "${rel%.rs}" "$mods" && continue
        fi

        in_list "$rel" "$bins" && continue

        echo "$file"
    done < <(find "$src" -name '*.rs' | sort)
}

current_orphans="$(mktemp)"
trap 'rm -f "$current_orphans"' EXIT

: > "$current_orphans"
for crate in storage tools; do
    find_orphans "$crate" >> "$current_orphans"
done

total=$(wc -l < "$current_orphans" | tr -d ' ')

echo "[1] Enumerating orphan .rs files under crates/{storage,tools}/src ..."
echo "    found: $total"
echo ""

new_orphans=()
while IFS= read -r file; do
    [ -z "$file" ] && continue
    rel="${file#./}"
    if ! grep -qxF "$rel" <<< "$KNOWN_ORPHANS"; then
        new_orphans+=("$file")
    fi
done < "$current_orphans"

if [ ${#new_orphans[@]} -eq 0 ]; then
    echo "[PASS] No new orphan sources. $total known orphan(s), all baselined."
    PASS=$((PASS + 1))
else
    echo "[FAIL] ${#new_orphans[@]} NEW orphan source(s) — these files are not compiled"
    echo "       and their tests never run:"
    for f in "${new_orphans[@]}"; do
        n=$(grep -c '#\[test\]' "$f" 2>/dev/null || echo 0)
        echo "         $f  (${n} test(s) not running)"
    done
    echo ""
    echo "       Fix: declare the module in lib.rs, register a [[bin]] target,"
    echo "       or delete the file. If the debt is deliberate, add it to"
    echo "       KNOWN_ORPHANS in $0 with a note saying why."
    FAIL=$((FAIL + 1))
fi

echo ""
echo "=== Summary: $PASS passed, $FAIL failed ==="
[ "$FAIL" -eq 0 ] || exit 1
exit 0

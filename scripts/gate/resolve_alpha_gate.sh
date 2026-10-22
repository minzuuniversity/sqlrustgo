#!/usr/bin/env bash
# resolve_alpha_gate.sh — single source of truth for "which alpha gate script
# serves release version <VER>?".
#
# The gate scripts are named per version and the spelling is not uniform:
#   scripts/gate/check_alpha_v3.12.0.sh   (full version)
#   scripts/gate/check_alpha_v410.sh      (short, dots removed)
#   scripts/gate/check_alpha_v380.sh      (short, legacy spelling)
#
# #5446: the postcheck job hardcoded `check_alpha_v380.sh`, so on the v4.1.0
# branch it ran the **v3.8.0** alpha gate while claiming to check the current
# release line. #5447: gate.yml hardcoded `check_alpha_v410.sh`, which is the
# mirror-image defect — on a v4.0.0 checkout it ran the v4.1.0 gate.
#
# Both call sites now resolve through this script, so there is exactly one
# implementation of the naming rule.
#
# Usage:
#   GATE="$(bash scripts/gate/resolve_alpha_gate.sh v4.1.0)"
#
# Prints the gate path on stdout.
# Exit codes:
#   0 = found (path on stdout)
#   2 = usage error (no version given)
#   3 = no gate script exists for this version
#
# Exit code 3 is deliberately distinct from 2 so a caller can record
# "no gate for this release line" as a NOT-RUN with a reason code instead of
# pretending the gate passed.

set -euo pipefail

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
  cat >&2 <<'EOF'
Usage: resolve_alpha_gate.sh <version>

Prints the path of the alpha gate script that serves <version>.
Exit 0 = found, 2 = usage error, 3 = no gate for this version.
EOF
  exit 0
fi

VER="${1:-}"
if [ -z "$VER" ]; then
  echo "Usage: $0 <version>" >&2
  exit 2
fi

# The version may be given with or without the leading "v"; the gate scripts
# are named after the version **with** it (`check_alpha_v3.12.0.sh`), so the
# form is normalised here.
case "$VER" in
  v[0-9]*) ;;
  *) VER="v${VER}" ;;
esac

SHORT="${VER#v}"
SHORT="${SHORT//./}"

# Two spellings exist in the tree:
#   check_alpha_v410.sh     — the version with the dots removed
#   check_alpha_v3.12.0.sh  — the version as written
# (`check_alpha_v${VER}` would produce "check_alpha_vv3.12.0.sh" — a double
# "v" — and silently resolve to nothing.)
for cand in \
    "scripts/gate/check_alpha_v${SHORT}.sh" \
    "scripts/gate/check_alpha_${VER}.sh"; do
  if [ -f "$cand" ]; then
    echo "$cand"
    exit 0
  fi
done

echo "resolve_alpha_gate: no alpha gate script serves version ${VER}" >&2
exit 3

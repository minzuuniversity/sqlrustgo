#!/usr/bin/env bash
# 变异验证: 确认 test_5remotes_sync_ssh.sh 真的能抓住目标缺陷
#
# MUT-1: 从 SSH_OPTS 里删掉 IdentityAgent=none
#        → 预期: 脚本在失效 agent 下重新挂起, 测试 FAIL
# MUT-2: 把容器发现改回全盘 `find / -maxdepth 6`
#        → 预期: 静态检查 FAIL
# MUT-3: 把 GIT_SSH_COMMAND 的设置删掉
#        → 预期: 静态检查 FAIL
#
# 任何一项"存活"(测试仍然 PASS)都说明测试没有鉴别力。

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
SYNC="${REPO_ROOT}/scripts/sync/5remotes_sync.sh"
TEST="${REPO_ROOT}/scripts/sync/test_5remotes_sync_ssh.sh"
BACKUP="$(mktemp "${TMPDIR:-/tmp}/sync_mut.XXXXXX")"

cp "${SYNC}" "${BACKUP}"
restore() { cp "${BACKUP}" "${SYNC}"; rm -f "${BACKUP}"; }
trap restore EXIT INT TERM

MUT_CAUGHT=0
MUT_SURVIVED=0

run_mut() {
    local name=$1 desc=$2
    echo ""
    echo "─── ${name}: ${desc} ───"
    if bash "${TEST}" --quiet > /tmp/mut_out.log 2>&1; then
        echo "  ❌ SURVIVED — 测试仍然通过，该变异没有被抓住"
        MUT_SURVIVED=$((MUT_SURVIVED + 1))
    else
        echo "  ✅ CAUGHT — 测试失败（退出码 $?）"
        grep -E '❌ FAIL' /tmp/mut_out.log | head -4 | sed 's/^/     /'
        MUT_CAUGHT=$((MUT_CAUGHT + 1))
    fi
}

# MUT-1
cp "${BACKUP}" "${SYNC}"
python3 - "${SYNC}" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
s2 = s.replace("    -o IdentityAgent=none\n", "")
assert s2 != s, "MUT-1 未能定位 IdentityAgent=none"
open(p, "w").write(s2)
PY
run_mut "MUT-1" "删掉 SSH_OPTS 里的 IdentityAgent=none"

# MUT-2
cp "${BACKUP}" "${SYNC}"
python3 - "${SYNC}" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
old = "find /git /data/git -maxdepth 4 -type d -name sqlrustgo.git 2>/dev/null | head -1"
new = "find / -maxdepth 6 -type d -name sqlrustgo.git 2>/dev/null | head -1"
assert old in s, "MUT-2 未能定位 find 语句"
open(p, "w").write(s.replace(old, new))
PY
run_mut "MUT-2" "容器发现改回全盘 find /"

# MUT-3
cp "${BACKUP}" "${SYNC}"
python3 - "${SYNC}" <<'PY'
import sys
p = sys.argv[1]
s = open(p).read()
old = """if [ -z "${GIT_SSH_COMMAND:-}" ]; then
    export GIT_SSH_COMMAND="ssh ${SSH_OPTS[*]}"
fi"""
assert old in s, "MUT-3 未能定位 GIT_SSH_COMMAND"
open(p, "w").write(s.replace(old, ""))
PY
run_mut "MUT-3" "删掉 GIT_SSH_COMMAND 的设置"

cp "${BACKUP}" "${SYNC}"

echo ""
echo "=========================================="
echo "MUT CAUGHT=${MUT_CAUGHT}  SURVIVED=${MUT_SURVIVED}"
echo "=========================================="
[ "${MUT_SURVIVED}" -eq 0 ] || exit 1
exit 0

#!/bin/sh
# Syntax check for old-stack LuCI lua files (luci/luci-app-*/luasrc/**.lua,
# excluding the JS-based luci-app-qmodem-next stack).
#   - luac -p when available, else lua loadfile, else SKIP (exit 0).
# CI contract: exit 0 = pass or SKIP; exit 1 = at least one syntax error.
cd "$(CDPATH= cd "$(dirname "$0")/../.." && pwd)" || exit 1

files=$(find luci -type f -name '*.lua' -path '*luasrc*' \
    ! -path '*luci-app-qmodem-next*' | sort)
if [ -z "$files" ]; then
    echo "SKIP: no old-stack lua files found under luci/*/luasrc/"
    exit 0
fi

if command -v luac >/dev/null 2>&1; then
    check() { luac -p "$1"; }
    tool="luac -p"
elif command -v lua >/dev/null 2>&1; then
    check() {
        lua -e 'local f,e=loadfile(arg[1]) if not f then io.stderr:write(e,"\n") os.exit(1) end' "$1"
    }
    tool="lua loadfile"
else
    echo "SKIP: luac/lua not installed, cannot syntax-check lua files"
    exit 0
fi

pass=0
fail=0
for f in $files; do
    if check "$f"; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        echo "SYNTAX-FAIL: $f" >&2
    fi
done

echo "lua smoke ($tool): PASS=$pass FAIL=$fail"
[ "$fail" -eq 0 ]

#!/bin/sh
set -eu

ROOT=$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)
REPO=$(CDPATH= cd -- "$ROOT/../.." && pwd)

# 组件合并后源码布局：
#   C 组件 -> application/qmodem/src/<组件>/
#   前端    -> luci/luci-app-qmodem/（原 luci-app-qmodem 已并入；
#              原 Lua 栈 luci-app-qmodem/luasrc 已退役，改用 JS 视图断言）
grep -q 'use_ubus", section); uci_set(key, "1")' \
	"$REPO/application/qmodem/src/modem_scan/modem_scand.c"
grep -q 'config_get use_ubus .* use_ubus 1' \
	"$ROOT/files/usr/share/qmodem/modem_dial.sh"
grep -q 'use_ubus=${use_ubus:-1}' \
	"$ROOT/files/usr/libexec/rpcd/qmodem"
grep -Fq "o.default = '1';" \
	"$REPO/luci/luci-app-qmodem/htdocs/luci-static/resources/view/qmodem/network_config.js"
grep -q "uci.set('qmodem', sid, 'use_ubus', '1')" \
	"$REPO/luci/luci-app-qmodem/htdocs/luci-static/resources/view/qmodem/service.js"
grep -q 'option_string(uci, section, "use_ubus", "1")' \
	"$REPO/application/qmodem/src/qmodem_smsd/main.c"

echo 'PASS: use_ubus defaults to enabled across creation and runtime paths'

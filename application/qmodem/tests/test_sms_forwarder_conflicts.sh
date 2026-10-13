#!/bin/sh

set -eu

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../../.." && pwd)
legacy="$ROOT/application/sms_forwarder/Makefile"
merged="$ROOT/application/qmodem/Makefile"

# 两个包都往 /etc/init.d/sms_forwarder 落文件，必须保持互斥。
grep -qx 'PKG_CONFLICTS:=sms-forwarder-next' "$legacy"
grep -Fq 'files/etc/init.d/sms_forwarder $(1)/etc/init.d/' "$legacy"
grep -Fq 'files/etc/init.d/sms_forwarder $(1)/etc/init.d/' "$merged"

# 合并后的 qmodem 把原 sms-forwarder-next 收进包内，因此
#   provides  让 `+sms-forwarder-next` / `+tom_modem` 这类旧依赖仍能解析，
#   conflicts 保证与 legacy sms-forwarder 不会同时安装。
grep -q '^PKG_PROVIDES:=tom_modem' "$merged"
grep -q '^PKG_CONFLICTS:=tom_modem' "$merged"
grep -Eq '^[[:space:]]+sms-tool_q sms-forwarder-next quectel-CM-5G-M$' "$merged"
grep -Eq '^[[:space:]]+sms-tool_q sms-forwarder-next quectel-CM-5G-M sms-forwarder$' "$merged"

echo 'PASS: SMS forwarder packages declare their shared init script conflict'

# shellcheck shell=bash
# Endpoint-level tests for the rpcd qmodem methods: get_stats / clear_stats
# and the get_current_band family (agent-rpcd-gap delivery).
#
# Chosen approach (recorded decision, same technique as case_rpcd_valid_section.sh):
#   Sourcing the rpcd script or generic.sh/vendor scripts whole is NOT viable
#   on a dev host: top-level `. /usr/share/libubox/jshn.sh` and chained
#   `source` are OpenWrt-only, and the `call)` dispatch branch does
#   `read -r input` (blocks on stdin). Instead we:
#     - mock the jshn.sh json_* API with call-recording stubs, and
#     - extract the backing functions (get_current_band,
#       get_current_band_capabilities, get_usage_stats) from generic.sh and
#       vendor/quectel.sh with a line-anchored sed range (^func()/,/^}/).
#   Auto-discovery contract: missing script or function -> SKIP, not FAIL,
#   so CI never breaks on unrelated refactors or delivery order.
#
# Semantics pinned here (get_stats "increment" contract):
#   The endpoint layer computes NO deltas. get_stats reports the modem's own
#   cumulative counters verbatim (available=1, updated_at=now) and falls back
#   to an explicit unavailable snapshot (available=0, all-zero counters,
#   updated_at=0) when the modem answer lacks the expected marker. Consumers
#   (LuCI stats view) own any delta/accumulation on top of these totals.
t test_rpcd_rpcd_script_registers_stats_methods
t test_rpcd_generic_current_band_unsupported
t test_rpcd_generic_band_capabilities_supported_false
t test_rpcd_generic_usage_stats_unavailable_fallback
t test_rpcd_quectel_usage_stats_counters_passthrough
t test_rpcd_quectel_usage_stats_marker_missing_fallback

RPCD_SCRIPT="$REPO_ROOT/application/qmodem/files/usr/libexec/rpcd/qmodem"
GENERIC_SH="$REPO_ROOT/application/qmodem/files/usr/share/qmodem/generic.sh"
QUECTEL_SH="$REPO_ROOT/application/qmodem/files/usr/share/qmodem/vendor/quectel.sh"

# jshn.sh json_* API mock: record the call sequence into _EPCALLS so the
# tests can pin the emitted JSON structure without libubox.
_EPCALLS=""
json_init()         { _EPCALLS="$_EPCALLS init;"; }
json_add_object()   { _EPCALLS="$_EPCALLS +obj:$1;"; }
json_close_object() { _EPCALLS="$_EPCALLS -obj;"; }
json_add_array()    { _EPCALLS="$_EPCALLS +arr:$1;"; }
json_close_array()  { _EPCALLS="$_EPCALLS -arr;"; }
json_add_string()   { _EPCALLS="$_EPCALLS str:$1=$2;"; }
json_add_int()      { _EPCALLS="$_EPCALLS int:$1=$2;"; }
json_add_boolean()  { _EPCALLS="$_EPCALLS bool:$1=$2;"; }
json_dump()         { :; }

assert_not_contains() {
    case "$2" in
        *"$3"*) fail "$1: [$3] unexpectedly found in [$2]" ;;
    esac
}

# Extract $2() from script $1 and source it; SKIP if script or function gone.
_ep_load() {
    [ -f "$1" ] || skip "source script not found: $1"
    local frag tmp
    frag=$(sed -n "/^$2()/,/^}/p" "$1")
    [ -n "$frag" ] || skip "$2() not found in $1"
    tmp=$(mktemp "${TMPDIR:-/tmp}/qmodem_ep.XXXXXX")
    printf '%s\n' "$frag" > "$tmp"
    # shellcheck disable=SC1090
    . "$tmp"
    rm -f "$tmp"
}

# The rpcd script itself must keep the methods registered (methods table +
# dispatch branches); cheap grep-level pin, no sourcing involved.
test_rpcd_rpcd_script_registers_stats_methods() {
    [ -f "$RPCD_SCRIPT" ] || skip "rpcd qmodem script not found"
    local body
    body=$(cat "$RPCD_SCRIPT")
    assert_contains "methods table lists get_stats" "$body" '"get_stats": {'
    assert_contains "methods table lists get_current_band" "$body" '"get_current_band": {'
    assert_contains "dispatch handles get_stats" "$body" "get_stats)"
    assert_contains "dispatch handles get_current_band" "$body" "get_current_band)"
    assert_contains "dispatch handles clear_stats" "$body" "clear_stats)"
}

# generic.sh fallback: get_current_band reports the supported:false-style
# branch (status "unsupported") with an empty cells array.
test_rpcd_generic_current_band_unsupported() {
    _ep_load "$GENERIC_SH" get_current_band
    _Vendor=generic
    json_init
    get_current_band
    assert_contains "current_band object opened" "$_EPCALLS" "+obj:current_band;"
    assert_contains "status unsupported pinned" "$_EPCALLS" "str:status=unsupported;"
    assert_contains "vendor echoed" "$_EPCALLS" "str:vendor=generic;"
    assert_contains "cells array opened and closed" "$_EPCALLS" "arr:cells; -arr;"
    assert_contains "object closed" "$_EPCALLS" "-obj;"
}

# generic.sh fallback: get_current_band_capabilities advertises supported:false.
test_rpcd_generic_band_capabilities_supported_false() {
    _ep_load "$GENERIC_SH" get_current_band_capabilities
    _Vendor=generic
    json_init
    get_current_band_capabilities
    assert_contains "capabilities object opened" "$_EPCALLS" "+obj:current_band_capabilities;"
    assert_contains "supported:false pinned" "$_EPCALLS" "bool:supported=0;"
    assert_contains "schema current_band" "$_EPCALLS" "str:schema=current_band;"
    assert_contains "object closed" "$_EPCALLS" "-obj;"
}

# generic.sh fallback: get_stats returns the explicit unavailable snapshot.
test_rpcd_generic_usage_stats_unavailable_fallback() {
    _ep_load "$GENERIC_SH" get_usage_stats
    json_init
    get_usage_stats
    assert_contains "available=0 pinned" "$_EPCALLS" "bool:available=0;"
    assert_contains "updated_at=0 pinned" "$_EPCALLS" "int:updated_at=0;"
    assert_contains "rx counter zeroed" "$_EPCALLS" "int:total_rx_bytes=0;"
    assert_contains "tx counter zeroed" "$_EPCALLS" "int:total_tx_bytes=0;"
}

# quectel.sh: when the modem answers with the counter marker, get_stats
# passes the modem's cumulative counters through verbatim (no delta math in
# the endpoint layer) and stamps updated_at with the current epoch.
test_rpcd_quectel_usage_stats_counters_passthrough() {
    _ep_load "$QUECTEL_SH" get_usage_stats
    platform=ath11k
    at_port=/dev/ttyUSB2
    cmd_qgdnrcnt_query() { printf '+QGDNRCNT: 111222, 333444\r\n'; }
    json_init
    get_usage_stats
    assert_contains "available=1 pinned" "$_EPCALLS" "bool:available=1;"
    assert_not_contains "updated_at is not the zero fallback" "$_EPCALLS" "int:updated_at=0;"
    assert_contains "rx counter = modem cumulative rx" "$_EPCALLS" "int:total_rx_bytes=333444;"
    assert_contains "tx counter = modem cumulative tx" "$_EPCALLS" "int:total_tx_bytes=111222;"
}

# quectel.sh: marker missing (e.g. ERROR) -> same unavailable snapshot as the
# generic fallback; non-numeric counters are coerced to 0.
test_rpcd_quectel_usage_stats_marker_missing_fallback() {
    _ep_load "$QUECTEL_SH" get_usage_stats
    platform=ath11k
    at_port=/dev/ttyUSB2
    cmd_qgdnrcnt_query() { printf 'ERROR\r\n'; }
    json_init
    get_usage_stats
    assert_contains "available=0 pinned" "$_EPCALLS" "bool:available=0;"
    assert_contains "updated_at=0 pinned" "$_EPCALLS" "int:updated_at=0;"
    assert_contains "rx counter zeroed" "$_EPCALLS" "int:total_rx_bytes=0;"
    assert_contains "tx counter zeroed" "$_EPCALLS" "int:total_tx_bytes=0;"
}

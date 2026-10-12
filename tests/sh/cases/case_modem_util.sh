# shellcheck shell=bash
# Function-level tests for files/usr/share/qmodem/modem_util.sh
# (AT command plumbing + testcase-slug cleaning logic).
t test_modem_util_at_forwards_command
t test_modem_util_at_dollar_cleaning_identity
t test_modem_util_path_segment_slug
t test_modem_util_path_segment_fallback
t test_modem_util_bool_enabled
t test_modem_util_atd_dispatch_at_3000
t test_modem_util_atd_dispatch_fastat_1000
t test_modem_util_atd_sendat_socket_missing
t test_modem_util_atd_sendat_ok_roundtrip
t test_modem_util_atd_sendat_status_timeout
t test_modem_util_atd_sendat_status_error
t test_modem_util_atd_sendat_curl_fallback

QMODEM_SHARE="$REPO_ROOT/application/qmodem/files/usr/share/qmodem"
QMUT_STUBS="$REPO_ROOT/application/qmodem/tests/lib"

# Fresh stub environment per test (tests run in isolated subshells).
# - uci stub answers the at_tool seam with 0 so at() drives tom_modem
# - tom_modem stub records its argv for inspection
_qmut_setup() {
    export QMODEM_HOME="$QMODEM_SHARE"
    export QMODEM_LIB_FUNCTIONS="$QMUT_STUBS/functions_stub.sh"
    export QMODEM_COLLECT_TESTCASE=0
    use_ubus_flag=""
    options=""
    clear_buffer=0
    QMUT_TOM_ARGS=""
    uci() {
        case "$*" in
            *qmodem.main.at_tool*) echo 0 ;;
            *) return 1 ;;
        esac
    }
    logger() { :; }
    tom_modem() { QMUT_TOM_ARGS="$*"; }
    # shellcheck disable=SC1090
    . "$QMODEM_SHARE/modem_util.sh"
}

# at() must hand the AT command through to the device tool (-c <cmd>)
test_modem_util_at_forwards_command() {
    _qmut_setup
    at /dev/ttyUSB2 'AT+CGSN' >/dev/null
    assert_contains "tom_modem receives -c <cmd>" "$QMUT_TOM_ARGS" "-c AT+CGSN"
    assert_contains "tom_modem receives the port" "$QMUT_TOM_ARGS" "-d /dev/ttyUSB2"
}

# modem_util.sh at()/fastat() "cleaning" expansions:
#   new_str="${2/[$]/$}"  and  atcmd="${new_str/\"/\"}"
# are currently identity transforms (glob [$] matches the literal $ inside
# [$]; replacement equals the match). Pin the observed behavior so any
# future change to the cleaning logic is a deliberate, visible decision.
test_modem_util_at_dollar_cleaning_identity() {
    _qmut_setup
    at /dev/ttyUSB2 'AT+CGPIAF[$]1,"x"' >/dev/null
    assert_contains '[$] placeholder passes through verbatim' \
        "$QMUT_TOM_ARGS" '-c AT+CGPIAF[$]1,"x"'
}

# qmodem_testcase_path_segment: slug cleaning (lowercase, non [a-z0-9._-]
# folded to underscore, truncated to 40 chars)
test_modem_util_path_segment_slug() {
    _qmut_setup
    assert_eq "slug lowercases and folds specials" \
        "rm500u_cnv_x" \
        "$(qmodem_testcase_path_segment 'RM500U CNV+X' core)"
    assert_eq "slug truncates to 40 chars" \
        "aaaaaaaaaa""bbbbbbbbbb""cccccccccc""dddddddddd" \
        "$(qmodem_testcase_path_segment 'aaaaaaaaaabbbbbbbbbbccccccccccddddddddddeeeeeeeeee' core)"
}

# empty input falls back to the fallback argument
test_modem_util_path_segment_fallback() {
    _qmut_setup
    assert_eq "empty value uses fallback" \
        "core" \
        "$(qmodem_testcase_path_segment '' core)"
}

# qmodem_bool_enabled: truthy set only
test_modem_util_bool_enabled() {
    _qmut_setup
    qmodem_bool_enabled 1;   assert_rc "1 is truthy" 0 $?
    qmodem_bool_enabled yes; assert_rc "yes is truthy" 0 $?
    qmodem_bool_enabled ON;  assert_rc "ON is truthy" 0 $?
    qmodem_bool_enabled 0;   assert_rc "0 is falsy" 1 $?
    qmodem_bool_enabled '';  assert_rc "empty is falsy" 1 $?
}

# ---------------------------------------------------------------------------
# at_tool=2 (qmodem-atd unix socket) dispatch tests.
#
# Two layers, per recorded decision:
#   1. dispatch layer: uci stub answers at_tool=2; qmodem_atd_sendat is
#      overridden with a recording function AFTER sourcing modem_util.sh
#      (later definitions win, so function override IS feasible here).
#   2. transport layer: qmodem_atd_sendat's real implementation against a
#      fake qmodem-atd server. A real unix-socket responder cannot be built
#      with shell function overrides alone (the socket test `[ -S ]` is a
#      builtin), so a socat SYSTEM cat server fakes the JSON line protocol.
#      socat is optional on hosts -> those tests SKIP with the reason.
#   The curl-fallback branch is exercised by overriding nc to produce no
#   output (simulating a busybox nc built without FEATURE_UNIX_LOCAL).
# ---------------------------------------------------------------------------

# base setup + at_tool=2 uci stub + qmodem_atd_sendat override (dispatch layer)
_qmut_atd_setup() {
    _qmut_setup
    QMUT_ATD_ARGS=""
    QMUT_ATD_OUT=""
    QMUT_ATD_RC=0
    qmodem_atd_sendat() {
        QMUT_ATD_ARGS="$*"
        printf '%s' "$QMUT_ATD_OUT"
        return "$QMUT_ATD_RC"
    }
    uci() {
        case "$*" in
            *qmodem.main.at_tool*) echo 2 ;;
            *qmodem.main.at_socket*) echo "${QMUT_ATD_SOCK:-}" ;;
            *) return 1 ;;
        esac
    }
}

# at() routes through qmodem_atd_sendat with the documented 3000ms timeout
test_modem_util_atd_dispatch_at_3000() {
    _qmut_atd_setup
    QMUT_ATD_OUT="bridge-ok"
    # NOTE: run at() directly (no command substitution): the dispatch target
    # sets QMUT_ATD_ARGS, and variables set inside $() would be lost.
    out_file=$(mktemp "${TMPDIR:-/tmp}/qmodem_at_out.XXXXXX")
    at /dev/ttyUSB2 'AT+CGSN' > "$out_file"
    assert_rc "at() propagates atd rc" 0 $?
    assert_eq "at() dispatches at_tool=2 with 3000ms" \
        "/dev/ttyUSB2 AT+CGSN 3000" "$QMUT_ATD_ARGS"
    assert_eq "at() prints atd response lines" "bridge-ok" "$(cat "$out_file")"
    rm -f "$out_file"
}

# fastat() routes through qmodem_atd_sendat with the documented 1000ms
# timeout and propagates a non-zero rc
test_modem_util_atd_dispatch_fastat_1000() {
    _qmut_atd_setup
    QMUT_ATD_RC=1
    QMUT_ATD_OUT="partial"
    out_file=$(mktemp "${TMPDIR:-/tmp}/qmodem_at_out.XXXXXX")
    fastat /dev/ttyUSB2 'AT+CSQ' > "$out_file"
    assert_rc "fastat() propagates atd failure rc" 1 $?
    assert_eq "fastat() dispatches at_tool=2 with 1000ms" \
        "/dev/ttyUSB2 AT+CSQ 1000" "$QMUT_ATD_ARGS"
    assert_eq "fastat() still prints partial lines" "partial" "$(cat "$out_file")"
    rm -f "$out_file"
}

# transport layer, no server: missing socket file -> rc 1, NO fallback to
# at_tool 0/1 (documented bridge contract)
test_modem_util_atd_sendat_socket_missing() {
    _qmut_setup
    uci() {
        case "$*" in
            *qmodem.main.at_socket*) echo "/nonexistent/qmodem/atd.sock" ;;
            *qmodem.main.at_tool*) echo 2 ;;
            *) return 1 ;;
        esac
    }
    qmodem_atd_sendat /dev/ttyUSB2 'AT+CGSN' 3000 >/dev/null 2>&1
    assert_rc "missing socket -> rc 1" 1 $?
}

# start a fake qmodem-atd: socat listens on $1 and answers every connection
# with the content of file $2; skip (reason recorded) when socat is absent
_atd_fake_server_start() {
    command -v socat >/dev/null 2>&1 || skip "socat not available to fake qmodem-atd"
    _ATD_SOCAT_PID=""
    socat "UNIX-LISTEN:$1,fork,reuseaddr" "SYSTEM:cat $2" >/dev/null 2>&1 &
    _ATD_SOCAT_PID=$!
    # hygiene: if the test aborts (fail/skip) before _atd_fake_server_stop,
    # the enclosing subshell EXIT trap still reaps the fake server
    trap 'kill "${_ATD_SOCAT_PID:-0}" 2>/dev/null' EXIT
    local i=0
    while [ ! -S "$1" ] && [ "$i" -lt 30 ]; do
        sleep 0.1
        i=$((i + 1))
    done
    if [ ! -S "$1" ]; then
        kill "$_ATD_SOCAT_PID" 2>/dev/null
        skip "fake atd socket did not come up"
    fi
}

_atd_fake_server_stop() {
    [ -n "$_ATD_SOCAT_PID" ] && kill "$_ATD_SOCAT_PID" 2>/dev/null
    wait "${_ATD_SOCAT_PID:-}" 2>/dev/null
    :
}

_atd_json_parser_available() {
    command -v jq >/dev/null 2>&1 || command -v jsonfilter >/dev/null 2>&1 \
        || skip "jq/jsonfilter unavailable, cannot parse fake atd reply"
}

# transport layer, status ok: response lines printed one per line, rc 0
test_modem_util_atd_sendat_ok_roundtrip() {
    _qmut_setup
    _atd_json_parser_available
    local tmpd sock out
    tmpd=$(mktemp -d "${TMPDIR:-/tmp}/qmodem_atd_t.XXXXXX")
    sock="$tmpd/atd.sock"
    printf '{"status":"ok","lines":["AT+CGSN","867123456789012","OK"]}\n' \
        > "$tmpd/reply.json"
    # NOTE: uci must NOT reference $sock: qmodem_atd_socket declares its own
    # local sock, which (bash dynamic scoping) shadows the test-local one.
    QMUT_ATD_SOCK="$sock"
    uci() { case "$*" in *qmodem.main.at_socket*) echo "$QMUT_ATD_SOCK" ;; *) return 1 ;; esac; }
    _atd_fake_server_start "$sock" "$tmpd/reply.json"
    out=$(qmodem_atd_sendat /dev/ttyUSB2 'AT+CGSN' 3000)
    assert_rc "status ok -> rc 0" 0 $?
    assert_eq "response lines printed one per line" \
        "$(printf 'AT+CGSN\n867123456789012\nOK')" "$out"
    _atd_fake_server_stop
    rm -rf "$tmpd"
}

# transport layer, status timeout: partial lines printed, rc 1
test_modem_util_atd_sendat_status_timeout() {
    _qmut_setup
    _atd_json_parser_available
    local tmpd sock out
    tmpd=$(mktemp -d "${TMPDIR:-/tmp}/qmodem_atd_t.XXXXXX")
    sock="$tmpd/atd.sock"
    printf '{"status":"timeout","lines":["AT+QSPNVFW=1"]}\n' > "$tmpd/reply.json"
    QMUT_ATD_SOCK="$sock"
    uci() { case "$*" in *qmodem.main.at_socket*) echo "$QMUT_ATD_SOCK" ;; *) return 1 ;; esac; }
    _atd_fake_server_start "$sock" "$tmpd/reply.json"
    out=$(qmodem_atd_sendat /dev/ttyUSB2 'AT+QSPNVFW=1' 3000)
    assert_rc "status timeout -> rc 1" 1 $?
    assert_eq "timeout keeps partial lines" "AT+QSPNVFW=1" "$out"
    _atd_fake_server_stop
    rm -rf "$tmpd"
}

# transport layer, status error: no stdout, rc 1
test_modem_util_atd_sendat_status_error() {
    _qmut_setup
    _atd_json_parser_available
    local tmpd sock out
    tmpd=$(mktemp -d "${TMPDIR:-/tmp}/qmodem_atd_t.XXXXXX")
    sock="$tmpd/atd.sock"
    printf '{"status":"error","error":"port busy"}\n' > "$tmpd/reply.json"
    QMUT_ATD_SOCK="$sock"
    uci() { case "$*" in *qmodem.main.at_socket*) echo "$QMUT_ATD_SOCK" ;; *) return 1 ;; esac; }
    _atd_fake_server_start "$sock" "$tmpd/reply.json"
    out=$(qmodem_atd_sendat /dev/ttyUSB2 'AT+CFUN=1,1' 3000)
    assert_rc "status error -> rc 1" 1 $?
    assert_eq "error path prints nothing" "" "$out"
    _atd_fake_server_stop
    rm -rf "$tmpd"
}

# transport layer, curl fallback: with nc neutered (busybox nc without -U
# simulation), the curl --http0.9 path still delivers the ok response
test_modem_util_atd_sendat_curl_fallback() {
    _qmut_setup
    _atd_json_parser_available
    command -v curl >/dev/null 2>&1 || skip "curl not available for fallback path"
    local tmpd sock out
    tmpd=$(mktemp -d "${TMPDIR:-/tmp}/qmodem_atd_t.XXXXXX")
    sock="$tmpd/atd.sock"
    printf '{"status":"ok","lines":["AT+CGMR","RG500SEQ","OK"]}\n' > "$tmpd/reply.json"
    uci() { case "$*" in *qmodem.main.at_socket*) echo "$sock" ;; *) return 1 ;; esac; }
    nc() { :; }
    QMUT_ATD_SOCK="$sock"
    uci() { case "$*" in *qmodem.main.at_socket*) echo "$QMUT_ATD_SOCK" ;; *) return 1 ;; esac; }
    _atd_fake_server_start "$sock" "$tmpd/reply.json"
    out=$(qmodem_atd_sendat /dev/ttyUSB2 'AT+CGMR' 3000)
    assert_rc "curl fallback status ok -> rc 0" 0 $?
    assert_eq "curl fallback delivers response lines" \
        "$(printf 'AT+CGMR\nRG500SEQ\nOK')" "$out"
    _atd_fake_server_stop
    rm -rf "$tmpd"
}

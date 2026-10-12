# shellcheck shell=bash
# Function-level tests for files/usr/share/qmodem/modem_util.sh
# (AT command plumbing + testcase-slug cleaning logic).
t test_modem_util_at_forwards_command
t test_modem_util_at_dollar_cleaning_identity
t test_modem_util_path_segment_slug
t test_modem_util_path_segment_fallback
t test_modem_util_bool_enabled

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

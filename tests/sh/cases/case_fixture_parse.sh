# shellcheck shell=bash
# Parse-consistency smoke over two real AT+CGSN fixtures from testcases/:
#   decode response_hex -> replay raw response via a stubbed at() ->
#   vendor get_imei() parse -> assert the imei field matches the 15-digit
#   value embedded in the fixture hex.
#
# Chosen fixtures (paths pinned on purpose; update here if renamed):
#   testcases/quectel/qualcomm/example-1a79a4d6/AT_CGSN-7c58b773.json
#   testcases/quectel/unisoc/rm500u-cnv-49196729/AT_CGSN-7c58b773.json
#
# Reuses the existing stubs from application/qmodem/tests/lib/ (no wheels
# reinvented); jq is required to read fixture metadata (SKIP if absent).
t test_fixture_cgsn_quectel_qualcomm
t test_fixture_cgsn_quectel_unisoc

QMFP_STUBS="$REPO_ROOT/application/qmodem/tests/lib"
FIXTURE_QUALCOMM="$REPO_ROOT/testcases/quectel/qualcomm/example-1a79a4d6/AT_CGSN-7c58b773.json"
FIXTURE_UNISOC="$REPO_ROOT/testcases/quectel/unisoc/rm500u-cnv-49196729/AT_CGSN-7c58b773.json"

# pure-shell hex decode (no xxd dependency): bytes -> raw text on stdout
hex_decode() {
    local h=$1 i
    for ((i = 0; i < ${#h}; i += 2)); do
        printf "\\x${h:i:2}"
    done
}

# one-shot env: existing test_env stubs + quectel vendor stack + at() replay
_qmfp_setup() {
    command -v jq >/dev/null || skip "jq not available for fixture metadata"
    [ -f "$QMFP_STUBS/test_env.sh" ] || skip "existing test stubs not found"
    # shellcheck disable=SC1090,SC1091
    . "$QMFP_STUBS/test_env.sh"
    export QMODEM_HOME="$REPO_ROOT/application/qmodem/files/usr/share/qmodem"
    # shellcheck disable=SC1090
    . "$QMODEM_HOME/vendor/quectel.sh"
    # replay stub: at() prints the fixture's raw decoded response
    at() { printf '%s' "$QMFP_RAW_RESPONSE"; }
}

# run one fixture end-to-end; $1 fixture path, $2 expected IMEI
_qmfp_run_fixture() {
    local fixture=$1 expected=$2 hex raw out
    _qmfp_setup
    hex=$(jq -r '.response_hex // empty' "$fixture")
    assert_nonempty "fixture has response_hex" "$hex"
    assert_contains "fixture command is AT+CGSN" \
        "$(jq -r '.command' "$fixture")" "AT+CGSN"
    raw=$(hex_decode "$hex")
    QMFP_RAW_RESPONSE=$raw
    json_init
    get_imei
    out=$(json_dump)
    assert_contains "get_imei emits non-empty 15-digit imei" \
        "$out" "\"imei\":\"$expected\""
}

test_fixture_cgsn_quectel_qualcomm() {
    _qmfp_run_fixture "$FIXTURE_QUALCOMM" "860000000000012"
}

test_fixture_cgsn_quectel_unisoc() {
    _qmfp_run_fixture "$FIXTURE_UNISOC" "869955065336336"
}

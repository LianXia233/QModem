# shellcheck shell=bash
# Function-level tests for cmds/qmodem_lib.sh (common parsing helpers
# delivered by the vendor-lib task V1).
#
# Auto-discovery contract: if the library file or a specific function is
# missing, the corresponding case SKIPs instead of FAILing, so CI stays
# green regardless of test/feature delivery order.
t test_ql_at_clean_strips_cr
t test_ql_csv_field_nth
t test_ql_csv_field_usage_error
t test_ql_num_or_valid
t test_ql_num_or_default
t test_ql_regex_pick
t test_ql_kv_get
t test_ql_kv_get_trailing_cr
t test_ql_csv_field_empty_fields
t test_ql_csv_field_quotes_not_special
t test_ql_regex_pick_multiline_all_matches
t test_ql_regex_pick_match_on_later_line

QMODEM_LIB="$REPO_ROOT/application/qmodem/files/usr/share/qmodem/cmds/qmodem_lib.sh"

# Source the library if present; otherwise SKIP the calling test.
_qlib_load() {
    [ -f "$QMODEM_LIB" ] || skip "cmds/qmodem_lib.sh not delivered yet"
    # shellcheck disable=SC1090
    . "$QMODEM_LIB"
}

test_ql_at_clean_strips_cr() {
    _qlib_load
    declare -F ql_at_clean >/dev/null || skip "ql_at_clean not present"
    out=$(printf 'a\r\nb\r\n' | ql_at_clean)
    assert_rc "ql_at_clean rc 0" 0 $?
    assert_eq "CR stripped, lines kept" "$(printf 'a\nb')" "$out"
}

test_ql_csv_field_nth() {
    _qlib_load
    declare -F ql_csv_field >/dev/null || skip "ql_csv_field not present"
    out=$(printf 'a,b,c\n1,2,3\n' | ql_csv_field 2)
    assert_eq "2nd field of each line" "$(printf 'b\n2')" "$out"
}

test_ql_csv_field_usage_error() {
    _qlib_load
    declare -F ql_csv_field >/dev/null || skip "ql_csv_field not present"
    printf 'a,b\n' | ql_csv_field x >/dev/null 2>&1
    assert_rc "non-numeric index is usage error" 2 $?
}

test_ql_num_or_valid() {
    _qlib_load
    declare -F ql_num_or >/dev/null || skip "ql_num_or not present"
    assert_eq "numeric input kept" "42" "$(printf ' 42\r\n' | ql_num_or 0)"
}

test_ql_num_or_default() {
    _qlib_load
    declare -F ql_num_or >/dev/null || skip "ql_num_or not present"
    assert_eq "non-numeric input -> default" "7" "$(printf 'abc\n' | ql_num_or 7)"
    assert_eq "empty input -> default" "7" "$(printf '\n' | ql_num_or 7)"
}

test_ql_regex_pick() {
    _qlib_load
    declare -F ql_regex_pick >/dev/null || skip "ql_regex_pick not present"
    out=$(printf '+QCFG: "usbnet",1\r\n' | ql_regex_pick '"usbnet",([0-9])')
    assert_rc "ql_regex_pick rc 0" 0 $?
    assert_eq "first capture group" "1" "$out"
    printf 'no match here\n' | ql_regex_pick 'x(9)y' >/dev/null 2>&1
    assert_rc "no match -> rc 1" 1 $?
}

test_ql_kv_get() {
    _qlib_load
    declare -F ql_kv_get >/dev/null || skip "ql_kv_get not present"
    out=$(printf '  PIN: 1234\r\n' | ql_kv_get 'PIN')
    assert_rc "ql_kv_get rc 0" 0 $?
    assert_eq "value extracted with padding/CR stripped" "1234" "$out"
    printf 'other: 1\n' | ql_kv_get 'PIN' >/dev/null 2>&1
    assert_rc "no match -> rc 1" 1 $?
}

# Pins the documented contract "trailing whitespace/CR are stripped".
# KNOWN-FAIL until ql_kv_get is fixed: the greedy (.*) capture keeps the
# trailing \r, contradicting the library's own header comment.
test_ql_kv_get_trailing_cr() {
    _qlib_load
    declare -F ql_kv_get >/dev/null || skip "ql_kv_get not present"
    out=$(printf '  PIN: 1234\r\n' | ql_kv_get 'PIN')
    assert_eq "CR stripped per documented contract" "1234" "$out"
}

# Boundary: interior empty fields yield empty output lines; a field beyond
# the end of the line also yields an empty line (awk $n contract).
test_ql_csv_field_empty_fields() {
    _qlib_load
    declare -F ql_csv_field >/dev/null || skip "ql_csv_field not present"
    out=$(printf 'a,,c\n' | ql_csv_field 2)
    assert_rc "empty interior field rc 0" 0 $?
    assert_eq "empty interior field yields empty output" "" "$out"
    out=$(printf 'a,,c\n' | ql_csv_field 3)
    assert_eq "field after empty one is intact" "c" "$out"
    out=$(printf 'a,b\n' | ql_csv_field 5)
    assert_rc "missing field rc 0" 0 $?
    assert_eq "field beyond EOL yields empty output" "" "$out"
}

# Boundary: double quotes are NOT field delimiters (documented contract:
# byte-identical to awk -F','), so quoted commas split and quote chars are
# passed through verbatim.
test_ql_csv_field_quotes_not_special() {
    _qlib_load
    declare -F ql_csv_field >/dev/null || skip "ql_csv_field not present"
    out=$(printf '"a,b",c\n' | ql_csv_field 1)
    assert_eq "quote kept verbatim in field 1" '"a' "$out"
    out=$(printf '"a,b",c\n' | ql_csv_field 2)
    assert_eq "comma inside quotes still splits" 'b"' "$out"
    out=$(printf 'AT+QCFG="nwscanseq",020301\n' | ql_csv_field 2)
    assert_eq "realistic quoted-cmd field 2" "020301" "$out"
}

# Multi-line input: sed prints one value PER matching line. Note the library
# header comment claims "first matching line wins" — the implementation
# (sed -n -E s///p) actually emits every match; pinned here as the observed
# contract (defect note recorded in journal-tests-expand.md for the lib
# owner to fix the comment or the behavior).
test_ql_regex_pick_multiline_all_matches() {
    _qlib_load
    declare -F ql_regex_pick >/dev/null || skip "ql_regex_pick not present"
    out=$(printf '+QENG: "LTE",3\nnoise\n+QENG: "LTE",7\n' | ql_regex_pick '"LTE",([0-9])')
    assert_rc "multi-line match rc 0" 0 $?
    assert_eq "one value per matching line, in order" \
        "$(printf '3\n7')" "$out"
}

# Multi-line input: a match on a later line is still found.
test_ql_regex_pick_match_on_later_line() {
    _qlib_load
    declare -F ql_regex_pick >/dev/null || skip "ql_regex_pick not present"
    out=$(printf 'noise line\n+QENG: "NR5G-NSA",42\nmore noise\n' \
        | ql_regex_pick '"NR5G-NSA",([0-9]+)')
    assert_rc "match beyond first line rc 0" 0 $?
    assert_eq "later-line capture group returned" "42" "$out"
}

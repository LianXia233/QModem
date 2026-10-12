# shellcheck shell=bash
# Micro test framework for QModem regression tests (no external deps).
# Sourced by tests/sh/run_tests.sh; case files under tests/sh/cases/
# register test functions with t() and run them via run_registered().
#
# Contract:
#   - test functions run inside a command-substitution subshell, so
#     fail()/skip() only abort the current test, never the whole suite.
#   - fail()  -> FAIL record + abort current test
#   - skip()  -> SKIP record + abort current test
#   - normal return -> PASS record
# Exit code contract for CI: suite fails (1) iff any test FAILs; SKIP is OK.

TEST_PASS=0
TEST_FAIL=0
TEST_SKIP=0
_CURRENT=""
_reg=()

# register a test function by name
t() {
    _reg+=("$1")
}

# abort current test with a FAIL record; $1: reason
fail() {
    echo "FAIL: ${_CURRENT:-?}: $1" >&2
    exit 1
}

# abort current test with a SKIP record; $1: reason
skip() {
    echo "SKIP: ${_CURRENT:-?}: $1" >&2
    exit 77
}

# assert_eq <desc> <expected> <actual>
assert_eq() {
    if [ "$2" != "$3" ]; then
        fail "$1: expected [$2], got [$3]"
    fi
}

# assert_rc <desc> <expected_rc> <actual_rc>
assert_rc() {
    if [ "$2" -ne "$3" ]; then
        fail "$1: expected rc=$2, got rc=$3"
    fi
}

# assert_contains <desc> <haystack> <needle>
assert_contains() {
    case "$2" in
        *"$3"*) ;;
        *) fail "$1: [$3] not found in [$2]" ;;
    esac
}

# assert_nonempty <desc> <value>
assert_nonempty() {
    [ -n "$2" ] || fail "$1: value is empty"
}

# run every registered test; prints one PASS/SKIP/FAIL line each
run_registered() {
    local fn out rc
    for fn in "${_reg[@]}"; do
        _CURRENT=$fn
        out=$("$fn" 2>&1)
        rc=$?
        case $rc in
            0)
                TEST_PASS=$((TEST_PASS + 1))
                echo "PASS: $fn"
                ;;
            77)
                TEST_SKIP=$((TEST_SKIP + 1))
                echo "SKIP: $fn"
                ;;
            *)
                TEST_FAIL=$((TEST_FAIL + 1))
                echo "FAIL: $fn (rc=$rc)"
                [ -n "$out" ] && printf '%s\n' "$out" | sed 's/^/    /'
                ;;
        esac
    done
}

# print summary line; returns 0 iff no FAIL
summarize() {
    echo "----------------------------------------"
    echo "summary: PASS=$TEST_PASS SKIP=$TEST_SKIP FAIL=$TEST_FAIL"
    [ "$TEST_FAIL" -eq 0 ]
}

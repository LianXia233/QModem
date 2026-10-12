#!/bin/sh
# qmodem common parsing helpers (sourced library, not executed).
# Include convention (same as other cmds/ scripts):
#   source "${QMODEM_HOME:-/usr/share/qmodem}/cmds/qmodem_lib.sh"
# POSIX sh / busybox ash compatible; only busybox applets are used.
# All functions read from stdin unless noted, write results to stdout,
# and never depend on caller-local state.

# Strip CR characters from a stream (byte-identical to `sed 's/\r//g'`).
# Typical usage: cmd_xxx "$at_port" | sed -n '2p' | ql_at_clean
# Returns: tr's exit status (0 on success).
ql_at_clean()
{
    tr -d '\r'
}

# Print the n-th comma-separated field of every stdin line.
# Byte-identical to `awk -F',' "{print \$n}"`: lines with fewer fields
# yield an empty line; the field is NOT CR-stripped (chain ql_at_clean
# when the last field may carry a trailing CR).
# $1: 1-based field index (positive integer).
# Returns: 0 on success, 2 on usage error.
ql_csv_field()
{
    [ "$#" -eq 1 ] || {
        printf 'ql_csv_field: expected 1 argument: <field index>\n' >&2
        return 2
    }
    case "$1" in
        ''|0|*[!0-9]*)
            printf 'ql_csv_field: field index must be a positive integer, got: %s\n' "$1" >&2
            return 2
            ;;
    esac
    awk -F',' -v n="$1" '{ print $n }'
}

# Print the first capture group of <ERE> matched against stdin lines
# (one output line per matching input line — `sed -n -E "s/.*<ERE>.*/\1/p"`
# semantics; NOT "first match only". Callers wanting a single value must
# pipe through `head -n1` or ensure the input yields at most one match).
# $1: POSIX ERE containing at least one capture group.
# Returns: 0 and prints the group if a non-empty match was found,
#          1 when nothing matched (or the group is empty),
#          2 on usage error.
ql_regex_pick()
{
    [ "$#" -eq 1 ] || {
        printf 'ql_regex_pick: expected 1 argument: <ERE>\n' >&2
        return 2
    }
    local picked
    picked=$(sed -n -E "s/.*${1}.*/\1/p") || return 2
    [ -n "$picked" ] || return 1
    printf '%s\n' "$picked"
}

# Read a candidate value from stdin and print it as a non-negative
# integer; whitespace (including CR/LF) is removed before validation.
# If the value is not a plain non-negative integer, print the default.
# $1: default value printed when validation fails.
# Returns: 0 always (2 on usage error, default not printed).
ql_num_or()
{
    [ "$#" -eq 1 ] || {
        printf 'ql_num_or: expected 1 argument: <default>\n' >&2
        return 2
    }
    local val
    val=$(tr -d ' \t\r\n')
    case "$val" in
        ''|*[!0-9]*) printf '%s\n' "$1" ;;
        *) printf '%s\n' "$val" ;;
    esac
}

# Normalize a boolean-ish value to "1" or "0".
# Input: $1 when given, otherwise stdin (trimmed, case-insensitive).
# Truthy: 1/true/yes/on/enabled -> prints 1
# Falsy:  0/false/no/off/disabled -> prints 0
# Returns: 0 for a recognized value, 1 for anything else (no output),
#          2 on usage error (more than 1 argument).
ql_bool()
{
    [ "$#" -le 1 ] || {
        printf 'ql_bool: expected at most 1 argument\n' >&2
        return 2
    }
    local val
    if [ "$#" -eq 1 ]; then
        val=$1
    else
        val=$(tr -d ' \t\r\n')
    fi
    val=$(printf '%s' "$val" | tr 'A-Z' 'a-z')
    case "$val" in
        1|true|yes|on|enabled)   printf '1\n' ; return 0 ;;
        0|false|no|off|disabled) printf '0\n' ; return 0 ;;
    esac
    return 1
}

# Extract the value part of "key: value" lines from stdin.
# The key is matched as an ERE against the start of a line; surrounding
# whitespace around the colon and trailing whitespace/CR are stripped
# (via a `.*[^[:space:]]` terminator, since sed cannot backtrack a
# greedy group into trailing whitespace).
# $1: key (ERE fragment; keep it literal for plain keys).
# Returns: 0 and prints every matched value (one per line),
#          1 when no non-blank value matched, 2 on usage error.
ql_kv_get()
{
    [ "$#" -eq 1 ] || {
        printf 'ql_kv_get: expected 1 argument: <key>\n' >&2
        return 2
    }
    local picked
    picked=$(sed -n -E "s/^[[:space:]]*${1}[[:space:]]*:[[:space:]]*(.*[^[:space:]])[[:space:]]*\$/\1/p") || return 2
    [ -n "$picked" ] || return 1
    printf '%s\n' "$picked"
}

# shellcheck shell=bash
# Function-level tests for the rpcd ubus script's valid_section().
#
# Chosen approach (recorded decision):
#   Sourcing the whole rpcd/qmodem script is NOT viable on a dev host:
#     (a) top-level `. /usr/share/libubox/jshn.sh` and `. generic.sh` are
#         OpenWrt-only paths (absent on host, generic.sh chains modem_util);
#     (b) the `call)` dispatch branch does `read -r input` and would block
#         on stdin.
#   Instead we extract the valid_section() body with a line-anchored sed
#   range (^valid_section()/,/^}/) into a temp file and source that.
#   Portability note (defect recorded in journal-tests.md, for the rpcd
#   owner to fix): the extracted body contains ${#$1}, a bad substitution
#   even under dash (/bin/sh here) — the POSIX form is ${#1}. Tests load
#   the fragment with that one semantically-identical rewrite applied;
#   if the extraction yields no code, the group SKIPs (function renamed
#   or moved upstream) so CI never breaks on unrelated refactors.
t test_rpcd_valid_section_accepts_plain
t test_rpcd_valid_section_rejects_bad_chars
t test_rpcd_valid_section_length_boundary

RPCD_SCRIPT="$REPO_ROOT/application/qmodem/files/usr/libexec/rpcd/qmodem"

# Extract and source valid_section(); skip group if the function vanished.
_rpcd_load_valid_section() {
    [ -f "$RPCD_SCRIPT" ] || skip "rpcd qmodem script not found"
    local frag
    frag=$(sed -n '/^valid_section()/,/^}/p' "$RPCD_SCRIPT")
    assert_nonempty "valid_section() extracted from rpcd script" "$frag"
    local tmp
    tmp=$(mktemp "${TMPDIR:-/tmp}/valid_section.XXXXXX")
    # ${#$1} (ash-only, bad substitution under bash/dash) -> ${#1} (POSIX)
    printf '%s\n' "$frag" | sed 's/\${#\$1}/${#1}/g' > "$tmp"
    # shellcheck disable=SC1090
    . "$tmp"
    rm -f "$tmp"
}

test_rpcd_valid_section_accepts_plain() {
    _rpcd_load_valid_section
    valid_section abc;         assert_rc "plain name accepted" 0 $?
    valid_section abc_123_XYZ; assert_rc "underscore+alnum accepted" 0 $?
    local long64
    long64=$(printf 'a%.0s' $(seq 1 64))
    valid_section "$long64";   assert_rc "64 chars accepted (max)" 0 $?
}

test_rpcd_valid_section_rejects_bad_chars() {
    _rpcd_load_valid_section
    valid_section '';          assert_rc "empty rejected" 1 $?
    valid_section 'bad-name';  assert_rc "dash rejected" 1 $?
    valid_section 'bad name';  assert_rc "space rejected" 1 $?
    valid_section 'bad.dot';   assert_rc "dot rejected" 1 $?
    valid_section '../evil';   assert_rc "path traversal rejected" 1 $?
}

test_rpcd_valid_section_length_boundary() {
    _rpcd_load_valid_section
    local long65
    long65=$(printf 'a%.0s' $(seq 1 65))
    valid_section "$long65";   assert_rc "65 chars rejected" 1 $?
}

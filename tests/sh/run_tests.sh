#!/usr/bin/env bash
# QModem lightweight shell regression suite (no pytest, no external framework).
# Usage: ./tests/sh/run_tests.sh
# CI contract: exit 0 = pass (SKIP allowed); exit 1 = at least one FAIL.
set -u

TESTS_DIR=$(CDPATH= cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(CDPATH= cd "$TESTS_DIR/../.." && pwd)
export REPO_ROOT

# shellcheck source=framework.sh
. "$TESTS_DIR/framework.sh"

shopt -s nullglob
case_files=("$TESTS_DIR"/cases/case_*.sh)
shopt -u nullglob

if [ "${#case_files[@]}" -eq 0 ]; then
    echo "FAIL: no case files found under tests/sh/cases/" >&2
    exit 1
fi

for case_file in "${case_files[@]}"; do
    # shellcheck disable=SC1090
    . "$case_file"
done

run_registered
summarize

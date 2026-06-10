#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
set +e
output="$(cargo run --quiet -- --jsno 2>&1)"
status=$?
set -e
test "$status" -eq 2
grep -F 'did you mean `--json`' <<<"$output"
grep -F "next: assess capabilities --json" <<<"$output"

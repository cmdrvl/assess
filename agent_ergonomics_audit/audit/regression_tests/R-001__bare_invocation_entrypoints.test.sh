#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
output="$(cargo run --quiet --)"
grep -F "Agent entrypoints:" <<<"$output"
grep -F "assess --robot-triage" <<<"$output"
grep -F "assess capabilities --json" <<<"$output"
grep -F "assess robot-docs guide" <<<"$output"

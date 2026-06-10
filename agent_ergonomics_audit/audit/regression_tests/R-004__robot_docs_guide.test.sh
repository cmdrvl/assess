#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
output="$(cargo run --quiet -- robot-docs guide)"
grep -F "assess robot-docs guide" <<<"$output"
grep -F "assess <ARTIFACT>... --policy <PATH> --json" <<<"$output"

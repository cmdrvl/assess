#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
cargo run --quiet -- capabilities --json | jq -e '.schema == "assess.doctor.capabilities.v1" and (.agent_entrypoints | length) >= 3'
cargo run --quiet -- --json | jq -e '.schema == "assess.doctor.capabilities.v1"'

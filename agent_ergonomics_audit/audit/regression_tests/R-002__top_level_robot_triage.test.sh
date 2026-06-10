#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
cargo run --quiet -- --robot-triage | jq -e '.schema == "assess.doctor.triage.v1" and .read_only == true'

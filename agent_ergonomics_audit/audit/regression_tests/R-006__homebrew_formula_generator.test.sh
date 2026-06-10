#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../../.."
if grep -F 'version "{bare}"' .github/workflows/release.yml; then
  exit 1
fi
if grep -F 'blocks.append(f"\n  {os_block} do")' .github/workflows/release.yml; then
  exit 1
fi
grep -F 'blocks.append(f"  {os_block} do")' .github/workflows/release.yml

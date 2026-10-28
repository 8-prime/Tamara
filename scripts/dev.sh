#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run &
backend_pid=$!
trap 'kill "$backend_pid" 2>/dev/null || true' EXIT INT TERM
pnpm --dir frontend dev

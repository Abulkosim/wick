#!/usr/bin/env bash
# Every check that must pass before a commit. Stops at the first failure.
set -euo pipefail
cd "$(dirname "$0")/.."
pnpm typecheck
pnpm test
pnpm test:e2e --reporter=dot
node --test scripts/*.test.mjs
cd src-tauri
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --quiet
echo "All checks passed."

#!/usr/bin/env bash
# Full local gate: formatting, lints, tests.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --all --check
cargo clippy --all-targets -q -- -D warnings
cargo test -q

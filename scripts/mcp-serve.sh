#!/usr/bin/env bash
#
# Build and run the Aether system MCP server from a development checkout.
# MCP clients spawn this on stdio; keep build output off stdout.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cargo build --quiet -p aether-mcp --manifest-path "${ROOT}/os/compositor/Cargo.toml"
exec "${ROOT}/os/compositor/target/debug/aether-mcp" "$@"

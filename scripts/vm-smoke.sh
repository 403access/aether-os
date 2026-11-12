#!/usr/bin/env bash
# Convenience wrapper: boot the VM headlessly and verify it reaches multi-user.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "${ROOT}/scripts/vm-run.sh" --smoke

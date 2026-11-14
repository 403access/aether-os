#!/usr/bin/env bash
# Development wrapper: launch the Water-themed terminal used by Xvfb smokes.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export LANG="${LANG:-C.UTF-8}"
exec foot -c "${ROOT}/foot/foot.ini" -T "Wayland Terminal" "$@"

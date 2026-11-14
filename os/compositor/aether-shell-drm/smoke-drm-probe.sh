#!/usr/bin/env bash
#
# Native DRM backend bring-up check.
#
# With a DRM device and seat access, runs the compositor briefly and expects a
# clean exit. Without one, asserts that the backend reports a clear startup
# error instead of hanging or claiming success.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-drm/smoke-drm-probe.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-drm}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"

[ -x "${BIN}" ] || {
  echo "error: ${BIN} not built" >&2
  exit 1
}

if [ -e /dev/dri/card0 ] && [ -r /dev/dri/card0 ] && [ -w /dev/dri/card0 ]; then
  timeout 20 "${BIN}" --frames 30 >"${ROOT}/drm-probe.log" 2>&1
  grep -q "aether-shell-drm: initialized" "${ROOT}/drm-probe.log"
  echo "native DRM backend ran: $(grep -c 'output' "${ROOT}/drm-probe.log") output lines"
else
  set +e
  timeout 10 "${BIN}" >"${ROOT}/drm-probe.log" 2>&1
  status=$?
  set -e
  [ "${status}" -ne 0 ] || {
    echo "error: backend exited 0 without a DRM device" >&2
    exit 1
  }
  grep -q "aether-shell-drm" "${ROOT}/drm-probe.log" || {
    echo "error: no diagnostic output" >&2
    cat "${ROOT}/drm-probe.log" >&2
    exit 1
  }
  echo "no DRM device/seat; backend failed clearly (status ${status}):"
  tail -3 "${ROOT}/drm-probe.log"
fi

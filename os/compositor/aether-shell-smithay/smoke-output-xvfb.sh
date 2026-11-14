#!/usr/bin/env bash
#
# Output globals: start the compositor and list its globals with wayland-info,
# asserting wl_output/xdg-output geometry, mode and scale.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-output-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-83}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-4}"

[ -x "${BIN}" ] || {
  echo "error: ${BIN} not built; run cargo build --manifest-path ${ROOT}/Cargo.toml" >&2
  exit 1
}
command -v wayland-info >/dev/null || {
  echo "error: wayland-info missing; use nix develop .#smithay" >&2
  exit 1
}

mkdir -p "${OUT}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
unset AETHER_TEST_CLIENT

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-output.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
cleanup() {
  [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true
  kill "${XVFB_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
  wait "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

sleep 1
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/output-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

SOCKET="$(grep -o 'WAYLAND_DISPLAY=[^ ]*' "${OUT}/output-app.log" | head -1 | cut -d= -f2)"
if [ -z "${SOCKET}" ]; then
  echo "error: compositor did not report a socket" >&2
  cat "${OUT}/output-app.log" >&2
  exit 1
fi
WAYLAND_DISPLAY="${SOCKET}" wayland-info >"${OUT}/wayland-info.txt" 2>&1 || true

for expected in \
  "interface: 'wl_output'" \
  "interface: 'zxdg_output_manager_v1'" \
  "logical_width: 1280" \
  "logical_height: 800" \
  "width: 1280 px" \
  "height: 800 px" \
  "refresh: 60.000 Hz" \
  "scale: 1"; do
  if ! grep -q "${expected}" "${OUT}/wayland-info.txt"; then
    echo "error: missing output property: ${expected}" >&2
    cat "${OUT}/wayland-info.txt" >&2
    exit 1
  fi
done

cat "${OUT}/output-app.log"
grep -E "interface: 'wl_output'|interface: 'zxdg_output_manager_v1'|logical_|width: 1280|height: 800|refresh:|scale:" "${OUT}/wayland-info.txt"

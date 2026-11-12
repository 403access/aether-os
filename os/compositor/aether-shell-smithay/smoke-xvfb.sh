#!/usr/bin/env bash
#
# Run the Smithay winit backend under Xvfb and capture the window.
# Run inside `nix develop .#smithay` (it provides Mesa/EGL, Xvfb and ImageMagick).
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-99}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-3}"
ARGS="${ARGS:---demo}"
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"

[ -x "${BIN}" ] || {
  echo "error: ${BIN} not built; run cargo build --manifest-path ${ROOT}/Cargo.toml" >&2
  exit 1
}

mkdir -p "${OUT}"
Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb.log" 2>&1 &
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
# shellcheck disable=SC2086
DISPLAY=":${DISPLAY_NUM}" "${BIN}" ${ARGS} >"${OUT}/app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

DISPLAY=":${DISPLAY_NUM}" import -window root "${OUT}/window.png"
cat "${OUT}/app.log"
echo "wrote ${OUT}/window.png"

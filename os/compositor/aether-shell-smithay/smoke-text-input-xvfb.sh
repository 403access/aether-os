#!/usr/bin/env bash
#
# Text input/IME: verify the compositor advertises text-input and input-method.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-text-input-xvfb.sh
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-82}"
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

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-text-input.log" 2>&1 &
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
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/text-input-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

SOCKET="$(grep -o 'WAYLAND_DISPLAY=[^ ]*' "${OUT}/text-input-app.log" | head -1 | cut -d= -f2)"
if [ -z "${SOCKET}" ]; then
  echo "error: compositor did not report a socket" >&2
  cat "${OUT}/text-input-app.log" >&2
  exit 1
fi
WAYLAND_DISPLAY="${SOCKET}" wayland-info >"${OUT}/text-input-info.txt" 2>&1 || true

for global in zwp_text_input_manager_v3 zwp_input_method_manager_v2; do
  if ! grep -q "interface: '${global}'" "${OUT}/text-input-info.txt"; then
    echo "error: ${global} global is missing" >&2
    cat "${OUT}/text-input-info.txt" >&2
    exit 1
  fi
done

cat "${OUT}/text-input-app.log"
grep -E "zwp_text_input_manager_v3|zwp_input_method_manager_v2" "${OUT}/text-input-info.txt"

#!/usr/bin/env bash
#
# Clipboard persistence: wl-clip-persist re-owns a clipboard after the original
# source exits, so pastes keep working once the copying client is gone.
#
#   nix develop .#smithay -c ./os/compositor/aether-shell-smithay/smoke-clipboard-persist-xvfb.sh
#
# The copy keeps its source in the foreground and then kills it. The previous
# mechanism, `wl-copy --paste-once`, makes the source exit on whichever client
# reads first, so the assertion depended on whether the persist helper or the
# paste happened to win that race. An explicit exit tests the same claim, in a
# fixed order, and reports where it broke when it does.
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="${BIN:-${ROOT}/target/debug/aether-shell-smithay}"
OUT="${OUT:-${ROOT}/../../../artifacts/smithay}"
DISPLAY_NUM="${DISPLAY_NUM:-76}"
WIDTH="${WIDTH:-1280}"
HEIGHT="${HEIGHT:-800}"
SETTLE="${SETTLE:-5}"

[ -x "${BIN}" ] || {
  echo "error: ${BIN} not built; run cargo build --manifest-path ${ROOT}/Cargo.toml" >&2
  exit 1
}
command -v xdotool >/dev/null || {
  echo "error: xdotool missing; use nix develop .#smithay" >&2
  exit 1
}
command -v wl-clip-persist >/dev/null || {
  echo "error: wl-clip-persist missing; use nix develop .#smithay" >&2
  exit 1
}

mkdir -p "${OUT}"
export OUT
export XDG_RUNTIME_DIR="${XDG_RUNTIME_DIR:-$(mktemp -d)}"
chmod 700 "${XDG_RUNTIME_DIR}"
export AETHER_TEST_CLIENT="${AETHER_TEST_CLIENT:-weston-terminal}"
export AETHER_TEST_CLIENT_COUNT=2

FIRST="${OUT}/persist-first.txt"
SECOND="${OUT}/persist-second.txt"
COPIED="${OUT}/persist-copied.marker"
PASTED="${OUT}/persist-pasted.marker"
PID_FILE="${OUT}/persist-copy.pid"
ready_left="${OUT}/persist-left-ready.marker"
ready_right="${OUT}/persist-right-ready.marker"
rm -f "${FIRST}" "${SECOND}" "${COPIED}" "${PASTED}" "${PID_FILE}" \
  "${ready_left}" "${ready_right}"

Xvfb ":${DISPLAY_NUM}" -screen 0 "${WIDTH}x${HEIGHT}x24" -ac >"${OUT}/xvfb-persist.log" 2>&1 &
XVFB_PID=$!
APP_PID=""
PERSIST_PID=""
cleanup() {
  [ -n "${APP_PID}" ] && kill "${APP_PID}" 2>/dev/null || true
  [ -n "${PERSIST_PID}" ] && kill "${PERSIST_PID}" 2>/dev/null || true
  kill "${XVFB_PID}" 2>/dev/null || true
  wait "${APP_PID}" 2>/dev/null || true
  wait "${PERSIST_PID}" 2>/dev/null || true
  wait "${XVFB_PID}" 2>/dev/null || true
}
trap cleanup EXIT

sleep 1
DISPLAY=":${DISPLAY_NUM}" "${BIN}" >"${OUT}/persist-app.log" 2>&1 &
APP_PID=$!
sleep "${SETTLE}"

# CI can take longer than SETTLE to map both terminals. Do not type until the
# compositor has actually accepted them.
for _ in $(seq 1 40); do
  [ "$(grep -c 'client connected' "${OUT}/persist-app.log" || true)" -ge 2 ] && break
  sleep 0.25
done

wait_for() {
  local path="$1"
  local seconds="${2:-10}"
  local deadline=$(( $(date +%s) + seconds ))
  while [ "$(date +%s)" -lt "${deadline}" ]; do
    [ -s "${path}" ] && return 0
    sleep 0.25
  done
  return 1
}

SOCKET="$(grep -o 'WAYLAND_DISPLAY=[^ ]*' "${OUT}/persist-app.log" | head -1 | cut -d= -f2)"
selection_types() {
  WAYLAND_DISPLAY="${SOCKET}" wl-paste --list-types 2>/dev/null || true
}

# The helper has to be running before the source exists, so it sees the change.
WAYLAND_DISPLAY="${SOCKET}" wl-clip-persist --clipboard regular \
  >"${OUT}/persist-helper.log" 2>&1 &
PERSIST_PID=$!
sleep 1

export DISPLAY=":${DISPLAY_NUM}"
WINDOW_ID="$(xdotool search --name "Aether OS" | head -1)"
xdotool windowfocus "${WINDOW_ID}"

# Type one line into a terminal and press Enter.
type_line() {
  local x="$1" text="$2"
  xdotool mousemove "${x}" 300 click 1
  sleep 0.8
  xdotool type --delay 40 "${text}"
  xdotool key Return
}

# Prove a terminal takes keyboard input before relying on it. Without this the
# first typed command of a smoke silently disappears when the window has not
# been focused yet, and the failure surfaces as whatever the command was
# supposed to produce.
ready_terminal() {
  local x="$1" name="$2" marker="$3"
  local attempt=0
  until wait_for "${marker}" 5; do
    attempt=$((attempt + 1))
    if [ "${attempt}" -gt 4 ]; then
      echo "error: ${name} terminal did not accept input" >&2
      cat "${OUT}/persist-app.log" >&2
      exit 1
    fi
    rm -f "${marker}"
    type_line "${x}" "echo ok > \"${marker}\""
  done
}

diagnose() {
  echo "--- selection types: $(selection_types | tr '\n' ' ')" >&2
  echo "--- first: $(wc -c <"${FIRST}" 2>/dev/null || echo missing) bytes" >&2
  cat "${FIRST}" 2>/dev/null >&2 || true
  echo "--- helper log" >&2
  cat "${OUT}/persist-helper.log" >&2 || true
  echo "--- app log" >&2
  cat "${OUT}/persist-app.log" >&2 || true
}

ready_terminal 300 left "${ready_left}"
ready_terminal 900 right "${ready_right}"

# Publish a selection and keep the source alive so its exit is ours to choose.
# `$!` is expanded by the terminal's shell, not here.
type_line 300 "printf 'persist-live' | wl-copy --foreground & echo \$! > \"${PID_FILE}\""

attempt=0
until [ -n "$(selection_types)" ]; do
  attempt=$((attempt + 1))
  if [ "${attempt}" -gt 40 ]; then
    echo "error: the source never published a selection" >&2
    diagnose
    exit 1
  fi
  sleep 0.25
done
echo ok >"${COPIED}"

COPY_PID="$(cat "${PID_FILE}" 2>/dev/null || true)"
[ -n "${COPY_PID}" ] || {
  echo "error: copy pid missing; the terminal did not run the copy" >&2
  diagnose
  exit 1
}
kill "${COPY_PID}" 2>/dev/null || true
for _ in $(seq 1 40); do
  kill -0 "${COPY_PID}" 2>/dev/null || break
  sleep 0.1
done

# The source is gone. Anything the compositor still serves now belongs to the
# persist helper, and it must carry the data it read while the source lived.
attempt=0
until [ -n "$(selection_types)" ]; do
  attempt=$((attempt + 1))
  if [ "${attempt}" -gt 40 ]; then
    echo "error: selection vanished when the source exited" >&2
    diagnose
    exit 1
  fi
  sleep 0.25
done

# First paste: consumes nothing retroactively, but it is the claim users feel.
paste_attempts=0
until [ -s "${FIRST}" ] && grep -q "persist-live" "${FIRST}"; do
  paste_attempts=$((paste_attempts + 1))
  if [ "${paste_attempts}" -gt 4 ]; then
    echo "error: first paste failed" >&2
    diagnose
    exit 1
  fi
  type_line 900 "wl-paste > \"${FIRST}\" && echo ok > \"${PASTED}\""
  sleep 1
done

# Second paste: still owned by the helper, no source anywhere.
type_line 900 "wl-paste > \"${SECOND}\""
wait_for "${SECOND}" 10 || true
import -window root "${OUT}/clipboard-persist.png"

if ! grep -q "persist-live" "${SECOND}"; then
  echo "error: clipboard did not persist after the source exited" >&2
  diagnose
  exit 1
fi

cat "${OUT}/persist-helper.log"
echo "clipboard persisted after source exit"
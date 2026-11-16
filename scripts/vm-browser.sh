#!/usr/bin/env bash
#
# Serve the development VM desktop to a browser with noVNC.
#
# Boots (or reuses) the QEMU/VNC VM from ./scripts/vm-run.sh --gui, proxies its
# RFB stream to a loopback HTTP/WebSocket endpoint and opens noVNC. The VM keeps
# running when the bridge stops; use --stop-vm for a VM this script started.
#
#   ./scripts/vm-browser.sh                  # boot if needed, start bridge, open a browser
#   ./scripts/vm-browser.sh --no-open        # start only, print the URL
#   ./scripts/vm-browser.sh --no-boot        # require an already running VNC VM
#   ./scripts/vm-browser.sh --stop           # stop the noVNC bridge
#   ./scripts/vm-browser.sh --stop-vm        # stop a VM started by this script
#   ./scripts/vm-browser.sh --stop-all       # both
#
# Environment:
#   VM_VNC_DISPLAY=5          QEMU VNC display (port 5900 + N)
#   VM_VNC_PASSWORD=aether    VNC password set by vm-run.sh, pre-filled in the URL
#   VM_BROWSER_PORT=6080      HTTP/WebSocket port for noVNC
#   VM_BROWSER_APP="Google Chrome"  macOS app passed to `open -a`
#   NOVNC_DIR=artifacts/novnc noVNC checkout (gitignored)
#   NOVNC_REF=                optional branch or tag to clone
#
# Requires: git, python3, curl and websockify (via `websockify`, `uvx`, or
# `python3 -m websockify`). Booting also needs qemu (see scripts/vm-run.sh).
#
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ART="${ROOT}/artifacts"
VM_ART="${ART}/vm"

NOVNC_DIR="${NOVNC_DIR:-${ART}/novnc}"
VM_VNC_DISPLAY="${VM_VNC_DISPLAY:-5}"
WEB_PORT="${VM_BROWSER_PORT:-6080}"
BROWSER_APP="${VM_BROWSER_APP:-Google Chrome}"
VNC_PASSWORD="${VM_VNC_PASSWORD:-aether}"
VM_TIMEOUT="${VM_TIMEOUT:-240}"

OPEN=1
BOOT=1
ACTION=start
BRIDGE_STOP_HINT=1

say() { printf '\n\033[1;36m==>\033[0m %s\n' "$*"; }
die() { printf '\n\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

usage() {
  cat <<'EOF'
Serve the development VM desktop to a browser with noVNC.

Boots (or reuses) the QEMU/VNC VM from ./scripts/vm-run.sh --gui, proxies its
RFB stream to a loopback HTTP/WebSocket endpoint and opens noVNC. The VM keeps
running when the bridge stops; use --stop-vm for a VM this script started.

  ./scripts/vm-browser.sh                  boot if needed, start bridge, open a browser
  ./scripts/vm-browser.sh --no-open        start only, print the URL
  ./scripts/vm-browser.sh --no-boot        require an already running VNC VM
  ./scripts/vm-browser.sh --stop           stop the noVNC bridge
  ./scripts/vm-browser.sh --stop-vm        stop a VM started by this script
  ./scripts/vm-browser.sh --stop-all       both

Environment:
  VM_VNC_DISPLAY=5          QEMU VNC display (port 5900 + N)
  VM_VNC_PASSWORD=aether    VNC password set by vm-run.sh, pre-filled in the URL
  VM_BROWSER_PORT=6080      HTTP/WebSocket port for noVNC
  VM_BROWSER_APP="Google Chrome"  macOS app passed to `open -a`
  NOVNC_DIR=artifacts/novnc noVNC checkout (gitignored)
  NOVNC_REF=                optional branch or tag to clone

Requires: git, python3, curl and websockify (via `websockify`, `uvx`, or
`python3 -m websockify`). Booting also needs qemu (see scripts/vm-run.sh).
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --no-open) OPEN=0 ;;
    --open) OPEN=1 ;;
    --no-boot) BOOT=0 ;;
    --stop) ACTION=stop ;;
    --stop-vm) ACTION=stop-vm ;;
    --stop-all) ACTION=stop-all ;;
    --port)
      WEB_PORT="${2:?--port needs a value}"
      shift
      ;;
    --vnc-port)
      VNC_PORT="${2:?--vnc-port needs a value}"
      shift
      ;;
    --help | -h)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1 (try --help)"
      ;;
  esac
  shift
done

VNC_PORT="${VNC_PORT:-$((5900 + VM_VNC_DISPLAY))}"
BRIDGE_PID="${VM_ART}/browser-bridge-${WEB_PORT}.pid"
BRIDGE_LOG="${VM_ART}/browser-bridge-${WEB_PORT}.log"
VM_PID="${VM_ART}/browser-vm.pid"
VM_LOG="${VM_ART}/browser-vm.log"

listening() {
  python3 - "$1" <<'PY'
import socket, sys
sock = socket.socket()
sock.settimeout(0.3)
try:
    sock.connect(("127.0.0.1", int(sys.argv[1])))
except OSError:
    raise SystemExit(1)
finally:
    sock.close()
raise SystemExit(0)
PY
}

port_open() {
  local port="$1" seconds="$2" start
  start="$(date +%s)"
  while ! listening "${port}"; do
    if [ $(( $(date +%s) - start )) -ge "${seconds}" ]; then
      return 1
    fi
    sleep 0.5
  done
}

url_encode() {
  python3 -c 'import sys, urllib.parse; print(urllib.parse.quote(sys.argv[1], safe=""))' "$1"
}

stop_pidfile() {
  local file="$1" label="$2"
  if [ ! -f "${file}" ]; then
    say "no ${label} pid file (${file}); nothing to stop"
    return
  fi
  local pid
  pid="$(cat "${file}")"
  if kill -0 "${pid}" 2>/dev/null; then
    kill "${pid}" 2>/dev/null || true
    say "stopped ${label} (pid ${pid})"
  else
    say "${label} was not running; removed a stale pid file"
  fi
  rm -f "${file}"
}

ensure_novnc() {
  [ -f "${NOVNC_DIR}/vnc.html" ] && return
  command -v git >/dev/null 2>&1 || die "git is required to fetch noVNC"
  say "fetching noVNC into ${NOVNC_DIR}"
  mkdir -p "$(dirname "${NOVNC_DIR}")"
  local tmp
  tmp="$(mktemp -d)"
  if [ -n "${NOVNC_REF:-}" ]; then
    git clone --depth 1 --branch "${NOVNC_REF}" https://github.com/novnc/noVNC.git "${tmp}/noVNC"
  else
    git clone --depth 1 https://github.com/novnc/noVNC.git "${tmp}/noVNC"
  fi
  rm -rf "${NOVNC_DIR}"
  mv "${tmp}/noVNC" "${NOVNC_DIR}"
  rmdir "${tmp}" 2>/dev/null || true
}

boot_vm() {
  if listening "${VNC_PORT}"; then
    return
  fi
  if pgrep -f "qemu-system-aarch64" >/dev/null 2>&1; then
    say "a QEMU VM is already running; waiting for VNC on 127.0.0.1:${VNC_PORT}"
    port_open "${VNC_PORT}" "${VM_TIMEOUT}" || die "timed out waiting for VNC on ${VNC_PORT}"
    return
  fi
  [ "${BOOT}" = "1" ] || die "no VNC listener on 127.0.0.1:${VNC_PORT}; run ./scripts/vm-run.sh --gui (or drop --no-boot)"
  [ $((VNC_PORT - 5900)) -ge 1 ] || die "invalid VNC port ${VNC_PORT} (expected 5900 + display)"
  mkdir -p "${VM_ART}"
  say "booting the VM with VNC on 127.0.0.1:${VNC_PORT}"
  : >"${VM_LOG}"
  nohup env \
    VM_GUI_DISPLAY=vnc \
    VM_VNC_DISPLAY="$((VNC_PORT - 5900))" \
    VM_VNC_PASSWORD="${VNC_PASSWORD}" \
    "${ROOT}/scripts/vm-run.sh" --gui >"${VM_LOG}" 2>&1 &
  echo $! >"${VM_PID}"
  local start
  start="$(date +%s)"
  while true; do
    if grep -aq "AETHER_VM_READY" "${VM_LOG}" 2>/dev/null; then
      break
    fi
    if ! kill -0 "$(cat "${VM_PID}")" 2>/dev/null; then
      tail -40 "${VM_LOG}" >&2 || true
      die "QEMU exited before AETHER_VM_READY (see ${VM_LOG})"
    fi
    if [ $(( $(date +%s) - start )) -ge "${VM_TIMEOUT}" ]; then
      die "timed out waiting for the VM after ${VM_TIMEOUT}s (see ${VM_LOG})"
    fi
    sleep 2
  done
  say "VM ready; the graphics session starts on tty8"
}

start_bridge() {
  if listening "${WEB_PORT}"; then
    if curl -sf "http://127.0.0.1:${WEB_PORT}/vnc.html" >/dev/null 2>&1; then
      say "reusing the noVNC bridge already listening on 127.0.0.1:${WEB_PORT}"
      if [ -f "${BRIDGE_PID}" ]; then
        say "stop it with ./scripts/vm-browser.sh --port ${WEB_PORT} --stop"
      else
        say "it was started outside this script; stop that process yourself"
        BRIDGE_STOP_HINT=0
      fi
      return
    fi
    die "port ${WEB_PORT} is in use and does not serve noVNC; set VM_BROWSER_PORT or --port"
  fi

  local web_cmd
  if command -v websockify >/dev/null 2>&1; then
    web_cmd=(websockify)
  elif command -v uvx >/dev/null 2>&1; then
    web_cmd=(uvx --from websockify websockify)
  elif python3 -c 'import websockify' >/dev/null 2>&1; then
    web_cmd=(python3 -m websockify)
  else
    die "websockify not found; install it with 'uv tool install websockify' or 'pip install websockify'"
  fi

  ensure_novnc
  mkdir -p "${VM_ART}"
  say "starting noVNC on 127.0.0.1:${WEB_PORT} -> VNC 127.0.0.1:${VNC_PORT}"
  nohup "${web_cmd[@]}" --web "${NOVNC_DIR}" \
    "127.0.0.1:${WEB_PORT}" "127.0.0.1:${VNC_PORT}" >"${BRIDGE_LOG}" 2>&1 &
  echo $! >"${BRIDGE_PID}"
  if ! port_open "${WEB_PORT}" 90; then
    tail -30 "${BRIDGE_LOG}" >&2 || true
    die "noVNC did not start within 90s (see ${BRIDGE_LOG})"
  fi
  # `uvx` may hand off to the tool process after binding; record the actual
  # listener so --stop terminates websockify rather than a wrapper.
  if command -v lsof >/dev/null 2>&1; then
    local listener
    listener="$(lsof -tiTCP:"${WEB_PORT}" -sTCP:LISTEN 2>/dev/null | head -1 || true)"
    [ -n "${listener}" ] && echo "${listener}" >"${BRIDGE_PID}"
  fi
}

open_browser() {
  local url="$1"
  [ "${OPEN}" = "1" ] || return
  case "$(uname -s)" in
    Darwin)
      if [ -d "/Applications/${BROWSER_APP}.app" ]; then
        open -a "${BROWSER_APP}" "${url}"
      else
        open "${url}"
      fi
      ;;
    *)
      if command -v xdg-open >/dev/null 2>&1; then
        xdg-open "${url}" >/dev/null 2>&1 &
      else
        say "open ${url}"
      fi
      ;;
  esac
}

case "${ACTION}" in
  stop)
    stop_pidfile "${BRIDGE_PID}" "noVNC bridge"
    exit 0
    ;;
  stop-vm)
    stop_pidfile "${VM_PID}" "VM"
    exit 0
    ;;
  stop-all)
    stop_pidfile "${BRIDGE_PID}" "noVNC bridge"
    stop_pidfile "${VM_PID}" "VM"
    exit 0
    ;;
esac

boot_vm
start_bridge

URL="http://127.0.0.1:${WEB_PORT}/vnc.html?autoconnect=1&resize=scale&reconnect=1&password=$(url_encode "${VNC_PASSWORD}")"
say "desktop: ${URL}"
say "password: ${VNC_PASSWORD} (pre-filled); loopback only"
if [ "${BRIDGE_STOP_HINT}" = "1" ]; then
  say "stop the bridge: ./scripts/vm-browser.sh --port ${WEB_PORT} --stop"
fi
say "logs: ${BRIDGE_LOG} · ${VM_LOG}"
open_browser "${URL}"
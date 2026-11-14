#!/usr/bin/env bash
#
# Send keys to the running Aether VM through QEMU's QMP socket.
# Works even when the VNC client does not deliver modifier keys.
#
#   ./scripts/vm-send-key.sh meta_l-spc
#   ./scripts/vm-send-key.sh --text "new" ret
#   ./scripts/vm-send-key.sh meta_l-ret
#
# The VM must be running with scripts/vm-run.sh --gui (which enables QMP).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOCKET="${VM_QMP_SOCKET:-}"

if [ -z "${SOCKET}" ]; then
  for pid in $(pgrep -f 'qemu-system-aarch64.*-qmp' 2>/dev/null || true); do
    command="$(ps -ww -o command= -p "${pid}" 2>/dev/null || true)"
    candidate="$(printf '%s\n' "${command}" | sed -n 's/.*-qmp unix:\([^,]*\),server.*/\1/p')"
    if [ -n "${candidate}" ] && [ -S "${candidate}" ]; then
      SOCKET="${candidate}"
      break
    fi
  done
fi

[ -n "${SOCKET}" ] || {
  echo "error: no running VM QMP socket; start ./scripts/vm-run.sh --gui" >&2
  exit 1
}
[ -S "${SOCKET}" ] || {
  echo "error: QMP socket ${SOCKET} does not exist" >&2
  exit 1
}

args=("$@")
if [ "${1:-}" = "--text" ]; then
  shift
  text="${1:-}"
  shift || true
  keys=()
  for (( i=0; i<${#text}; i++ )); do
    character="${text:i:1}"
    case "${character}" in
      ' ') keys+=("spc") ;;
      '.') keys+=("dot") ;;
      '/') keys+=("slash") ;;
      '-') keys+=("minus") ;;
      ':') keys+=("shift-semicolon") ;;
      *) keys+=("${character}") ;;
    esac
  done
  args=("${keys[@]}" "$@")
fi

[ "${#args[@]}" -gt 0 ] || {
  echo "usage: $0 [--text STRING] <qemu-key> [qemu-key...]" >&2
  exit 1
}

python3 - "${SOCKET}" "${args[@]}" <<'PY'
import json
import socket
import sys
import time

path, *keys = sys.argv[1:]
sock = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
sock.connect(path)
stream = sock.makefile("rwb")
stream.readline()


def command(payload):
    stream.write((json.dumps(payload) + "\n").encode())
    stream.flush()
    return json.loads(stream.readline().decode())


command({"execute": "qmp_capabilities"})
for key in keys:
    result = command(
        {
            "execute": "human-monitor-command",
            "arguments": {"command-line": f"sendkey {key}"},
        }
    )
    if "return" not in result:
        raise SystemExit(f"sendkey {key} failed: {result}")
    time.sleep(0.05)
PY

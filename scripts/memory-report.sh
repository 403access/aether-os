#!/usr/bin/env bash
#
# Idle memory report for the Aether session.
#
# Samples /proc twice: once at start and once after --seconds of *no input*.
# Reports machine-wide use, per-process PSS, DRM/GEM resident memory from
# fdinfo, and the growth during the idle window. The growth number is the
# cache-retention signal: a bounded shell should be flat, an unbounded cache
# keeps climbing after the user stops typing.
#
#   ./scripts/memory-report.sh                     # 60 s idle window
#   ./scripts/memory-report.sh --seconds 600       # long idle run
#   ./scripts/memory-report.sh --check             # gate: budget + growth
#   ./scripts/memory-report.sh --gate attributed   # session commit only
#   ./scripts/memory-report.sh --json artifacts/perf/memory/idle.json
#
# Linux only. Run as the session user (oliver) from the DRM session or over
# the console; PSS for other users is not readable without privileges.
#
# Budget semantics (see architecture/memory-footprint.md):
#   machine     MemTotal - MemAvailable            the literal idle footprint
#   attributed  Σ PSS + resident DRM of the session processes only
# The machine number includes the kernel and system services; the attributed
# number is what the shell and its own daemons actually commit.
#
# Exit status: 0 pass, 1 gate failure, 2 usage/environment error.

set -uo pipefail

ROOT="/"
WINDOW=60
BUDGET_MB=200
GROWTH_LIMIT_MB=1
GATE="machine"
JSON_PATH=""
CHECK=0
TOP=8
SESSION_UID="$(id -u 2>/dev/null || echo 0)"

usage() {
  sed -n '2,30p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
}

while [ $# -gt 0 ]; do
  case "$1" in
    --root)
      ROOT="${2:?--root needs a path}"
      ROOT="${ROOT%/}"
      [ -n "$ROOT" ] || ROOT="/"
      shift 2
      ;;
    --seconds)
      WINDOW="${2:?--seconds needs a number}"
      shift 2
      ;;
    --budget-mb)
      BUDGET_MB="${2:?--budget-mb needs a number}"
      shift 2
      ;;
    --growth-limit-mb)
      GROWTH_LIMIT_MB="${2:?--growth-limit-mb needs a number}"
      shift 2
      ;;
    --gate)
      GATE="${2:?--gate needs machine or attributed}"
      shift 2
      ;;
    --uid)
      SESSION_UID="${2:?--uid needs a number}"
      shift 2
      ;;
    --json)
      JSON_PATH="${2:?--json needs a path}"
      shift 2
      ;;
    --top)
      TOP="${2:?--top needs a number}"
      shift 2
      ;;
    --check)
      CHECK=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument: $1" >&2
      exit 2
      ;;
  esac
done

case "$GATE" in
  machine | attributed) ;;
  *)
    echo "error: --gate must be machine or attributed" >&2
    exit 2
    ;;
esac

case "$WINDOW" in
  '' | *[!0-9]*)
    echo "error: --seconds must be a whole number" >&2
    exit 2
    ;;
esac

if ! command -v awk >/dev/null 2>&1; then
  echo "error: awk is required" >&2
  exit 2
fi

if [ "$ROOT" = "/" ]; then
  PROC="/proc"
  SYS="/sys"
else
  PROC="$ROOT/proc"
  SYS="$ROOT/sys"
fi

if [ ! -d "$PROC" ]; then
  echo "error: $PROC not found; this harness is Linux-only (use --root for fixtures)" >&2
  exit 2
fi

WORK="$(mktemp -d "${TMPDIR:-/tmp}/aether-memory.XXXXXX")"
trap 'rm -rf "$WORK"' EXIT

# --- helpers ---------------------------------------------------------------

mb() {
  awk -v kb="${1:-0}" 'BEGIN { printf "%.1f", kb / 1024 }'
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

sample_machine() {
  local meminfo="$PROC/meminfo"
  if [ ! -r "$meminfo" ]; then
    printf '0\t0\t0\t0\t0\t0\n'
    return
  fi
  awk '
    /^MemTotal:/ { total = $2 }
    /^MemAvailable:/ { available = $2 }
    /^Shmem:/ { shmem = $2 }
    /^SReclaimable:/ { sreclaim = $2 }
    /^SwapTotal:/ { swap_total = $2 }
    /^SwapFree:/ { swap_free = $2 }
    END {
      printf "%d\t%d\t%d\t%d\t%d\t%d\n", total, available, shmem, sreclaim, swap_total, swap_free
    }
  ' "$meminfo"
}

process_memory() {
  local pid_dir="$1"
  local pss="" rss="" swap=""
  if [ -r "$pid_dir/smaps_rollup" ]; then
    while read -r key value _unit; do
      case "$key" in
        Pss:) pss="$value" ;;
        Rss:) rss="$value" ;;
        Swap:) swap="$value" ;;
      esac
    done <"$pid_dir/smaps_rollup"
  fi
  if [ -z "$rss" ] && [ -r "$pid_dir/status" ]; then
    rss="$(awk '/^VmRSS:/ { print $2; exit }' "$pid_dir/status")"
    pss="${pss:-$rss}"
    swap="$(awk '/^VmSwap:/ { print $2; exit }' "$pid_dir/status")"
  fi
  printf '%s\t%s\t%s\n' "${pss:-0}" "${rss:-0}" "${swap:-0}"
}

process_drm() {
  local pid_dir="$1"
  local files=("$pid_dir"/fdinfo/*)
  if [ ! -e "${files[0]}" ]; then
    printf '0'
    return
  fi
  awk '
    /^drm-resident-system:/ { sum += $2 }
    /^drm-resident-vram[0-9]*:/ { sum += $2 }
    /^drm-memory-vram:/ { sum += $2 }
    /^drm-memory-gtt:/ { sum += $2 }
    /^drm-memory-system:/ { sum += $2 }
    END { printf "%d", sum + 0 }
  ' "${files[@]}" 2>/dev/null
}

process_uid() {
  local pid_dir="$1"
  local uid
  uid="$(awk '/^Uid:/ { print $2; exit }' "$pid_dir/status" 2>/dev/null)"
  printf '%s' "${uid:-0}"
}

categorize() {
  local name="$1" uid="$2"
  case "$name" in
    aether-shell | aether-shell-drm | aether-shell-smithay | aether-session | aether-shell-preview)
      printf 'shell'
      ;;
    aether-index | aether-index-service)
      printf 'index'
      ;;
    pipewire | pipewire-pulse | wireplumber)
      printf 'audio'
      ;;
    fcitx5 | fcitx5-* | ibus-daemon)
      printf 'ime'
      ;;
    *)
      if [ "$uid" = "$SESSION_UID" ]; then
        printf 'session'
      elif [ "$uid" = "0" ]; then
        printf 'system'
      else
        printf 'other'
      fi
      ;;
  esac
}

sample_processes() {
  local out="$1"
  local pid_dir pid name uid pss rss swap drm category
  : >"$out"
  for pid_dir in "$PROC"/[0-9]*; do
    [ -d "$pid_dir" ] || continue
    pid="${pid_dir##*/}"
    name="$(tr -d '\n' <"$pid_dir/comm" 2>/dev/null)"
    if [ -z "$name" ]; then
      name="$(tr '\0' ' ' <"$pid_dir/cmdline" 2>/dev/null | awk '{ print $1 }')"
      name="${name##*/}"
    fi
    [ -n "$name" ] || name="pid-$pid"
    uid="$(process_uid "$pid_dir")"
    category="$(categorize "$name" "$uid")"
    IFS=$'\t' read -r pss rss swap < <(process_memory "$pid_dir")
    drm="$(process_drm "$pid_dir")"
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' \
      "$pid" "$category" "$name" "${pss:-0}" "${rss:-0}" "${swap:-0}" "${drm:-0}" "$uid" >>"$out"
  done
}

# --- sampling --------------------------------------------------------------

read -r M_TOTAL M_AVAIL M_SHMEM M_SRECLAIM M_SWAP_TOTAL M_SWAP_FREE < <(sample_machine)
sample_processes "$WORK/first.tsv"
FIRST_MACHINE_USED=$((M_TOTAL - M_AVAIL))

SECOND_MACHINE_USED=""
if [ "$WINDOW" -gt 0 ]; then
  sleep "$WINDOW"
  read -r M_TOTAL2 M_AVAIL2 M_SHMEM2 M_SRECLAIM2 M_SWAP_TOTAL2 M_SWAP_FREE2 < <(sample_machine)
  sample_processes "$WORK/second.tsv"
  SECOND_MACHINE_USED=$((M_TOTAL2 - M_AVAIL2))
fi

# --- aggregation -----------------------------------------------------------

CATEGORIES="shell index audio ime session system other"

aggregate() {
  local input="$1" output="$2"
  local category
  : >"$output"
  for category in $CATEGORIES; do
    awk -F'\t' -v cat="$category" '
      $2 == cat { n++; pss += $4; rss += $5; swap += $6; drm += $7 }
      END { printf "%s\t%d\t%d\t%d\t%d\t%d\n", cat, n + 0, pss + 0, rss + 0, swap + 0, drm + 0 }
    ' "$input" >>"$output"
  done
}

aggregate "$WORK/first.tsv" "$WORK/first-categories.tsv"

TOTAL_PSS="$(awk -F'\t' '{ s += $4 } END { print s + 0 }' "$WORK/first.tsv")"
TOTAL_DRM="$(awk -F'\t' '{ s += $7 } END { print s + 0 }' "$WORK/first.tsv")"
TOTAL_SWAP="$(awk -F'\t' '{ s += $6 } END { print s + 0 }' "$WORK/first.tsv")"

SESSION_PSS="$(awk -F'\t' '$2 == "shell" || $2 == "index" || $2 == "audio" || $2 == "ime" || $2 == "session" { s += $4 } END { print s + 0 }' "$WORK/first.tsv")"
SESSION_DRM="$(awk -F'\t' '$2 == "shell" || $2 == "index" || $2 == "audio" || $2 == "ime" || $2 == "session" { s += $7 } END { print s + 0 }' "$WORK/first.tsv")"
SESSION_SWAP="$(awk -F'\t' '$2 == "shell" || $2 == "index" || $2 == "audio" || $2 == "ime" || $2 == "session" { s += $6 } END { print s + 0 }' "$WORK/first.tsv")"
ATTRIBUTED_KB=$((SESSION_PSS + SESSION_DRM))

GROWTH_PSS=""
GROWTH_MACHINE=""
if [ "$WINDOW" -gt 0 ]; then
  SECOND_ATTRIBUTED="$(awk -F'\t' '
    $2 == "shell" || $2 == "index" || $2 == "audio" || $2 == "ime" || $2 == "session" {
      s += $4 + $7
    }
    END { print s + 0 }
  ' "$WORK/second.tsv")"
  GROWTH_PSS=$((SECOND_ATTRIBUTED - ATTRIBUTED_KB))
  GROWTH_MACHINE=$((SECOND_MACHINE_USED - FIRST_MACHINE_USED))
fi

# --- budget verdict --------------------------------------------------------

GATE_KB=0
GATE_LABEL=""
if [ "$GATE" = "attributed" ]; then
  GATE_KB="$ATTRIBUTED_KB"
  GATE_LABEL="attributed (session PSS + DRM resident)"
else
  GATE_KB="$FIRST_MACHINE_USED"
  GATE_LABEL="machine (MemTotal - MemAvailable)"
fi
BUDGET_KB=$((BUDGET_MB * 1024))
GROWTH_LIMIT_KB=$((GROWTH_LIMIT_MB * 1024))

STATUS=0
REASONS=""
if [ "$GATE_KB" -gt "$BUDGET_KB" ]; then
  STATUS=1
  REASONS="over budget: $(mb "$GATE_KB") MiB > ${BUDGET_MB} MiB"
fi
if [ "$WINDOW" -gt 0 ] && [ "$GROWTH_PSS" -gt "$GROWTH_LIMIT_KB" ]; then
  STATUS=1
  REASONS="${REASONS:+$REASONS; }idle growth: $(mb "$GROWTH_PSS") MiB > ${GROWTH_LIMIT_MB} MiB"
fi

HOST="$(
  if [ -r "$ROOT/etc/hostname" ]; then
    tr -d '\n' <"$ROOT/etc/hostname"
  else
    hostname 2>/dev/null || echo unknown
  fi
)"
GENERATED="$(date -u +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || echo unknown)"

# --- human report ----------------------------------------------------------

echo "Aether memory report · $GENERATED · $HOST"
echo
printf 'machine   total %s MiB · used %s MiB · available %s MiB\n' \
  "$(mb "$M_TOTAL")" "$(mb "$FIRST_MACHINE_USED")" "$(mb "$M_AVAIL")"
printf '          shmem %s MiB · sreclaimable %s MiB · swap %s/%s MiB\n' \
  "$(mb "$M_SHMEM")" "$(mb "$M_SRECLAIM")" \
  "$(mb "$((M_SWAP_TOTAL - M_SWAP_FREE))")" "$(mb "$M_SWAP_TOTAL")"
echo
printf '%-10s %6s %10s %10s %9s %9s\n' "category" "procs" "pss MiB" "rss MiB" "swap MiB" "drm MiB"
printf '%-10s %6s %10s %10s %9s %9s\n' "--------" "-----" "-------" "-------" "--------" "-------"
while IFS=$'\t' read -r category count pss rss swap drm; do
  [ "$count" -eq 0 ] && continue
  printf '%-10s %6s %10s %10s %9s %9s\n' \
    "$category" "$count" "$(mb "$pss")" "$(mb "$rss")" "$(mb "$swap")" "$(mb "$drm")"
done <"$WORK/first-categories.tsv"
echo
printf '%-10s %6s %10s %10s %9s %9s\n' \
  "attributed" "-" "$(mb "$SESSION_PSS")" "-" "$(mb "$SESSION_SWAP")" "$(mb "$SESSION_DRM")"
echo "  attributed = shell + index + audio + ime + session processes (PSS + resident DRM)"
echo "  machine    = MemTotal - MemAvailable, including kernel and system services"
echo

echo "top processes by PSS"
printf '  %7s %6s %9s  %-10s %s\n' "pid" "uid" "pss MiB" "category" "name"
sort -t$'\t' -k4,4 -nr "$WORK/first.tsv" | head -n "$TOP" | while IFS=$'\t' read -r pid category name pss _rss _swap _drm uid; do
  printf '  %7s %6s %9s  %-10s %s\n' "$pid" "$uid" "$(mb "$pss")" "$category" "$name"
done
echo

if [ "$WINDOW" -gt 0 ]; then
  echo "idle window: ${WINDOW}s, no input"
  printf '  attributed   %+0.1f MiB  (%+0.0f MiB/h)\n' \
    "$(awk -v kb="$GROWTH_PSS" 'BEGIN { print kb / 1024 }')" \
    "$(awk -v kb="$GROWTH_PSS" -v s="$WINDOW" 'BEGIN { print (kb / 1024) * 3600 / s }')"
  printf '  machine used %+0.1f MiB  (%+0.0f MiB/h)\n' \
    "$(awk -v kb="$GROWTH_MACHINE" 'BEGIN { print kb / 1024 }')" \
    "$(awk -v kb="$GROWTH_MACHINE" -v s="$WINDOW" 'BEGIN { print (kb / 1024) * 3600 / s }')"
  echo "  growth is the cache-retention signal; idle should be flat (allow non-zero kernel noise)"
  echo
fi

echo "gate: $GATE_LABEL · budget ${BUDGET_MB} MiB"
if [ "$WINDOW" -gt 0 ]; then
  echo "      growth limit ${GROWTH_LIMIT_MB} MiB/${WINDOW}s"
fi
if [ "$STATUS" -eq 0 ]; then
  echo "RESULT: pass ($(mb "$GATE_KB") MiB of ${BUDGET_MB} MiB)"
else
  echo "RESULT: fail ($REASONS)"
fi

# --- JSON ------------------------------------------------------------------

if [ -n "$JSON_PATH" ]; then
  mkdir -p "$(dirname "$JSON_PATH")" 2>/dev/null || true
  {
    printf '{\n'
    printf '  "generated_at": "%s",\n' "$GENERATED"
    printf '  "host": "%s",\n' "$(json_escape "$HOST")"
    printf '  "gate": "%s",\n' "$GATE"
    printf '  "budget_mb": %s,\n' "$BUDGET_MB"
    printf '  "growth_limit_mb": %s,\n' "$GROWTH_LIMIT_MB"
    printf '  "window_seconds": %s,\n' "$WINDOW"
    printf '  "machine": { "total_mb": %s, "available_mb": %s, "used_mb": %s, "shmem_mb": %s, "sreclaimable_mb": %s, "swap_total_mb": %s },\n' \
      "$(mb "$M_TOTAL")" "$(mb "$M_AVAIL")" "$(mb "$FIRST_MACHINE_USED")" "$(mb "$M_SHMEM")" "$(mb "$M_SRECLAIM")" "$(mb "$M_SWAP_TOTAL")"
    printf '  "attributed": { "pss_mb": %s, "drm_mb": %s, "swap_mb": %s },\n' \
      "$(mb "$SESSION_PSS")" "$(mb "$SESSION_DRM")" "$(mb "$SESSION_SWAP")"
    printf '  "categories": [\n'
    first=1
    while IFS=$'\t' read -r category count pss rss swap drm; do
      [ "$first" -eq 0 ] && printf ',\n'
      first=0
      printf '    { "name": "%s", "processes": %s, "pss_mb": %s, "rss_mb": %s, "swap_mb": %s, "drm_mb": %s }' \
        "$category" "$count" "$(mb "$pss")" "$(mb "$rss")" "$(mb "$swap")" "$(mb "$drm")"
    done <"$WORK/first-categories.tsv"
    printf '\n  ],\n'
    printf '  "top": [\n'
    first=1
    sort -t$'\t' -k4,4 -nr "$WORK/first.tsv" | head -n "$TOP" | while IFS=$'\t' read -r pid category name pss rss swap drm uid; do
      [ "$first" -eq 0 ] && printf ',\n'
      first=0
      printf '    { "pid": %s, "name": "%s", "category": "%s", "pss_mb": %s, "rss_mb": %s, "swap_mb": %s, "drm_mb": %s }' \
        "$pid" "$(json_escape "$name")" "$category" "$(mb "$pss")" "$(mb "$rss")" "$(mb "$swap")" "$(mb "$drm")"
    done
    printf '\n  ],\n'
    if [ "$WINDOW" -gt 0 ]; then
      printf '  "growth": { "attributed_mb": %s, "machine_used_mb": %s, "attributed_mb_per_hour": %s },\n' \
        "$(awk -v kb="$GROWTH_PSS" 'BEGIN { printf "%.1f", kb / 1024 }')" \
        "$(awk -v kb="$GROWTH_MACHINE" 'BEGIN { printf "%.1f", kb / 1024 }')" \
        "$(awk -v kb="$GROWTH_PSS" -v s="$WINDOW" 'BEGIN { printf "%.0f", (kb / 1024) * 3600 / s }')"
    else
      printf '  "growth": null,\n'
    fi
    if [ "$STATUS" -eq 0 ]; then
      printf '  "pass": true\n'
    else
      printf '  "pass": false\n'
    fi
    printf '}\n'
  } >"$JSON_PATH"
  echo "json: $JSON_PATH"
fi

if [ "$CHECK" -eq 1 ] && [ "$STATUS" -ne 0 ]; then
  exit 1
fi
exit 0

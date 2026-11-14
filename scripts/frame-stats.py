#!/usr/bin/env python3
"""Summarise AETHER_FRAME_STATS NDJSON (present/vblank records).

Usage: scripts/frame-stats.py artifacts/perf/p2/frames.ndjson [--json]
Exit status is non-zero when an output queued more than one frame.
"""
import json
import sys


def pct(values, p):
    if not values:
        return 0
    values = sorted(values)
    index = max(0, min(len(values) - 1, (len(values) - 1) * p // 100))
    return values[index]


def main():
    if len(sys.argv) < 2:
        print(__doc__, file=sys.stderr)
        return 2
    records = []
    with open(sys.argv[1], encoding="utf-8") as handle:
        for line in handle:
            line = line.strip()
            if line:
                records.append(json.loads(line))

    presents = [r for r in records if r.get("kind") == "present"]
    intervals = [r["interval_ns"] for r in presents if r.get("interval_ns", 0) > 0]
    input_latency = [r["input_to_damage_ns"] for r in presents if r.get("input_to_damage_ns")]
    queue = max((r.get("max_queue_depth", 0) for r in presents), default=0)
    queued = max((r.get("queue_depth", 0) for r in presents), default=0)
    missed = max((r.get("missed", 0) for r in presents), default=0)
    damage = [r["damage_area"] for r in presents if r.get("damage_area")]
    summary = {
        "frames": len(presents),
        "interval_ns": {
            "p50": pct(intervals, 50),
            "p95": pct(intervals, 95),
            "p99": pct(intervals, 99),
        },
        "input_to_damage_ns": {
            "p50": pct(input_latency, 50),
            "p95": pct(input_latency, 95),
            "p99": pct(input_latency, 99),
            "max": max(input_latency, default=0),
        },
        "queue_depth_max": queued,
        "max_queue_depth": queue,
        "missed_deadlines_max": missed,
        "damage_area": {
            "p50": pct(damage, 50),
            "p95": pct(damage, 95),
            "p99": pct(damage, 99),
            "max": max(damage, default=0),
        },
    }
    if "--json" in sys.argv:
        print(json.dumps(summary, indent=2))
    else:
        print(f"frames: {summary['frames']}")
        print(
            "present interval ns: "
            f"p50={summary['interval_ns']['p50']} "
            f"p95={summary['interval_ns']['p95']} "
            f"p99={summary['interval_ns']['p99']}"
        )
        print(
            "input -> damage ns: "
            f"p50={summary['input_to_damage_ns']['p50']} "
            f"p95={summary['input_to_damage_ns']['p95']} "
            f"p99={summary['input_to_damage_ns']['p99']} "
            f"max={summary['input_to_damage_ns']['max']}"
        )
        print(
            f"queue depth: {summary['queue_depth_max']} "
            f"(max {summary['max_queue_depth']}), "
            f"missed deadlines max {summary['missed_deadlines_max']}"
        )
        print(
            "damage area px: "
            f"p50={summary['damage_area']['p50']} "
            f"p95={summary['damage_area']['p95']} "
            f"p99={summary['damage_area']['p99']} "
            f"max={summary['damage_area']['max']}"
        )
    return 0 if queued <= 1 and queue <= 1 else 1


if __name__ == "__main__":
    sys.exit(main())

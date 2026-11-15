export class LatencyLedger {
  readonly samples: number[] = [];

  push(ms: number): void {
    if (!Number.isFinite(ms) || ms < 0) return;
    this.samples.push(ms);
  }

  get length(): number {
    return this.samples.length;
  }

  percentile(percentile: number): number | null {
    if (this.samples.length === 0) return null;
    const sorted = [...this.samples].sort((a, b) => a - b);
    const clamped = Math.max(0, Math.min(1, percentile));
    const rank = Math.max(1, Math.ceil(clamped * sorted.length));
    return sorted[rank - 1] ?? null;
  }

  p50(): number | null {
    return this.percentile(0.5);
  }

  p99(): number | null {
    return this.percentile(0.99);
  }

  max(): number | null {
    return this.samples.length === 0 ? null : Math.max(...this.samples);
  }

  mean(): number | null {
    if (this.samples.length === 0) return null;
    return this.samples.reduce((total, value) => total + value, 0) / this.samples.length;
  }
}

export interface ClientStats {
  frames: number;
  dropped: number;
  bytes: number;
  lastSequence: number;
  latencyP50Ms: number | null;
  latencyP99Ms: number | null;
  intervalP50Ms: number | null;
  intervalP99Ms: number | null;
  targetIntervalMs: number;
  intervalBudgetOk: boolean;
  bitrateKbps: number;
  tier: string;
  degraded: boolean;
}

export class ClientStatsTracker {
  readonly latency = new LatencyLedger();
  readonly intervals = new LatencyLedger();
  readonly targetFps: number;
  frames = 0;
  dropped = 0;
  bytes = 0;
  lastSequence = -1;
  #lastPtsMs: number | null = null;
  #startedAt = 0;
  #now: () => number;

  constructor(targetFps = 60, now: () => number = () => performance.now()) {
    this.targetFps = Math.max(1, targetFps);
    this.#now = now;
  }

  get targetIntervalMs(): number {
    return 1000 / this.targetFps;
  }

  onFrame(sequence: number, ptsMs: number, bytes: number): void {
    if (this.frames === 0) this.#startedAt = this.#now();
    if (this.lastSequence >= 0) {
      const expected = this.lastSequence + 1;
      if (sequence > expected) this.dropped += sequence - expected;
    }
    this.lastSequence = sequence;
    this.frames += 1;
    this.bytes += bytes;
    if (this.#lastPtsMs !== null) {
      this.intervals.push(Math.max(0, ptsMs - this.#lastPtsMs));
    }
    this.#lastPtsMs = ptsMs;
  }

  onRoundTrip(ms: number): void {
    this.latency.push(ms);
  }

  get intervalBudgetOk(): boolean {
    const p99 = this.intervals.p99();
    return p99 !== null && p99 <= this.targetIntervalMs * 1.5;
  }

  get bitrateKbps(): number {
    const elapsed = (this.#now() - this.#startedAt) / 1000;
    if (elapsed <= 0) return 0;
    return Math.round((this.bytes * 8) / elapsed / 1000);
  }

  snapshot(tier: string, degraded: boolean): ClientStats {
    return {
      frames: this.frames,
      dropped: this.dropped,
      bytes: this.bytes,
      lastSequence: this.lastSequence,
      latencyP50Ms: this.latency.p50(),
      latencyP99Ms: this.latency.p99(),
      intervalP50Ms: this.intervals.p50(),
      intervalP99Ms: this.intervals.p99(),
      targetIntervalMs: this.targetIntervalMs,
      intervalBudgetOk: this.intervalBudgetOk,
      bitrateKbps: this.bitrateKbps,
      tier,
      degraded,
    };
  }
}

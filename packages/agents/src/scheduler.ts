import type { AgentSpec } from "@aether/protocol";

/**
 * Admission control for the i7-7700K: four physical cores, eight threads.
 *
 * The budget is expressed in "CPU points", where 100 points = one full core.
 * One core (or a configurable reserve) is kept out of the agent pool so the
 * compositor, indexer and audio path stay responsive under load. The practical
 * agent ceiling is six concurrent workers, not the forty a 16-core machine
 * could sustain.
 */

export interface SchedulerBudget {
  cores: number;
  memoryMiB: number;
  /** Cores kept for the shell, indexer and audio. Default 1. */
  reserveCores?: number;
  /** Concurrent agent ceiling. Default floor(cores * 1.5). */
  maxAgents?: number;
}

export interface Capacity {
  cpuPoints: number;
  memoryMiB: number;
  maxAgents: number;
}

export interface Usage {
  agents: number;
  cpuPoints: number;
  memoryMiB: number;
  cpuPercent: number;
  memoryPercent: number;
}

export type AdmissionFailure = "cpu" | "memory" | "agents" | "duplicate" | "oversized";

export type AdmissionResult = { ok: true } | { ok: false; reason: AdmissionFailure };

export class AdmissionController {
  readonly #memoryMiB: number;
  readonly #maxAgents: number;
  readonly #cpuPoints: number;
  readonly #running = new Map<string, AgentSpec>();
  #cpuUsed = 0;
  #memoryUsed = 0;

  constructor(budget: SchedulerBudget) {
    if (!Number.isFinite(budget.cores) || budget.cores < 1) {
      throw new RangeError("cores must be >= 1");
    }
    if (!Number.isFinite(budget.memoryMiB) || budget.memoryMiB < 64) {
      throw new RangeError("memoryMiB must be >= 64");
    }
    const reserve = budget.reserveCores ?? 1;
    const usableCores = Math.max(0.5, budget.cores - reserve);
    this.#cpuPoints = Math.floor(usableCores * 100);
    this.#memoryMiB = budget.memoryMiB;
    this.#maxAgents =
      budget.maxAgents ?? Math.max(1, Math.floor(budget.cores * 1.5));
  }

  get capacity(): Capacity {
    return {
      cpuPoints: this.#cpuPoints,
      memoryMiB: this.#memoryMiB,
      maxAgents: this.#maxAgents,
    };
  }

  has(id: string): boolean {
    return this.#running.has(id);
  }

  tryAdmit(spec: AgentSpec): AdmissionResult {
    if (this.#running.has(spec.id)) return { ok: false, reason: "duplicate" };
    if (spec.memoryMiB > this.#memoryMiB || spec.cpuWeight > this.#cpuPoints) {
      return { ok: false, reason: "oversized" };
    }
    if (this.#running.size >= this.#maxAgents) return { ok: false, reason: "agents" };
    if (this.#cpuUsed + spec.cpuWeight > this.#cpuPoints) return { ok: false, reason: "cpu" };
    if (this.#memoryUsed + spec.memoryMiB > this.#memoryMiB) {
      return { ok: false, reason: "memory" };
    }

    this.#running.set(spec.id, spec);
    this.#cpuUsed += spec.cpuWeight;
    this.#memoryUsed += spec.memoryMiB;
    return { ok: true };
  }

  release(id: string): boolean {
    const spec = this.#running.get(id);
    if (spec === undefined) return false;
    this.#running.delete(id);
    this.#cpuUsed -= spec.cpuWeight;
    this.#memoryUsed -= spec.memoryMiB;
    return true;
  }

  usage(): Usage {
    return {
      agents: this.#running.size,
      cpuPoints: this.#cpuUsed,
      memoryMiB: this.#memoryUsed,
      cpuPercent: this.#cpuPoints === 0 ? 0 : (this.#cpuUsed / this.#cpuPoints) * 100,
      memoryPercent: this.#memoryMiB === 0 ? 0 : (this.#memoryUsed / this.#memoryMiB) * 100,
    };
  }
}

export interface Lease {
  id: string;
  spec: AgentSpec;
  priority: number;
  admittedAt: number;
}

export type SubmitResult =
  | { status: "running"; lease: Lease }
  | { status: "queued"; position: number }
  | { status: "rejected"; reason: AdmissionFailure };

export interface SchedulerStats extends Usage {
  queued: number;
  capacity: Capacity;
}

interface Queued {
  spec: AgentSpec;
  priority: number;
  sequence: number;
}

export interface SchedulerOptions {
  now?: () => number;
}

/**
 * Priority admission with head-of-line avoidance: when a slot frees up, the
 * scheduler scans the priority-ordered queue for the first agent that fits
 * rather than blocking on an oversized head entry.
 */
export class AgentScheduler {
  readonly #admission: AdmissionController;
  readonly #queue: Queued[] = [];
  readonly #leases = new Map<string, Lease>();
  readonly #now: () => number;
  #sequence = 0;

  constructor(budget: SchedulerBudget, options: SchedulerOptions = {}) {
    this.#admission = new AdmissionController(budget);
    this.#now = options.now ?? (() => Date.now());
  }

  submit(spec: AgentSpec, priority = 0): SubmitResult {
    if (this.#leases.has(spec.id) || this.#queue.some((entry) => entry.spec.id === spec.id)) {
      return { status: "rejected", reason: "duplicate" };
    }

    const admission = this.#admission.tryAdmit(spec);
    if (admission.ok) {
      return { status: "running", lease: this.#lease(spec, priority) };
    }
    if (admission.reason === "oversized") {
      return { status: "rejected", reason: admission.reason };
    }

    this.#sequence += 1;
    this.#queue.push({ spec, priority, sequence: this.#sequence });
    this.#sortQueue();
    return { status: "queued", position: this.#queue.findIndex((entry) => entry.spec.id === spec.id) };
  }

  /** Release an agent and admit the next queued agent that fits. */
  complete(id: string): { released: boolean; admittedNext?: Lease } {
    const lease = this.#leases.get(id);
    if (lease === undefined) return { released: false };

    this.#leases.delete(id);
    this.#admission.release(id);

    for (let index = 0; index < this.#queue.length; index += 1) {
      const candidate = this.#queue[index];
      if (candidate === undefined) continue;
      const admission = this.#admission.tryAdmit(candidate.spec);
      if (!admission.ok) continue;
      this.#queue.splice(index, 1);
      const admitted = this.#lease(candidate.spec, candidate.priority);
      return { released: true, admittedNext: admitted };
    }

    return { released: true };
  }

  running(): Lease[] {
    return [...this.#leases.values()];
  }

  queued(): AgentSpec[] {
    return this.#queue.map((entry) => entry.spec);
  }

  get(id: string): Lease | undefined {
    return this.#leases.get(id);
  }

  stats(): SchedulerStats {
    return {
      ...this.#admission.usage(),
      queued: this.#queue.length,
      capacity: this.#admission.capacity,
    };
  }

  #lease(spec: AgentSpec, priority: number): Lease {
    const lease: Lease = {
      id: spec.id,
      spec,
      priority,
      admittedAt: this.#now(),
    };
    this.#leases.set(spec.id, lease);
    return lease;
  }

  #sortQueue(): void {
    this.#queue.sort(
      (a, b) => b.priority - a.priority || a.sequence - b.sequence,
    );
  }
}

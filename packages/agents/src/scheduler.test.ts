import { describe, expect, test } from "bun:test";
import type { AgentSpec } from "@aether/protocol";
import { AdmissionController, AgentScheduler } from "../src/index";

function spec(id: string, overrides: Partial<AgentSpec> = {}): AgentSpec {
  return {
    id,
    name: id,
    trust: "untrusted",
    tier: "process-landlock",
    cpuWeight: 100,
    memoryMiB: 512,
    capabilities: [],
    timeoutMs: 60_000,
    ...overrides,
  };
}

const BUDGET = { cores: 4, memoryMiB: 2_048, reserveCores: 1, maxAgents: 6 };

describe("AdmissionController", () => {
  test("reserves one core and exposes capacity", () => {
    const controller = new AdmissionController(BUDGET);
    expect(controller.capacity).toEqual({ cpuPoints: 300, memoryMiB: 2_048, maxAgents: 6 });
  });

  test("admits up to the CPU budget, then rejects", () => {
    const controller = new AdmissionController(BUDGET);
    expect(controller.tryAdmit(spec("a"))).toEqual({ ok: true });
    expect(controller.tryAdmit(spec("b"))).toEqual({ ok: true });
    expect(controller.tryAdmit(spec("c"))).toEqual({ ok: true });
    expect(controller.tryAdmit(spec("d"))).toEqual({ ok: false, reason: "cpu" });
    expect(controller.usage().cpuPoints).toBe(300);
    expect(controller.usage().cpuPercent).toBe(100);
  });

  test("enforces memory and agent ceilings independently", () => {
    const memoryBound = new AdmissionController({
      cores: 8,
      memoryMiB: 1_000,
      reserveCores: 1,
      maxAgents: 10,
    });
    expect(memoryBound.tryAdmit(spec("a", { cpuWeight: 1, memoryMiB: 600 })).ok).toBe(true);
    expect(memoryBound.tryAdmit(spec("b", { cpuWeight: 1, memoryMiB: 600 }))).toEqual({
      ok: false,
      reason: "memory",
    });

    const agentBound = new AdmissionController({
      cores: 8,
      memoryMiB: 100_000,
      reserveCores: 1,
      maxAgents: 2,
    });
    expect(agentBound.tryAdmit(spec("a", { cpuWeight: 1, memoryMiB: 16 })).ok).toBe(true);
    expect(agentBound.tryAdmit(spec("b", { cpuWeight: 1, memoryMiB: 16 })).ok).toBe(true);
    expect(agentBound.tryAdmit(spec("c", { cpuWeight: 1, memoryMiB: 16 }))).toEqual({
      ok: false,
      reason: "agents",
    });
  });

  test("rejects a single request larger than the machine", () => {
    const controller = new AdmissionController(BUDGET);
    expect(controller.tryAdmit(spec("huge", { memoryMiB: 4_096 }))).toEqual({
      ok: false,
      reason: "oversized",
    });
    expect(controller.tryAdmit(spec("wide", { cpuWeight: 400 }))).toEqual({
      ok: false,
      reason: "oversized",
    });
  });

  test("releases resources and rejects duplicates", () => {
    const controller = new AdmissionController(BUDGET);
    expect(controller.tryAdmit(spec("a")).ok).toBe(true);
    expect(controller.tryAdmit(spec("a"))).toEqual({ ok: false, reason: "duplicate" });
    expect(controller.release("a")).toBe(true);
    expect(controller.release("a")).toBe(false);
    expect(controller.usage()).toMatchObject({ agents: 0, cpuPoints: 0, memoryMiB: 0 });
  });

  test("validates the budget", () => {
    expect(() => new AdmissionController({ cores: 0, memoryMiB: 1_024 })).toThrow();
    expect(() => new AdmissionController({ cores: 4, memoryMiB: 8 })).toThrow();
  });
});

describe("AgentScheduler", () => {
  test("runs immediately while capacity remains, then queues", () => {
    const scheduler = new AgentScheduler(BUDGET, { now: () => 1_000 });
    expect(scheduler.submit(spec("a")).status).toBe("running");
    expect(scheduler.submit(spec("b")).status).toBe("running");
    expect(scheduler.submit(spec("c")).status).toBe("running");
    const queued = scheduler.submit(spec("d"));
    expect(queued).toEqual({ status: "queued", position: 0 });
    expect(scheduler.queued().map((item) => item.id)).toEqual(["d"]);
    expect(scheduler.stats()).toMatchObject({ agents: 3, queued: 1, cpuPoints: 300 });
  });

  test("admitted leases carry the injected clock", () => {
    const scheduler = new AgentScheduler(BUDGET, { now: () => 42_000 });
    const result = scheduler.submit(spec("a"));
    expect(result.status).toBe("running");
    if (result.status !== "running") return;
    expect(result.lease.admittedAt).toBe(42_000);
  });

  test("completion admits the highest-priority queued agent", () => {
    const scheduler = new AgentScheduler(BUDGET);
    scheduler.submit(spec("a"));
    scheduler.submit(spec("b"));
    scheduler.submit(spec("c"));
    scheduler.submit(spec("low"), 1);
    scheduler.submit(spec("high"), 10);
    scheduler.submit(spec("mid"), 5);

    const result = scheduler.complete("a");
    expect(result.released).toBe(true);
    expect(result.admittedNext?.id).toBe("high");
    expect(scheduler.running().map((lease) => lease.id)).toEqual(["b", "c", "high"]);

    expect(scheduler.complete("b").admittedNext?.id).toBe("mid");
    expect(scheduler.complete("c").admittedNext?.id).toBe("low");
    expect(scheduler.complete("high").admittedNext).toBeUndefined();
  });

  test("head-of-line: skips a queued agent that cannot fit", () => {
    const scheduler = new AgentScheduler({
      cores: 4,
      memoryMiB: 5_000,
      reserveCores: 1,
      maxAgents: 2,
    });
    // Occupy 4,500 of 5,000 MiB and both agent slots.
    expect(scheduler.submit(spec("anchor-1", { cpuWeight: 1, memoryMiB: 3_000 })).status).toBe(
      "running",
    );
    expect(scheduler.submit(spec("anchor-2", { cpuWeight: 1, memoryMiB: 1_500 })).status).toBe(
      "running",
    );
    expect(scheduler.submit(spec("big", { cpuWeight: 1, memoryMiB: 2_500 }), 100).status).toBe(
      "queued",
    );
    expect(scheduler.submit(spec("small", { cpuWeight: 1, memoryMiB: 400 }), 1).status).toBe(
      "queued",
    );

    // Freeing anchor-2 leaves 2,000 MiB: big cannot fit, small can.
    const result = scheduler.complete("anchor-2");
    expect(result.admittedNext?.id).toBe("small");
    expect(scheduler.queued().map((item) => item.id)).toEqual(["big"]);
  });

  test("rejects oversized agents instead of queueing them forever", () => {
    const scheduler = new AgentScheduler(BUDGET);
    expect(scheduler.submit(spec("huge", { memoryMiB: 8_192 }))).toEqual({
      status: "rejected",
      reason: "oversized",
    });
    expect(scheduler.queued()).toEqual([]);
  });

  test("duplicate ids are rejected whether running or queued", () => {
    const scheduler = new AgentScheduler(BUDGET);
    scheduler.submit(spec("a"));
    expect(scheduler.submit(spec("a")).status).toBe("rejected");
    scheduler.submit(spec("b"));
    scheduler.submit(spec("c"));
    scheduler.submit(spec("queued-one"));
    expect(scheduler.submit(spec("queued-one"))).toEqual({
      status: "rejected",
      reason: "duplicate",
    });
  });

  test("completing an unknown agent is a no-op", () => {
    const scheduler = new AgentScheduler(BUDGET);
    expect(scheduler.complete("ghost")).toEqual({ released: false });
  });

  test("stats report capacity and utilisation", () => {
    const scheduler = new AgentScheduler(BUDGET);
    scheduler.submit(spec("a", { cpuWeight: 50, memoryMiB: 512 }));
    const stats = scheduler.stats();
    expect(stats.capacity.cpuPoints).toBe(300);
    expect(stats.cpuPercent).toBeCloseTo(16.67, 1);
    expect(stats.memoryPercent).toBeCloseTo(25, 5);
  });
});
